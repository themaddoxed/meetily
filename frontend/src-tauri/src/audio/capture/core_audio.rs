// Core Audio implementation for macOS system audio capture

use anyhow::Result;
use futures_util::Stream;
use log::{error, info, warn};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};
#[cfg(target_os = "macos")]
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[cfg(target_os = "macos")]
use cidre::{arc, av, cat, cf, core_audio as ca, os};

#[derive(Debug)]
pub enum CoreAudioSetupError {
    PermissionDenied(String),
    DeviceUnavailable(String),
    CaptureFailed(String),
}

impl std::fmt::Display for CoreAudioSetupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied(message)
            | Self::DeviceUnavailable(message)
            | Self::CaptureFailed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CoreAudioSetupError {}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum SetupStage {
    DefaultOutputDevice,
    OutputDeviceDetails,
    ProcessTap,
    AggregateDevice,
    IoProc,
    DeviceStart,
    TapFormat,
}

#[cfg(target_os = "macos")]
fn setup_error(stage: SetupStage, error: os::Error) -> CoreAudioSetupError {
    let message = match stage {
        SetupStage::DefaultOutputDevice => {
            format!("Failed to get default output device: {error:?}")
        }
        SetupStage::OutputDeviceDetails => {
            format!("Failed to get output device details: {error:?}")
        }
        SetupStage::ProcessTap => format!("Failed to create process tap: {error:?}"),
        SetupStage::AggregateDevice => format!("Failed to create aggregate device: {error:?}"),
        SetupStage::IoProc => format!("Failed to create Core Audio IO proc: {error:?}"),
        SetupStage::DeviceStart => format!("Failed to start Core Audio device: {error:?}"),
        SetupStage::TapFormat => format!("Failed to get Core Audio tap format: {error:?}"),
    };

    if error == os::Error::from_be_bytes(*b"perm") {
        CoreAudioSetupError::PermissionDenied(message)
    } else if matches!(
        stage,
        SetupStage::DefaultOutputDevice | SetupStage::OutputDeviceDetails
    ) {
        CoreAudioSetupError::DeviceUnavailable(message)
    } else {
        CoreAudioSetupError::CaptureFailed(message)
    }
}

#[cfg(target_os = "macos")]
fn tap_creation_error(error: os::Error) -> CoreAudioSetupError {
    setup_error(SetupStage::ProcessTap, error)
}

/// Waker state for async polling
struct WakerState {
    waker: Option<Waker>,
    has_data: bool,
}

/// Core Audio speaker input using aggregate device + tap
#[cfg(target_os = "macos")]
pub struct CoreAudioCapture {
    tap: ca::TapGuard,
    agg_desc: arc::Retained<cf::DictionaryOf<cf::String, cf::Type>>,
}

/// Core Audio stream that produces audio samples
#[cfg(target_os = "macos")]
pub struct CoreAudioStream {
    consumer: HeapCons<f32>,
    _device: Option<ca::hardware::StartedDevice<ca::AggregateDevice>>,
    _ctx: Box<AudioContext>,
    _tap: ca::TapGuard,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
    drain_frame_cursor: u64,
}

/// Audio processing context
#[cfg(target_os = "macos")]
struct AudioContext {
    format: arc::R<av::AudioFormat>,
    producer: HeapProd<f32>,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    should_terminate: Arc<AtomicBool>,
    callback_frame_cursor: Arc<AtomicU64>,
}

#[cfg(target_os = "macos")]
fn discard_buffered_samples(consumer: &mut HeapCons<f32>) -> usize {
    let mut discarded = 0;
    while consumer.try_pop().is_some() {
        discarded += 1;
    }
    discarded
}

#[cfg(target_os = "macos")]
impl CoreAudioCapture {
    /// Create a new Core Audio capture for system audio
    pub fn new() -> Result<Self> {
        Self::new_for_preference(None)
    }

    pub fn new_for_preference(preferred_device: Option<&str>) -> Result<Self> {
        Self::new_for_preference_cause(preferred_device).map_err(anyhow::Error::new)
    }

