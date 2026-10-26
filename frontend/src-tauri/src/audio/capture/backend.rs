use crossbeam::queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const CAPTURE_CONTROL_MAX_MESSAGES: usize = 8;
pub const CAPTURE_REPLY_TIMEOUT: Duration = Duration::from_secs(5);
pub const TRACK_SINK_MAX_MESSAGES: usize = 64;
// Raw writers synchronously encode/fsync a 30-second checkpoint. A 64-message
// queue holds only 640ms at 10ms callbacks, less than a normal encode on Intel.
// Keep the existing byte budget; allow small callbacks to use that headroom.
pub const RAW_TRACK_SINK_MAX_MESSAGES: usize = 4096;
pub const TRACK_SINK_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const TIMELINE_SAMPLE_RATE: u32 = 48_000;
pub const CAPTURE_BLOCK_MAX_FRAMES: usize = 4096;
pub const CAPTURE_BLOCK_MAX_CHANNELS: u16 = 8;
pub const CAPTURE_BLOCK_MAX_SAMPLES: usize =
    CAPTURE_BLOCK_MAX_FRAMES * CAPTURE_BLOCK_MAX_CHANNELS as usize;
pub const CAPTURE_BLOCK_MAX_BYTES: usize = CAPTURE_BLOCK_MAX_SAMPLES * size_of::<f32>();
pub const CAPTURE_BLOCK_POOL_SLOTS: usize = TRACK_SINK_MAX_BYTES / CAPTURE_BLOCK_MAX_BYTES;

/// Shared active-time origin for microphone and system capture.
///
/// Both sources can start at different wall-clock moments (system tap startup
/// is asynchronous), so per-source callback counters cannot describe one
/// recording timeline. The timeline is read lock-free by callbacks and only
/// changed by the capture-owner thread at pause/resume boundaries.
#[derive(Debug)]
pub struct CaptureTimeline {
    origin: Instant,
    pause_started_nanos: AtomicU64,
    paused_nanos: AtomicU64,
    epoch: AtomicU64,
}

