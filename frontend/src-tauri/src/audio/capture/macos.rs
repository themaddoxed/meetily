use super::backend::{
    CaptureBackend, CaptureCapabilities, CaptureCause, CapturePublishContext, CaptureRequest,
    CaptureRuntimeEvent, CaptureStopped, CaptureTimeline, SourceBlockReceiver, SourceControl,
    SourceSinks, TimestampQuality, TrackBlockKind, TrackChunk, TrackKind, TrackSinks, TrackSpan,
};
use super::core_audio::{CoreAudioCapture, CoreAudioSetupError, CoreAudioStream};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3;
pub const MICROPHONE_RECOVERY_WINDOW: Duration = Duration::from_secs(3);

trait MicrophoneStreamControl {
    fn play_capture(&self) -> Result<(), CaptureCause>;
    fn pause_capture(&self) -> Result<(), CaptureCause>;
}

impl MicrophoneStreamControl for cpal::Stream {
    fn play_capture(&self) -> Result<(), CaptureCause> {
        self.play()
            .map_err(|error| CaptureCause::CaptureFailed(error.to_string()))
    }

    fn pause_capture(&self) -> Result<(), CaptureCause> {
        self.pause()
            .map_err(|error| CaptureCause::CaptureFailed(error.to_string()))
    }
}

struct MicrophoneRecovery {
    previous_device: String,
    attempts: u8,
    deadline: Instant,
    next_attempt: Instant,
}

impl MicrophoneRecovery {
    fn new(previous_device: String, now: Instant) -> Self {
        Self {
            previous_device,
            attempts: 0,
            deadline: now + MICROPHONE_RECOVERY_WINDOW,
            next_attempt: now,
        }
    }

    fn from_fault(cause: CaptureCause, now: Instant) -> Result<Self, CaptureCause> {
        match cause {
            CaptureCause::DeviceUnavailable(previous_device) => Ok(Self::new(previous_device, now)),
            cause => Err(cause),
        }
    }

    fn accepts_device_name(previous_device: &str, device_name: &str) -> bool {
        device_name != previous_device
    }

    fn may_attempt(&self, now: Instant) -> bool {
        self.attempts < MICROPHONE_RECOVERY_MAX_ATTEMPTS
            && now < self.deadline
            && self.next_attempt <= now
    }

    fn record_failed_attempt(&mut self) {
        self.attempts += 1;
        let started_at = self.deadline - MICROPHONE_RECOVERY_WINDOW;
        self.next_attempt = match self.attempts {
            1 => started_at + Duration::from_millis(500),
            2 => started_at + Duration::from_millis(1_500),
            _ => self.deadline,
        };
    }

    fn is_exhausted(&self, now: Instant) -> bool {
        self.attempts >= MICROPHONE_RECOVERY_MAX_ATTEMPTS || now >= self.deadline
    }
}

struct MicrophoneFault {
    generation: u64,
    cause: CaptureCause,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TrackFormat {
    sample_rate: u32,
    channels: u16,
}

pub struct MacosCaptureBackend {
    expected: BTreeSet<TrackKind>,
    paused: Arc<AtomicBool>,
    fault: Arc<Mutex<Option<MicrophoneFault>>>,
    microphone_stream: Option<Box<dyn MicrophoneStreamControl>>,
    microphone_sinks: Option<SourceSinks>,
    microphone_device_name: Option<String>,
    microphone_track_format: Option<TrackFormat>,
    microphone_blocks: Option<SourceBlockReceiver>,
    active_microphone_generation: Option<u64>,
    next_microphone_generation: u64,
    microphone_recovery: Option<MicrophoneRecovery>,
    system_stream: Option<CoreAudioStream>,
    system_sinks: Option<SourceSinks>,
    system_blocks: Option<SourceBlockReceiver>,
    system_start_rx: Option<Receiver<Result<CoreAudioStream, CoreAudioSetupError>>>,
    system_start_worker: Option<thread::JoinHandle<Result<(), CoreAudioSetupError>>>,
    system_stop_worker: Option<thread::JoinHandle<Result<(), CoreAudioSetupError>>>,
    system_has_samples: bool,
    microphone_control: SourceControl,
    system_control: SourceControl,
    pending_start_event: Option<CaptureRuntimeEvent>,
    timeline: Option<Arc<CaptureTimeline>>,
}

impl MacosCaptureBackend {
    pub fn new() -> Self {
        Self {
            expected: BTreeSet::new(),
            paused: Arc::new(AtomicBool::new(false)),
            fault: Arc::new(Mutex::new(None)),
            microphone_stream: None,
            microphone_sinks: None,
            microphone_device_name: None,
            microphone_track_format: None,
            microphone_blocks: None,
            active_microphone_generation: None,
            next_microphone_generation: 1,
            microphone_recovery: None,
            system_stream: None,
            system_sinks: None,
            system_blocks: None,
            system_start_rx: None,
            system_start_worker: None,
            system_stop_worker: None,
            system_has_samples: false,
            microphone_control: SourceControl::default(),
            system_control: SourceControl::default(),
            pending_start_event: None,
            timeline: None,
        }
    }

    fn runtime_event_for_setup_error(error: CoreAudioSetupError) -> CaptureRuntimeEvent {
        let reason = match error {
            CoreAudioSetupError::PermissionDenied(reason) => {
                format!("permission denied: {reason}")
            }
            CoreAudioSetupError::DeviceUnavailable(reason) => {
                format!("device unavailable: {reason}")
            }
            CoreAudioSetupError::CaptureFailed(reason) => format!("capture failed: {reason}"),
        };
        log::warn!("System capture setup degraded to microphone-only: {reason}");
        CaptureRuntimeEvent::SystemAudioDegraded { reason }
    }

    fn join_core_audio_worker(
        worker: thread::JoinHandle<Result<(), CoreAudioSetupError>>,
        name: &str,
    ) -> Result<(), CaptureCause> {
        match worker.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(CaptureCause::CaptureFailed(format!(
                "{name} worker failed: {error}"
            ))),
            Err(_) => Err(CaptureCause::CaptureFailed(format!(
                "{name} worker panicked"
            ))),
        }
    }

    fn finish_system_start_before_stop(&mut self) -> Result<(), CaptureCause> {
        if let Some(receiver) = self.system_start_rx.take() {
            let result = receiver.recv();
            drop(receiver);
            match result {
                Ok(Ok(stream)) => self.system_stream = Some(stream),
                Ok(Err(error)) => {
                    self.system_sinks.take();
                    self.system_blocks.take();
                    self.expected.remove(&TrackKind::System);
                    self.pending_start_event = Some(Self::runtime_event_for_setup_error(error));
                }
                Err(error) => {
                    self.system_sinks.take();
                    self.system_blocks.take();
                    self.expected.remove(&TrackKind::System);
                    self.pending_start_event = Some(Self::runtime_event_for_setup_error(
                        CoreAudioSetupError::CaptureFailed(format!(
                            "system audio startup worker disconnected: {error}"
                        )),
                    ));
                }
            }
        }
        if let Some(worker) = self.system_start_worker.take() {
            Self::join_core_audio_worker(worker, "system startup")?;
        }
        Ok(())
    }

    fn degrade_system_audio(&mut self, reason: String) -> CaptureRuntimeEvent {
        if let Some(mut stream) = self.system_stream.take() {
            stream.signal_termination();
            if let Err(error) = stream.stop_capture() {
                log::warn!("CoreAudio degradation cleanup failed: {error}");
            }
        }
        self.system_sinks.take();
        self.system_blocks.take();
        self.expected.remove(&TrackKind::System);
        log::warn!("System capture stopped; continuing microphone-only: {reason}");
        CaptureRuntimeEvent::SystemAudioDegraded { reason }
    }

    fn record_fault(fault: &Mutex<Option<MicrophoneFault>>, incoming: MicrophoneFault) {
        if let Ok(mut slot) = fault.lock() {
            let should_replace = slot
                .as_ref()
                .map(|current| incoming.generation > current.generation)
                .unwrap_or(true);
            if should_replace {
                *slot = Some(incoming);
            }
        }
    }

    fn take_fault(&self) -> Option<MicrophoneFault> {
        self.fault.lock().ok().and_then(|mut fault| fault.take())
    }

    fn next_microphone_generation(&mut self) -> u64 {
        let generation = self.next_microphone_generation;
        self.next_microphone_generation += 1;
        generation
    }