    pub fn new_for_preference_cause(
        preferred_device: Option<&str>,
    ) -> std::result::Result<Self, CoreAudioSetupError> {
        info!("🎙️ CoreAudio: Starting Core Audio capture initialization...");

        // Note: Audio Capture permission (NSAudioCaptureUsageDescription) is required for macOS 14.4+
        // The permission dialog is automatically triggered when creating the Core Audio tap.
        // If permission is denied, the tap will return silence (all zeros).

        // Get default output device
        info!("🎙️ CoreAudio: Getting default output device...");
        let default_output_device = ca::System::default_output_device().map_err(|e| {
            error!("❌ CoreAudio: Failed to get default output device: {:?}", e);
            setup_error(SetupStage::DefaultOutputDevice, e)
        })?;
        let output_device = preferred_device
            .filter(|name| !name.is_empty())
            .and_then(|preferred| {
                ca::System::devices().ok()?.into_iter().find(|device| {
                    device
                        .name()
                        .ok()
                        .is_some_and(|name| name.to_string() == preferred)
                })
            })
            .unwrap_or_else(|| {
                if let Some(preferred) = preferred_device.filter(|name| !name.is_empty()) {
                    warn!(
                        "CoreAudio preferred output '{}' is unavailable; using the current default",
                        preferred
                    );
                }
                default_output_device
            });

        info!("✅ CoreAudio: Got default output device");

        let output_uid = output_device.uid().map_err(|e| {
            error!("❌ CoreAudio: Failed to get device UID: {:?}", e);
            setup_error(SetupStage::OutputDeviceDetails, e)
        })?;

        // Get device name for better debugging
        let device_name = output_device
            .name()
            .unwrap_or_else(|_| cf::String::from_str("Unknown"));
        info!(
            "✅ CoreAudio: Default output device: '{}' (UID: {:?})",
            device_name, output_uid
        );

        // IMPORTANT: We do NOT create a sub_device dictionary here
        // When using a tap, the tap provides all the audio we need
        // Including both the tap AND the device creates duplicate audio (echo issue)

        // Create process tap with mono global tap, excluding no processes
        // Note: Mono tap is more reliable for system audio capture on macOS
        info!("🎙️ CoreAudio: Creating process tap (global mono tap)...");
        let tap_desc =
            ca::TapDesc::with_mono_global_tap_excluding_processes(&cidre::ns::Array::new());
        let tap = tap_desc.create_process_tap().map_err(|e| {
            error!("❌ CoreAudio: Failed to create process tap: {:?}", e);
            tap_creation_error(e)
        })?;

        // Get tap information
        let tap_uid = tap.uid().unwrap_or_else(|_| cf::Uuid::new().to_cf_string());
        let tap_asbd = tap.asbd();

        match tap_asbd {
            Ok(asbd) => {
                info!("✅ CoreAudio: Process tap created - UID: {:?}", tap_uid);
                info!(
                    "📊 CoreAudio: Tap format - sample_rate: {} Hz, channels: {}",
                    asbd.sample_rate, asbd.channels_per_frame
                );
            }
            Err(e) => {
                warn!(
                    "⚠️ CoreAudio: Tap created but couldn't get format info: {:?}",
                    e
                );
            }
        }

        // Create sub-tap dictionary
        let sub_tap = cf::DictionaryOf::with_keys_values(
            &[ca::sub_device_keys::uid()],
            &[tap.uid().unwrap().as_type_ref()],
        );

        // Create aggregate device descriptor
        // CRITICAL FIX: Use ONLY the tap, NOT the output device + tap
        // Previous configuration included both sub_device_list (output device) and tap_list (tap of same device)
        // This caused duplicate audio capture, resulting in echo (YouTube audio appeared twice)
        // The tap alone provides all the system audio we need
        let agg_desc = cf::DictionaryOf::with_keys_values(
            &[
                ca::aggregate_device_keys::is_private(),
                ca::aggregate_device_keys::is_stacked(),
                ca::aggregate_device_keys::tap_auto_start(),
                ca::aggregate_device_keys::name(),
                ca::aggregate_device_keys::main_sub_device(),
                ca::aggregate_device_keys::uid(),
                // REMOVED: sub_device_list (was causing duplicate audio)
                ca::aggregate_device_keys::tap_list(),
            ],
            &[
                cf::Boolean::value_true().as_type_ref(),
                cf::Boolean::value_false(),
                cf::Boolean::value_true(),
                cf::str!(c"meetily-audio-tap").as_type_ref(),
                &output_uid,
                &cf::Uuid::new().to_cf_string(),
                // REMOVED: sub_device array (was causing echo)
                &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
            ],
        );

        info!("✅ CoreAudio: Aggregate device descriptor created");
        info!("✅ CoreAudio: Core Audio capture initialized successfully!");

        Ok(Self { tap, agg_desc })
    }