impl CaptureTimeline {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            origin: Instant::now(),
            pause_started_nanos: AtomicU64::new(0),
            paused_nanos: AtomicU64::new(0),
            epoch: AtomicU64::new(0),
        })
    }

    fn elapsed_nanos(&self) -> u64 {
        self.origin.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    pub fn pause(&self) {
        let now = self.elapsed_nanos();
        // Store one-based values so an origin-relative timestamp of zero is
        // still distinguishable from "not paused".
        let _ = self.pause_started_nanos.compare_exchange(
            0,
            now.saturating_add(1),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    pub fn resume(&self) {
        let now = self.elapsed_nanos();
        let started = self.pause_started_nanos.swap(0, Ordering::AcqRel);
        if started != 0 {
            self.paused_nanos.fetch_add(
                now.saturating_sub(started.saturating_sub(1)),
                Ordering::AcqRel,
            );
            self.epoch.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Return active-time frame position at `sample_rate` and current epoch.
    pub fn frame_at(&self, sample_rate: u32) -> (u64, u64) {
        if sample_rate == 0 {
            return (0, self.epoch());
        }
        let now = self.elapsed_nanos();
        let paused_total = self.paused_nanos.load(Ordering::Acquire);
        let paused_now = self
            .pause_started_nanos
            .load(Ordering::Acquire)
            .checked_sub(1)
            .map(|started| now.saturating_sub(started))
            .unwrap_or(0);
        let active_nanos = now.saturating_sub(paused_total.saturating_add(paused_now));
        let frames = (u128::from(active_nanos) * u128::from(sample_rate)
            / u128::from(1_000_000_000_u64))
        .min(u128::from(u64::MAX)) as u64;
        (frames, self.epoch())
    }
}

/// A fixed-size buffer owned by the capture callback until the owner thread
/// has copied the samples into the ordinary track sinks. The bounded queue is
/// the ownership record, so the callback never locks, allocates, or frees.
#[derive(Debug)]
struct CaptureBlockPool {
    free: ArrayQueue<Vec<f32>>,
}

impl CaptureBlockPool {
    fn new() -> Arc<Self> {
        let free = ArrayQueue::new(CAPTURE_BLOCK_POOL_SLOTS);
        for _ in 0..CAPTURE_BLOCK_POOL_SLOTS {
            free.push(vec![0.0; CAPTURE_BLOCK_MAX_SAMPLES])
                .expect("preallocated capture block pool has capacity");
        }
        Arc::new(Self { free })
    }

    fn acquire(self: &Arc<Self>) -> Option<PooledSamples> {
        self.free.pop().map(|storage| PooledSamples {
            pool: self.clone(),
            storage,
            len: 0,
        })
    }

    fn recycle(&self, mut storage: Vec<f32>) {
        storage.fill(0.0);
        if let Err(storage) = self.free.push(storage) {
            // A duplicate return would violate PooledSamples ownership. Do not
            // free the buffer on the callback thread while preserving the
            // bounded queue's no-blocking guarantee.
            std::mem::forget(storage);
        }
    }

    #[cfg(test)]
    fn available(&self) -> usize {
        self.free.len()
    }
}

#[derive(Debug)]
struct PooledSamples {
    pool: Arc<CaptureBlockPool>,
    storage: Vec<f32>,
    len: usize,
}

impl PooledSamples {
    fn copy_from_slice(&mut self, samples: &[f32]) -> Result<(), ()> {
        if samples.len() > self.storage.len() {
            return Err(());
        }
        self.storage[..samples.len()].copy_from_slice(samples);
        self.len = samples.len();
        Ok(())
    }

    fn copy_i16(&mut self, samples: &[i16]) -> Result<(), ()> {
        if samples.len() > self.storage.len() {
            return Err(());
        }
        for (destination, sample) in self
            .storage
            .iter_mut()
            .zip(samples.iter())
            .take(samples.len())
        {
            *destination = f32::from(*sample) / f32::from(i16::MAX);
        }
        self.len = samples.len();
        Ok(())
    }

    fn copy_i32(&mut self, samples: &[i32]) -> Result<(), ()> {
        if samples.len() > self.storage.len() {
            return Err(());
        }
        for (destination, sample) in self
            .storage
            .iter_mut()
            .zip(samples.iter())
            .take(samples.len())
        {
            *destination = *sample as f32 / i32::MAX as f32;
        }
        self.len = samples.len();
        Ok(())
    }

    fn as_slice(&self) -> &[f32] {
        &self.storage[..self.len]
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl Drop for PooledSamples {
    fn drop(&mut self) {
        let storage = std::mem::take(&mut self.storage);
        self.pool.recycle(storage);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Microphone,
    System,
}

/// Controls the live/downstream representation of one source.
///
/// Raw source samples are intentionally left untouched so a muted or
/// amplified recording remains available for later reprocessing.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControl {
    pub muted: bool,
    pub gain: f32,
}

impl Default for SourceControl {
    fn default() -> Self {
        Self {
            muted: false,
            gain: 1.0,
        }
    }
}

impl SourceControl {
    pub const MAX_GAIN: f32 = 4.0;

    pub fn validate(self) -> Result<(), CaptureCause> {
        if !self.gain.is_finite() || !(0.0..=Self::MAX_GAIN).contains(&self.gain) {
            return Err(CaptureCause::CaptureFailed(format!(
                "source gain must be finite and between 0 and {}",
                Self::MAX_GAIN
            )));
        }
        Ok(())
    }

    pub fn apply_to_downstream(self, samples: &mut [f32]) {
        if self.muted {
            samples.fill(0.0);
            return;
        }
        if (self.gain - 1.0).abs() < f32::EPSILON {
            return;
        }
        for sample in samples {
            *sample = (*sample * self.gain).clamp(-1.0, 1.0);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampQuality {
    HostAndSample,
    HostOnly,
    Relative,
    Contiguous,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackBlockKind {
    Samples,
    Mute,
    Gap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackSpan {
    pub start_frame: u64,
    pub frame_count: u32,
    pub epoch: u64,
    pub quality: TimestampQuality,
}

impl TrackSpan {
    pub const fn new(start_frame: u64, frame_count: u32) -> Self {
        Self {
            start_frame,
            frame_count,
            epoch: 0,
            quality: TimestampQuality::Contiguous,
        }
    }

    pub fn end_frame(self) -> u64 {
        self.start_frame.saturating_add(u64::from(self.frame_count))
    }
}

#[derive(Clone, Debug)]
pub struct TrackChunk {
    pub kind: TrackKind,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub span: TrackSpan,
    pub block_kind: TrackBlockKind,
}

impl TrackChunk {
    pub fn new(
        kind: TrackKind,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        span: TrackSpan,
        block_kind: TrackBlockKind,
    ) -> Self {
        Self {
            kind,
            samples,
            sample_rate,
            channels,
            span,
            block_kind,
        }
    }

    pub fn contiguous(kind: TrackKind, samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        let frame_count = if channels == 0 {
            0
        } else {
            samples.len() / channels as usize
        };
        Self::new(
            kind,
            samples,
            sample_rate,
            channels,
            TrackSpan::new(0, frame_count.min(u32::MAX as usize) as u32),
            TrackBlockKind::Samples,
        )
    }
}

#[derive(Debug)]
pub struct CaptureBlock {
    pub kind: TrackKind,
    pub native_start_frame: u64,
    pub native_frame_count: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub epoch: u64,
    pub quality: TimestampQuality,
    pub block_kind: TrackBlockKind,
    samples: PooledSamples,
}

impl CaptureBlock {
    pub fn samples(&self) -> &[f32] {
        self.samples.as_slice()
    }

    pub fn sample_len(&self) -> usize {
        self.samples.len()
    }
}

#[derive(Debug)]
pub struct SourceBlockReceiver {
    receiver: Receiver<CaptureBlock>,
}

impl SourceBlockReceiver {
    pub fn try_recv(&self) -> Result<CaptureBlock, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn recv(&self) -> Result<CaptureBlock, mpsc::RecvError> {
        self.receiver.recv()
    }
}

#[derive(Debug)]
pub struct PartialFanoutError {
    pub kind: TrackKind,
    pub first_sent: bool,
    pub cause: CaptureCause,
}

#[derive(Clone, Copy, Debug)]
pub struct CapturePublishContext {
    pub kind: TrackKind,
    pub sample_rate: u32,
    pub channels: u16,
    pub epoch: u64,
    pub quality: TimestampQuality,
    pub native_start_frame: Option<u64>,
}

fn add_signed_frame(frame: u64, offset: i64) -> u64 {
    if offset >= 0 {
        frame.saturating_add(offset as u64)
    } else {
        frame.saturating_sub(offset.unsigned_abs())
    }
}

impl SourceSinks {
    pub fn new(raw: TrackSink, downstream: TrackSink) -> Self {
        Self::with_timeline(raw, downstream, CaptureTimeline::new())
    }

    pub fn with_timeline(
        raw: TrackSink,
        downstream: TrackSink,
        timeline: Arc<CaptureTimeline>,
    ) -> Self {
        let pool = CaptureBlockPool::new();
        let (block_tx, block_rx) = mpsc::sync_channel(CAPTURE_BLOCK_POOL_SLOTS);
        Self {
            raw,
            downstream,
            block_tx,
            block_rx: Arc::new(Mutex::new(Some(block_rx))),
            pool,
            timeline,
            next_native_frame: Arc::new(AtomicU64::new(0)),
            implicit_epoch: Arc::new(AtomicU64::new(u64::MAX)),
            native_epoch: Arc::new(AtomicU64::new(u64::MAX)),
            native_offset: Arc::new(AtomicI64::new(0)),
        }
    }

    pub fn timeline(&self) -> Arc<CaptureTimeline> {
        self.timeline.clone()
    }

    pub fn take_block_receiver(&self) -> Option<SourceBlockReceiver> {
        self.block_rx
            .lock()
            .ok()
            .and_then(|mut receiver| receiver.take())
            .map(|receiver| SourceBlockReceiver { receiver })
    }

    pub fn publish_samples(
        &self,
        kind: TrackKind,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        epoch: u64,
        quality: TimestampQuality,
    ) -> Result<(), CaptureCause> {
        self.publish_samples_at(
            samples,
            CapturePublishContext {
                kind,
                sample_rate,
                channels,
                epoch,
                quality,
                native_start_frame: None,
            },
        )
    }

    pub fn publish_samples_at(
        &self,
        samples: &[f32],
        context: CapturePublishContext,
    ) -> Result<(), CaptureCause> {
        self.publish_with(samples.len(), context, |buffer, start, end| {
            buffer.copy_from_slice(&samples[start..end]).map_err(|_| ())
        })
    }

    pub fn publish_i16(
        &self,
        kind: TrackKind,
        samples: &[i16],
        sample_rate: u32,
        channels: u16,
        epoch: u64,
        quality: TimestampQuality,
    ) -> Result<(), CaptureCause> {
        self.publish_with(
            samples.len(),
            CapturePublishContext {
                kind,
                sample_rate,
                channels,
                epoch,
                quality,
                native_start_frame: None,
            },
            |buffer, start, end| buffer.copy_i16(&samples[start..end]).map_err(|_| ()),
        )
    }

    pub fn publish_i32(
        &self,
        kind: TrackKind,
        samples: &[i32],
        sample_rate: u32,
        channels: u16,
        epoch: u64,
        quality: TimestampQuality,
    ) -> Result<(), CaptureCause> {
        self.publish_with(
            samples.len(),
            CapturePublishContext {
                kind,
                sample_rate,
                channels,
                epoch,
                quality,
                native_start_frame: None,
            },
            |buffer, start, end| buffer.copy_i32(&samples[start..end]).map_err(|_| ()),
        )
    }

    fn publish_with<F>(
        &self,
        sample_len: usize,
        context: CapturePublishContext,
        copy: F,
    ) -> Result<(), CaptureCause>
    where
        F: FnMut(&mut PooledSamples, usize, usize) -> Result<(), ()>,
    {
        if context.sample_rate == 0
            || context.channels == 0
            || context.channels > CAPTURE_BLOCK_MAX_CHANNELS
        {
            return Err(CaptureCause::TrackBackpressure(context.kind));
        }
        let frame_count = sample_len / context.channels as usize;
        if frame_count == 0 || frame_count.saturating_mul(context.channels as usize) != sample_len {
            return Err(CaptureCause::TrackBackpressure(context.kind));
        }
        let timeline_epoch = self.timeline.epoch();
        // A zero epoch is the live callback convention: resolve it from the
        // shared pause/resume timeline. Non-zero epochs remain explicit for
        // callers that already carry their own source epoch (and for tests).
        let effective_epoch = if context.quality == TimestampQuality::Contiguous {
            context.epoch
        } else if context.epoch == 0 {
            timeline_epoch
        } else {
            context.epoch
        };
        let base_frame = self.resolve_start_frame(&context, frame_count as u64, effective_epoch);
        let mut offset = 0usize;
        let mut fill = copy;
        while offset < frame_count {
            let frames = (frame_count - offset).min(CAPTURE_BLOCK_MAX_FRAMES);
            let samples_start = offset * context.channels as usize;
            let samples_end = samples_start + frames * context.channels as usize;
            let mut pooled = self
                .pool
                .acquire()
                .ok_or(CaptureCause::TrackBackpressure(context.kind))?;
            fill(&mut pooled, samples_start, samples_end)
                .map_err(|_| CaptureCause::TrackBackpressure(context.kind))?;
            let native_start_frame = base_frame.saturating_add(offset as u64);
            let block = CaptureBlock {
                kind: context.kind,
                native_start_frame,
                native_frame_count: frames as u32,
                sample_rate: context.sample_rate,
                channels: context.channels,
                epoch: effective_epoch,
                quality: context.quality,
                block_kind: TrackBlockKind::Samples,
                samples: pooled,
            };
            self.block_tx.try_send(block).map_err(|error| match error {
                TrySendError::Full(_) => CaptureCause::TrackBackpressure(context.kind),
                TrySendError::Disconnected(_) => CaptureCause::CaptureOwnerDisconnected,
            })?;
            offset += frames;
        }
        Ok(())
    }

    fn resolve_start_frame(
        &self,
        context: &CapturePublishContext,
        frame_count: u64,
        effective_epoch: u64,
    ) -> u64 {
        if let Some(native_start_frame) = context.native_start_frame {
            if context.quality == TimestampQuality::Contiguous {
                return native_start_frame;
            }
            if self.native_epoch.load(Ordering::Acquire) != effective_epoch {
                let (timeline_frame, _) = self.timeline.frame_at(context.sample_rate);
                let difference = i128::from(timeline_frame) - i128::from(native_start_frame);
                let offset = if difference > i128::from(i64::MAX) {
                    i64::MAX
                } else if difference < i128::from(i64::MIN) {
                    i64::MIN
                } else {
                    difference as i64
                };
                self.native_offset.store(offset, Ordering::Release);
                self.native_epoch.store(effective_epoch, Ordering::Release);
            }
            return add_signed_frame(
                native_start_frame,
                self.native_offset.load(Ordering::Acquire),
            );
        }

        if context.quality != TimestampQuality::Contiguous
            && context.epoch == 0
            && self.implicit_epoch.load(Ordering::Acquire) != effective_epoch
        {
            let (timeline_frame, _) = self.timeline.frame_at(context.sample_rate);
            self.next_native_frame
                .store(timeline_frame, Ordering::Release);
            self.implicit_epoch
                .store(effective_epoch, Ordering::Release);
        }
        if context.quality != TimestampQuality::Contiguous && context.epoch == 0 {
            // A microphone replacement can be opened without a user pause.
            // Rebase only forward, so a recovery gap becomes explicit silence
            // while ordinary callback jitter cannot move the cursor backwards.
            let (timeline_frame, _) = self.timeline.frame_at(context.sample_rate);
            let mut cursor = self.next_native_frame.load(Ordering::Acquire);
            while timeline_frame > cursor {
                match self.next_native_frame.compare_exchange(
                    cursor,
                    timeline_frame,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(observed) => cursor = observed,
                }
            }
        }
        self.next_native_frame
            .fetch_add(frame_count, Ordering::AcqRel)
    }

    pub fn send_pair(
        &self,
        raw: TrackChunk,
        downstream: TrackChunk,
    ) -> Result<(), PartialFanoutError> {
        let kind = raw.kind;
        if let Err(cause) = self.raw.try_send(raw) {
            self.raw.mark_incomplete();
            return Err(PartialFanoutError {
                kind,
                first_sent: false,
                cause,
            });
        }
        // The raw source writer is the durable recording path. Downstream
        // transcription is best-effort, so a slow or disconnected consumer
        // must not invalidate an already queued recoverable audio block.
        let _ = self.downstream.try_send(downstream);
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureCapabilities {
    pub microphone: bool,
    pub system: bool,
}

#[derive(Clone, Debug)]
pub struct CaptureRequest {
    pub microphone_device: Option<String>,
    pub fallback_to_default_microphone: bool,
    pub system_device: Option<String>,
    pub capture_system: bool,
}

#[derive(Clone, Debug)]
pub enum CaptureCause {
    PermissionDenied(String),
    DeviceUnavailable(String),
    TrackBackpressure(TrackKind),
    CaptureReplyTimeout,
    CaptureOwnerDisconnected,
    UnsupportedCapability(TrackKind),
    CaptureFailed(String),
    PartialFanout {
        kind: TrackKind,
        first_sent: bool,
        cause: Box<CaptureCause>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureRuntimeEvent {
    MicrophoneSwitched {
        previous_device: String,
        current_device: String,
        reason: String,
    },
    SystemAudioDegraded {
        reason: String,
    },
}

#[derive(Clone, Debug)]
pub struct CaptureStopped {
    pub closed_sources: BTreeSet<TrackKind>,
}

pub trait CaptureBackend {
    fn capabilities(&self) -> CaptureCapabilities;
    fn start(&mut self, request: CaptureRequest, sinks: TrackSinks) -> Result<(), CaptureCause>;
    fn opened_sources(&self) -> BTreeSet<TrackKind>;
    fn set_source_control(
        &mut self,
        kind: TrackKind,
        control: SourceControl,
    ) -> Result<(), CaptureCause> {
        control.validate()?;
        Err(CaptureCause::UnsupportedCapability(kind))
    }
    fn pause(&mut self) -> Result<(), CaptureCause>;
    fn resume(&mut self) -> Result<(), CaptureCause>;
    fn pump(&mut self) -> Result<Option<CaptureRuntimeEvent>, CaptureCause>;
    fn stop(&mut self) -> Result<CaptureStopped, CaptureCause>;
}

pub enum CaptureControl {
    SetSourceControl {
        kind: TrackKind,
        control: SourceControl,
        reply: SyncSender<Result<(), CaptureCause>>,
    },
    Pause {
        reply: SyncSender<Result<(), CaptureCause>>,
    },
    Resume {
        reply: SyncSender<Result<(), CaptureCause>>,
    },
    Stop {
        reply: SyncSender<Result<CaptureStopped, CaptureCause>>,
    },
}

#[derive(Clone)]
pub struct TrackSink {
    kind: TrackKind,
    sender: SyncSender<TrackChunk>,
    max_bytes: usize,
    in_flight_bytes: Arc<AtomicUsize>,
    terminal_incomplete: Arc<std::sync::atomic::AtomicBool>,
}

pub struct TrackReceiver {
    receiver: Receiver<TrackChunk>,
    in_flight_bytes: Arc<AtomicUsize>,
    terminal_incomplete: Arc<std::sync::atomic::AtomicBool>,
}

impl TrackSink {
    pub fn new(kind: TrackKind) -> (Self, TrackReceiver) {
        Self::with_message_capacity(kind, TRACK_SINK_MAX_MESSAGES)
    }

    pub fn for_checkpoint_writer(kind: TrackKind) -> (Self, TrackReceiver) {
        Self::with_message_capacity(kind, RAW_TRACK_SINK_MAX_MESSAGES)
    }

    fn with_message_capacity(kind: TrackKind, capacity: usize) -> (Self, TrackReceiver) {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let in_flight_bytes = Arc::new(AtomicUsize::new(0));
        let terminal_incomplete = Arc::new(std::sync::atomic::AtomicBool::new(false));
        (
            Self {
                kind,
                sender,
                max_bytes: TRACK_SINK_MAX_BYTES,
                in_flight_bytes: in_flight_bytes.clone(),
                terminal_incomplete: terminal_incomplete.clone(),
            },
            TrackReceiver {
                receiver,
                in_flight_bytes,
                terminal_incomplete,
            },
        )
    }

    pub fn try_send(&self, chunk: TrackChunk) -> Result<(), CaptureCause> {
        if self.terminal_incomplete.load(Ordering::Acquire) {
            return Err(CaptureCause::CaptureOwnerDisconnected);
        }
        let chunk_bytes = chunk.samples.len().saturating_mul(size_of::<f32>());
        if chunk.kind != self.kind || chunk_bytes > self.max_bytes {
            return Err(CaptureCause::TrackBackpressure(self.kind));
        }
        let mut current = self.in_flight_bytes.load(Ordering::Acquire);
        loop {
            let next = current
                .checked_add(chunk_bytes)
                .filter(|next| *next <= self.max_bytes)
                .ok_or(CaptureCause::TrackBackpressure(self.kind))?;
            match self.in_flight_bytes.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
        self.sender.try_send(chunk).map_err(|error| {
            self.in_flight_bytes
                .fetch_sub(chunk_bytes, Ordering::AcqRel);
            match error {
                TrySendError::Full(_) => CaptureCause::TrackBackpressure(self.kind),
                TrySendError::Disconnected(_) => CaptureCause::CaptureOwnerDisconnected,
            }
        })
    }

    pub fn mark_incomplete(&self) {
        self.terminal_incomplete.store(true, Ordering::Release);
    }
}

impl TrackReceiver {
    pub fn recv(&self) -> Result<TrackChunk, mpsc::RecvError> {
        let chunk = self.receiver.recv()?;
        self.in_flight_bytes.fetch_sub(
            chunk.samples.len().saturating_mul(size_of::<f32>()),
            Ordering::AcqRel,
        );
        Ok(chunk)
    }

    pub fn is_incomplete(&self) -> bool {
        self.terminal_incomplete.load(Ordering::Acquire)
    }

    pub fn mark_incomplete(&self) {
        self.terminal_incomplete.store(true, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct SourceSinks {
    pub raw: TrackSink,
    pub downstream: TrackSink,
    block_tx: SyncSender<CaptureBlock>,
    block_rx: Arc<Mutex<Option<Receiver<CaptureBlock>>>>,
    pool: Arc<CaptureBlockPool>,
    timeline: Arc<CaptureTimeline>,
    next_native_frame: Arc<AtomicU64>,
    implicit_epoch: Arc<AtomicU64>,
    native_epoch: Arc<AtomicU64>,
    native_offset: Arc<AtomicI64>,
}

pub struct TrackSinks {
    pub microphone: SourceSinks,
    pub system: Option<SourceSinks>,
}

pub struct CaptureThreadHandle {
    pub control_tx: SyncSender<CaptureControl>,
    pub join: Option<JoinHandle<()>>,
    pub opened_sources: BTreeSet<TrackKind>,
    owner_exit_cause: Arc<std::sync::Mutex<Option<CaptureCause>>>,
    owner_exit_rx: Option<Receiver<CaptureCause>>,
    runtime_event_rx: Option<Receiver<CaptureRuntimeEvent>>,
    shutdown_requested: Arc<std::sync::atomic::AtomicBool>,
    #[cfg(test)]
    join_calls: Option<Arc<AtomicUsize>>,
}

fn retain_runtime_event(pending: &mut Option<CaptureRuntimeEvent>, incoming: CaptureRuntimeEvent) {
    match pending {
        None => *pending = Some(incoming),
        Some(CaptureRuntimeEvent::SystemAudioDegraded { .. })
            if matches!(incoming, CaptureRuntimeEvent::MicrophoneSwitched { .. }) =>
        {
            *pending = Some(incoming);
        }
        Some(_) => {}
    }
}

fn try_forward_runtime_event(
    sender: &SyncSender<CaptureRuntimeEvent>,
    pending: &mut Option<CaptureRuntimeEvent>,
) {
    let Some(event) = pending.take() else {
        return;
    };
    if let Err(TrySendError::Full(event)) = sender.try_send(event) {
        *pending = Some(event);
    }
}

struct OwnerExitGuard {
    sender: Option<SyncSender<CaptureCause>>,
}

impl OwnerExitGuard {
    fn report(&mut self, cause: CaptureCause) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.try_send(cause);
        }
    }

    fn disarm(&mut self) {
        self.sender.take();
    }
}

impl Drop for OwnerExitGuard {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.try_send(CaptureCause::CaptureOwnerDisconnected);
        }
    }
}

impl CaptureThreadHandle {
    pub fn spawn(request: CaptureRequest, sinks: TrackSinks) -> Result<Self, CaptureCause> {
        Self::spawn_owner(request, sinks, super::macos::MacosCaptureBackend::new)
    }

    #[cfg(test)]
    pub(crate) fn spawn_with_backend<B, F>(
        request: CaptureRequest,
        sinks: TrackSinks,
        backend_factory: F,
    ) -> Result<Self, CaptureCause>
    where
        B: CaptureBackend + 'static,
        F: FnOnce() -> B + Send + 'static,
    {
        Self::spawn_owner(request, sinks, backend_factory)
    }

    fn spawn_owner<B, F>(
        request: CaptureRequest,
        sinks: TrackSinks,
        backend_factory: F,
    ) -> Result<Self, CaptureCause>
    where
        B: CaptureBackend + 'static,
        F: FnOnce() -> B + Send + 'static,
    {
        let (control_tx, control_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);
        let (startup_tx, startup_rx) = mpsc::sync_channel(1);
        let owner_exit_cause = Arc::new(std::sync::Mutex::new(None));
        let owner_exit_cause_for_thread = owner_exit_cause.clone();
        let (owner_exit_tx, owner_exit_rx) = mpsc::sync_channel(1);
        let (runtime_event_tx, runtime_event_rx) = mpsc::sync_channel(CAPTURE_CONTROL_MAX_MESSAGES);
        let shutdown_requested = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let owner_shutdown_requested = shutdown_requested.clone();
        let join = std::thread::Builder::new()
            .name("meetily-capture-owner".into())
            .spawn(move || {
                let mut owner_exit = OwnerExitGuard {
                    sender: Some(owner_exit_tx),
                };
                let mut backend = backend_factory();
                let capabilities = backend.capabilities();
                let mut request = request;
                let mut sinks = sinks;
                let started = if !capabilities.microphone {
                    Err(CaptureCause::UnsupportedCapability(TrackKind::Microphone))
                } else {
                    if request.capture_system && !capabilities.system {
                        request.capture_system = false;
                        sinks.system.take();
                    }
                    backend
                        .start(request, sinks)
                        .map(|()| backend.opened_sources())
                };
                let can_run = started.is_ok();
                let _ = startup_tx.send(started);
                if !can_run {
                    owner_exit.disarm();
                    return;
                }
                let mut pending_runtime_event = None;
                loop {
                    if owner_shutdown_requested.load(Ordering::Acquire) {
                        let _ = backend.stop();
                        owner_exit.disarm();
                        break;
                    }
                    match control_rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(control) => match control {
                            CaptureControl::SetSourceControl {
                                kind,
                                control,
                                reply,
                            } => {
                                let _ = reply.send(backend.set_source_control(kind, control));
                            }
                            CaptureControl::Pause { reply } => {
                                let _ = reply.send(backend.pause());
                            }
                            CaptureControl::Resume { reply } => {
                                let _ = reply.send(backend.resume());
                            }
                            CaptureControl::Stop { reply } => {
                                let _ = reply.send(backend.stop());
                                owner_exit.disarm();
                                break;
                            }
                        },
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            try_forward_runtime_event(
                                &runtime_event_tx,
                                &mut pending_runtime_event,
                            );
                            match backend.pump() {
                                Ok(Some(event)) => {
                                    retain_runtime_event(&mut pending_runtime_event, event);
                                    try_forward_runtime_event(
                                        &runtime_event_tx,
                                        &mut pending_runtime_event,
                                    );
                                }
                                Ok(None) => {}
                                Err(cause) => {
                                    if let Ok(mut slot) = owner_exit_cause_for_thread.lock() {
                                        *slot = Some(cause.clone());
                                    }
                                    owner_exit.report(cause);
                                    let _ = backend.stop();
                                    break;
                                }
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            let _ = backend.stop();
                            break;
                        }
                    }
                }
            })
            .map_err(|error| CaptureCause::CaptureFailed(error.to_string()))?;
        let startup = startup_rx.recv_timeout(CAPTURE_REPLY_TIMEOUT);
        match startup {
            Ok(Ok(opened_sources)) => Ok(Self {
                control_tx,
                join: Some(join),
                opened_sources,
                owner_exit_cause,
                owner_exit_rx: Some(owner_exit_rx),
                runtime_event_rx: Some(runtime_event_rx),
                shutdown_requested,
                #[cfg(test)]
                join_calls: None,
            }),
            Ok(Err(cause)) => {
                let _ = join.join();
                Err(cause)
            }
            Err(error) => {
                drop(control_tx);
                let _ = join.join();
                Err(match error {
                    mpsc::RecvTimeoutError::Timeout => CaptureCause::CaptureReplyTimeout,
                    mpsc::RecvTimeoutError::Disconnected => CaptureCause::CaptureOwnerDisconnected,
                })
            }
        }
    }

    pub fn join_once(&mut self) -> Result<(), CaptureCause> {
        #[cfg(test)]
        if let Some(join_calls) = &self.join_calls {
            join_calls.fetch_add(1, Ordering::AcqRel);
        }
        self.join
            .take()
            .ok_or(CaptureCause::CaptureOwnerDisconnected)?
            .join()
            .map_err(|_| CaptureCause::CaptureOwnerDisconnected)
    }

    #[cfg(test)]
    pub(crate) fn track_join_calls(&mut self, join_calls: Arc<AtomicUsize>) {
        self.join_calls = Some(join_calls);
    }

    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Release);
    }

    pub fn set_source_control(
        &self,
        kind: TrackKind,
        control: SourceControl,
    ) -> Result<(), CaptureCause> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.control_tx
            .try_send(CaptureControl::SetSourceControl {
                kind,
                control,
                reply: reply_tx,
            })
            .map_err(|error| CaptureCause::CaptureFailed(error.to_string()))?;
        match reply_rx.recv_timeout(CAPTURE_REPLY_TIMEOUT) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(CaptureCause::CaptureReplyTimeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(CaptureCause::CaptureOwnerDisconnected)
            }
        }
    }

    pub fn take_owner_exit_cause(&self) -> Option<CaptureCause> {
        self.owner_exit_cause
            .lock()
            .ok()
            .and_then(|mut cause| cause.take())
    }

    pub fn take_owner_exit_receiver(&mut self) -> Option<Receiver<CaptureCause>> {
        self.owner_exit_rx.take()
    }

    pub fn take_runtime_event_receiver(&mut self) -> Option<Receiver<CaptureRuntimeEvent>> {
        self.runtime_event_rx.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    #[test]
    fn capture_timeline_freezes_during_pause_and_advances_epoch() {
        let timeline = CaptureTimeline::new();
        std::thread::sleep(Duration::from_millis(5));
        let (before_pause, initial_epoch) = timeline.frame_at(48_000);
        assert_eq!(initial_epoch, 0);

        timeline.pause();
        let (pause_start, pause_epoch) = timeline.frame_at(48_000);
        std::thread::sleep(Duration::from_millis(20));
        let (pause_end, still_paused_epoch) = timeline.frame_at(48_000);
        assert_eq!(pause_epoch, still_paused_epoch);
        assert!(pause_end.saturating_sub(pause_start) < 240);

        timeline.resume();
        std::thread::sleep(Duration::from_millis(5));
        let (after_resume, resumed_epoch) = timeline.frame_at(48_000);
        assert_eq!(resumed_epoch, 1);
        assert!(after_resume >= before_pause);
        assert!(after_resume > pause_end);
    }

    #[derive(Clone, Copy)]
    enum FakeMode {
        Healthy,
        StartFails,
        PauseTimesOut,
        PumpFails,
        StopFails,
        SystemDegrades,
        MicrophoneSwitches,
        RuntimeEventsSaturate,
        SystemUnavailable,
        Panics,
    }

    struct FakeCaptureBackend {
        mode: FakeMode,
        events: Arc<Mutex<Vec<&'static str>>>,
        dropped: Arc<AtomicBool>,
        sinks: Option<TrackSinks>,
        expected: BTreeSet<TrackKind>,
        runtime_events: VecDeque<CaptureRuntimeEvent>,
        runtime_events_drained: Option<SyncSender<()>>,
    }

    impl FakeCaptureBackend {
        fn new(
            mode: FakeMode,
            events: Arc<Mutex<Vec<&'static str>>>,
            dropped: Arc<AtomicBool>,
        ) -> Self {
            let runtime_events = match mode {
                FakeMode::SystemDegrades => {
                    VecDeque::from([CaptureRuntimeEvent::SystemAudioDegraded {
                        reason: "system stream stopped".into(),
                    }])
                }
                FakeMode::MicrophoneSwitches => {
                    VecDeque::from([CaptureRuntimeEvent::MicrophoneSwitched {
                        previous_device: "Disconnected microphone".into(),
                        current_device: "Default microphone".into(),
                        reason: "active microphone became unavailable".into(),
                    }])
                }
                FakeMode::RuntimeEventsSaturate => {
                    let mut events = (0..=CAPTURE_CONTROL_MAX_MESSAGES)
                        .map(|index| CaptureRuntimeEvent::SystemAudioDegraded {
                            reason: format!("degraded-{index}"),
                        })
                        .collect::<VecDeque<_>>();
                    events.push_back(CaptureRuntimeEvent::MicrophoneSwitched {
                        previous_device: "Previous microphone".into(),
                        current_device: "Current microphone".into(),
                        reason: "default route changed".into(),
                    });
                    events.push_back(CaptureRuntimeEvent::SystemAudioDegraded {
                        reason: "lower priority".into(),
                    });
                    events
                }
                _ => VecDeque::new(),
            };
            Self {
                mode,
                events,
                dropped,
                sinks: None,
                expected: BTreeSet::new(),
                runtime_events,
                runtime_events_drained: None,
            }
        }

        fn event(&self, event: &'static str) {
            self.events.lock().unwrap().push(event);
        }
    }

    impl Drop for FakeCaptureBackend {
        fn drop(&mut self) {
            self.event("drop");
            self.dropped.store(true, Ordering::Release);
        }
    }

    impl CaptureBackend for FakeCaptureBackend {
        fn capabilities(&self) -> CaptureCapabilities {
            CaptureCapabilities {
                microphone: true,
                system: !matches!(self.mode, FakeMode::SystemUnavailable),
            }
        }

        fn start(
            &mut self,
            request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<(), CaptureCause> {
            self.event("start");
            if matches!(self.mode, FakeMode::StartFails) {
                return Err(CaptureCause::CaptureFailed("fake start".into()));
            }
            self.expected.insert(TrackKind::Microphone);
            if request.capture_system && !matches!(self.mode, FakeMode::SystemDegrades) {
                self.expected.insert(TrackKind::System);
            }
            self.sinks = Some(sinks);
            Ok(())
        }

        fn opened_sources(&self) -> BTreeSet<TrackKind> {
            self.expected.clone()
        }

        fn pause(&mut self) -> Result<(), CaptureCause> {
            self.event("pause");
            if matches!(self.mode, FakeMode::PauseTimesOut) {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(())
        }

        fn resume(&mut self) -> Result<(), CaptureCause> {
            self.event("resume");
            Ok(())
        }

        fn pump(&mut self) -> Result<Option<CaptureRuntimeEvent>, CaptureCause> {
            if matches!(self.mode, FakeMode::Panics) {
                panic!("fake owner panic");
            } else if matches!(self.mode, FakeMode::PumpFails) {
                Err(CaptureCause::CaptureFailed("fake pump".into()))
            } else {
                let event = self.runtime_events.pop_front();
                if event.is_none() {
                    if let Some(drained) = self.runtime_events_drained.take() {
                        let _ = drained.send(());
                    }
                }
                Ok(event)
            }
        }

        fn stop(&mut self) -> Result<CaptureStopped, CaptureCause> {
            self.event("stop");
            self.sinks.take();
            if matches!(self.mode, FakeMode::StopFails) {
                return Err(CaptureCause::CaptureFailed("fake stop".into()));
            }
            Ok(CaptureStopped {
                closed_sources: self.expected.clone(),
            })
        }
    }

    fn fake_request_and_sinks() -> (CaptureRequest, TrackSinks) {
        let (raw, _raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        (
            CaptureRequest {
                microphone_device: None,
                fallback_to_default_microphone: false,
                system_device: None,
                capture_system: false,
            },
            TrackSinks {
                microphone: SourceSinks::new(raw, downstream),
                system: None,
            },
        )
    }

    fn spawn_fake(
        mode: FakeMode,
        events: Arc<Mutex<Vec<&'static str>>>,
        dropped: Arc<AtomicBool>,
    ) -> Result<CaptureThreadHandle, CaptureCause> {
        let (request, sinks) = fake_request_and_sinks();
        CaptureThreadHandle::spawn_with_backend(request, sinks, move || {
            FakeCaptureBackend::new(mode, events, dropped)
        })
    }

    #[test]
    fn partial_start_failure_is_joined_and_never_detached() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        assert!(spawn_fake(FakeMode::StartFails, events.clone(), dropped.clone()).is_err());
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(*events.lock().unwrap(), vec!["start", "drop"]);
    }

    #[test]
    fn every_control_has_a_capacity_one_ack_and_owner_is_joined_once() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::Healthy, events.clone(), dropped.clone()).unwrap();
        for command in ["pause", "resume"] {
            let (reply, receiver) = mpsc::sync_channel(1);
            let message = if command == "pause" {
                CaptureControl::Pause { reply }
            } else {
                CaptureControl::Resume { reply }
            };
            handle.control_tx.send(message).unwrap();
            receiver
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
        }
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        let stopped = receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert_eq!(
            stopped.closed_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
        handle.join_once().unwrap();
        assert!(handle.join_once().is_err());
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(
            *events.lock().unwrap(),
            vec!["start", "pause", "resume", "stop", "drop"]
        );
    }

    #[test]
    fn delayed_reply_is_observable_as_a_timeout_without_detaching_owner() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::PauseTimesOut, events, dropped.clone()).unwrap();
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Pause { reply })
            .unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(5)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn capture_thread_exit_disconnects_control_and_remains_joinable() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::PumpFails, events, dropped.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let (reply, _) = mpsc::sync_channel(1);
        assert!(handle
            .control_tx
            .try_send(CaptureControl::Pause { reply })
            .is_err());
        handle.join_once().unwrap();
        assert!(matches!(
            handle.take_owner_exit_cause(),
            Some(CaptureCause::CaptureFailed(message)) if message == "fake pump"
        ));
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn pump_failure_is_reported_and_owner_remains_joinable() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::PumpFails, events, dropped).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        handle.join_once().unwrap();
        assert!(matches!(
            handle.take_owner_exit_cause(),
            Some(CaptureCause::CaptureFailed(message)) if message == "fake pump"
        ));
    }

    #[test]
    fn system_degradation_event_keeps_owner_running() {
        let (microphone, _microphone_raw, _microphone_downstream) =
            fake_source_sinks(TrackKind::Microphone);
        let (system, _system_raw, _system_downstream) = fake_source_sinks(TrackKind::System);
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = CaptureThreadHandle::spawn_with_backend(
            CaptureRequest {
                microphone_device: None,
                fallback_to_default_microphone: false,
                system_device: None,
                capture_system: true,
            },
            TrackSinks {
                microphone,
                system: Some(system),
            },
            move || FakeCaptureBackend::new(FakeMode::SystemDegrades, events, dropped),
        )
        .unwrap();
        let runtime_event_rx = handle.take_runtime_event_receiver().unwrap();

        assert_eq!(
            handle.opened_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
        assert_eq!(
            runtime_event_rx
                .recv_timeout(CAPTURE_REPLY_TIMEOUT)
                .unwrap(),
            CaptureRuntimeEvent::SystemAudioDegraded {
                reason: "system stream stopped".into(),
            }
        );
        assert!(handle.join.is_some());

        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(CAPTURE_REPLY_TIMEOUT)
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
    }

    #[test]
    fn microphone_switch_event_preserves_the_capture_owner() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::MicrophoneSwitches, events, dropped).unwrap();
        let runtime_event_rx = handle.take_runtime_event_receiver().unwrap();

        assert_eq!(
            runtime_event_rx
                .recv_timeout(CAPTURE_REPLY_TIMEOUT)
                .unwrap(),
            CaptureRuntimeEvent::MicrophoneSwitched {
                previous_device: "Disconnected microphone".into(),
                current_device: "Default microphone".into(),
                reason: "active microphone became unavailable".into(),
            }
        );
        assert!(handle.join.is_some());

        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(CAPTURE_REPLY_TIMEOUT)
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
    }

    #[test]
    fn saturated_runtime_event_channel_keeps_owner_running_and_forwards_retained_route() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (drained_tx, drained_rx) = mpsc::sync_channel(1);
        let (request, sinks) = fake_request_and_sinks();
        let mut handle = CaptureThreadHandle::spawn_with_backend(request, sinks, move || {
            let mut backend =
                FakeCaptureBackend::new(FakeMode::RuntimeEventsSaturate, events, dropped);
            backend.runtime_events_drained = Some(drained_tx);
            backend
        })
        .unwrap();
        let runtime_event_rx = handle.take_runtime_event_receiver().unwrap();

        // Keep the receiver blocked until all generated events have traversed
        // the owner loop. A sleep cannot establish channel saturation on CI:
        // early draining forwards degraded-8 before route coalescing happens.
        drained_rx.recv_timeout(CAPTURE_REPLY_TIMEOUT).unwrap();
        for _ in 0..CAPTURE_CONTROL_MAX_MESSAGES {
            assert!(matches!(
                runtime_event_rx
                    .recv_timeout(CAPTURE_REPLY_TIMEOUT)
                    .unwrap(),
                CaptureRuntimeEvent::SystemAudioDegraded { .. }
            ));
        }
        assert_eq!(
            runtime_event_rx
                .recv_timeout(CAPTURE_REPLY_TIMEOUT)
                .unwrap(),
            CaptureRuntimeEvent::MicrophoneSwitched {
                previous_device: "Previous microphone".into(),
                current_device: "Current microphone".into(),
                reason: "default route changed".into(),
            }
        );
        assert!(handle.join.is_some());

        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(CAPTURE_REPLY_TIMEOUT)
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
    }

    #[test]
    fn default_system_capture_uses_no_device_override() {
        let (microphone, _microphone_raw, _microphone_downstream) =
            fake_source_sinks(TrackKind::Microphone);
        let (system, _system_raw, _system_downstream) = fake_source_sinks(TrackKind::System);
        let request = CaptureRequest {
            microphone_device: None,
            fallback_to_default_microphone: false,
            system_device: None,
            capture_system: true,
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = CaptureThreadHandle::spawn_with_backend(
            request,
            TrackSinks {
                microphone,
                system: Some(system),
            },
            move || FakeCaptureBackend::new(FakeMode::Healthy, events, dropped),
        )
        .unwrap();
        assert_eq!(
            handle.opened_sources,
            BTreeSet::from([TrackKind::Microphone, TrackKind::System])
        );
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap()
                .closed_sources,
            handle.opened_sources
        );
        handle.join_once().unwrap();
    }

    #[test]
    fn preferred_system_capture_keeps_the_requested_device_preference() {
        let (microphone, _microphone_raw, _microphone_downstream) =
            fake_source_sinks(TrackKind::Microphone);
        let (system, _system_raw, _system_downstream) = fake_source_sinks(TrackKind::System);
        let request = CaptureRequest {
            microphone_device: None,
            fallback_to_default_microphone: false,
            system_device: Some("Preferred output".into()),
            capture_system: true,
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = CaptureThreadHandle::spawn_with_backend(
            request,
            TrackSinks {
                microphone,
                system: Some(system),
            },
            move || FakeCaptureBackend::new(FakeMode::Healthy, events, dropped),
        )
        .unwrap();
        assert!(handle.opened_sources.contains(&TrackKind::System));
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
    }

    #[test]
    fn unavailable_requested_system_capture_falls_back_to_microphone_only() {
        let (microphone, _microphone_raw, _microphone_downstream) =
            fake_source_sinks(TrackKind::Microphone);
        let (system, _system_raw, _system_downstream) = fake_source_sinks(TrackKind::System);
        let events = Arc::new(Mutex::new(Vec::new()));
        let inspected_events = events.clone();
        let dropped = Arc::new(AtomicBool::new(false));
        let result = CaptureThreadHandle::spawn_with_backend(
            CaptureRequest {
                microphone_device: None,
                fallback_to_default_microphone: false,
                system_device: None,
                capture_system: true,
            },
            TrackSinks {
                microphone,
                system: Some(system),
            },
            move || FakeCaptureBackend::new(FakeMode::SystemUnavailable, events, dropped),
        );
        let mut handle = result.unwrap();
        assert_eq!(
            handle.opened_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
        assert_eq!(
            *inspected_events.lock().unwrap(),
            vec!["start", "stop", "drop"]
        );
    }

    #[test]
    fn explicit_microphone_only_request_opens_no_system_source() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::Healthy, events, dropped).unwrap();
        assert_eq!(
            handle.opened_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        handle.join_once().unwrap();
    }

    fn fake_source_sinks(kind: TrackKind) -> (SourceSinks, TrackReceiver, TrackReceiver) {
        let (raw, raw_receiver) = TrackSink::new(kind);
        let (downstream, downstream_receiver) = TrackSink::new(kind);
        (
            SourceSinks::new(raw, downstream),
            raw_receiver,
            downstream_receiver,
        )
    }

    #[test]
    fn capture_block_pool_splits_oversized_callbacks_and_recycles_without_allocating() {
        let (raw, _raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let sinks = SourceSinks::new(raw, downstream);
        let receiver = sinks.take_block_receiver().unwrap();
        let sample_count = (CAPTURE_BLOCK_MAX_FRAMES + 17) * 2;
        let samples = vec![0.25; sample_count];

        sinks
            .publish_samples(
                TrackKind::Microphone,
                &samples,
                48_000,
                2,
                4,
                TimestampQuality::Relative,
            )
            .unwrap();
        let first = receiver.recv().unwrap();
        let second = receiver.recv().unwrap();
        assert_eq!(first.native_frame_count, CAPTURE_BLOCK_MAX_FRAMES as u32);
        assert_eq!(second.native_start_frame, CAPTURE_BLOCK_MAX_FRAMES as u64);
        assert_eq!(second.native_frame_count, 17);
        assert_eq!(first.epoch, 4);
        assert_eq!(sinks.pool.available(), CAPTURE_BLOCK_POOL_SLOTS - 2);
        drop(first);
        drop(second);
        assert_eq!(sinks.pool.available(), CAPTURE_BLOCK_POOL_SLOTS);
    }

    #[test]
    fn implicit_source_cursor_rebases_after_elapsed_recovery_gap() {
        let (raw, _raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let sinks = SourceSinks::with_timeline(raw, downstream, CaptureTimeline::new());
        let receiver = sinks.take_block_receiver().unwrap();

        sinks
            .publish_samples(
                TrackKind::Microphone,
                &[0.1, 0.2, 0.3, 0.4],
                48_000,
                1,
                0,
                TimestampQuality::Relative,
            )
            .unwrap();
        let first = receiver.recv().unwrap();
        let first_end = first.native_start_frame + u64::from(first.native_frame_count);

        // Simulate a replacement stream whose local cursor starts over after
        // a short device outage. The shared timeline must win over that reset.
        sinks.next_native_frame.store(0, Ordering::Release);
        std::thread::sleep(Duration::from_millis(5));
        sinks
            .publish_samples(
                TrackKind::Microphone,
                &[0.5, 0.6, 0.7, 0.8],
                48_000,
                1,
                0,
                TimestampQuality::Relative,
            )
            .unwrap();
        let replacement = receiver.recv().unwrap();
        assert!(replacement.native_start_frame > first_end);
    }

    #[test]
    fn capture_block_pool_recycles_under_bounded_contention() {
        let pool = CaptureBlockPool::new();
        let worker_pool = pool.clone();
        let worker = std::thread::spawn(move || {
            for _ in 0..10_000 {
                if let Some(block) = worker_pool.acquire() {
                    drop(block);
                }
            }
        });
        for _ in 0..10_000 {
            if let Some(block) = pool.acquire() {
                drop(block);
            }
        }
        worker.join().unwrap();
        assert_eq!(pool.available(), CAPTURE_BLOCK_POOL_SLOTS);
    }

    #[test]
    fn downstream_backpressure_keeps_the_durable_raw_track() {
        let (raw, raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        drop(downstream_receiver);
        let sinks = SourceSinks::new(raw, downstream);
        let raw_chunk = TrackChunk::contiguous(TrackKind::Microphone, vec![0.1; 4], 48_000, 1);
        let downstream_chunk =
            TrackChunk::contiguous(TrackKind::Microphone, vec![0.1; 4], 48_000, 1);

        sinks.send_pair(raw_chunk, downstream_chunk).unwrap();
        assert_eq!(raw_receiver.recv().unwrap().samples.len(), 4);
    }

    #[test]
    fn stop_failure_is_acknowledged_before_the_owner_is_joined() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::StopFails, events, dropped.clone()).unwrap();
        let (reply, receiver) = mpsc::sync_channel(1);
        handle
            .control_tx
            .send(CaptureControl::Stop { reply })
            .unwrap();
        assert!(matches!(
            receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
            Err(CaptureCause::CaptureFailed(_))
        ));
        handle.join_once().unwrap();
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn track_sink_rejects_wrong_source_and_oversize_chunks() {
        let (sink, _receiver) = TrackSink::new(TrackKind::Microphone);
        assert!(matches!(
            sink.try_send(TrackChunk {
                kind: TrackKind::System,
                samples: vec![0.0],
                sample_rate: 48_000,
                channels: 1,
                span: TrackSpan::new(0, 1),
                block_kind: TrackBlockKind::Samples,
            }),
            Err(CaptureCause::TrackBackpressure(TrackKind::Microphone))
        ));
        assert!(matches!(
            sink.try_send(TrackChunk {
                kind: TrackKind::Microphone,
                samples: vec![0.0; TRACK_SINK_MAX_BYTES / 4 + 1],
                sample_rate: 48_000,
                channels: 1,
                span: TrackSpan::new(0, (TRACK_SINK_MAX_BYTES / 4 + 1) as u32),
                block_kind: TrackBlockKind::Samples,
            }),
            Err(CaptureCause::TrackBackpressure(TrackKind::Microphone))
        ));
    }

    #[test]
    fn checkpoint_sink_buffers_three_seconds_of_small_callbacks_without_losing_order() {
        let (sink, receiver) = TrackSink::for_checkpoint_writer(TrackKind::Microphone);
        // A paused disk consumer must not interrupt capture during an ordinary
        // checkpoint encode. Simulate 3s at 48kHz, stereo, 128-frame callbacks.
        for index in 0..1125 {
            sink.try_send(TrackChunk {
                kind: TrackKind::Microphone,
                samples: vec![index as f32; 256],
                sample_rate: 48_000,
                channels: 2,
                span: TrackSpan::new(index * 128, 128),
                block_kind: TrackBlockKind::Samples,
            })
            .unwrap();
        }
        for index in 0..1125 {
            let chunk = receiver.recv().unwrap();
            assert_eq!(chunk.samples, vec![index as f32; 256]);
        }
        // The additional message headroom must not relax the memory bound.
        sink.try_send(TrackChunk {
            kind: TrackKind::Microphone,
            samples: vec![0.0; TRACK_SINK_MAX_BYTES / size_of::<f32>()],
            sample_rate: 48_000,
            channels: 2,
            span: TrackSpan::new(0, (TRACK_SINK_MAX_BYTES / size_of::<f32>() / 2) as u32),
            block_kind: TrackBlockKind::Samples,
        })
        .unwrap();
        assert!(matches!(
            sink.try_send(TrackChunk {
                kind: TrackKind::Microphone,
                samples: vec![0.0; 256],
                sample_rate: 48_000,
                channels: 2,
                span: TrackSpan::new(0, 128),
                block_kind: TrackBlockKind::Samples,
            }),
            Err(CaptureCause::TrackBackpressure(TrackKind::Microphone))
        ));
    }

    #[test]
    fn track_sink_accepts_exact_message_capacity_and_rejects_the_next() {
        let (sink, _receiver) = TrackSink::new(TrackKind::Microphone);
        for _ in 0..TRACK_SINK_MAX_MESSAGES {
            sink.try_send(TrackChunk {
                kind: TrackKind::Microphone,
                samples: vec![0.0],
                sample_rate: 48_000,
                channels: 1,
                span: TrackSpan::new(0, 1),
                block_kind: TrackBlockKind::Samples,
            })
            .unwrap();
        }
        assert!(matches!(
            sink.try_send(TrackChunk {
                kind: TrackKind::Microphone,
                samples: vec![0.0],
                sample_rate: 48_000,
                channels: 1,
                span: TrackSpan::new(0, 1),
                block_kind: TrackBlockKind::Samples,
            }),
            Err(CaptureCause::TrackBackpressure(_))
        ));
    }

    #[test]
    fn track_sink_releases_its_byte_budget_when_the_consumer_receives() {
        let (sink, receiver) = TrackSink::new(TrackKind::Microphone);
        let samples = vec![0.0; TRACK_SINK_MAX_BYTES / size_of::<f32>()];
        sink.try_send(TrackChunk {
            kind: TrackKind::Microphone,
            samples,
            sample_rate: 48_000,
            channels: 1,
            span: TrackSpan::new(0, (TRACK_SINK_MAX_BYTES / size_of::<f32>()) as u32),
            block_kind: TrackBlockKind::Samples,
        })
        .unwrap();
        assert!(matches!(
            sink.try_send(TrackChunk {
                kind: TrackKind::Microphone,
                samples: vec![0.0],
                sample_rate: 48_000,
                channels: 1,
                span: TrackSpan::new(0, 1),
                block_kind: TrackBlockKind::Samples,
            }),
            Err(CaptureCause::TrackBackpressure(_))
        ));
        receiver.recv().unwrap();
        sink.try_send(TrackChunk {
            kind: TrackKind::Microphone,
            samples: vec![0.0],
            sample_rate: 48_000,
            channels: 1,
            span: TrackSpan::new(0, 1),
            block_kind: TrackBlockKind::Samples,
        })
        .unwrap();
    }

    #[test]
    fn concurrent_track_senders_share_one_byte_budget() {
        let (mut sink, receiver) = TrackSink::with_message_capacity(TrackKind::Microphone, 8);
        sink.max_bytes = 4 * size_of::<f32>();
        let sink = Arc::new(sink);
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let senders: Vec<_> = (0..8)
            .map(|_| {
                let sink = sink.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    sink.try_send(TrackChunk {
                        kind: TrackKind::Microphone,
                        samples: vec![0.0],
                        sample_rate: 48_000,
                        channels: 1,
                        span: TrackSpan::new(0, 1),
                        block_kind: TrackBlockKind::Samples,
                    })
                })
            })
            .collect();
        let accepted = senders
            .into_iter()
            .map(|sender| sender.join().unwrap())
            .filter(|result| match result {
                Ok(()) => true,
                Err(CaptureCause::TrackBackpressure(TrackKind::Microphone)) => false,
                other => panic!("unexpected send result: {other:?}"),
            })
            .count();
        assert_eq!(accepted, 4);
        assert_eq!(
            sink.in_flight_bytes.load(Ordering::Acquire),
            4 * size_of::<f32>()
        );
        for _ in 0..accepted {
            receiver.recv().unwrap();
        }
        assert_eq!(sink.in_flight_bytes.load(Ordering::Acquire), 0);
    }

    #[test]
    fn disconnected_track_consumer_is_a_capture_failure() {
        let (sink, receiver) = TrackSink::new(TrackKind::System);
        drop(receiver);
        assert!(matches!(
            sink.try_send(TrackChunk {
                kind: TrackKind::System,
                samples: vec![0.0],
                sample_rate: 48_000,
                channels: 1,
                span: TrackSpan::new(0, 1),
                block_kind: TrackBlockKind::Samples,
            }),
            Err(CaptureCause::CaptureOwnerDisconnected)
        ));
    }

    #[test]
    fn explicit_shutdown_signal_exits_a_healthy_owner_without_a_control_slot() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::Healthy, events, dropped.clone()).unwrap();
        handle.request_shutdown();
        handle.join_once().unwrap();
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn owner_panic_closes_the_terminal_channel_as_a_typed_disconnect() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let mut handle = spawn_fake(FakeMode::Panics, events, dropped).unwrap();
        let exit = handle
            .take_owner_exit_receiver()
            .unwrap()
            .recv_timeout(Duration::from_secs(1));
        assert!(matches!(exit, Ok(CaptureCause::CaptureOwnerDisconnected)));
        assert!(handle.join_once().is_err());
    }
}