    fn microphone_stream_error(error: cpal::StreamError, current_device: &str) -> CaptureCause {
        match error {
            cpal::StreamError::DeviceNotAvailable => {
                CaptureCause::DeviceUnavailable(current_device.into())
            }
            error => CaptureCause::CaptureFailed(error.to_string()),
        }
    }

    fn commit_pause_after_source_transition(
        paused: &AtomicBool,
        transition: impl FnOnce() -> Result<(), CaptureCause>,
    ) -> Result<(), CaptureCause> {
        transition()?;
        paused.store(true, Ordering::Release);
        Ok(())
    }

    fn microphone_device(
        name: Option<&str>,
        fallback_to_default: bool,
    ) -> Result<cpal::Device, CaptureCause> {
        let host = cpal::default_host();
        match name {
            Some(name) if !name.is_empty() => {
                let preferred = host
                    .input_devices()
                    .map_err(|error| CaptureCause::DeviceUnavailable(error.to_string()))?
                    .find(|device| device.name().ok().as_deref() == Some(name));
                preferred
                    .or_else(|| {
                        fallback_to_default
                            .then(|| host.default_input_device())
                            .flatten()
                    })
                    .ok_or_else(|| CaptureCause::DeviceUnavailable(name.into()))
            }
            _ => host
                .default_input_device()
                .ok_or_else(|| CaptureCause::DeviceUnavailable("default microphone".into())),
        }
    }

    fn resample_channel(samples: &[f32], from_sample_rate: u32, to_sample_rate: u32) -> Vec<f32> {
        if samples.is_empty() || from_sample_rate == to_sample_rate {
            return samples.to_vec();
        }
        let expected_len = ((samples.len() as u64 * u64::from(to_sample_rate)
            + u64::from(from_sample_rate) / 2)
            / u64::from(from_sample_rate)) as usize;
        let resampled = super::super::audio_processing::resample_audio(
            samples,
            from_sample_rate,
            to_sample_rate,
        );
        if resampled.len() == expected_len {
            return resampled;
        }
        if expected_len <= 1 {
            return vec![samples[0]; expected_len];
        }
        let source_last = (samples.len() - 1) as f32;
        let output_last = (expected_len - 1) as f32;
        (0..expected_len)
            .map(|index| {
                let position = index as f32 * source_last / output_last;
                let lower = position.floor() as usize;
                let upper = (lower + 1).min(samples.len() - 1);
                let fraction = position - lower as f32;
                samples[lower] + (samples[upper] - samples[lower]) * fraction
            })
            .collect()
    }

    fn convert_channels(samples: &[f32], from_channels: u16, to_channels: u16) -> Vec<f32> {
        if from_channels == to_channels {
            return samples.to_vec();
        }
        let mono = super::super::audio_processing::audio_to_mono(samples, from_channels);
        if to_channels == 1 {
            return mono;
        }
        mono.into_iter()
            .flat_map(|sample| std::iter::repeat(sample).take(to_channels as usize))
            .collect()
    }

    fn resample_interleaved(
        samples: &[f32],
        from_sample_rate: u32,
        to_sample_rate: u32,
        channels: u16,
    ) -> Vec<f32> {
        if from_sample_rate == to_sample_rate || samples.is_empty() {
            return samples.to_vec();
        }
        let channel_count = channels as usize;
        if channel_count == 1 {
            return Self::resample_channel(samples, from_sample_rate, to_sample_rate);
        }
        let mut per_channel =
            vec![Vec::with_capacity(samples.len() / channel_count); channel_count];
        for frame in samples.chunks(channel_count) {
            for (channel, sample) in frame.iter().enumerate() {
                per_channel[channel].push(*sample);
            }
        }
        let resampled = per_channel
            .iter()
            .map(|channel| Self::resample_channel(channel, from_sample_rate, to_sample_rate))
            .collect::<Vec<_>>();
        let frame_count = resampled.iter().map(Vec::len).min().unwrap_or(0);
        let mut output = Vec::with_capacity(frame_count * channel_count);
        for frame in 0..frame_count {
            for channel in &resampled {
                output.push(channel[frame]);
            }
        }
        output
    }

    fn normalize_to_format(samples: &[f32], input: TrackFormat, target: TrackFormat) -> Vec<f32> {
        let channel_normalized = Self::convert_channels(samples, input.channels, target.channels);
        Self::resample_interleaved(
            &channel_normalized,
            input.sample_rate,
            target.sample_rate,
            target.channels,
        )
    }

    fn scale_frame(frame: u64, from_sample_rate: u32, to_sample_rate: u32) -> u64 {
        if from_sample_rate == 0 || from_sample_rate == to_sample_rate {
            return frame;
        }
        ((u128::from(frame) * u128::from(to_sample_rate) + u128::from(from_sample_rate / 2))
            / u128::from(from_sample_rate)) as u64
    }