    /// Start the audio device and create a stream
    fn start_device(
        &self,
        ctx: &mut Box<AudioContext>,
    ) -> std::result::Result<ca::hardware::StartedDevice<ca::AggregateDevice>, CoreAudioSetupError>
    {
        extern "C" fn audio_proc(
            device: ca::Device,
            _now: &cat::AudioTimeStamp,
            input_data: &cat::AudioBufList<1>,
            _input_time: &cat::AudioTimeStamp,
            _output_data: &mut cat::AudioBufList<1>,
            _output_time: &cat::AudioTimeStamp,
            ctx: Option<&mut AudioContext>,
        ) -> os::Status {
            let ctx = ctx.unwrap();

            // Check for sample rate changes
            let after = device
                .nominal_sample_rate()
                .unwrap_or(ctx.format.absd().sample_rate) as u32;
            let before = ctx.current_sample_rate.load(Ordering::Acquire);

            if before != after {
                ctx.current_sample_rate.store(after, Ordering::Release);
            }

            // Try to get audio data from the buffer list
            if let Some(view) =
                av::AudioPcmBuf::with_buf_list_no_copy(&ctx.format, input_data, None)
            {
                if let Some(data) = view.data_f32_at(0) {
                    process_audio_data(ctx, data);
                }
            } else if ctx.format.common_format() == av::audio::CommonFormat::PcmF32 {
                // Fallback: manual extraction if AudioPcmBuf fails
                let first_buffer = &input_data.buffers[0];
                let byte_count = first_buffer.data_bytes_size as usize;
                let float_count = byte_count / std::mem::size_of::<f32>();

                if float_count > 0 && !first_buffer.data.is_null() {
                    let data = unsafe {
                        std::slice::from_raw_parts(first_buffer.data as *const f32, float_count)
                    };
                    process_audio_data(ctx, data);
                }
            }

            os::Status::NO_ERR
        }

        // Create aggregate device
        info!("🎙️ CoreAudio: Creating aggregate device...");
        let agg_device = ca::AggregateDevice::with_desc(&self.agg_desc).map_err(|e| {
            error!("❌ CoreAudio: Failed to create aggregate device: {:?}", e);
            setup_error(SetupStage::AggregateDevice, e)
        })?;

        info!("✅ CoreAudio: Aggregate device created");

        // Create IO proc ID for audio processing
        info!("🎙️ CoreAudio: Creating IO proc...");
        let proc_id = agg_device
            .create_io_proc_id(audio_proc, Some(ctx))
            .map_err(|e| {
                error!("❌ CoreAudio: Failed to create IO proc: {:?}", e);
                setup_error(SetupStage::IoProc, e)
            })?;

        info!("✅ CoreAudio: IO proc created with ID: {:?}", proc_id);

        // Start the device
        info!("🎙️ CoreAudio: Starting audio device...");
        let started_device = ca::device_start(agg_device, Some(proc_id)).map_err(|e| {
            error!("❌ CoreAudio: Failed to start device: {:?}", e);
            setup_error(SetupStage::DeviceStart, e)
        })?;

        info!("✅ CoreAudio: Audio device started successfully!");

        // Get device sample rate
        let device_ref = started_device.as_ref();
        let sample_rate = device_ref.nominal_sample_rate().unwrap_or(0.0);
        info!(
            "📊 CoreAudio: Aggregate device sample_rate: {} Hz",
            sample_rate
        );

        Ok(started_device)
    }

    /// Create a stream from this capture
    pub fn stream(self) -> Result<CoreAudioStream> {
        self.stream_with_pause(Arc::new(AtomicBool::new(false)))
    }

    pub fn stream_with_pause(self, paused: Arc<AtomicBool>) -> Result<CoreAudioStream> {
        self.stream_with_pause_cause(paused)
            .map_err(anyhow::Error::new)
    }