    #[cfg(test)]
    fn send_source(
        sinks: &SourceSinks,
        kind: TrackKind,
        paused: &AtomicBool,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        raw_format: TrackFormat,
    ) -> Result<(), CaptureCause> {
        let frame_count = if channels == 0 {
            0
        } else {
            samples.len() / channels as usize
        };
        Self::send_source_with_span(
            sinks,
            kind,
            paused,
            samples,
            sample_rate,
            channels,
            raw_format,
            TrackSpan::new(0, frame_count.min(u32::MAX as usize) as u32),
            SourceControl::default(),
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn send_source_with_span(
        sinks: &SourceSinks,
        kind: TrackKind,
        paused: &AtomicBool,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        raw_format: TrackFormat,
        span: TrackSpan,
        control: SourceControl,
        allow_paused: bool,
    ) -> Result<(), CaptureCause> {
        if (!allow_paused && paused.load(Ordering::Acquire)) || samples.is_empty() {
            return Ok(());
        }
        let downstream = super::super::audio_processing::audio_to_mono(&samples, channels);
        let mut downstream = if sample_rate == 48_000 {
            downstream
        } else {
            super::super::audio_processing::resample_audio(&downstream, sample_rate, 48_000)
        };
        control.apply_to_downstream(&mut downstream);
        let raw_samples = Self::normalize_to_format(
            &samples,
            TrackFormat {
                sample_rate,
                channels,
            },
            raw_format,
        );
        let raw_frame_count = if raw_format.channels == 0 {
            0
        } else {
            raw_samples.len() / raw_format.channels as usize
        };
        let downstream_frame_count = downstream.len();
        let raw_span = TrackSpan {
            start_frame: Self::scale_frame(span.start_frame, sample_rate, raw_format.sample_rate),
            frame_count: raw_frame_count.min(u32::MAX as usize) as u32,
            ..span
        };
        let downstream_span = TrackSpan {
            start_frame: Self::scale_frame(span.start_frame, sample_rate, 48_000),
            frame_count: downstream_frame_count.min(u32::MAX as usize) as u32,
            ..span
        };
        sinks
            .send_pair(
                TrackChunk::new(
                    kind,
                    raw_samples,
                    raw_format.sample_rate,
                    raw_format.channels,
                    raw_span,
                    TrackBlockKind::Samples,
                ),
                TrackChunk::new(
                    kind,
                    downstream,
                    48_000,
                    1,
                    downstream_span,
                    TrackBlockKind::Samples,
                ),
            )
            .map_err(|error| error.cause)
    }

    fn build_microphone_stream(
        device: &cpal::Device,
        current_device: String,
        generation: u64,
        config: &cpal::SupportedStreamConfig,
        sinks: SourceSinks,
        paused: Arc<AtomicBool>,
        fault: Arc<Mutex<Option<MicrophoneFault>>>,
    ) -> Result<cpal::Stream, CaptureCause> {
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let stream_config = config.clone().into();
        let error_fault = fault.clone();
        let error_callback = move |error: cpal::StreamError| {
            Self::record_fault(
                &error_fault,
                MicrophoneFault {
                    generation,
                    cause: Self::microphone_stream_error(error, &current_device),
                },
            );
        };

        match config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        if !paused.load(Ordering::Acquire) {
                            if let Err(cause) = sinks.publish_samples(
                                TrackKind::Microphone,
                                data,
                                sample_rate,
                                channels,
                                0,
                                TimestampQuality::Relative,
                            ) {
                                Self::record_fault(&fault, MicrophoneFault { generation, cause });
                            }
                        }
                    },
                    error_callback,
                    None,
                )
                .map_err(|error| CaptureCause::CaptureFailed(error.to_string())),
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        if !paused.load(Ordering::Acquire) {
                            if let Err(cause) = sinks.publish_i16(
                                TrackKind::Microphone,
                                data,
                                sample_rate,
                                channels,
                                0,
                                TimestampQuality::Relative,
                            ) {
                                Self::record_fault(&fault, MicrophoneFault { generation, cause });
                            }
                        }
                    },
                    error_callback,
                    None,
                )
                .map_err(|error| CaptureCause::CaptureFailed(error.to_string())),
            cpal::SampleFormat::I32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i32], _| {
                        if !paused.load(Ordering::Acquire) {
                            if let Err(cause) = sinks.publish_i32(
                                TrackKind::Microphone,
                                data,
                                sample_rate,
                                channels,
                                0,
                                TimestampQuality::Relative,
                            ) {
                                Self::record_fault(&fault, MicrophoneFault { generation, cause });
                            }
                        }
                    },
                    error_callback,
                    None,
                )
                .map_err(|error| CaptureCause::CaptureFailed(error.to_string())),
            format => Err(CaptureCause::CaptureFailed(format!(
                "unsupported microphone format {format:?}"
            ))),
        }
    }

    fn open_replacement_microphone(
        &mut self,
        previous_device: &str,
    ) -> Result<(Box<dyn MicrophoneStreamControl>, u64, String), CaptureCause> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or_else(|| CaptureCause::DeviceUnavailable("default microphone".into()))?;
        let current_device = device
            .name()
            .map_err(|error| CaptureCause::DeviceUnavailable(error.to_string()))?;
        if !MicrophoneRecovery::accepts_device_name(previous_device, &current_device) {
            return Err(CaptureCause::DeviceUnavailable(current_device));
        }
        let config = device
            .default_input_config()
            .map_err(|error| CaptureCause::DeviceUnavailable(error.to_string()))?;
        let sinks = self.microphone_sinks.clone().ok_or_else(|| {
            CaptureCause::CaptureFailed("microphone sinks are unavailable".into())
        })?;
        let generation = self.next_microphone_generation();
        let stream = Self::build_microphone_stream(
            &device,
            current_device.clone(),
            generation,
            &config,
            sinks,
            self.paused.clone(),
            self.fault.clone(),
        )?;
        Ok((Box::new(stream), generation, current_device))
    }

    fn install_replacement_microphone(
        &mut self,
        stream: Box<dyn MicrophoneStreamControl>,
        generation: u64,
        current_device: String,
    ) -> Result<CaptureRuntimeEvent, CaptureCause> {
        // Start a replacement once and let the shared paused flag gate the
        // callback. Calling pause/play on an AudioUnit during a live system
        // tap can block inside CoreAudio's HAL; the flag keeps recovery and
        // pause/resume responsive without replaying buffered samples.
        stream.play_capture()?;
        let previous_device = self
            .microphone_recovery
            .as_ref()
            .ok_or_else(|| CaptureCause::CaptureFailed("microphone recovery is inactive".into()))?
            .previous_device
            .clone();
        self.microphone_stream = Some(stream);
        self.microphone_device_name = Some(current_device.clone());
        self.active_microphone_generation = Some(generation);
        self.microphone_recovery.take();
        Ok(CaptureRuntimeEvent::MicrophoneSwitched {
            previous_device,
            current_device,
            reason: "active microphone became unavailable".into(),
        })
    }

    fn complete_microphone_recovery_attempt(
        &mut self,
        now: Instant,
        attempt: Result<(Box<dyn MicrophoneStreamControl>, u64, String), CaptureCause>,
    ) -> Result<Option<CaptureRuntimeEvent>, CaptureCause> {
        let attempts = {
            let recovery = self.microphone_recovery.as_mut().ok_or_else(|| {
                CaptureCause::CaptureFailed("microphone recovery is inactive".into())
            })?;
            recovery.record_failed_attempt();
            recovery.attempts
        };
        let result = match attempt {
            Ok((stream, generation, current_device)) => self
                .install_replacement_microphone(stream, generation, current_device)
                .map(Some),
            Err(cause) => Err(cause),
        };
        match result {
            Ok(event) => Ok(event),
            Err(cause) => {
                log::warn!("Microphone recovery attempt {attempts} failed: {cause:?}");
                let recovery = self.microphone_recovery.as_ref().ok_or_else(|| {
                    CaptureCause::CaptureFailed("microphone recovery is inactive".into())
                })?;
                if recovery.is_exhausted(now) {
                    let previous_device = recovery.previous_device.clone();
                    self.microphone_recovery.take();
                    Err(CaptureCause::DeviceUnavailable(previous_device))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn attempt_microphone_recovery(
        &mut self,
        now: Instant,
    ) -> Result<Option<CaptureRuntimeEvent>, CaptureCause> {
        let Some(recovery) = self.microphone_recovery.as_ref() else {
            return Ok(None);
        };
        if recovery.is_exhausted(now) {
            let previous_device = recovery.previous_device.clone();
            self.microphone_recovery.take();
            return Err(CaptureCause::DeviceUnavailable(previous_device));
        }
        if !recovery.may_attempt(now) {
            return Ok(None);
        }

        let previous_device = recovery.previous_device.clone();
        let attempt = self.open_replacement_microphone(&previous_device);
        self.complete_microphone_recovery_attempt(now, attempt)
    }

    fn dispatch_block(
        sinks: &SourceSinks,
        paused: &AtomicBool,
        block: super::backend::CaptureBlock,
        raw_format: TrackFormat,
        control: SourceControl,
        allow_paused: bool,
    ) -> Result<(), CaptureCause> {
        if block.block_kind != TrackBlockKind::Samples || block.sample_len() == 0 {
            return Ok(());
        }
        let span = TrackSpan {
            start_frame: block.native_start_frame,
            frame_count: block.native_frame_count,
            epoch: block.epoch,
            quality: block.quality,
        };
        Self::send_source_with_span(
            sinks,
            block.kind,
            paused,
            block.samples().to_vec(),
            block.sample_rate,
            block.channels,
            raw_format,
            span,
            control,
            allow_paused,
        )
    }

    fn drain_blocks(
        receiver: Option<&SourceBlockReceiver>,
        sinks: Option<&SourceSinks>,
        paused: &AtomicBool,
        raw_format: Option<TrackFormat>,
        control: SourceControl,
        allow_paused: bool,
    ) -> Result<(), CaptureCause> {
        let (Some(receiver), Some(sinks), Some(raw_format)) = (receiver, sinks, raw_format) else {
            return Ok(());
        };
        loop {
            match receiver.try_recv() {
                Ok(block) => {
                    Self::dispatch_block(sinks, paused, block, raw_format, control, allow_paused)?
                }
                Err(std::sync::mpsc::TryRecvError::Empty)
                | Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(()),
            }
        }
    }

    fn drain_system_samples_for_stop(
        stream: &mut CoreAudioStream,
        sinks: &SourceSinks,
        first_error: &mut Option<CaptureCause>,
    ) {
        loop {
            let (native_start_frame, samples) = stream.drain_samples_with_start(4096);
            if samples.is_empty() {
                return;
            }
            if let Err(cause) = sinks.publish_samples_at(
                &samples,
                CapturePublishContext {
                    kind: TrackKind::System,
                    sample_rate: stream.sample_rate(),
                    channels: 1,
                    epoch: 0,
                    quality: TimestampQuality::Relative,
                    native_start_frame: Some(native_start_frame),
                },
            ) {
                if first_error.is_none() {
                    *first_error = Some(cause);
                }
            }
        }
    }

    fn drain_blocks_for_stop(
        receiver: Option<&SourceBlockReceiver>,
        sinks: Option<&SourceSinks>,
        paused: &AtomicBool,
        raw_format: Option<TrackFormat>,
        control: SourceControl,
        first_error: &mut Option<CaptureCause>,
    ) {
        let (Some(receiver), Some(sinks), Some(raw_format)) = (receiver, sinks, raw_format) else {
            return;
        };
        loop {
            match receiver.try_recv() {
                Ok(block) => {
                    if let Err(cause) =
                        Self::dispatch_block(sinks, paused, block, raw_format, control, true)
                    {
                        if first_error.is_none() {
                            *first_error = Some(cause);
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty)
                | Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
            }
        }
    }

    pub fn pump(&mut self) -> Result<Option<CaptureRuntimeEvent>, CaptureCause> {
        let now = Instant::now();
        if self
            .microphone_recovery
            .as_ref()
            .map(|recovery| recovery.is_exhausted(now))
            .unwrap_or(false)
        {
            return self.attempt_microphone_recovery(now);
        }

        let mut system_start_result = None;
        let mut system_start_disconnected = false;
        if let Some(receiver) = self.system_start_rx.as_ref() {
            match receiver.try_recv() {
                Ok(result) => system_start_result = Some(result),
                Err(TryRecvError::Disconnected) => system_start_disconnected = true,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(result) = system_start_result {
            self.system_start_rx.take();
            if let Some(worker) = self.system_start_worker.take() {
                Self::join_core_audio_worker(worker, "system startup")?;
            }
            match result {
                Ok(stream) => {
                    self.system_stream = Some(stream);
                    log::info!("System audio capture became available after asynchronous startup");
                }
                Err(error) => {
                    self.system_sinks.take();
                    self.system_blocks.take();
                    self.expected.remove(&TrackKind::System);
                    self.pending_start_event = Some(Self::runtime_event_for_setup_error(error));
                }
            }
        } else if system_start_disconnected {
            self.system_start_rx.take();
            if let Some(worker) = self.system_start_worker.take() {
                Self::join_core_audio_worker(worker, "system startup")?;
            }
            self.system_sinks.take();
            self.system_blocks.take();
            self.expected.remove(&TrackKind::System);
            self.pending_start_event = Some(Self::runtime_event_for_setup_error(
                CoreAudioSetupError::CaptureFailed(
                    "system audio startup worker disconnected".into(),
                ),
            ));
        }
        if let Some(event) = self.pending_start_event.take() {
            return Ok(Some(event));
        }
        Self::drain_blocks(
            self.microphone_blocks.as_ref(),
            self.microphone_sinks.as_ref(),
            &self.paused,
            self.microphone_track_format,
            self.microphone_control,
            false,
        )?;
        let mut started_microphone_recovery = false;
        if let Some(fault) = self.take_fault() {
            if Some(fault.generation) == self.active_microphone_generation {
                if self.microphone_recovery.is_some() {
                    if !matches!(fault.cause, CaptureCause::DeviceUnavailable(_)) {
                        return Err(fault.cause);
                    }
                } else {
                    let recovery = MicrophoneRecovery::from_fault(fault.cause, now)?;
                    self.microphone_stream.take();
                    self.active_microphone_generation.take();
                    self.microphone_recovery = Some(recovery);
                    started_microphone_recovery = true;
                }
            }
        }
        if let Some(stream) = &mut self.system_stream {
            if stream.should_terminate() {
                return Ok(Some(self.degrade_system_audio(
                    "CoreAudio ring buffer overflow terminated system capture".into(),
                )));
            }
            if self.paused.load(Ordering::Acquire) {
                stream.discard_system_samples();
            }
        }
        let system_fault = if !self.paused.load(Ordering::Acquire) {
            if let (Some(stream), Some(sinks)) = (&mut self.system_stream, &self.system_sinks) {
                let (native_start_frame, samples) = stream.drain_samples_with_start(4096);
                if !samples.is_empty() {
                    self.system_has_samples = true;
                    sinks
                        .publish_samples_at(
                            &samples,
                            CapturePublishContext {
                                kind: TrackKind::System,
                                sample_rate: stream.sample_rate(),
                                channels: 1,
                                epoch: 0,
                                quality: TimestampQuality::Relative,
                                native_start_frame: Some(native_start_frame),
                            },
                        )
                        .err()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some(cause) = system_fault {
            return Ok(Some(self.degrade_system_audio(format!("{cause:?}"))));
        }
        Self::drain_blocks(
            self.system_blocks.as_ref(),
            self.system_sinks.as_ref(),
            &self.paused,
            self.system_sinks.as_ref().map(|_| TrackFormat {
                sample_rate: self
                    .system_stream
                    .as_ref()
                    .map(CoreAudioStream::sample_rate)
                    .unwrap_or(48_000),
                channels: 1,
            }),
            self.system_control,
            false,
        )?;
        if started_microphone_recovery {
            Ok(None)
        } else {
            self.attempt_microphone_recovery(now)
        }
    }
}

impl Default for MacosCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureBackend for MacosCaptureBackend {
    fn capabilities(&self) -> CaptureCapabilities {
        CaptureCapabilities {
            microphone: true,
            system: true,
        }
    }

    fn start(
        &mut self,
        request: CaptureRequest,
        mut sinks: TrackSinks,
    ) -> Result<(), CaptureCause> {
        self.timeline = Some(sinks.microphone.timeline());
        self.microphone_control = SourceControl::default();
        self.system_control = SourceControl::default();
        let device = Self::microphone_device(
            request.microphone_device.as_deref(),
            request.fallback_to_default_microphone,
        )?;
        let config = device
            .default_input_config()
            .map_err(|error| CaptureCause::DeviceUnavailable(error.to_string()))?;
        let current_device = device
            .name()
            .map_err(|error| CaptureCause::DeviceUnavailable(error.to_string()))?;
        let microphone_track_format = TrackFormat {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
        };
        let microphone_sinks = sinks.microphone.clone();
        let microphone_blocks = microphone_sinks.take_block_receiver();
        let generation = self.next_microphone_generation();
        let stream = Self::build_microphone_stream(
            &device,
            current_device.clone(),
            generation,
            &config,
            sinks.microphone,
            self.paused.clone(),
            self.fault.clone(),
        )?;
        stream.play_capture()?;
        self.microphone_stream = Some(Box::new(stream));
        self.microphone_sinks = Some(microphone_sinks);
        self.microphone_blocks = microphone_blocks;
        self.microphone_device_name = Some(current_device);
        self.microphone_track_format = Some(microphone_track_format);
        self.active_microphone_generation = Some(generation);
        self.expected.insert(TrackKind::Microphone);

        if request.capture_system {
            let Some(system_sinks) = sinks.system.take() else {
                self.pending_start_event = Some(Self::runtime_event_for_setup_error(
                    CoreAudioSetupError::CaptureFailed("requested system Track has no sink".into()),
                ));
                return Ok(());
            };
            let system_blocks = system_sinks.take_block_receiver();
            let (system_start_tx, system_start_rx) =
                mpsc::sync_channel::<Result<CoreAudioStream, CoreAudioSetupError>>(1);
            let preferred_device = request.system_device.clone();
            let paused = self.paused.clone();
            match thread::Builder::new()
                .name("meetily-coreaudio-start".into())
                .spawn(move || {
                    let result =
                        CoreAudioCapture::new_for_preference_cause(preferred_device.as_deref())
                            .and_then(|capture| capture.stream_with_pause_cause(paused));
                    if let Err(send_error) = system_start_tx.send(result) {
                        if let Ok(mut stream) = send_error.0 {
                            stream.stop_capture()?;
                        }
                    }
                    Ok(())
                }) {
                Ok(worker) => {
                    self.system_start_rx = Some(system_start_rx);
                    self.system_start_worker = Some(worker);
                    self.system_sinks = Some(system_sinks);
                    self.system_blocks = system_blocks;
                    self.system_has_samples = false;
                    self.expected.insert(TrackKind::System);
                    log::info!("System audio capture startup moved off the capture owner thread");
                }
                Err(error) => {
                    self.pending_start_event = Some(Self::runtime_event_for_setup_error(
                        CoreAudioSetupError::CaptureFailed(format!(
                            "failed to spawn system audio startup worker: {error}"
                        )),
                    ));
                }
            }
        }
        Ok(())
    }

    fn opened_sources(&self) -> BTreeSet<TrackKind> {
        self.expected.clone()
    }

    fn set_source_control(
        &mut self,
        kind: TrackKind,
        control: SourceControl,
    ) -> Result<(), CaptureCause> {
        control.validate()?;
        if !self.expected.contains(&kind) {
            return Err(CaptureCause::UnsupportedCapability(kind));
        }
        match kind {
            TrackKind::Microphone => self.microphone_control = control,
            TrackKind::System => self.system_control = control,
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<(), CaptureCause> {
        if self.microphone_recovery.is_none() {
            if let Some(event) = self.pump()? {
                self.pending_start_event = Some(event);
            }
        }
        Self::commit_pause_after_source_transition(&self.paused, || Ok(()))?;
        if let Some(timeline) = &self.timeline {
            timeline.pause();
        }
        if let Some(stream) = &mut self.system_stream {
            stream.discard_system_samples();
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<(), CaptureCause> {
        let mut system_terminated = false;
        if let Some(stream) = &mut self.system_stream {
            stream.discard_system_samples();
            if stream.should_terminate() {
                system_terminated = true;
            }
        }
        if system_terminated {
            let event = self.degrade_system_audio(
                "CoreAudio ring buffer overflow terminated system capture".into(),
            );
            self.pending_start_event = Some(event);
        }
        if let Some(timeline) = &self.timeline {
            timeline.resume();
        }
        self.paused.store(false, Ordering::Release);
        Ok(())
    }

    fn pump(&mut self) -> Result<Option<CaptureRuntimeEvent>, CaptureCause> {
        MacosCaptureBackend::pump(self)
    }

    fn stop(&mut self) -> Result<CaptureStopped, CaptureCause> {
        let active_microphone_generation = self.active_microphone_generation;
        self.microphone_recovery.take();

        let mut stop_error = None;
        // Device operations can block while HAL reconfigures. Gate callbacks,
        // drain accepted samples, and join CoreAudio cleanup before returning
        // so a later start cannot race with the previous device teardown.
        self.paused.store(true, Ordering::Release);
        if let Some(timeline) = &self.timeline {
            timeline.pause();
        }

        // Resolve and join startup before releasing any stream or returning.
        if let Err(cause) = self.finish_system_start_before_stop() {
            stop_error = Some(cause);
        }

        let mut system_stream = self.system_stream.take();
        let system_sample_rate = system_stream
            .as_ref()
            .map(CoreAudioStream::sample_rate)
            .unwrap_or(48_000);
        if let Some(stream) = system_stream.as_ref() {
            stream.signal_termination();
        }
        if let (Some(stream), Some(sinks)) = (system_stream.as_mut(), self.system_sinks.as_ref()) {
            Self::drain_system_samples_for_stop(stream, sinks, &mut stop_error);
        }
        if let Some(stream) = system_stream {
            // Keep an owner-side recovery path if thread creation fails. The
            // worker takes the stream only after it starts successfully.
            let stream_slot = Arc::new(Mutex::new(Some(stream)));
            let worker_stream_slot = Arc::clone(&stream_slot);
            match thread::Builder::new()
                .name("meetily-coreaudio-stop".into())
                .spawn(move || {
                    let mut stream = worker_stream_slot
                        .lock()
                        .map_err(|_| {
                            CoreAudioSetupError::CaptureFailed(
                                "CoreAudio stop stream lock was poisoned".into(),
                            )
                        })?
                        .take()
                        .ok_or_else(|| {
                            CoreAudioSetupError::CaptureFailed(
                                "CoreAudio stop stream was already taken".into(),
                            )
                        })?;
                    stream.stop_capture()
                }) {
                Ok(worker) => self.system_stop_worker = Some(worker),
                Err(error) => {
                    let fallback_error = match stream_slot.lock() {
                        Ok(mut slot) => match slot.take() {
                            Some(mut stream) => {
                                stream.stop_capture().err().map(|error| error.to_string())
                            }
                            None => Some("stream was unavailable for synchronous cleanup".into()),
                        },
                        Err(_) => Some("stream cleanup slot was poisoned".into()),
                    };
                    let message = match fallback_error {
                        Some(cleanup_error) => format!(
                            "failed to spawn CoreAudio stop cleanup: {error}; synchronous cleanup failed: {cleanup_error}"
                        ),
                        None => format!(
                            "failed to spawn CoreAudio stop cleanup: {error}; synchronous cleanup succeeded"
                        ),
                    };
                    if stop_error.is_none() {
                        stop_error = Some(CaptureCause::CaptureFailed(message));
                    }
                }
            }
        }
        if let Some(stream) = self.microphone_stream.take() {
            if let Err(cause) = stream.pause_capture() {
                if stop_error.is_none() {
                    stop_error = Some(cause);
                }
            }
            drop(stream);
        }
        if !self.system_has_samples {
            // A CoreAudio tap can be created successfully while yielding no
            // frames (for example when macOS has not granted the system-audio
            // route). Treat that source as optional for this session so the
            // microphone recording can still be finalized cleanly.
            self.expected.remove(&TrackKind::System);
            log::warn!("System audio capture produced no frames; closing microphone-only");
        }
        Self::drain_blocks_for_stop(
            self.microphone_blocks.as_ref(),
            self.microphone_sinks.as_ref(),
            &self.paused,
            self.microphone_track_format,
            self.microphone_control,
            &mut stop_error,
        );
        Self::drain_blocks_for_stop(
            self.system_blocks.as_ref(),
            self.system_sinks.as_ref(),
            &self.paused,
            self.system_sinks.as_ref().map(|_| TrackFormat {
                sample_rate: system_sample_rate,
                channels: 1,
            }),
            self.system_control,
            &mut stop_error,
        );
        self.microphone_sinks.take();
        self.microphone_blocks.take();
        self.microphone_device_name.take();
        self.microphone_track_format.take();
        self.active_microphone_generation.take();
        self.system_sinks.take();
        self.system_blocks.take();
        self.system_has_samples = false;
        self.timeline.take();
        if let Some(worker) = self.system_stop_worker.take() {
            if let Err(cause) = Self::join_core_audio_worker(worker, "system stop") {
                if stop_error.is_none() {
                    stop_error = Some(cause);
                }
            }
        }
        if let Ok(mut fault) = self.fault.lock() {
            if let Some(fault) = fault.take() {
                if Some(fault.generation) == active_microphone_generation
                    && !matches!(fault.cause, CaptureCause::DeviceUnavailable(_))
                {
                    return Err(fault.cause);
                }
            }
        }
        if let Some(cause) = stop_error {
            return Err(cause);
        }
        Ok(CaptureStopped {
            closed_sources: self.expected.clone(),
        })
    }
}

impl Drop for MacosCaptureBackend {
    fn drop(&mut self) {
        self.paused.store(true, Ordering::Release);
        if let Some(timeline) = &self.timeline {
            timeline.pause();
        }
        if let Err(cause) = self.finish_system_start_before_stop() {
            log::warn!("CoreAudio startup cleanup during backend drop failed: {cause:?}");
        }
        if let Some(mut stream) = self.system_stream.take() {
            stream.signal_termination();
            if let Err(error) = stream.stop_capture() {
                log::warn!("CoreAudio stream cleanup during backend drop failed: {error}");
            }
        }
        if let Some(worker) = self.system_stop_worker.take() {
            if let Err(cause) = Self::join_core_audio_worker(worker, "system stop during drop") {
                log::warn!("CoreAudio stop worker during backend drop failed: {cause:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::capture::backend::{TrackReceiver, TrackSink};
    use crate::audio::incremental_saver::SourceTrackSaver;
    use std::time::Duration;
    use tempfile::tempdir;

    fn backend_with_gated_startup(
        release: Receiver<()>,
        started: std::sync::mpsc::SyncSender<()>,
        finished: std::sync::mpsc::SyncSender<()>,
    ) -> MacosCaptureBackend {
        let (start_tx, start_rx) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            started.send(()).unwrap();
            release.recv().unwrap();
            let result = start_tx
                .send(Err(CoreAudioSetupError::CaptureFailed(
                    "synthetic startup failure".into(),
                )))
                .map_err(|error| {
                    CoreAudioSetupError::CaptureFailed(format!("startup receiver closed: {error}"))
                });
            finished.send(()).unwrap();
            result
        });

        let mut backend = MacosCaptureBackend::new();
        backend.expected.insert(TrackKind::System);
        backend.system_start_rx = Some(start_rx);
        backend.system_start_worker = Some(worker);
        backend
    }

    #[test]
    fn stop_waits_for_pending_startup_worker() {
        let (startup_release_tx, startup_release_rx) = mpsc::sync_channel(1);
        let (startup_started_tx, startup_started_rx) = mpsc::sync_channel(1);
        let (startup_finished_tx, startup_finished_rx) = mpsc::sync_channel(1);
        let mut backend =
            backend_with_gated_startup(startup_release_rx, startup_started_tx, startup_finished_tx);

        startup_started_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        let (operation_started_tx, operation_started_rx) = mpsc::sync_channel(1);
        let releaser = thread::spawn(move || {
            operation_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
            thread::sleep(Duration::from_millis(100));
            startup_release_tx.send(()).unwrap();
        });

        operation_started_tx.send(()).unwrap();
        let stopped = backend.stop().unwrap();
        assert!(stopped.closed_sources.is_empty());
        assert!(startup_finished_rx.try_recv().is_ok());
        releaser.join().unwrap();
    }

    #[test]
    fn stop_waits_for_pending_cleanup_worker() {
        let mut backend = MacosCaptureBackend::new();
        let (cleanup_release_tx, cleanup_release_rx) = mpsc::sync_channel(1);
        let (cleanup_started_tx, cleanup_started_rx) = mpsc::sync_channel(1);
        let (cleanup_finished_tx, cleanup_finished_rx) = mpsc::sync_channel(1);
        backend.system_stop_worker = Some(thread::spawn(move || {
            cleanup_started_tx.send(()).unwrap();
            cleanup_release_rx.recv().unwrap();
            cleanup_finished_tx.send(()).unwrap();
            Ok(())
        }));

        cleanup_started_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        let (operation_started_tx, operation_started_rx) = mpsc::sync_channel(1);
        let releaser = thread::spawn(move || {
            operation_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
            thread::sleep(Duration::from_millis(100));
            cleanup_release_tx.send(()).unwrap();
        });
        operation_started_tx.send(()).unwrap();
        let stopped = backend.stop().unwrap();
        assert!(stopped.closed_sources.is_empty());
        assert!(cleanup_finished_rx.try_recv().is_ok());
        releaser.join().unwrap();
    }

    #[test]
    fn backend_drop_waits_for_pending_startup_worker() {
        let (startup_release_tx, startup_release_rx) = mpsc::sync_channel(1);
        let (startup_started_tx, startup_started_rx) = mpsc::sync_channel(1);
        let (startup_finished_tx, startup_finished_rx) = mpsc::sync_channel(1);
        let backend =
            backend_with_gated_startup(startup_release_rx, startup_started_tx, startup_finished_tx);

        startup_started_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        let (operation_started_tx, operation_started_rx) = mpsc::sync_channel(1);
        let releaser = thread::spawn(move || {
            operation_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
            thread::sleep(Duration::from_millis(100));
            startup_release_tx.send(()).unwrap();
        });

        operation_started_tx.send(()).unwrap();
        drop(backend);
        assert!(startup_finished_rx.try_recv().is_ok());
        releaser.join().unwrap();
    }

    #[test]
    fn backend_drop_waits_for_pending_cleanup_worker() {
        let mut backend = MacosCaptureBackend::new();
        let (cleanup_release_tx, cleanup_release_rx) = mpsc::sync_channel(1);
        let (cleanup_started_tx, cleanup_started_rx) = mpsc::sync_channel(1);
        let (cleanup_finished_tx, cleanup_finished_rx) = mpsc::sync_channel(1);
        backend.system_stop_worker = Some(thread::spawn(move || {
            cleanup_started_tx.send(()).unwrap();
            cleanup_release_rx.recv().unwrap();
            cleanup_finished_tx.send(()).unwrap();
            Ok(())
        }));

        cleanup_started_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        let (operation_started_tx, operation_started_rx) = mpsc::sync_channel(1);
        let releaser = thread::spawn(move || {
            operation_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
            thread::sleep(Duration::from_millis(100));
            cleanup_release_tx.send(()).unwrap();
        });
        operation_started_tx.send(()).unwrap();
        drop(backend);
        assert!(cleanup_finished_rx.try_recv().is_ok());
        releaser.join().unwrap();
    }

    #[test]
    fn core_audio_worker_join_propagates_errors_and_panics() {
        let failed_worker = thread::spawn(|| {
            Err(CoreAudioSetupError::CaptureFailed(
                "synthetic cleanup failure".into(),
            ))
        });
        assert!(matches!(
            MacosCaptureBackend::join_core_audio_worker(failed_worker, "synthetic cleanup"),
            Err(CaptureCause::CaptureFailed(message))
                if message.contains("synthetic cleanup failure")
        ));

        let panicked_worker = thread::spawn(|| -> Result<(), CoreAudioSetupError> {
            panic!("synthetic worker panic");
        });
        assert!(matches!(
            MacosCaptureBackend::join_core_audio_worker(panicked_worker, "synthetic cleanup"),
            Err(CaptureCause::CaptureFailed(message)) if message.contains("worker panicked")
        ));
    }

    #[test]
    fn common_timeline_scales_native_start_frames() {
        assert_eq!(
            MacosCaptureBackend::scale_frame(44_100, 44_100, 48_000),
            48_000
        );
        assert_eq!(
            MacosCaptureBackend::scale_frame(48_000, 48_000, 48_000),
            48_000
        );
        assert_eq!(
            MacosCaptureBackend::scale_frame(96_000, 96_000, 48_000),
            48_000
        );
    }

    struct FakeMicrophoneStream {
        controls: Arc<Mutex<Vec<&'static str>>>,
        fail_pause: bool,
        on_pause: Option<Arc<dyn Fn() + Send + Sync>>,
    }

    impl MicrophoneStreamControl for FakeMicrophoneStream {
        fn play_capture(&self) -> Result<(), CaptureCause> {
            self.controls.lock().unwrap().push("play");
            if self.fail_pause {
                Err(CaptureCause::CaptureFailed("pause failed".into()))
            } else {
                Ok(())
            }
        }

        fn pause_capture(&self) -> Result<(), CaptureCause> {
            self.controls.lock().unwrap().push("pause");
            if let Some(callback) = &self.on_pause {
                callback();
            }
            if self.fail_pause {
                Err(CaptureCause::CaptureFailed("pause failed".into()))
            } else {
                Ok(())
            }
        }
    }

    fn microphone_source_fixture() -> (
        SourceSinks,
        SourceBlockReceiver,
        TrackReceiver,
        TrackReceiver,
    ) {
        let (raw, raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let sinks = SourceSinks::new(raw, downstream);
        let blocks = sinks.take_block_receiver().unwrap();
        (sinks, blocks, raw_receiver, downstream_receiver)
    }

    fn install_microphone_fixture(
        backend: &mut MacosCaptureBackend,
        sinks: SourceSinks,
        blocks: SourceBlockReceiver,
        stream: FakeMicrophoneStream,
    ) {
        backend.expected.insert(TrackKind::Microphone);
        backend.microphone_stream = Some(Box::new(stream));
        backend.microphone_sinks = Some(sinks);
        backend.microphone_blocks = Some(blocks);
        backend.microphone_track_format = Some(TrackFormat {
            sample_rate: 48_000,
            channels: 1,
        });
        backend.active_microphone_generation = Some(1);
    }

    fn count_raw_chunks(receiver: TrackReceiver) -> std::sync::mpsc::Receiver<usize> {
        let (done_sender, done_receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut count = 0;
            while receiver.recv().is_ok() {
                count += 1;
            }
            done_sender.send(count).unwrap();
        });
        done_receiver
    }

    fn publish_if_running(
        sinks: SourceSinks,
        paused: Arc<AtomicBool>,
    ) -> Arc<dyn Fn() + Send + Sync> {
        Arc::new(move || {
            if !paused.load(Ordering::Acquire) {
                sinks
                    .publish_samples(
                        TrackKind::Microphone,
                        &[0.25; 64],
                        48_000,
                        1,
                        0,
                        TimestampQuality::Relative,
                    )
                    .unwrap();
            }
        })
    }

    #[test]
    fn stop_drains_block_accepted_while_microphone_quiesces() {
        let (sinks, blocks, raw_receiver, _downstream_receiver) = microphone_source_fixture();
        let accepted_before_stop = sinks.clone();
        let mut backend = MacosCaptureBackend::new();
        let done = count_raw_chunks(raw_receiver);
        install_microphone_fixture(
            &mut backend,
            sinks,
            blocks,
            FakeMicrophoneStream {
                controls: Arc::new(Mutex::new(Vec::new())),
                fail_pause: false,
                on_pause: None,
            },
        );
        accepted_before_stop
            .publish_samples(
                TrackKind::Microphone,
                &[0.25; 64],
                48_000,
                1,
                0,
                TimestampQuality::Relative,
            )
            .unwrap();
        drop(accepted_before_stop);

        backend.stop().unwrap();

        assert_eq!(done.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
    }

    #[test]
    fn paused_stop_drains_only_blocks_accepted_before_the_pause() {
        let (sinks, blocks, raw_receiver, _downstream_receiver) = microphone_source_fixture();
        sinks
            .publish_samples(
                TrackKind::Microphone,
                &[0.5; 64],
                48_000,
                1,
                0,
                TimestampQuality::Relative,
            )
            .unwrap();
        let mut backend = MacosCaptureBackend::new();
        backend.paused.store(true, Ordering::Release);
        let on_pause = publish_if_running(sinks.clone(), backend.paused.clone());
        let done = count_raw_chunks(raw_receiver);
        install_microphone_fixture(
            &mut backend,
            sinks,
            blocks,
            FakeMicrophoneStream {
                controls: Arc::new(Mutex::new(Vec::new())),
                fail_pause: false,
                on_pause: Some(on_pause),
            },
        );

        backend.stop().unwrap();

        assert_eq!(done.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
    }

    #[test]
    fn stop_reports_microphone_pause_error_after_quiescing() {
        let (sinks, blocks, raw_receiver, _downstream_receiver) = microphone_source_fixture();
        let mut backend = MacosCaptureBackend::new();
        let done = count_raw_chunks(raw_receiver);
        install_microphone_fixture(
            &mut backend,
            sinks,
            blocks,
            FakeMicrophoneStream {
                controls: Arc::new(Mutex::new(Vec::new())),
                fail_pause: true,
                on_pause: None,
            },
        );

        assert!(matches!(
            backend.stop(),
            Err(CaptureCause::CaptureFailed(message)) if message == "pause failed"
        ));
        assert_eq!(done.recv_timeout(Duration::from_secs(1)).unwrap(), 0);
    }

    #[test]
    fn device_not_available_stream_error_starts_microphone_recovery() {
        let started_at = Instant::now();
        let cause = MacosCaptureBackend::microphone_stream_error(
            cpal::StreamError::DeviceNotAvailable,
            "Disconnected microphone",
        );
        let recovery = MicrophoneRecovery::from_fault(cause, started_at).unwrap();

        assert_eq!(recovery.previous_device, "Disconnected microphone");
        assert_eq!(recovery.attempts, 0);
        assert_eq!(recovery.next_attempt, started_at);
        assert_eq!(recovery.deadline, started_at + MICROPHONE_RECOVERY_WINDOW);

        let backend_error = MacosCaptureBackend::microphone_stream_error(
            cpal::StreamError::BackendSpecific {
                err: cpal::BackendSpecificError {
                    description: "callback failed".into(),
                },
            },
            "Disconnected microphone",
        );
        assert!(matches!(backend_error, CaptureCause::CaptureFailed(_)));
    }

    #[test]
    fn microphone_recovery_rejects_the_same_device_name() {
        let recovery = MicrophoneRecovery::new("Built-in microphone".into(), Instant::now());

        assert!(!MicrophoneRecovery::accepts_device_name(
            &recovery.previous_device,
            "Built-in microphone"
        ));
        assert!(MicrophoneRecovery::accepts_device_name(
            &recovery.previous_device,
            "USB microphone"
        ));
    }

    #[test]
    fn microphone_recovery_stops_after_three_attempts_or_three_seconds() {
        let started_at = Instant::now();
        let mut recovery = MicrophoneRecovery::new("Disconnected microphone".into(), started_at);

        assert!(recovery.may_attempt(started_at));
        recovery.record_failed_attempt();
        assert!(!recovery.may_attempt(started_at + Duration::from_millis(499)));
        assert!(recovery.may_attempt(started_at + Duration::from_millis(500)));
        recovery.record_failed_attempt();
        assert!(!recovery.may_attempt(started_at + Duration::from_millis(1_499)));
        assert!(recovery.may_attempt(started_at + Duration::from_millis(1_500)));
        recovery.record_failed_attempt();
        assert!(!recovery.may_attempt(started_at + Duration::from_millis(1_501)));
        assert!(recovery.is_exhausted(started_at + Duration::from_millis(1_501)));

        let deadline_recovery =
            MicrophoneRecovery::new("Disconnected microphone".into(), started_at);
        assert!(!deadline_recovery.may_attempt(started_at + MICROPHONE_RECOVERY_WINDOW));
        assert!(deadline_recovery.is_exhausted(started_at + MICROPHONE_RECOVERY_WINDOW));

        let mut backend = MacosCaptureBackend::new();
        backend.microphone_recovery = Some(deadline_recovery);
        assert!(matches!(
            backend.attempt_microphone_recovery(started_at + MICROPHONE_RECOVERY_WINDOW),
            Err(CaptureCause::DeviceUnavailable(device)) if device == "Disconnected microphone"
        ));
    }

    #[test]
    fn stale_device_fault_does_not_restart_active_microphone_recovery() {
        let mut backend = MacosCaptureBackend::new();
        let mut recovery =
            MicrophoneRecovery::new("Disconnected microphone".into(), Instant::now());
        recovery.record_failed_attempt();
        backend.microphone_recovery = Some(recovery);
        MacosCaptureBackend::record_fault(
            &backend.fault,
            MicrophoneFault {
                generation: 1,
                cause: CaptureCause::DeviceUnavailable("Disconnected microphone".into()),
            },
        );

        backend.pump().unwrap();

        assert_eq!(backend.microphone_recovery.as_ref().unwrap().attempts, 1);
    }

    #[test]
    fn old_generation_fault_after_switch_keeps_replacement_installed() {
        let mut backend = MacosCaptureBackend::new();
        let controls = Arc::new(Mutex::new(Vec::new()));
        backend.active_microphone_generation = Some(1);
        backend.microphone_recovery = Some(MicrophoneRecovery::new(
            "Disconnected microphone".into(),
            Instant::now(),
        ));
        backend
            .install_replacement_microphone(
                Box::new(FakeMicrophoneStream {
                    controls,
                    fail_pause: false,
                    on_pause: None,
                }),
                2,
                "Replacement microphone".into(),
            )
            .unwrap();
        MacosCaptureBackend::record_fault(
            &backend.fault,
            MicrophoneFault {
                generation: 1,
                cause: CaptureCause::DeviceUnavailable("Disconnected microphone".into()),
            },
        );

        assert_eq!(backend.pump().unwrap(), None);
        assert!(backend.microphone_stream.is_some());
        assert!(backend.microphone_recovery.is_none());
        assert_eq!(backend.active_microphone_generation, Some(2));
        assert_eq!(
            backend.microphone_device_name.as_deref(),
            Some("Replacement microphone")
        );
    }

    #[test]
    fn old_generation_capture_failure_does_not_fail_stop() {
        let mut backend = MacosCaptureBackend::new();
        backend.paused.store(true, Ordering::Release);
        backend.active_microphone_generation = Some(2);
        MacosCaptureBackend::record_fault(
            &backend.fault,
            MicrophoneFault {
                generation: 1,
                cause: CaptureCause::CaptureFailed("superseded stream failed".into()),
            },
        );

        backend.stop().unwrap();
    }

    #[test]
    fn active_generation_capture_failure_still_fails_stop() {
        let mut backend = MacosCaptureBackend::new();
        backend.paused.store(true, Ordering::Release);
        backend.active_microphone_generation = Some(2);
        MacosCaptureBackend::record_fault(
            &backend.fault,
            MicrophoneFault {
                generation: 2,
                cause: CaptureCause::CaptureFailed("active stream failed".into()),
            },
        );

        assert!(matches!(
            backend.stop(),
            Err(CaptureCause::CaptureFailed(message)) if message == "active stream failed"
        ));
    }

    #[test]
    fn microphone_recovery_deadline_precedes_pending_runtime_event() {
        let now = Instant::now();
        let mut recovery = MicrophoneRecovery::new("Disconnected microphone".into(), now);
        recovery.deadline = now;
        let mut backend = MacosCaptureBackend::new();
        backend.microphone_recovery = Some(recovery);
        backend.pending_start_event = Some(CaptureRuntimeEvent::SystemAudioDegraded {
            reason: "system stream stopped".into(),
        });

        assert!(matches!(
            backend.pump(),
            Err(CaptureCause::DeviceUnavailable(device)) if device == "Disconnected microphone"
        ));
        assert!(backend.pending_start_event.is_some());
    }

    #[test]
    fn stop_cancels_microphone_recovery() {
        let mut backend = MacosCaptureBackend::new();
        backend.expected.insert(TrackKind::Microphone);
        backend.microphone_recovery = Some(MicrophoneRecovery::new(
            "Disconnected microphone".into(),
            Instant::now(),
        ));

        let stopped = backend.stop().unwrap();

        assert!(backend.microphone_recovery.is_none());
        assert_eq!(
            stopped.closed_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
    }

    #[test]
    fn paused_replacement_starts_once_and_resume_keeps_it_running() {
        let mut backend = MacosCaptureBackend::new();
        let controls = Arc::new(Mutex::new(Vec::new()));
        backend.paused.store(true, Ordering::Release);
        backend.microphone_recovery = Some(MicrophoneRecovery::new(
            "Disconnected microphone".into(),
            Instant::now(),
        ));

        backend
            .install_replacement_microphone(
                Box::new(FakeMicrophoneStream {
                    controls: controls.clone(),
                    fail_pause: false,
                    on_pause: None,
                }),
                1,
                "Replacement microphone".into(),
            )
            .unwrap();

        assert_eq!(*controls.lock().unwrap(), vec!["play"]);
        backend.resume().unwrap();
        assert_eq!(*controls.lock().unwrap(), vec!["play"]);
    }

    #[test]
    fn pause_and_resume_gate_microphone_without_hal_restart() {
        let controls = Arc::new(Mutex::new(Vec::new()));
        let mut backend = MacosCaptureBackend::new();
        backend.expected.insert(TrackKind::Microphone);
        backend.microphone_stream = Some(Box::new(FakeMicrophoneStream {
            controls: controls.clone(),
            fail_pause: false,
            on_pause: None,
        }));

        backend.pause().unwrap();
        backend.resume().unwrap();

        assert!(controls.lock().unwrap().is_empty());
        assert!(!backend.paused.load(Ordering::Acquire));
    }

    #[test]
    fn failed_replacement_pause_counts_as_bounded_recovery_attempt() {
        let now = Instant::now();
        let mut backend = MacosCaptureBackend::new();
        let controls = Arc::new(Mutex::new(Vec::new()));
        backend.paused.store(true, Ordering::Release);
        backend.microphone_recovery = Some(MicrophoneRecovery::new(
            "Disconnected microphone".into(),
            now,
        ));

        assert_eq!(
            backend
                .complete_microphone_recovery_attempt(
                    now,
                    Ok((
                        Box::new(FakeMicrophoneStream {
                            controls: controls.clone(),
                            fail_pause: true,
                            on_pause: None,
                        }),
                        1,
                        "Replacement microphone".into(),
                    )),
                )
                .unwrap(),
            None
        );

        let recovery = backend.microphone_recovery.as_ref().unwrap();
        assert_eq!(recovery.attempts, 1);
        assert_eq!(recovery.next_attempt, now + Duration::from_millis(500));
        assert_eq!(*controls.lock().unwrap(), vec!["play"]);
        assert!(backend.microphone_stream.is_none());
        assert!(backend.active_microphone_generation.is_none());
    }

    #[test]
    fn replacement_microphone_format_is_normalized_for_existing_raw_writer() {
        let temp = tempdir().unwrap();
        let (raw, raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let sinks = SourceSinks::new(raw, downstream);
        let paused = AtomicBool::new(false);
        let target_format = TrackFormat {
            sample_rate: 48_000,
            channels: 2,
        };

        MacosCaptureBackend::send_source(
            &sinks,
            TrackKind::Microphone,
            &paused,
            vec![0.1; 48_000 * 2],
            48_000,
            2,
            target_format,
        )
        .unwrap();
        MacosCaptureBackend::send_source(
            &sinks,
            TrackKind::Microphone,
            &paused,
            vec![0.2; 44_100],
            44_100,
            1,
            target_format,
        )
        .unwrap();
        drop(sinks);

        let mut saver =
            SourceTrackSaver::new(temp.path().to_path_buf(), TrackKind::Microphone).unwrap();
        while let Ok(chunk) = raw_receiver.recv() {
            saver.add_chunk(chunk).unwrap();
        }
        let finalized = saver.finalize().unwrap();
        assert_eq!(finalized.sample_rate, target_format.sample_rate);
        assert_eq!(finalized.channels, target_format.channels);
        assert_eq!(finalized.sample_count, 192_000);
        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.sample_rate, target_format.sample_rate);
        assert_eq!(recovered.channels, target_format.channels);
        assert_eq!(recovered.sample_count, 192_000);
        assert_eq!(recovered.sample_count, finalized.sample_count);
    }

    #[test]
    fn source_control_mutes_downstream_but_preserves_raw_samples() {
        let (raw, raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let sinks = SourceSinks::new(raw, downstream);
        let paused = AtomicBool::new(false);

        MacosCaptureBackend::send_source_with_span(
            &sinks,
            TrackKind::Microphone,
            &paused,
            vec![0.25, -0.5, 0.75, -1.0],
            48_000,
            1,
            TrackFormat {
                sample_rate: 48_000,
                channels: 1,
            },
            TrackSpan::new(0, 4),
            SourceControl {
                muted: true,
                gain: 2.0,
            },
            false,
        )
        .unwrap();

        let raw_chunk = raw_receiver.recv().unwrap();
        let downstream_chunk = downstream_receiver.recv().unwrap();
        assert_eq!(raw_chunk.samples, vec![0.25, -0.5, 0.75, -1.0]);
        assert_eq!(downstream_chunk.samples, vec![0.0; 4]);
        assert_eq!(raw_chunk.span, downstream_chunk.span);
    }

    fn system_sinks() -> SourceSinks {
        let (raw, _raw_receiver) = super::super::backend::TrackSink::new(TrackKind::System);
        let (downstream, _downstream_receiver) =
            super::super::backend::TrackSink::new(TrackKind::System);
        SourceSinks::new(raw, downstream)
    }

    #[test]
    fn every_system_setup_error_retains_its_typed_runtime_reason() {
        let cases = [
            (
                CoreAudioSetupError::PermissionDenied("screen recording denied".into()),
                "permission denied: screen recording denied",
            ),
            (
                CoreAudioSetupError::DeviceUnavailable("no output device".into()),
                "device unavailable: no output device",
            ),
            (
                CoreAudioSetupError::CaptureFailed("tap creation failed".into()),
                "capture failed: tap creation failed",
            ),
        ];

        for (error, expected_reason) in cases {
            assert_eq!(
                MacosCaptureBackend::runtime_event_for_setup_error(error),
                CaptureRuntimeEvent::SystemAudioDegraded {
                    reason: expected_reason.into(),
                }
            );
        }
    }

    #[test]
    fn system_runtime_degradation_drops_source_and_current_sinks() {
        let mut backend = MacosCaptureBackend::new();
        backend.expected.insert(TrackKind::Microphone);
        backend.expected.insert(TrackKind::System);
        backend.system_sinks = Some(system_sinks());

        assert_eq!(
            backend.degrade_system_audio("system stream stopped".into()),
            CaptureRuntimeEvent::SystemAudioDegraded {
                reason: "system stream stopped".into(),
            }
        );
        assert!(backend.system_stream.is_none());
        assert!(backend.system_sinks.is_none());
        assert_eq!(
            backend.opened_sources(),
            BTreeSet::from([TrackKind::Microphone])
        );

        backend.pause().unwrap();
        assert!(backend.paused.load(Ordering::Acquire));
        backend.resume().unwrap();
        assert!(!backend.paused.load(Ordering::Acquire));
        let stopped = backend.stop().unwrap();
        assert_eq!(
            stopped.closed_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
    }

    #[test]
    fn pending_system_start_degradation_is_emitted_once() {
        let mut backend = MacosCaptureBackend::new();
        backend.pending_start_event = Some(CaptureRuntimeEvent::SystemAudioDegraded {
            reason: "permission denied: screen recording denied".into(),
        });

        assert_eq!(
            backend.pump().unwrap(),
            Some(CaptureRuntimeEvent::SystemAudioDegraded {
                reason: "permission denied: screen recording denied".into(),
            })
        );
        assert_eq!(backend.pump().unwrap(), None);
    }

    #[test]
    fn failed_microphone_pause_does_not_commit_backend_pause() {
        let paused = AtomicBool::new(false);
        let result = MacosCaptureBackend::commit_pause_after_source_transition(&paused, || {
            Err(CaptureCause::CaptureFailed("pause failed".into()))
        });
        assert!(result.is_err());
        assert!(!paused.load(Ordering::Acquire));
    }

    #[test]
    fn successful_microphone_pause_commits_backend_pause() {
        let paused = AtomicBool::new(false);
        MacosCaptureBackend::commit_pause_after_source_transition(&paused, || Ok(())).unwrap();
        assert!(paused.load(Ordering::Acquire));
    }
}