    pub fn stream_with_pause_cause(
        self,
        paused: Arc<AtomicBool>,
    ) -> std::result::Result<CoreAudioStream, CoreAudioSetupError> {
        info!("🎙️ CoreAudio: Creating CoreAudioStream...");

        // Get tap audio format
        let asbd = self.tap.asbd().map_err(|e| {
            error!("❌ CoreAudio: Failed to get tap ASBD: {:?}", e);
            setup_error(SetupStage::TapFormat, e)
        })?;

        let format = av::AudioFormat::with_asbd(&asbd).ok_or_else(|| {
            error!("❌ CoreAudio: Failed to create audio format");
            CoreAudioSetupError::CaptureFailed("Failed to create audio format".to_owned())
        })?;

        info!(
            "✅ CoreAudio: Tap audio format: {} Hz, {} channels",
            asbd.sample_rate, asbd.channels_per_frame
        );

        // Create ring buffer for lock-free audio transfer
        let buffer_size = 1024 * 128; // 128KB buffer
        let rb = HeapRb::<f32>::new(buffer_size);
        let (producer, consumer) = rb.split();

        let waker_state = Arc::new(Mutex::new(WakerState {
            waker: None,
            has_data: false,
        }));

        let current_sample_rate = Arc::new(AtomicU32::new(asbd.sample_rate as u32));
        let callback_frame_cursor = Arc::new(AtomicU64::new(0));
        info!("✅ CoreAudio: Initial sample rate: {} Hz", asbd.sample_rate);

        let mut ctx = Box::new(AudioContext {
            format,
            producer,
            waker_state: waker_state.clone(),
            current_sample_rate: current_sample_rate.clone(),
            paused,
            should_terminate: Arc::new(AtomicBool::new(false)),
            callback_frame_cursor,
        });

        info!("🎙️ CoreAudio: Starting audio device...");
        let device = self.start_device(&mut ctx)?;

        info!("✅ CoreAudio: CoreAudioStream created successfully!");

        Ok(CoreAudioStream {
            consumer,
            _device: Some(device),
            _ctx: ctx,
            _tap: self.tap,
            waker_state,
            current_sample_rate,
            drain_frame_cursor: 0,
        })
    }
}

/// Process audio data from the IO proc callback
#[cfg(target_os = "macos")]
fn process_audio_data(ctx: &mut AudioContext, data: &[f32]) {
    // Push raw samples directly to ring buffer
    // Let the pipeline handle all gain adjustments (post-mix 3x gain + mic normalization)
    let Ok(pushed) =
        push_callback_samples(&mut ctx.producer, data, &ctx.paused, &ctx.should_terminate)
    else {
        return;
    };

    if pushed > 0 {
        ctx.callback_frame_cursor
            .fetch_add(pushed as u64, Ordering::AcqRel);
        let should_wake = {
            let mut waker_state = ctx.waker_state.lock().unwrap();
            if !waker_state.has_data {
                waker_state.has_data = true;
                waker_state.waker.take()
            } else {
                None
            }
        };

        if let Some(waker) = should_wake {
            waker.wake();
        }
    }
}

#[cfg(target_os = "macos")]
fn push_callback_samples(
    producer: &mut HeapProd<f32>,
    data: &[f32],
    paused: &AtomicBool,
    should_terminate: &AtomicBool,
) -> std::result::Result<usize, ()> {
    if paused.load(Ordering::Acquire) || data.is_empty() {
        return Ok(0);
    }
    let pushed = producer.push_slice(data);
    if pushed != data.len() {
        should_terminate.store(true, Ordering::Release);
        return Err(());
    }
    Ok(pushed)
}

#[cfg(target_os = "macos")]
impl CoreAudioStream {
    /// Ask the callback to stop publishing before handing the HAL object to a
    /// detached cleanup worker. CoreAudio can block while stopping a tap; the
    /// capture owner must remain responsive to the stop acknowledgement.
    pub fn signal_termination(&self) {
        self._ctx.should_terminate.store(true, Ordering::Release);
    }

    /// Stop the Core Audio producer while retaining the ring consumer for a final drain.
    pub fn stop_capture(&mut self) -> std::result::Result<(), CoreAudioSetupError> {
        self.signal_termination();
        let Some(device) = self._device.take() else {
            return Ok(());
        };
        device.stop().map(|_| ()).map_err(|error| {
            CoreAudioSetupError::CaptureFailed(format!(
                "Failed to stop Core Audio device: {error:?}"
            ))
        })
    }

    /// Get current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::Acquire)
    }

    /// Drain without awaiting so the capture-owner thread can keep servicing stop controls.
    pub fn drain_samples(&mut self, limit: usize) -> Vec<f32> {
        self.drain_samples_with_start(limit).1
    }

    /// Drain without awaiting and preserve the callback-relative frame start.
    pub fn drain_samples_with_start(&mut self, limit: usize) -> (u64, Vec<f32>) {
        let start = self.drain_frame_cursor;
        let mut samples = Vec::with_capacity(limit);
        while samples.len() < limit {
            match self.consumer.try_pop() {
                Some(sample) => samples.push(sample),
                None => break,
            }
        }
        self.drain_frame_cursor = self.drain_frame_cursor.saturating_add(samples.len() as u64);
        (start, samples)
    }

    /// Drop every currently buffered system sample at a pause boundary.
    pub fn discard_system_samples(&mut self) -> usize {
        let discarded = discard_buffered_samples(&mut self.consumer);
        self.drain_frame_cursor = self.drain_frame_cursor.saturating_add(discarded as u64);
        discarded
    }

    pub fn should_terminate(&self) -> bool {
        self._ctx.should_terminate.load(Ordering::Acquire)
    }
}

#[cfg(target_os = "macos")]
impl Stream for CoreAudioStream {
    type Item = f32;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Try to pop a sample from the ring buffer
        if let Some(sample) = self.consumer.try_pop() {
            return Poll::Ready(Some(sample));
        }

        // Check if we should terminate
        if self._ctx.should_terminate.load(Ordering::Acquire) {
            warn!("Stream terminating due to buffer pressure");
            return match self.consumer.try_pop() {
                Some(sample) => Poll::Ready(Some(sample)),
                None => Poll::Ready(None),
            };
        }

        // No data available, register waker and return pending
        {
            let mut state = self.waker_state.lock().unwrap();
            state.has_data = false;
            state.waker = Some(cx.waker().clone());
        }

        Poll::Pending
    }
}

#[cfg(target_os = "macos")]
impl Drop for CoreAudioStream {
    fn drop(&mut self) {
        info!("CoreAudioStream dropped, signaling termination");
        self._ctx.should_terminate.store(true, Ordering::Release);
    }
}

// Stub implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub struct CoreAudioCapture;

#[cfg(not(target_os = "macos"))]
pub struct CoreAudioStream;

#[cfg(not(target_os = "macos"))]
impl CoreAudioCapture {
    pub fn new() -> Result<Self> {
        Self::new_for_preference(None)
    }

    pub fn new_for_preference(_preferred_device: Option<&str>) -> Result<Self> {
        Err(anyhow::anyhow!("Core Audio is only supported on macOS"))
    }

    pub fn stream(self) -> Result<CoreAudioStream> {
        Err(anyhow::anyhow!("Core Audio is only supported on macOS"))
    }
}

#[cfg(not(target_os = "macos"))]
impl CoreAudioStream {
    pub fn signal_termination(&self) {}

    pub fn stop_capture(&mut self) -> std::result::Result<(), CoreAudioSetupError> {
        Ok(())
    }

    pub fn sample_rate(&self) -> u32 {
        0
    }

    pub fn drain_samples_with_start(&mut self, _limit: usize) -> (u64, Vec<f32>) {
        (0, Vec::new())
    }

    pub fn discard_system_samples(&mut self) -> usize {
        0
    }

    pub fn should_terminate(&self) -> bool {
        true
    }
}

#[cfg(not(target_os = "macos"))]
impl Stream for CoreAudioStream {
    type Item = f32;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn paused_system_audio_is_discarded_instead_of_replayed() {
        let ring = HeapRb::<f32>::new(8);
        let (mut producer, mut consumer) = ring.split();
        assert_eq!(producer.push_slice(&[11.0, 12.0, 13.0]), 3);
        assert_eq!(discard_buffered_samples(&mut consumer), 3);
        assert_eq!(consumer.try_pop(), None);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn callback_at_resume_boundary_cannot_publish_paused_samples() {
        let ring = HeapRb::<f32>::new(8);
        let (mut producer, mut consumer) = ring.split();
        let paused = AtomicBool::new(true);
        let should_terminate = AtomicBool::new(false);

        assert_eq!(
            push_callback_samples(&mut producer, &[41.0, 42.0], &paused, &should_terminate,)
                .unwrap(),
            0
        );
        paused.store(false, Ordering::Release);
        assert_eq!(
            push_callback_samples(&mut producer, &[99.0], &paused, &should_terminate).unwrap(),
            1
        );
        assert_eq!(consumer.try_pop(), Some(99.0));
        assert_eq!(consumer.try_pop(), None);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn any_partial_callback_write_terminalizes_capture() {
        let ring = HeapRb::<f32>::new(2);
        let (mut producer, _consumer) = ring.split();
        let paused = AtomicBool::new(false);
        let should_terminate = AtomicBool::new(false);

        assert!(
            push_callback_samples(&mut producer, &[1.0, 2.0, 3.0], &paused, &should_terminate,)
                .is_err()
        );
        assert!(should_terminate.load(Ordering::Acquire));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn intermittent_partial_callback_write_cannot_reset_loss() {
        let ring = HeapRb::<f32>::new(2);
        let (mut producer, mut consumer) = ring.split();
        let paused = AtomicBool::new(false);
        let should_terminate = AtomicBool::new(false);

        push_callback_samples(&mut producer, &[1.0], &paused, &should_terminate).unwrap();
        assert_eq!(consumer.try_pop(), Some(1.0));
        assert!(
            push_callback_samples(&mut producer, &[2.0, 3.0, 4.0], &paused, &should_terminate,)
                .is_err()
        );
        assert!(should_terminate.load(Ordering::Acquire));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn only_the_core_audio_permission_status_maps_to_permission_denied() {
        assert!(matches!(
            tap_creation_error(os::Error::from_be_bytes(*b"perm")),
            CoreAudioSetupError::PermissionDenied(_)
        ));
        assert!(matches!(
            tap_creation_error(os::Error::from_be_bytes(*b"what")),
            CoreAudioSetupError::CaptureFailed(_)
        ));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn default_output_device_errors_map_to_device_unavailable() {
        assert!(matches!(
            setup_error(
                SetupStage::DefaultOutputDevice,
                os::Error::from_be_bytes(*b"what")
            ),
            CoreAudioSetupError::DeviceUnavailable(_)
        ));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn output_device_detail_errors_map_to_device_unavailable() {
        assert!(matches!(
            setup_error(
                SetupStage::OutputDeviceDetails,
                os::Error::from_be_bytes(*b"what")
            ),
            CoreAudioSetupError::DeviceUnavailable(_)
        ));
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore] // Only run manually as it requires audio hardware
    fn test_core_audio_capture() {
        use futures_util::StreamExt;
        use std::sync::mpsc;
        use std::time::Duration;

        let (sender, receiver) = mpsc::sync_channel(1);
        let _worker = std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Vec<f32>> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(async {
                    let capture = CoreAudioCapture::new()?;
                    let mut stream = capture.stream()?;
                    info!("Stream sample rate: {} Hz", stream.sample_rate());

                    let mut samples = Vec::with_capacity(48_000);
                    while samples.len() < 48_000 {
                        let sample = stream.next().await.ok_or_else(|| {
                            anyhow::anyhow!("CoreAudio stream ended before one second was captured")
                        })?;
                        samples.push(sample);
                    }
                    Ok::<Vec<f32>, anyhow::Error>(samples)
                })
            })();
            let _ = sender.send(result);
        });

        let samples = receiver
        .recv_timeout(Duration::from_secs(15))
        .expect(
            "CoreAudio constructor/stream worker exceeded 15 seconds; play a known signal and verify macOS audio permission",
        )
        .expect("CoreAudio failed while a known signal should be playing");

        let peak = samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        let rms = (samples.iter().map(|sample| sample * sample).sum::<f32>()
            / samples.len() as f32)
            .sqrt();

        info!(
            "Collected {} samples (RMS {:.6}, peak {:.6})",
            samples.len(),
            rms,
            peak
        );
        assert!(
            rms > 0.001,
            "CoreAudio captured silence (RMS {rms:.6}); play a known signal and verify macOS audio permission"
        );
        assert!(
            peak > 0.01,
            "CoreAudio signal peak is too low ({peak:.6}); play a known signal and verify output volume"
        );
    }
}
