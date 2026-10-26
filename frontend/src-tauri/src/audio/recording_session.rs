use super::capture::backend::{
    CaptureCause, CaptureControl, CaptureRequest, CaptureRuntimeEvent, CaptureThreadHandle,
    CaptureTimeline, SourceControl, SourceSinks, TrackKind, TrackReceiver, TrackSink,
};
use super::device_detection::InputDeviceKind;
use super::incremental_saver::{
    cleanup_source_track_checkpoints, FinalizedTrack, IncrementalAudioSaver,
    SourceTrackFinalizationError, SourceTrackRecoveryError, SourceTrackSaver,
};
use super::permissions::{AudioPermissionCoordinator, AudioPermissionError};
use super::pipeline::AudioPipelineManager;
use super::recording_saver::TranscriptSegment;
use super::recording_session_state::{
    decide_start_identity, reduce_transition, RecordingSessionAction, RecordingSessionState,
    StartIdentity, StartIdentityDecision,
};
use super::recording_state::{AudioChunk, DeviceType, RecordingState};
use log::warn;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex as StdMutex, OnceLock, Weak};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::Mutex;

pub type SessionId = String;
type StartFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type TranscriptionTask = (
    tokio::sync::mpsc::Sender<AudioChunk>,
    tokio::task::JoinHandle<()>,
);
pub type CaptureEventNotifier = Arc<dyn Fn(&SessionId, &CaptureRuntimeEvent) + Send + Sync>;
type TerminalNotifier = Arc<dyn Fn(&RecordingSessionSnapshot) + Send + Sync>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingAudioRouteChanged {
    session_id: SessionId,
    previous_device: String,
    current_device: String,
    reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingAudioDegraded {
    session_id: SessionId,
    source: &'static str,
    reason: String,
}

pub trait RecordingSessionRuntime: Clone + Send + Sync + 'static {
    fn load_preferences(
        &self,
    ) -> StartFuture<'_, anyhow::Result<super::recording_preferences::RecordingPreferences>>;
    fn validate_model(&self) -> StartFuture<'_, Result<(), String>>;
    fn start_transcription(
        &self,
        transcript_history: Arc<StdMutex<Vec<TranscriptSegment>>>,
        meeting_id: &str,
        meeting_vocabulary: Option<&str>,
    ) -> TranscriptionTask;
    fn prepare_capture_permissions(
        &self,
        requested_system: bool,
    ) -> StartFuture<'_, Result<bool, AudioPermissionError>> {
        Box::pin(async move { Ok(requested_system) })
    }
    fn terminal_notifier(&self) -> Option<TerminalNotifier>;
    fn capture_event_notifier(&self) -> Option<CaptureEventNotifier> {
        None
    }

    /// Return the application database when the runtime has one. Test
    /// runtimes intentionally use the default no-op implementation.
    fn lifecycle_database(&self) -> Option<SqlitePool> {
        None
    }
}

impl<R: Runtime> RecordingSessionRuntime for AppHandle<R> {
    fn load_preferences(
        &self,
    ) -> StartFuture<'_, anyhow::Result<super::recording_preferences::RecordingPreferences>> {
        Box::pin(super::recording_preferences::load_recording_preferences(
            self,
        ))
    }

    fn validate_model(&self) -> StartFuture<'_, Result<(), String>> {
        Box::pin(super::transcription::validate_transcription_model_ready(
            self,
        ))
    }

    fn prepare_capture_permissions(
        &self,
        requested_system: bool,
    ) -> StartFuture<'_, Result<bool, AudioPermissionError>> {
        let app = self.clone();
        Box::pin(async move {
            #[cfg(target_os = "macos")]
            {
                let coordinator = app.state::<AudioPermissionCoordinator>();
                coordinator
                    .prepare_capture_permissions(&app, requested_system)
                    .await
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = app;
                Ok(requested_system)
            }
        })
    }

    fn start_transcription(
        &self,
        transcript_history: Arc<StdMutex<Vec<TranscriptSegment>>>,
        meeting_id: &str,
        meeting_vocabulary: Option<&str>,
    ) -> TranscriptionTask {
        super::transcription::start_transcription_task(
            self.clone(),
            transcript_history,
            meeting_id.to_string(),
            meeting_vocabulary.map(str::to_owned),
        )
    }

    fn terminal_notifier(&self) -> Option<TerminalNotifier> {
        let app = self.clone();
        Some(Arc::new(move |snapshot| {
            let _ = app.emit(
                "recording-stopped",
                serde_json::json!({ "sessionId": snapshot.session_id }),
            );
            crate::tray::update_tray_from_snapshot(&app, snapshot);
        }))
    }

    fn capture_event_notifier(&self) -> Option<CaptureEventNotifier> {
        let app = self.clone();
        Some(Arc::new(move |session_id, event| match event {
            CaptureRuntimeEvent::MicrophoneSwitched {
                previous_device,
                current_device,
                reason,
            } => {
                let _ = app.emit(
                    "recording-audio-route-changed",
                    RecordingAudioRouteChanged {
                        session_id: session_id.clone(),
                        previous_device: previous_device.clone(),
                        current_device: current_device.clone(),
                        reason: reason.clone(),
                    },
                );
            }
            CaptureRuntimeEvent::SystemAudioDegraded { reason } => {
                let _ = app.emit(
                    "recording-audio-degraded",
                    RecordingAudioDegraded {
                        session_id: session_id.clone(),
                        source: "system",
                        reason: reason.clone(),
                    },
                );
            }
        }))
    }

    fn lifecycle_database(&self) -> Option<SqlitePool> {
        self.try_state::<crate::state::AppState>()
            .map(|state| state.db_manager.pool().clone())
    }
}

trait CaptureOwnerFactory: Send + Sync {
    fn spawn(
        &self,
        request: CaptureRequest,
        sinks: super::capture::backend::TrackSinks,
    ) -> Result<CaptureThreadHandle, CaptureCause>;
}

struct MacosCaptureOwnerFactory;

impl CaptureOwnerFactory for MacosCaptureOwnerFactory {
    fn spawn(
        &self,
        request: CaptureRequest,
        sinks: super::capture::backend::TrackSinks,
    ) -> Result<CaptureThreadHandle, CaptureCause> {
        CaptureThreadHandle::spawn(request, sinks)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingRequest {
    pub meeting_id: String,
    pub idempotency_key: String,
    pub microphone_device: Option<String>,
    #[serde(default)]
    pub microphone_device_explicit: bool,
    pub system_device: Option<String>,
    #[serde(default = "default_true")]
    pub capture_system: bool,
    pub meeting_name: String,
    #[serde(default)]
    pub meeting_vocabulary: Option<String>,
    pub save_path: Option<String>,
    pub auto_save: Option<bool>,
}

fn default_true() -> bool {
    true
}

impl StartRecordingRequest {
    fn fingerprint(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.meeting_id,
            self.microphone_device.as_deref().unwrap_or(""),
            self.microphone_device_explicit,
            self.system_device.as_deref().unwrap_or(""),
            self.capture_system,
            self.meeting_name,
            self.meeting_vocabulary.as_deref().unwrap_or(""),
            self.save_path.as_deref().unwrap_or(""),
            self.auto_save
                .map(|value| value.to_string())
                .unwrap_or_default()
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractErrorCode {
    InvalidRequest,
    InvalidState,
    NotFound,
    AlreadyRunning,
    PermissionDenied,
    DeviceUnavailable,
    CaptureFailed,
    ModelUnavailable,
    ModelIncompatible,
    StorageUnavailable,
    IntegrityFailed,
    IdempotencyConflict,
    PersistenceConflict,
    PersistenceFailed,
    Cancelled,
    UnsupportedCapability,
    Internal,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractError {
    pub code: ContractErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_optional_state")]
    pub current_state: Option<RecordingSessionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedTrack {
    pub kind: TrackKind,
    pub sample_rate: u32,
    pub channels: u16,
    pub path: String,
    pub byte_length: u64,
    pub sample_count: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSessionSnapshot {
    pub session_id: SessionId,
    pub meeting_id: String,
    pub meeting_vocabulary: Option<String>,
    #[serde(serialize_with = "serialize_state")]
    pub state: RecordingSessionState,
    pub started_at_ms: u64,
    pub recording_duration: f64,
    pub active_duration: f64,
    pub pause_duration: f64,
    pub microphone_device: Option<String>,
    pub system_device: Option<String>,
    pub expected_tracks: BTreeSet<TrackKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_tracks: Option<Vec<CapturedTrack>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ContractError>,
    pub meeting_name: String,
    pub save_path: String,
    pub auto_save: bool,
}

struct ActiveSession {
    snapshot: RecordingSessionSnapshot,
    identity: StartIdentity,
    capture: Option<CaptureThreadHandle>,
    capture_sources: BTreeSet<TrackKind>,
    microphone_writer: TrackWriter,
    system_writer: Option<TrackWriter>,
    microphone_downstream: DownstreamWriter,
    system_downstream: Option<DownstreamWriter>,
    pipeline: AudioPipelineManager,
    pipeline_state: Arc<RecordingState>,
    transcription_join: Option<tokio::task::JoinHandle<()>>,
    transcript_history: Arc<StdMutex<Vec<TranscriptSegment>>>,
    mixed_writer: Option<tokio::task::JoinHandle<anyhow::Result<IncrementalAudioSaver>>>,
    started: Instant,
    pause_started: Option<Instant>,
    pause_total: Duration,
    lifecycle_pool: Option<SqlitePool>,
    lifecycle_run_id: Option<String>,
}

fn write_json_atomically(path: &std::path::Path, value: &serde_json::Value) -> anyhow::Result<()> {
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    std::fs::File::open(&temporary)?.sync_all()?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn initialize_session_artifacts(
    folder: &std::path::Path,
    session_id: &str,
    meeting_id: &str,
    meeting_name: &str,
    started_at_ms: u64,
    microphone: Option<&str>,
    system: Option<&str>,
    auto_save: bool,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(folder)?;
    if auto_save {
        std::fs::create_dir_all(folder.join(".checkpoints"))?;
    }
    write_json_atomically(
        &folder.join("metadata.json"),
        &serde_json::json!({
            "version": "1.0",
            "meeting_id": meeting_id,
            "session_id": session_id,
            "meeting_name": meeting_name,
            "created_at": chrono::DateTime::<chrono::Utc>::from_timestamp_millis(started_at_ms as i64)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
            "completed_at": serde_json::Value::Null,
            "duration_seconds": serde_json::Value::Null,
            "devices": { "microphone": microphone, "system_audio": system },
            "audio_file": if auto_save { "audio.mp4" } else { "" },
            "transcript_file": "transcripts.json",
            "sample_rate": 48_000,
            "status": "recording",
        }),
    )?;
    write_json_atomically(
        &folder.join("transcripts.json"),
        &serde_json::json!({
            "version": "1.0",
            "segments": [],
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "total_segments": 0,
        }),
    )?;
    std::fs::File::open(folder)?.sync_all()?;
    Ok(())
}

fn finalize_session_artifacts(
    snapshot: &RecordingSessionSnapshot,
    history: &StdMutex<Vec<TranscriptSegment>>,
) -> anyhow::Result<()> {
    let folder = std::path::Path::new(&snapshot.save_path);
    let segments = history
        .lock()
        .map_err(|_| anyhow::anyhow!("transcript history lock poisoned"))?
        .clone();
    write_json_atomically(
        &folder.join("transcripts.json"),
        &serde_json::json!({
            "version": "1.0",
            "segments": segments,
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "total_segments": segments.len(),
        }),
    )?;
    write_json_atomically(
        &folder.join("metadata.json"),
        &serde_json::json!({
            "version": "1.0",
            "meeting_id": snapshot.meeting_id.clone(),
            "session_id": snapshot.session_id.clone(),
            "meeting_name": snapshot.meeting_name.clone(),
            "created_at": chrono::DateTime::<chrono::Utc>::from_timestamp_millis(snapshot.started_at_ms as i64)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
            "completed_at": chrono::Utc::now().to_rfc3339(),
            "duration_seconds": snapshot.recording_duration,
            "devices": { "microphone": snapshot.microphone_device.clone(), "system_audio": snapshot.system_device.clone() },
            "audio_file": if snapshot.auto_save { "audio.mp4" } else { "" },
            "transcript_file": "transcripts.json",
            "sample_rate": 48_000,
            "status": snapshot.state.as_contract_str(),
        }),
    )?;
    std::fs::File::open(folder)?.sync_all()?;
    Ok(())
}

fn persist_failed_start_artifacts(
    snapshot: &RecordingSessionSnapshot,
    error: &ContractError,
) -> anyhow::Result<()> {
    if snapshot.save_path.is_empty() {
        return Ok(());
    }
    let folder = std::path::Path::new(&snapshot.save_path);
    if !folder.exists() {
        return Ok(());
    }
    write_json_atomically(
        &folder.join("metadata.json"),
        &serde_json::json!({
            "version": "1.0",
            "meeting_id": snapshot.meeting_id.clone(),
            "session_id": snapshot.session_id.clone(),
            "meeting_name": snapshot.meeting_name.clone(),
            "created_at": chrono::DateTime::<chrono::Utc>::from_timestamp_millis(snapshot.started_at_ms as i64)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
            "completed_at": chrono::Utc::now().to_rfc3339(),
            "duration_seconds": snapshot.recording_duration,
            "devices": { "microphone": snapshot.microphone_device.clone(), "system_audio": snapshot.system_device.clone() },
            "audio_file": if snapshot.auto_save { "audio.mp4" } else { "" },
            "transcript_file": "transcripts.json",
            "sample_rate": 48_000,
            "status": snapshot.state.as_contract_str(),
            "error": { "code": error.code, "message": error.message.clone() },
        }),
    )?;
    std::fs::File::open(folder)?.sync_all()?;
    Ok(())
}

struct TrackWriter {
    join: Option<std::thread::JoinHandle<Result<Option<SourceTrackSaver>, String>>>,
}

impl TrackWriter {
    fn spawn(
        kind: TrackKind,
        receiver: TrackReceiver,
        meeting_folder: std::path::PathBuf,
    ) -> Result<Self, std::io::Error> {
        let join = std::thread::Builder::new()
            .name(format!("meetily-track-writer-{kind:?}"))
            .spawn(move || {
                let mut saver = SourceTrackSaver::new(meeting_folder, kind)
                    .map_err(|error| error.to_string())?;
                while let Ok(chunk) = receiver.recv() {
                    if let Err(error) = saver.add_chunk(chunk) {
                        saver.record_write_error(error.to_string());
                        receiver.mark_incomplete();
                        break;
                    }
                }
                if receiver.is_incomplete() {
                    saver.mark_incomplete();
                }
                Ok(Some(saver))
            })?;
        Ok(Self { join: Some(join) })
    }

    fn discard(kind: TrackKind, receiver: TrackReceiver) -> Result<Self, std::io::Error> {
        let join = std::thread::Builder::new()
            .name(format!("meetily-track-discard-{kind:?}"))
            .spawn(move || {
                while receiver.recv().is_ok() {}
                Ok(None)
            })?;
        Ok(Self { join: Some(join) })
    }
}

fn spawn_source_writer(
    auto_save: bool,
    kind: TrackKind,
    receiver: TrackReceiver,
    meeting_folder: std::path::PathBuf,
) -> Result<TrackWriter, std::io::Error> {
    if auto_save {
        TrackWriter::spawn(kind, receiver, meeting_folder)
    } else {
        TrackWriter::discard(kind, receiver)
    }
}

async fn spawn_capture_owner(
    capture_factory: Arc<dyn CaptureOwnerFactory>,
    capture_request: CaptureRequest,
    sinks: super::capture::backend::TrackSinks,
) -> Result<CaptureThreadHandle, CaptureCause> {
    tokio::task::spawn_blocking(move || capture_factory.spawn(capture_request, sinks))
        .await
        .map_err(|error| CaptureCause::CaptureFailed(error.to_string()))?
}

fn durable_expected_tracks(
    auto_save: bool,
    capture_sources: &BTreeSet<TrackKind>,
) -> BTreeSet<TrackKind> {
    if auto_save {
        capture_sources.clone()
    } else {
        BTreeSet::new()
    }
}

fn reconcile_closed_capture_sources(
    capture_sources: &mut BTreeSet<TrackKind>,
    expected_tracks: &mut BTreeSet<TrackKind>,
    closed_sources: &BTreeSet<TrackKind>,
) -> bool {
    if closed_sources == capture_sources {
        return true;
    }

    // System audio is an optional source. macOS can create its tap and still
    // deliver no frames, or it can revoke the route while the microphone
    // continues. In that case the capture owner closes only the microphone
    // source and the session remains a valid microphone recording.
    let mut microphone_only = capture_sources.clone();
    let had_system = microphone_only.remove(&TrackKind::System);
    if had_system && closed_sources == &microphone_only {
        *capture_sources = closed_sources.clone();
        expected_tracks.remove(&TrackKind::System);
        return true;
    }

    false
}

fn collect_finalized_track(
    kind: TrackKind,
    result: std::result::Result<Option<FinalizedTrack>, SourceTrackFinalizationError>,
    captured_tracks: &mut Vec<CapturedTrack>,
    failures: &mut Vec<(ContractErrorCode, String)>,
) {
    match result {
        Ok(Some(track)) => captured_tracks.push(CapturedTrack {
            kind: track.kind,
            sample_rate: track.sample_rate,
            channels: track.channels,
            path: track.path.to_string_lossy().into_owned(),
            byte_length: track.byte_length,
            sample_count: track.sample_count,
            sha256: track.sha256,
        }),
        Ok(None) => {}
        // System audio is an optional source. A session that captured a valid
        // microphone track is still complete when no system samples arrived.
        Err(SourceTrackFinalizationError::Integrity(details))
            if kind == TrackKind::System && details == "No System Track data was captured" =>
        {
            warn!("System Track produced no samples; continuing microphone-only");
        }
        Err(error) => {
            let code = match error {
                SourceTrackFinalizationError::Integrity(_) => ContractErrorCode::IntegrityFailed,
                SourceTrackFinalizationError::Persistence(_) => {
                    ContractErrorCode::PersistenceFailed
                }
            };
            failures.push((code, format!("{kind:?} Track finalization: {error}")));
        }
    }
}

fn reconcile_optional_system_track(
    expected_tracks: &mut BTreeSet<TrackKind>,
    captured_tracks: &[CapturedTrack],
) {
    let captured_kinds: BTreeSet<_> = captured_tracks.iter().map(|track| track.kind).collect();
    if expected_tracks.contains(&TrackKind::Microphone)
        && expected_tracks.contains(&TrackKind::System)
        && captured_kinds.contains(&TrackKind::Microphone)
        && !captured_kinds.contains(&TrackKind::System)
    {
        expected_tracks.remove(&TrackKind::System);
    }
}

struct DownstreamWriter {
    join: Option<std::thread::JoinHandle<Result<(), String>>>,
}

async fn rollback_start_tasks(
    pipeline_state: Arc<RecordingState>,
    mut pipeline: AudioPipelineManager,
    transcription_join: tokio::task::JoinHandle<()>,
    mixed_writer: Option<tokio::task::JoinHandle<anyhow::Result<IncrementalAudioSaver>>>,
    mut source_writers: Vec<TrackWriter>,
    mut downstream_writers: Vec<DownstreamWriter>,
) -> Vec<FinalizedTrack> {
    pipeline_state.stop_recording();
    let _ = pipeline.force_flush_and_stop().await;
    let _ = transcription_join.await;
    if let Some(mixed_writer) = mixed_writer {
        let _ = mixed_writer.await;
    }
    tokio::task::spawn_blocking(move || {
        let mut tracks = Vec::new();
        for writer in &mut source_writers {
            if let Some(join) = writer.join.take() {
                if let Ok(Ok(Some(saver))) = join.join() {
                    if let Ok(track) = saver.finalize() {
                        tracks.push(track);
                    }
                }
            }
        }
        for writer in &mut downstream_writers {
            if let Some(join) = writer.join.take() {
                let _ = join.join();
            }
        }
        tracks
    })
    .await
    .unwrap_or_default()
}

async fn discard_unopened_source(
    mut writer: Option<TrackWriter>,
    mut downstream: Option<DownstreamWriter>,
    namespace: std::path::PathBuf,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        if let Some(join) = writer.as_mut().and_then(|writer| writer.join.take()) {
            join.join()
                .map_err(|_| "unused source Track writer panicked".to_string())??;
        }
        if let Some(join) = downstream
            .as_mut()
            .and_then(|downstream| downstream.join.take())
        {
            join.join()
                .map_err(|_| "unused source downstream writer panicked".to_string())??;
        }
        if namespace.exists() {
            std::fs::remove_dir_all(&namespace).map_err(|error| error.to_string())?;
            if let Some(parent) = namespace.parent() {
                std::fs::File::open(parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

fn stop_and_join_capture(
    capture: CaptureThreadHandle,
) -> (
    Result<super::capture::backend::CaptureStopped, CaptureCause>,
    Result<(), CaptureCause>,
) {
    stop_and_join_capture_with_timeout(capture, super::capture::backend::CAPTURE_REPLY_TIMEOUT)
}

fn stop_and_join_capture_with_timeout(
    mut capture: CaptureThreadHandle,
    reply_timeout: Duration,
) -> (
    Result<super::capture::backend::CaptureStopped, CaptureCause>,
    Result<(), CaptureCause>,
) {
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    let enqueued = match capture
        .control_tx
        .try_send(CaptureControl::Stop { reply: reply_tx })
    {
        Ok(()) => Ok(()),
        Err(mpsc::TrySendError::Disconnected(_)) => Err(CaptureCause::CaptureOwnerDisconnected),
        Err(mpsc::TrySendError::Full(_)) => Err(CaptureCause::CaptureReplyTimeout),
    };
    let acknowledged = match enqueued {
        Ok(()) => match reply_rx.recv_timeout(reply_timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(CaptureCause::CaptureReplyTimeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(CaptureCause::CaptureOwnerDisconnected)
            }
        },
        Err(cause) => Err(cause),
    };
    if acknowledged.is_err() {
        capture.request_shutdown();
    }
    let joined = capture.join_once();
    (acknowledged, joined)
}

fn take_matching_active<T>(
    active: &mut Option<T>,
    session_id: &str,
    id_of: impl FnOnce(&T) -> &str,
) -> Option<T> {
    if active
        .as_ref()
        .is_some_and(|candidate| id_of(candidate) == session_id)
    {
        active.take()
    } else {
        None
    }
}

impl DownstreamWriter {
    fn spawn(
        kind: TrackKind,
        receiver: TrackReceiver,
        state: Arc<RecordingState>,
        started: Instant,
    ) -> Result<Self, std::io::Error> {
        let join = std::thread::Builder::new()
            .name(format!("meetily-downstream-{kind:?}"))
            .spawn(move || {
                let mut chunk_id = 0_u64;
                while let Ok(chunk) = receiver.recv() {
                    state
                        .send_audio_chunk(AudioChunk {
                            data: chunk.samples,
                            sample_rate: chunk.sample_rate,
                            timestamp: started.elapsed().as_secs_f64(),
                            chunk_id,
                            device_type: match kind {
                                TrackKind::Microphone => DeviceType::Microphone,
                                TrackKind::System => DeviceType::System,
                            },
                            span: Some(chunk.span),
                        })
                        .map_err(|error| error.to_string())?;
                    chunk_id = chunk_id.saturating_add(1);
                }
                Ok(())
            })?;
        Ok(Self { join: Some(join) })
    }
}

#[derive(Default)]
struct SessionStore {
    active: Option<ActiveSession>,
    snapshots: BTreeMap<SessionId, RecordingSessionSnapshot>,
    identities: BTreeMap<String, StartIdentity>,
    transcript_histories: BTreeMap<SessionId, Arc<StdMutex<Vec<TranscriptSegment>>>>,
    last_session_id: Option<SessionId>,
    lifecycle_pools: BTreeMap<SessionId, SqlitePool>,
    lifecycle_run_ids: BTreeMap<SessionId, String>,
}

struct RecordingSessionInner {
    action_gate: Mutex<()>,
    store: Mutex<SessionStore>,
    capture_factory: Arc<dyn CaptureOwnerFactory>,
    terminal_notifier: StdMutex<Option<TerminalNotifier>>,
    shutdown_started: AtomicBool,
    shutdown_signal: tokio::sync::Notify,
}

fn failure_contract(failures: &[(ContractErrorCode, String)]) -> (ContractErrorCode, bool) {
    if let Some((code, _)) = failures
        .iter()
        .find(|(code, _)| matches!(code, ContractErrorCode::IntegrityFailed))
    {
        return (*code, false);
    }
    let code = failures
        .first()
        .map(|(code, _)| *code)
        .unwrap_or(ContractErrorCode::Internal);
    (code, true)
}

fn transition_state(
    current: RecordingSessionState,
    action: RecordingSessionAction,
) -> RecordingSessionState {
    reduce_transition(current, action).expect("RecordingSession transition must be legal")
}

fn transition_failure(
    current: RecordingSessionState,
    failures: &[(ContractErrorCode, String)],
) -> (RecordingSessionState, ContractErrorCode, bool) {
    let (code, retryable) = failure_contract(failures);
    let action = if retryable {
        RecordingSessionAction::Interrupted
    } else {
        RecordingSessionAction::Failed
    };
    (transition_state(current, action), code, retryable)
}

static RECORDING_SESSION_PROJECTION: OnceLock<Weak<RecordingSessionInner>> = OnceLock::new();

fn serialize_state<S>(state: &RecordingSessionState, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(state.as_contract_str())
}

fn serialize_optional_state<S>(
    state: &Option<RecordingSessionState>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    state
        .map(RecordingSessionState::as_contract_str)
        .serialize(serializer)
}

impl ContractError {
    pub fn not_found(session_id: Option<SessionId>) -> Self {
        Self {
            code: ContractErrorCode::NotFound,
            message: "Recording session not found".into(),
            retryable: false,
            session_id,
            job_id: None,
            current_state: None,
            details: None,
        }
    }
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Clone)]
pub struct RecordingSession {
    inner: Arc<RecordingSessionInner>,
}

impl RecordingSession {
    pub fn new() -> Self {
        Self::new_with_capture_factory(Arc::new(MacosCaptureOwnerFactory))
    }

    pub fn latch_shutdown(&self) {
        self.inner.shutdown_started.store(true, Ordering::Release);
        self.inner.shutdown_signal.notify_one();
    }

    fn new_with_capture_factory(capture_factory: Arc<dyn CaptureOwnerFactory>) -> Self {
        let inner = Arc::new(RecordingSessionInner {
            action_gate: Mutex::new(()),
            store: Mutex::new(SessionStore::default()),
            capture_factory,
            terminal_notifier: StdMutex::new(None),
            shutdown_started: AtomicBool::new(false),
            shutdown_signal: tokio::sync::Notify::new(),
        });
        let _ = RECORDING_SESSION_PROJECTION.set(Arc::downgrade(&inner));
        Self { inner }
    }

    pub async fn projected_active() -> Option<RecordingSessionSnapshot> {
        let inner = RECORDING_SESSION_PROJECTION.get()?.upgrade()?;
        let mut store = inner.store.lock().await;
        if let Some(active) = store.active.as_mut() {
            Self::refresh_durations(active);
        }
        let snapshot = store.active.as_ref().map(|active| active.snapshot.clone());
        snapshot
    }

    pub async fn projected_transcript_history() -> Vec<TranscriptSegment> {
        let Some(inner) = RECORDING_SESSION_PROJECTION.get().and_then(Weak::upgrade) else {
            return Vec::new();
        };
        let history = {
            let store = inner.store.lock().await;
            store
                .active
                .as_ref()
                .and_then(|active| {
                    store
                        .transcript_histories
                        .get(&active.snapshot.session_id)
                        .cloned()
                })
                .or_else(|| {
                    store
                        .last_session_id
                        .as_ref()
                        .and_then(|session_id| store.transcript_histories.get(session_id))
                        .cloned()
                })
        };
        history
            .and_then(|history| history.lock().ok().map(|segments| segments.clone()))
            .unwrap_or_default()
    }

    async fn insert_start_identity(
        &self,
        identity: StartIdentity,
        request: &StartRecordingRequest,
    ) -> RecordingSessionSnapshot {
        let snapshot = RecordingSessionSnapshot {
            session_id: identity.session_id.clone(),
            meeting_id: request.meeting_id.clone(),
            meeting_vocabulary: request.meeting_vocabulary.clone(),
            state: RecordingSessionState::Starting,
            started_at_ms: chrono::Utc::now().timestamp_millis().max(0) as u64,
            recording_duration: 0.0,
            active_duration: 0.0,
            pause_duration: 0.0,
            microphone_device: request.microphone_device.clone(),
            system_device: request.system_device.clone(),
            expected_tracks: BTreeSet::new(),
            captured_tracks: None,
            error: None,
            meeting_name: request.meeting_name.clone(),
            save_path: String::new(),
            auto_save: request.auto_save.unwrap_or(true),
        };
        let mut store = self.inner.store.lock().await;
        store
            .identities
            .insert(identity.idempotency_key.clone(), identity);
        store
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot.clone());
        store.last_session_id = Some(snapshot.session_id.clone());
        snapshot
    }

    async fn update_start_resolution(
        &self,
        session_id: &str,
        request: &StartRecordingRequest,
        save_path: &str,
        auto_save: bool,
    ) {
        let mut store = self.inner.store.lock().await;
        if let Some(snapshot) = store.snapshots.get_mut(session_id) {
            snapshot.microphone_device = request.microphone_device.clone();
            snapshot.system_device = request.system_device.clone();
            snapshot.save_path = save_path.into();
            snapshot.auto_save = auto_save;
            snapshot.expected_tracks = if auto_save {
                let mut tracks = BTreeSet::from([TrackKind::Microphone]);
                if request.capture_system {
                    tracks.insert(TrackKind::System);
                }
                tracks
            } else {
                BTreeSet::new()
            };
        }
    }

    async fn record_start_failure(&self, session_id: &str, error: ContractError) -> ContractError {
        self.record_start_failure_with_tracks(session_id, error, Vec::new())
            .await
    }

    async fn record_start_failure_with_tracks(
        &self,
        session_id: &str,
        mut error: ContractError,
        verified_tracks: Vec<FinalizedTrack>,
    ) -> ContractError {
        let (initial_snapshot, lifecycle_pool, lifecycle_run_id) = {
            let mut store = self.inner.store.lock().await;
            let lifecycle_pool = store.lifecycle_pools.get(session_id).cloned();
            let lifecycle_run_id = store.lifecycle_run_ids.get(session_id).cloned();
            let snapshot = store.snapshots.get_mut(session_id);
            let initial_snapshot = if let Some(snapshot) = snapshot {
                snapshot.state = transition_state(snapshot.state, RecordingSessionAction::Failed);
                if !verified_tracks.is_empty() {
                    snapshot.captured_tracks = Some(
                        verified_tracks
                            .into_iter()
                            .map(|track| CapturedTrack {
                                kind: track.kind,
                                sample_rate: track.sample_rate,
                                channels: track.channels,
                                path: track.path.to_string_lossy().into_owned(),
                                byte_length: track.byte_length,
                                sample_count: track.sample_count,
                                sha256: track.sha256,
                            })
                            .collect(),
                    );
                }
                error.current_state = Some(snapshot.state);
                snapshot.error = Some(error.clone());
                Some(snapshot.clone())
            } else {
                None
            };
            (initial_snapshot, lifecycle_pool, lifecycle_run_id)
        };
        error.current_state = Some(RecordingSessionState::Failed);
        if let Some(snapshot) = initial_snapshot {
            if let Err(persistence_error) = persist_failed_start_artifacts(&snapshot, &error) {
                error.details = Some(match error.details.take() {
                    Some(details) => {
                        format!("{details}; failed start metadata persistence: {persistence_error}")
                    }
                    None => format!("failed start metadata persistence: {persistence_error}"),
                });
            }
            let snapshot = {
                let mut store = self.inner.store.lock().await;
                let snapshot = store
                    .snapshots
                    .get_mut(session_id)
                    .expect("failed start snapshot must remain registered");
                snapshot.error = Some(error.clone());
                snapshot.clone()
            };
            if let Some(notifier) = self.inner.terminal_notifier.lock().unwrap().clone() {
                notifier(&snapshot);
            }
            if let (Some(pool), Some(lifecycle_run_id)) = (lifecycle_pool, lifecycle_run_id) {
                if let Err(persistence_error) = crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
                    &pool,
                    &snapshot.meeting_id,
                    crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                    &lifecycle_run_id,
                    crate::database::repositories::meeting::MEETING_STATUS_ERROR,
                    Some(snapshot.recording_duration),
                    Some("Recording could not be started"),
                )
                .await
                {
                    log::warn!("Failed to persist recording start failure: {persistence_error}");
                }
            }
        }
        error
    }

    pub async fn start<A: RecordingSessionRuntime>(
        &self,
        runtime: A,
        mut request: StartRecordingRequest,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        request.meeting_vocabulary = crate::database::repositories::setting::normalize_vocabulary(
            request.meeting_vocabulary.as_deref(),
        )
        .map_err(|message| {
            self.error(
                ContractErrorCode::InvalidRequest,
                &message,
                false,
                None,
                None,
                None,
            )
        })?;
        let _action = self.inner.action_gate.lock().await;
        if self.inner.shutdown_started.load(Ordering::Acquire) {
            return Err(self.error(
                ContractErrorCode::Cancelled,
                "Recording cannot start while the application is shutting down",
                false,
                None,
                None,
                None,
            ));
        }
        *self.inner.terminal_notifier.lock().unwrap() = runtime.terminal_notifier();
        let fingerprint = request.fingerprint();
        {
            let store = self.inner.store.lock().await;
            if let Some(identity) = store.identities.get(&request.idempotency_key) {
                match decide_start_identity(Some(identity), &request.idempotency_key, &fingerprint)
                {
                    StartIdentityDecision::Reuse(session_id) => {
                        return store.snapshots.get(&session_id).cloned().ok_or_else(|| {
                            self.error(
                                ContractErrorCode::Internal,
                                "Session identity is missing its snapshot",
                                false,
                                Some(session_id),
                                None,
                                None,
                            )
                        })
                    }
                    StartIdentityDecision::Conflict => {
                        return Err(self.error(
                            ContractErrorCode::IdempotencyConflict,
                            "The idempotency key belongs to another recording request",
                            false,
                            Some(identity.session_id.clone()),
                            store.snapshots.get(&identity.session_id).map(|s| s.state),
                            None,
                        ))
                    }
                    StartIdentityDecision::New => {}
                }
            }
            if let Some(active) = &store.active {
                return Err(self.error(
                    ContractErrorCode::AlreadyRunning,
                    "Another recording session owns the capture devices",
                    true,
                    Some(active.snapshot.session_id.clone()),
                    Some(active.snapshot.state),
                    None,
                ));
            }
        }

        let prepared_capture_system = tokio::select! {
            _ = self.inner.shutdown_signal.notified() => {
                return Err(self.error(
                    ContractErrorCode::Cancelled,
                    "Recording cannot start while the application is shutting down",
                    false,
                    None,
                    None,
                    None,
                ));
            }
            result = runtime.prepare_capture_permissions(request.capture_system) => {
                if self.inner.shutdown_started.load(Ordering::Acquire) {
                    return Err(self.error(
                        ContractErrorCode::Cancelled,
                        "Recording cannot start while the application is shutting down",
                        false,
                        None,
                        None,
                        None,
                    ));
                }
                match result {
                    Ok(capture_system) => capture_system,
                    Err(error) => return Err(self.capture_permission_error(error)),
                }
            }
        };
        request.capture_system = prepared_capture_system;

        let session_id = uuid::Uuid::new_v4().to_string();
        let identity = StartIdentity {
            session_id: session_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            fingerprint,
        };
        let start_snapshot = self.insert_start_identity(identity.clone(), &request).await;
        let lifecycle_pool = runtime.lifecycle_database();
        let lifecycle_run_id = if let Some(pool) = lifecycle_pool.clone() {
            self.inner
                .store
                .lock()
                .await
                .lifecycle_pools
                .insert(session_id.clone(), pool.clone());
            match crate::database::repositories::meeting::MeetingsRepository::ensure_lifecycle_meeting(
                &pool,
                &request.meeting_id,
                &request.meeting_name,
                i64::try_from(start_snapshot.started_at_ms).ok(),
                "suggested",
                crate::database::repositories::meeting::MEETING_STATUS_RECORDING,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
            )
            .await {
                Ok(run_id) => {
                    self.inner
                        .store
                        .lock()
                        .await
                        .lifecycle_run_ids
                        .insert(session_id.clone(), run_id.clone());
                    Some(run_id)
                }
                Err(error) => {
                    let contract = self.error(
                        ContractErrorCode::StorageUnavailable,
                        "Meeting status could not be saved",
                        true,
                        Some(session_id.clone()),
                        Some(RecordingSessionState::Starting),
                        Some(error.to_string()),
                    );
                    return Err(self.record_start_failure(&session_id, contract).await);
                }
            }
        } else {
            None
        };

        let preferences = match runtime.load_preferences().await {
            Ok(preferences) => preferences,
            Err(error) => {
                let contract = self.error(
                    ContractErrorCode::StorageUnavailable,
                    "Recording preferences could not be loaded",
                    true,
                    Some(session_id.clone()),
                    Some(RecordingSessionState::Starting),
                    Some(error.to_string()),
                );
                return Err(self.record_start_failure(&session_id, contract).await);
            }
        };
        let explicit_microphone =
            request.microphone_device.is_some() && request.microphone_device_explicit;
        if request.microphone_device.is_none() {
            request.microphone_device = preferences.preferred_mic_device.clone();
        }
        if request.system_device.is_none() {
            request.system_device = preferences.preferred_system_device.clone();
        }
        let auto_save = request.auto_save.unwrap_or(preferences.auto_save);
        if let Err(error) = runtime.validate_model().await {
            let code = if error.to_ascii_lowercase().contains("incompat") {
                ContractErrorCode::ModelIncompatible
            } else {
                ContractErrorCode::ModelUnavailable
            };
            let contract = self.error(
                code,
                "Transcription model is not ready",
                true,
                Some(session_id.clone()),
                Some(RecordingSessionState::Starting),
                Some(error),
            );
            return Err(self.record_start_failure(&session_id, contract).await);
        }
        let save_path = match request.save_path.as_deref().filter(|path| !path.is_empty()) {
            Some(path) => std::path::PathBuf::from(path),
            None => match super::audio_processing::create_meeting_folder(
                &preferences.save_folder,
                &request.meeting_name,
                false,
            ) {
                Ok(path) => path,
                Err(error) => {
                    let contract = self.error(
                        ContractErrorCode::StorageUnavailable,
                        "Recording folder could not be created",
                        true,
                        Some(session_id.clone()),
                        Some(RecordingSessionState::Starting),
                        Some(error.to_string()),
                    );
                    return Err(self.record_start_failure(&session_id, contract).await);
                }
            },
        }
        .to_string_lossy()
        .into_owned();
        request.save_path = Some(save_path.clone());
        request.auto_save = Some(auto_save);
        self.update_start_resolution(&session_id, &request, &save_path, auto_save)
            .await;
        if let Some(pool) = lifecycle_pool.as_ref() {
            if let Err(error) =
                crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_folder(
                    pool,
                    &request.meeting_id,
                    &save_path,
                )
                .await
            {
                log::warn!("Failed to persist recording folder: {error}");
            }
        }

        if let Err(error) = initialize_session_artifacts(
            std::path::Path::new(&save_path),
            &session_id,
            &request.meeting_id,
            &request.meeting_name,
            start_snapshot.started_at_ms,
            request.microphone_device.as_deref(),
            request.system_device.as_deref(),
            auto_save,
        ) {
            let contract = self.error(
                ContractErrorCode::StorageUnavailable,
                "Recording session artifacts could not be created",
                true,
                Some(session_id.clone()),
                Some(RecordingSessionState::Starting),
                Some(error.to_string()),
            );
            return Err(self.record_start_failure(&session_id, contract).await);
        }
        let meeting_folder = std::path::PathBuf::from(&save_path);
        let started = Instant::now();
        let pipeline_state = RecordingState::new();
        if let Err(error) = pipeline_state.start_recording() {
            let contract = self.error(
                ContractErrorCode::Internal,
                "Audio pipeline state could not start",
                true,
                Some(session_id.clone()),
                Some(RecordingSessionState::Starting),
                Some(error.to_string()),
            );
            return Err(self.record_start_failure(&session_id, contract).await);
        }
        let (mixed_tx, mixed_writer) = if auto_save {
            match IncrementalAudioSaver::start_mixed_writer(meeting_folder.clone(), 48_000) {
                Ok((sender, writer)) => (Some(sender), Some(writer)),
                Err(error) => {
                    let contract = self.error(
                        ContractErrorCode::StorageUnavailable,
                        "Mixed checkpoint saver could not start",
                        true,
                        Some(session_id.clone()),
                        Some(RecordingSessionState::Starting),
                        Some(error.to_string()),
                    );
                    return Err(self.record_start_failure(&session_id, contract).await);
                }
            }
        } else {
            (None, None)
        };
        let transcript_history = Arc::new(StdMutex::new(Vec::new()));
        self.inner
            .store
            .lock()
            .await
            .transcript_histories
            .insert(session_id.clone(), transcript_history.clone());
        super::transcription::reset_speech_detected_flag();
        let (transcription_tx, transcription_join) = runtime.start_transcription(
            transcript_history.clone(),
            &request.meeting_id,
            request.meeting_vocabulary.as_deref(),
        );
        let mic_name = request
            .microphone_device
            .clone()
            .unwrap_or_else(|| "Default microphone".into());
        let system_name = request.system_device.clone().unwrap_or_default();
        let mut pipeline = AudioPipelineManager::new();
        if let Err(error) = pipeline.start(
            pipeline_state.clone(),
            transcription_tx,
            500,
            48_000,
            mixed_tx,
            mic_name.clone(),
            InputDeviceKind::detect(&mic_name, 512, 48_000),
            system_name.clone(),
            InputDeviceKind::detect(&system_name, 512, 48_000),
        ) {
            pipeline_state.stop_recording();
            let _ = transcription_join.await;
            if let Some(mixed_writer) = mixed_writer {
                let _ = mixed_writer.await;
            }
            let contract = self.error(
                ContractErrorCode::Internal,
                "Audio pipeline could not start",
                true,
                Some(session_id.clone()),
                Some(RecordingSessionState::Starting),
                Some(error.to_string()),
            );
            return Err(self.record_start_failure(&session_id, contract).await);
        }

        let (microphone_raw_sink, microphone_rx) =
            TrackSink::for_checkpoint_writer(TrackKind::Microphone);
        let (microphone_downstream_sink, microphone_downstream_rx) =
            TrackSink::new(TrackKind::Microphone);
        let capture_timeline = CaptureTimeline::new();
        let (system_sinks, system_rx, system_downstream_rx) = if request.capture_system {
            let (raw, raw_receiver) = TrackSink::for_checkpoint_writer(TrackKind::System);
            let (downstream, downstream_receiver) = TrackSink::new(TrackKind::System);
            (
                Some(SourceSinks::with_timeline(
                    raw,
                    downstream,
                    capture_timeline.clone(),
                )),
                Some(raw_receiver),
                Some(downstream_receiver),
            )
        } else {
            (None, None, None)
        };
        let microphone_writer_result = spawn_source_writer(
            auto_save,
            TrackKind::Microphone,
            microphone_rx,
            meeting_folder.clone(),
        );
        let microphone_writer = match microphone_writer_result {
            Ok(writer) => writer,
            Err(error) => {
                drop(microphone_raw_sink);
                drop(microphone_downstream_sink);
                drop(system_sinks);
                let _ = rollback_start_tasks(
                    pipeline_state,
                    pipeline,
                    transcription_join,
                    mixed_writer,
                    Vec::new(),
                    Vec::new(),
                )
                .await;
                let contract = self.error(
                    ContractErrorCode::Internal,
                    "Microphone Track writer could not start",
                    true,
                    Some(session_id.clone()),
                    Some(RecordingSessionState::Starting),
                    Some(error.to_string()),
                );
                return Err(self.record_start_failure(&session_id, contract).await);
            }
        };
        let mut system_writer = match system_rx {
            Some(receiver) => {
                let writer = spawn_source_writer(
                    auto_save,
                    TrackKind::System,
                    receiver,
                    meeting_folder.clone(),
                );
                match writer {
                    Ok(writer) => Some(writer),
                    Err(error) => {
                        drop(microphone_raw_sink);
                        drop(microphone_downstream_sink);
                        drop(system_sinks);
                        let _ = rollback_start_tasks(
                            pipeline_state,
                            pipeline,
                            transcription_join,
                            mixed_writer,
                            vec![microphone_writer],
                            Vec::new(),
                        )
                        .await;
                        let contract = self.error(
                            ContractErrorCode::Internal,
                            "System Track writer could not start",
                            true,
                            Some(session_id.clone()),
                            Some(RecordingSessionState::Starting),
                            Some(error.to_string()),
                        );
                        return Err(self.record_start_failure(&session_id, contract).await);
                    }
                }
            }
            None => None,
        };
        let microphone_downstream = match DownstreamWriter::spawn(
            TrackKind::Microphone,
            microphone_downstream_rx,
            pipeline_state.clone(),
            started,
        ) {
            Ok(writer) => writer,
            Err(error) => {
                drop(microphone_raw_sink);
                drop(microphone_downstream_sink);
                drop(system_sinks);
                let mut source_writers = vec![microphone_writer];
                source_writers.extend(system_writer);
                let _ = rollback_start_tasks(
                    pipeline_state,
                    pipeline,
                    transcription_join,
                    mixed_writer,
                    source_writers,
                    Vec::new(),
                )
                .await;
                let contract = self.error(
                    ContractErrorCode::Internal,
                    "Microphone downstream writer could not start",
                    true,
                    Some(session_id.clone()),
                    Some(RecordingSessionState::Starting),
                    Some(error.to_string()),
                );
                return Err(self.record_start_failure(&session_id, contract).await);
            }
        };
        let mut system_downstream = match system_downstream_rx {
            Some(receiver) => match DownstreamWriter::spawn(
                TrackKind::System,
                receiver,
                pipeline_state.clone(),
                started,
            ) {
                Ok(writer) => Some(writer),
                Err(error) => {
                    drop(microphone_raw_sink);
                    drop(microphone_downstream_sink);
                    drop(system_sinks);
                    let mut source_writers = vec![microphone_writer];
                    source_writers.extend(system_writer);
                    let _ = rollback_start_tasks(
                        pipeline_state,
                        pipeline,
                        transcription_join,
                        mixed_writer,
                        source_writers,
                        vec![microphone_downstream],
                    )
                    .await;
                    let contract = self.error(
                        ContractErrorCode::Internal,
                        "System downstream writer could not start",
                        true,
                        Some(session_id.clone()),
                        Some(RecordingSessionState::Starting),
                        Some(error.to_string()),
                    );
                    return Err(self.record_start_failure(&session_id, contract).await);
                }
            },
            None => None,
        };
        let capture_request = CaptureRequest {
            microphone_device: request.microphone_device.clone(),
            fallback_to_default_microphone: !explicit_microphone,
            system_device: request.system_device.clone(),
            capture_system: request.capture_system,
        };
        let capture_factory = self.inner.capture_factory.clone();
        let microphone_source_sinks = SourceSinks::with_timeline(
            microphone_raw_sink,
            microphone_downstream_sink,
            capture_timeline,
        );
        let capture_result = spawn_capture_owner(
            capture_factory,
            capture_request,
            super::capture::backend::TrackSinks {
                microphone: microphone_source_sinks,
                system: system_sinks,
            },
        )
        .await;
        let mut capture = match capture_result {
            Ok(capture) => capture,
            Err(cause) => {
                let mut source_writers = vec![microphone_writer];
                source_writers.extend(system_writer);
                let mut downstream_writers = vec![microphone_downstream];
                downstream_writers.extend(system_downstream);
                let verified_tracks = rollback_start_tasks(
                    pipeline_state,
                    pipeline,
                    transcription_join,
                    mixed_writer,
                    source_writers,
                    downstream_writers,
                )
                .await;
                let error = self.capture_error(&session_id, RecordingSessionState::Starting, cause);
                return Err(self
                    .record_start_failure_with_tracks(&session_id, error, verified_tracks)
                    .await);
            }
        };
        let capture_sources = capture.opened_sources.clone();
        if !capture_sources.contains(&TrackKind::System) {
            let namespace = meeting_folder.join(".tracks").join("system");
            if let Err(error) =
                discard_unopened_source(system_writer.take(), system_downstream.take(), namespace)
                    .await
            {
                let _ = tokio::task::spawn_blocking(move || stop_and_join_capture(capture)).await;
                let verified_tracks = rollback_start_tasks(
                    pipeline_state,
                    pipeline,
                    transcription_join,
                    mixed_writer,
                    vec![microphone_writer],
                    vec![microphone_downstream],
                )
                .await;
                let contract = self.error(
                    ContractErrorCode::PersistenceFailed,
                    "Unused source saver could not be discarded",
                    true,
                    Some(session_id.clone()),
                    Some(RecordingSessionState::Starting),
                    Some(error),
                );
                return Err(self
                    .record_start_failure_with_tracks(&session_id, contract, verified_tracks)
                    .await);
            }
        }
        let expected_tracks = durable_expected_tracks(auto_save, &capture_sources);
        let owner_exit_rx = capture.take_owner_exit_receiver();
        let runtime_event_rx = capture.take_runtime_event_receiver();
        let capture_event_notifier = runtime.capture_event_notifier();
        let state = reduce_transition(
            RecordingSessionState::Starting,
            RecordingSessionAction::CaptureStarted,
        )
        .expect("contract transition");
        let snapshot = RecordingSessionSnapshot {
            session_id: session_id.clone(),
            meeting_id: request.meeting_id,
            meeting_vocabulary: request.meeting_vocabulary,
            state,
            started_at_ms: start_snapshot.started_at_ms,
            recording_duration: 0.0,
            active_duration: 0.0,
            pause_duration: 0.0,
            microphone_device: request.microphone_device,
            system_device: request.system_device,
            expected_tracks,
            captured_tracks: None,
            error: None,
            meeting_name: request.meeting_name,
            save_path,
            auto_save,
        };
        let mut store = self.inner.store.lock().await;
        store.snapshots.insert(session_id.clone(), snapshot.clone());
        store.active = Some(ActiveSession {
            snapshot: snapshot.clone(),
            identity,
            capture: Some(capture),
            capture_sources,
            microphone_writer,
            system_writer,
            microphone_downstream,
            system_downstream,
            pipeline,
            pipeline_state,
            transcription_join: Some(transcription_join),
            transcript_history,
            mixed_writer,
            started,
            pause_started: None,
            pause_total: Duration::ZERO,
            lifecycle_pool,
            lifecycle_run_id,
        });
        drop(store);
        if let Some(runtime_event_rx) = runtime_event_rx {
            self.monitor_capture_runtime_events(
                session_id.clone(),
                runtime_event_rx,
                capture_event_notifier,
            );
        }
        if let Some(owner_exit_rx) = owner_exit_rx {
            self.monitor_capture_owner_exit(session_id, owner_exit_rx);
        }
        Ok(snapshot)
    }

    fn monitor_capture_runtime_events(
        &self,
        session_id: SessionId,
        runtime_event_rx: mpsc::Receiver<CaptureRuntimeEvent>,
        notifier: Option<CaptureEventNotifier>,
    ) {
        let session = self.clone();
        tokio::spawn(async move {
            let mut receiver = runtime_event_rx;
            loop {
                let received = tokio::task::spawn_blocking(move || {
                    let event = receiver.recv().ok();
                    (receiver, event)
                })
                .await;
                let Ok((returned_receiver, Some(event))) = received else {
                    break;
                };
                receiver = returned_receiver;
                session
                    .project_capture_runtime_event(&session_id, &event, notifier.as_ref())
                    .await;
            }
        });
    }

    async fn project_capture_runtime_event(
        &self,
        session_id: &SessionId,
        event: &CaptureRuntimeEvent,
        notifier: Option<&CaptureEventNotifier>,
    ) {
        {
            let mut store = self.inner.store.lock().await;
            let snapshot = {
                let Some(active) = store
                    .active
                    .as_mut()
                    .filter(|active| active.snapshot.session_id == *session_id)
                else {
                    return;
                };
                if let CaptureRuntimeEvent::MicrophoneSwitched { current_device, .. } = event {
                    active.snapshot.microphone_device = Some(current_device.clone());
                    Some(active.snapshot.clone())
                } else {
                    None
                }
            };
            if let Some(snapshot) = snapshot {
                store.snapshots.insert(session_id.clone(), snapshot);
            }
        }
        if let Some(notifier) = notifier {
            notifier(session_id, event);
        }
    }

    fn monitor_capture_owner_exit(
        &self,
        session_id: SessionId,
        owner_exit_rx: mpsc::Receiver<CaptureCause>,
    ) {
        let session = self.clone();
        tokio::spawn(async move {
            let cause = tokio::task::spawn_blocking(move || owner_exit_rx.recv().ok())
                .await
                .ok()
                .flatten();
            if let Some(cause) = cause {
                log::error!("Capture owner exited unexpectedly: {cause:?}");
                let _ = session
                    .finalize_unexpected_owner_exit(session_id, cause)
                    .await;
            }
        });
    }

    async fn finalize_unexpected_owner_exit(
        &self,
        session_id: SessionId,
        cause: CaptureCause,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let _action = self.inner.action_gate.lock().await;
        let (active, failure) = {
            let mut store = self.inner.store.lock().await;
            let Some(mut active) = take_matching_active(&mut store.active, &session_id, |active| {
                active.snapshot.session_id.as_str()
            }) else {
                return store
                    .snapshots
                    .get(&session_id)
                    .cloned()
                    .ok_or_else(|| ContractError::not_found(Some(session_id)));
            };
            let failure = self.capture_error(&session_id, active.snapshot.state, cause);
            active.snapshot.state =
                transition_state(active.snapshot.state, RecordingSessionAction::StopRequested);
            store
                .snapshots
                .insert(session_id.clone(), active.snapshot.clone());
            (active, failure)
        };
        self.finish_active_session(
            session_id,
            active,
            vec![(
                failure.code,
                format!(
                    "unexpected capture owner exit: {}",
                    failure.details.unwrap_or(failure.message)
                ),
            )],
        )
        .await
    }

    pub async fn pause(
        &self,
        session_id: SessionId,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        self.control(session_id, true).await
    }

    pub async fn resume(
        &self,
        session_id: SessionId,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        self.control(session_id, false).await
    }

    pub async fn set_source_control(
        &self,
        session_id: SessionId,
        kind: TrackKind,
        control: SourceControl,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let _action = self.inner.action_gate.lock().await;
        let (current_state, control_tx) = {
            let store = self.inner.store.lock().await;
            let active = store
                .active
                .as_ref()
                .filter(|active| active.snapshot.session_id == session_id)
                .ok_or_else(|| {
                    if let Some(snapshot) = store.snapshots.get(&session_id) {
                        self.error(
                            ContractErrorCode::InvalidState,
                            "Recording source control is not valid in the current state",
                            false,
                            Some(session_id.clone()),
                            Some(snapshot.state),
                            None,
                        )
                    } else {
                        ContractError::not_found(Some(session_id.clone()))
                    }
                })?;
            let current_state = active.snapshot.state;
            let control_tx = active
                .capture
                .as_ref()
                .ok_or_else(|| {
                    self.capture_error(
                        &session_id,
                        current_state,
                        CaptureCause::CaptureOwnerDisconnected,
                    )
                })?
                .control_tx
                .clone();
            (current_state, control_tx)
        };
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let command = CaptureControl::SetSourceControl {
            kind,
            control,
            reply: reply_tx,
        };
        let control_result = match control_tx.try_send(command) {
            Err(error) => Err(self.capture_error(
                &session_id,
                current_state,
                CaptureCause::CaptureFailed(error.to_string()),
            )),
            Ok(()) => match tokio::task::spawn_blocking(move || {
                reply_rx.recv_timeout(super::capture::backend::CAPTURE_REPLY_TIMEOUT)
            })
            .await
            {
                Err(error) => Err(self.capture_error(
                    &session_id,
                    current_state,
                    CaptureCause::CaptureFailed(error.to_string()),
                )),
                Ok(Err(mpsc::RecvTimeoutError::Timeout)) => Err(self.capture_error(
                    &session_id,
                    current_state,
                    CaptureCause::CaptureReplyTimeout,
                )),
                Ok(Err(mpsc::RecvTimeoutError::Disconnected)) => Err(self.capture_error(
                    &session_id,
                    current_state,
                    CaptureCause::CaptureOwnerDisconnected,
                )),
                Ok(Ok(Err(cause))) => Err(self.capture_error(&session_id, current_state, cause)),
                Ok(Ok(Ok(()))) => Ok(()),
            },
        };
        control_result?;
        let store = self.inner.store.lock().await;
        store
            .active
            .as_ref()
            .filter(|active| active.snapshot.session_id == session_id)
            .map(|active| active.snapshot.clone())
            .ok_or_else(|| ContractError::not_found(Some(session_id)))
    }

    async fn control(
        &self,
        session_id: SessionId,
        pause: bool,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let _action = self.inner.action_gate.lock().await;
        let action = if pause {
            RecordingSessionAction::PauseRequested
        } else {
            RecordingSessionAction::ResumeRequested
        };
        let (next, current_state, control_tx) = {
            let store = self.inner.store.lock().await;
            let active = match store
                .active
                .as_ref()
                .filter(|active| active.snapshot.session_id == session_id)
            {
                Some(active) => active,
                None => {
                    if let Some(snapshot) = store.snapshots.get(&session_id) {
                        return Err(self.error(
                            ContractErrorCode::InvalidState,
                            "Recording action is not valid in the current state",
                            false,
                            Some(session_id.clone()),
                            Some(snapshot.state),
                            None,
                        ));
                    }
                    return Err(ContractError::not_found(Some(session_id.clone())));
                }
            };
            let current_state = active.snapshot.state;
            let next = reduce_transition(current_state, action).map_err(|_| {
                self.error(
                    ContractErrorCode::InvalidState,
                    "Recording action is not valid in the current state",
                    false,
                    Some(session_id.clone()),
                    Some(current_state),
                    None,
                )
            })?;
            if next == current_state {
                return Ok(active.snapshot.clone());
            }
            let control_tx = active
                .capture
                .as_ref()
                .ok_or_else(|| {
                    self.capture_error(
                        &session_id,
                        current_state,
                        CaptureCause::CaptureOwnerDisconnected,
                    )
                })?
                .control_tx
                .clone();
            (next, current_state, control_tx)
        };
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let command = if pause {
            CaptureControl::Pause { reply: reply_tx }
        } else {
            CaptureControl::Resume { reply: reply_tx }
        };
        let control_result = match control_tx.try_send(command) {
            Err(error) => Err(self.capture_error(
                &session_id,
                current_state,
                CaptureCause::CaptureFailed(error.to_string()),
            )),
            Ok(()) => match tokio::task::spawn_blocking(move || {
                reply_rx.recv_timeout(super::capture::backend::CAPTURE_REPLY_TIMEOUT)
            })
            .await
            {
                Err(error) => Err(self.capture_error(
                    &session_id,
                    current_state,
                    CaptureCause::CaptureFailed(error.to_string()),
                )),
                Ok(Err(mpsc::RecvTimeoutError::Timeout)) => Err(self.capture_error(
                    &session_id,
                    current_state,
                    CaptureCause::CaptureReplyTimeout,
                )),
                Ok(Err(mpsc::RecvTimeoutError::Disconnected)) => Err(self.capture_error(
                    &session_id,
                    current_state,
                    CaptureCause::CaptureOwnerDisconnected,
                )),
                Ok(Ok(Err(cause))) => Err(self.capture_error(&session_id, current_state, cause)),
                Ok(Ok(Ok(()))) => Ok(()),
            },
        };
        if let Err(error) = control_result {
            return self.shutdown_after_control_failure(session_id, error).await;
        }
        let mut store = self.inner.store.lock().await;
        let active = store
            .active
            .as_mut()
            .filter(|active| active.snapshot.session_id == session_id)
            .ok_or_else(|| ContractError::not_found(Some(session_id.clone())))?;
        if active.snapshot.state != current_state {
            return Err(self.error(
                ContractErrorCode::PersistenceConflict,
                "Recording session changed while the capture control was pending",
                true,
                Some(session_id),
                Some(active.snapshot.state),
                None,
            ));
        }
        if pause {
            active.pause_started = Some(Instant::now());
        } else if let Some(started) = active.pause_started.take() {
            active.pause_total += started.elapsed();
        }
        active.snapshot.state = next;
        Self::refresh_durations(active);
        let snapshot = active.snapshot.clone();
        store.snapshots.insert(session_id, snapshot.clone());
        Ok(snapshot)
    }

    pub async fn stop(
        &self,
        session_id: SessionId,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let _action = self.inner.action_gate.lock().await;
        self.stop_locked(session_id).await
    }

    async fn stop_locked(
        &self,
        session_id: SessionId,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let mut active = {
            let mut store = self.inner.store.lock().await;
            if let Some(snapshot) = store.snapshots.get(&session_id) {
                if matches!(
                    snapshot.state,
                    RecordingSessionState::Stopping
                        | RecordingSessionState::Finalizing
                        | RecordingSessionState::Completed
                        | RecordingSessionState::Interrupted
                        | RecordingSessionState::Failed
                ) {
                    return Ok(snapshot.clone());
                }
            }
            let mut active = take_matching_active(&mut store.active, &session_id, |active| {
                active.snapshot.session_id.as_str()
            })
            .ok_or_else(|| {
                self.error(
                    ContractErrorCode::NotFound,
                    "Recording session not found",
                    false,
                    Some(session_id.clone()),
                    None,
                    None,
                )
            })?;
            active.snapshot.state =
                reduce_transition(active.snapshot.state, RecordingSessionAction::StopRequested)
                    .map_err(|_| {
                        self.error(
                            ContractErrorCode::InvalidState,
                            "Recording action is not valid in the current state",
                            false,
                            Some(session_id.clone()),
                            Some(active.snapshot.state),
                            None,
                        )
                    })?;
            store
                .snapshots
                .insert(session_id.clone(), active.snapshot.clone());
            active
        };
        Self::refresh_durations(&mut active);
        if let (Some(pool), Some(lifecycle_run_id)) = (
            active.lifecycle_pool.as_ref(),
            active.lifecycle_run_id.as_deref(),
        ) {
            if let Err(error) = crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
                pool,
                &active.snapshot.meeting_id,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                lifecycle_run_id,
                crate::database::repositories::meeting::MEETING_STATUS_PROCESSING,
                Some(active.snapshot.recording_duration),
                None,
            )
            .await
            {
                log::warn!("Failed to persist recording processing state: {error}");
            }
        }
        self.finish_active_session(session_id, active, Vec::new())
            .await
    }

    async fn shutdown_after_control_failure(
        &self,
        session_id: SessionId,
        error: ContractError,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let active = {
            let mut store = self.inner.store.lock().await;
            let mut active = take_matching_active(&mut store.active, &session_id, |active| {
                active.snapshot.session_id.as_str()
            })
            .ok_or_else(|| ContractError::not_found(Some(session_id.clone())))?;
            active.snapshot.state =
                reduce_transition(active.snapshot.state, RecordingSessionAction::StopRequested)
                    .unwrap_or(RecordingSessionState::Stopping);
            store
                .snapshots
                .insert(session_id.clone(), active.snapshot.clone());
            active
        };
        let details = format!("control failure: {:?}", error.details);
        self.finish_active_session(session_id, active, vec![(error.code, details)])
            .await
    }

    async fn finish_active_session(
        &self,
        session_id: SessionId,
        mut active: ActiveSession,
        mut failures: Vec<(ContractErrorCode, String)>,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        // Make the post-capture work visible before waiting for audio writers,
        // transcription, and artifact finalization to finish.
        Self::refresh_durations(&mut active);
        if let (Some(pool), Some(lifecycle_run_id)) = (
            active.lifecycle_pool.as_ref(),
            active.lifecycle_run_id.as_deref(),
        ) {
            if let Err(error) = crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
                pool,
                &active.snapshot.meeting_id,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                lifecycle_run_id,
                crate::database::repositories::meeting::MEETING_STATUS_PROCESSING,
                Some(active.snapshot.recording_duration),
                None,
            )
            .await
            {
                log::warn!("Failed to persist recording finalization state: {error}");
            }
        }
        let capture_result = match active.capture.take() {
            Some(capture) => tokio::task::spawn_blocking(move || stop_and_join_capture(capture))
                .await
                .map_err(|error| error.to_string()),
            None => Err("Capture owner handle is missing".into()),
        };
        let mut closed_sources = None;
        match capture_result {
            Ok((acknowledged, joined)) => {
                match acknowledged {
                    Ok(stopped) => closed_sources = Some(stopped.closed_sources),
                    Err(cause) => failures.push((
                        ContractErrorCode::CaptureFailed,
                        format!("stop acknowledgement: {cause:?}"),
                    )),
                }
                if let Err(cause) = joined {
                    failures.push((
                        ContractErrorCode::CaptureFailed,
                        format!("capture owner join: {cause:?}"),
                    ));
                }
            }
            Err(error) => failures.push((
                ContractErrorCode::CaptureFailed,
                format!("capture owner task: {error}"),
            )),
        }
        if let Some(closed_sources) = closed_sources.as_ref() {
            if !reconcile_closed_capture_sources(
                &mut active.capture_sources,
                &mut active.snapshot.expected_tracks,
                closed_sources,
            ) {
                failures.push((
                    ContractErrorCode::CaptureFailed,
                    "closed source set differs from expected Tracks".into(),
                ));
            }
        } else {
            failures.push((
                ContractErrorCode::CaptureFailed,
                "closed source set differs from expected Tracks".into(),
            ));
        }
        active.snapshot.state = reduce_transition(
            RecordingSessionState::Stopping,
            RecordingSessionAction::CaptureOwnerJoined,
        )
        .expect("contract transition");
        {
            let mut store = self.inner.store.lock().await;
            store
                .snapshots
                .insert(session_id.clone(), active.snapshot.clone());
        }

        let microphone_join = active.microphone_writer.join.take();
        let system_join = active
            .system_writer
            .as_mut()
            .and_then(|writer| writer.join.take());
        let microphone_downstream_join = active.microphone_downstream.join.take();
        let system_downstream_join = active
            .system_downstream
            .as_mut()
            .and_then(|writer| writer.join.take());
        let drained = tokio::task::spawn_blocking(move || {
            let microphone = microphone_join
                .ok_or_else(|| "Microphone Track writer was already joined".to_string())?
                .join()
                .map_err(|_| "Microphone Track writer panicked".to_string())?;
            let system = match system_join {
                Some(join) => Some(
                    join.join()
                        .map_err(|_| "System Track writer panicked".to_string())?,
                ),
                None => None,
            };
            let microphone_downstream = microphone_downstream_join
                .ok_or_else(|| "Microphone downstream writer was already joined".to_string())?
                .join()
                .map_err(|_| "Microphone downstream writer panicked".to_string())?;
            let system_downstream = match system_downstream_join {
                Some(join) => Some(
                    join.join()
                        .map_err(|_| "System downstream writer panicked".to_string())?,
                ),
                None => None,
            };
            Ok::<_, String>((microphone, system, microphone_downstream, system_downstream))
        })
        .await;
        let (microphone_saver, system_saver) = match drained {
            Ok(Ok((microphone, system, microphone_downstream, system_downstream))) => {
                if let Err(error) = &microphone_downstream {
                    failures.push((
                        ContractErrorCode::PersistenceFailed,
                        format!("microphone downstream drain: {error}"),
                    ));
                }
                if let Some(Err(error)) = &system_downstream {
                    failures.push((
                        ContractErrorCode::PersistenceFailed,
                        format!("system downstream drain: {error}"),
                    ));
                }
                let microphone = match microphone {
                    Ok(saver) => saver,
                    Err(error) => {
                        failures.push((
                            ContractErrorCode::PersistenceFailed,
                            format!("microphone Track drain: {error}"),
                        ));
                        None
                    }
                };
                let system = match system {
                    Some(Ok(saver)) => saver,
                    Some(Err(error)) => {
                        failures.push((
                            ContractErrorCode::PersistenceFailed,
                            format!("system Track drain: {error}"),
                        ));
                        None
                    }
                    None => None,
                };
                (microphone, system)
            }
            Ok(Err(error)) => {
                failures.push((
                    ContractErrorCode::PersistenceFailed,
                    format!("Track drain: {error}"),
                ));
                (None, None)
            }
            Err(error) => {
                failures.push((
                    ContractErrorCode::PersistenceFailed,
                    format!("Track drain task: {error}"),
                ));
                (None, None)
            }
        };

        active.pipeline_state.stop_recording();
        if let Err(error) = active.pipeline.force_flush_and_stop().await {
            failures.push((
                ContractErrorCode::PersistenceFailed,
                format!("audio pipeline finalization: {error}"),
            ));
        }
        if let Some(transcription_join) = active.transcription_join.take() {
            if let Err(error) = transcription_join.await {
                failures.push((
                    ContractErrorCode::PersistenceFailed,
                    format!("transcription task join: {error}"),
                ));
            }
        }
        match active.mixed_writer.take() {
            Some(join) => match join.await {
                Ok(Ok(mut saver)) => {
                    if let Err(error) = saver.finalize().await {
                        failures.push((
                            ContractErrorCode::PersistenceFailed,
                            format!("mixed audio finalization: {error}"),
                        ));
                    }
                }
                Ok(Err(error)) => failures.push((
                    ContractErrorCode::PersistenceFailed,
                    format!("mixed checkpoint saver: {error}"),
                )),
                Err(error) => failures.push((
                    ContractErrorCode::PersistenceFailed,
                    format!("mixed saver task join: {error}"),
                )),
            },
            None if active.snapshot.auto_save => failures.push((
                ContractErrorCode::PersistenceFailed,
                "mixed saver task is missing".into(),
            )),
            None => {}
        }

        let finalized = tokio::task::spawn_blocking(move || {
            let microphone = microphone_saver.map(SourceTrackSaver::finalize).transpose();
            let system = system_saver.map(SourceTrackSaver::finalize).transpose();
            (microphone, system)
        })
        .await;
        let mut captured_tracks = Vec::new();
        match finalized {
            Ok((microphone, system)) => {
                collect_finalized_track(
                    TrackKind::Microphone,
                    microphone,
                    &mut captured_tracks,
                    &mut failures,
                );
                collect_finalized_track(
                    TrackKind::System,
                    system,
                    &mut captured_tracks,
                    &mut failures,
                );
            }
            Err(error) => failures.push((
                ContractErrorCode::PersistenceFailed,
                format!("source Track finalizer task: {error}"),
            )),
        }
        reconcile_optional_system_track(&mut active.snapshot.expected_tracks, &captured_tracks);
        let captured_kinds: BTreeSet<_> = captured_tracks.iter().map(|track| track.kind).collect();
        if captured_kinds != active.snapshot.expected_tracks {
            failures.push((
                ContractErrorCode::CaptureFailed,
                "verified Track set differs from expected Tracks".into(),
            ));
        }
        if !captured_tracks.is_empty() {
            active.snapshot.captured_tracks = Some(captured_tracks);
        }
        Self::refresh_durations(&mut active);
        if failures.is_empty() {
            let meeting_folder = std::path::Path::new(&active.snapshot.save_path);
            for kind in [TrackKind::Microphone, TrackKind::System] {
                if let Err(error) = cleanup_source_track_checkpoints(meeting_folder, kind) {
                    failures.push((
                        ContractErrorCode::PersistenceFailed,
                        format!("{kind:?} source checkpoint cleanup: {error}"),
                    ));
                }
            }
        }
        if failures.is_empty() {
            let mut persisted_snapshot = active.snapshot.clone();
            persisted_snapshot.state = transition_state(
                persisted_snapshot.state,
                RecordingSessionAction::FinalizationVerified,
            );
            if let Err(error) =
                finalize_session_artifacts(&persisted_snapshot, &active.transcript_history)
            {
                failures.push((
                    ContractErrorCode::PersistenceFailed,
                    format!("session artifact finalization: {error}"),
                ));
            } else {
                active.snapshot.state = transition_state(
                    active.snapshot.state,
                    RecordingSessionAction::FinalizationVerified,
                );
            }
        }
        if !failures.is_empty() {
            log::error!(
                "Recording session {session_id} finalization failed: {:?}",
                failures
            );
            let (state, code, retryable) = transition_failure(active.snapshot.state, &failures);
            active.snapshot.state = state;
            let error = self.error(
                code,
                "Recording did not stop cleanly",
                retryable,
                Some(session_id.clone()),
                Some(state),
                Some(
                    failures
                        .into_iter()
                        .map(|(_, details)| details)
                        .collect::<Vec<_>>()
                        .join("; "),
                ),
            );
            active.snapshot.error = Some(error.clone());
            let _ = finalize_session_artifacts(&active.snapshot, &active.transcript_history);
            let snapshot = active.snapshot.clone();
            self.inner
                .store
                .lock()
                .await
                .snapshots
                .insert(session_id, snapshot.clone());
            if let Some(notifier) = self.inner.terminal_notifier.lock().unwrap().clone() {
                notifier(&snapshot);
            }
            if let (Some(pool), Some(lifecycle_run_id)) = (
                active.lifecycle_pool.as_ref(),
                active.lifecycle_run_id.as_deref(),
            ) {
                if let Err(persistence_error) = crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
                    pool,
                    &snapshot.meeting_id,
                    crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                    lifecycle_run_id,
                    crate::database::repositories::meeting::MEETING_STATUS_ERROR,
                    Some(snapshot.recording_duration),
                    Some("Recording did not stop cleanly"),
                )
                .await
                {
                    log::warn!("Failed to persist recording error state: {persistence_error}");
                }
            }
            return Err(error);
        }
        let snapshot = active.snapshot.clone();
        if let (Some(pool), Some(lifecycle_run_id)) = (
            active.lifecycle_pool.as_ref(),
            active.lifecycle_run_id.as_deref(),
        ) {
            if let Err(error) = crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
                pool,
                &snapshot.meeting_id,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                lifecycle_run_id,
                crate::database::repositories::meeting::MEETING_STATUS_READY,
                Some(snapshot.recording_duration),
                None,
            )
            .await
            {
                log::warn!("Failed to persist recording ready state: {error}");
            }
        }
        self.inner
            .store
            .lock()
            .await
            .snapshots
            .insert(session_id, snapshot.clone());
        Ok(snapshot)
    }

    pub async fn recover(
        &self,
        session_id: SessionId,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let _action = self.inner.action_gate.lock().await;
        let mut snapshot = self
            .inner
            .store
            .lock()
            .await
            .snapshots
            .get(&session_id)
            .cloned()
            .ok_or_else(|| ContractError::not_found(Some(session_id.clone())))?;
        if snapshot.state == RecordingSessionState::Completed {
            return Ok(snapshot);
        }
        if matches!(
            snapshot.state,
            RecordingSessionState::Completed | RecordingSessionState::Failed
        ) {
            return Ok(snapshot);
        }
        if snapshot.state != RecordingSessionState::Interrupted {
            return Err(self.error(
                ContractErrorCode::InvalidState,
                "Only interrupted recording sessions can be recovered",
                false,
                Some(session_id),
                Some(snapshot.state),
                None,
            ));
        }
        snapshot.state = reduce_transition(
            RecordingSessionState::Interrupted,
            RecordingSessionAction::CaptureOwnerJoined,
        )
        .expect("contract transition");
        self.inner
            .store
            .lock()
            .await
            .snapshots
            .insert(session_id.clone(), snapshot.clone());

        let meeting_folder = std::path::PathBuf::from(&snapshot.save_path);
        let expected_tracks = snapshot.expected_tracks.clone();
        let recovered = tokio::task::spawn_blocking(move || {
            if expected_tracks.is_empty() {
                return (
                    Vec::new(),
                    Some((
                        ContractErrorCode::IntegrityFailed,
                        "Interrupted session has no expected Tracks".into(),
                    )),
                );
            }
            let mut verified_partial_tracks = Vec::with_capacity(expected_tracks.len());
            for kind in expected_tracks {
                let track = match SourceTrackSaver::recover(&meeting_folder, kind) {
                    Ok(Some(track)) => track,
                    Ok(None) => {
                        return (
                            verified_partial_tracks,
                            Some((
                                ContractErrorCode::CaptureFailed,
                                format!("Missing expected {kind:?} recovery namespace"),
                            )),
                        )
                    }
                    Err(error) => {
                        let failure = match error {
                            SourceTrackRecoveryError::Integrity(details) => {
                                (ContractErrorCode::IntegrityFailed, details)
                            }
                            SourceTrackRecoveryError::Persistence(details) => {
                                (ContractErrorCode::PersistenceFailed, details)
                            }
                        };
                        return (verified_partial_tracks, Some(failure));
                    }
                };
                if track.byte_length == 0 || track.sample_count == 0 || track.sha256.len() != 64 {
                    return (
                        verified_partial_tracks,
                        Some((
                            ContractErrorCode::IntegrityFailed,
                            format!("Recovered {kind:?} Track failed integrity validation"),
                        )),
                    );
                }
                verified_partial_tracks.push(CapturedTrack {
                    kind: track.kind,
                    sample_rate: track.sample_rate,
                    channels: track.channels,
                    path: track.path.to_string_lossy().into_owned(),
                    byte_length: track.byte_length,
                    sample_count: track.sample_count,
                    sha256: track.sha256,
                });
            }
            (verified_partial_tracks, None)
        })
        .await;

        let captured_tracks = match recovered {
            Ok((tracks, None)) => {
                let expected_tracks = snapshot.expected_tracks.clone();
                let captured_kinds: BTreeSet<_> = tracks.iter().map(|track| track.kind).collect();
                if captured_kinds != expected_tracks {
                    return self
                        .record_recovery_failure(
                            snapshot,
                            ContractErrorCode::IntegrityFailed,
                            "Recovered Track set differs from expected Tracks".into(),
                            tracks,
                        )
                        .await;
                }
                tracks
            }
            Ok((tracks, Some((code, details)))) => {
                return self
                    .record_recovery_failure(snapshot, code, details, tracks)
                    .await;
            }
            Err(error) => {
                return self
                    .record_recovery_failure(
                        snapshot,
                        ContractErrorCode::PersistenceFailed,
                        error.to_string(),
                        Vec::new(),
                    )
                    .await;
            }
        };
        snapshot.captured_tracks = Some(captured_tracks);
        snapshot.error = None;
        let transcript_history = self
            .inner
            .store
            .lock()
            .await
            .transcript_histories
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| Arc::new(StdMutex::new(Vec::new())));
        let mut persisted_snapshot = snapshot.clone();
        persisted_snapshot.state = transition_state(
            persisted_snapshot.state,
            RecordingSessionAction::FinalizationVerified,
        );
        if let Err(error) = finalize_session_artifacts(&persisted_snapshot, &transcript_history) {
            let verified_tracks = snapshot.captured_tracks.clone().unwrap_or_default();
            return self
                .record_recovery_failure(
                    snapshot,
                    ContractErrorCode::PersistenceFailed,
                    format!("session artifact finalization: {error}"),
                    verified_tracks,
                )
                .await;
        }
        snapshot.state =
            transition_state(snapshot.state, RecordingSessionAction::FinalizationVerified);
        let (lifecycle_pool, lifecycle_run_id) = {
            let store = self.inner.store.lock().await;
            (
                store.lifecycle_pools.get(&session_id).cloned(),
                store.lifecycle_run_ids.get(&session_id).cloned(),
            )
        };
        self.inner
            .store
            .lock()
            .await
            .snapshots
            .insert(session_id, snapshot.clone());
        if let (Some(pool), Some(lifecycle_run_id)) = (lifecycle_pool, lifecycle_run_id) {
            match crate::database::repositories::meeting::MeetingsRepository::recover_lifecycle_for_operation(
                &pool,
                &snapshot.meeting_id,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                &lifecycle_run_id,
                Some(snapshot.recording_duration),
            )
            .await
            {
                Ok(true) => {}
                Ok(false) => {
                    log::warn!(
                        "Recording recovery no longer owns meeting {}; leaving its lifecycle unchanged",
                        snapshot.meeting_id
                    );
                }
                Err(error) => {
                    log::warn!("Failed to persist recovered recording ready state: {error}");
                }
            }
        }
        Ok(snapshot)
    }

    async fn record_recovery_failure(
        &self,
        mut snapshot: RecordingSessionSnapshot,
        code: ContractErrorCode,
        details: String,
        verified_partial_tracks: Vec<CapturedTrack>,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let failures = [(code, details.clone())];
        let (state, _, retryable) = transition_failure(snapshot.state, &failures);
        snapshot.state = state;
        snapshot.captured_tracks =
            (!verified_partial_tracks.is_empty()).then_some(verified_partial_tracks);
        let error = self.error(
            code,
            "Recording recovery failed",
            retryable,
            Some(snapshot.session_id.clone()),
            Some(snapshot.state),
            Some(details),
        );
        snapshot.error = Some(error.clone());
        let (lifecycle_pool, lifecycle_run_id) = {
            let store = self.inner.store.lock().await;
            (
                store.lifecycle_pools.get(&snapshot.session_id).cloned(),
                store.lifecycle_run_ids.get(&snapshot.session_id).cloned(),
            )
        };
        let meeting_id = snapshot.meeting_id.clone();
        let recording_duration = snapshot.recording_duration;
        let session_id = snapshot.session_id.clone();
        self.inner
            .store
            .lock()
            .await
            .snapshots
            .insert(session_id, snapshot);
        if let (Some(pool), Some(lifecycle_run_id)) = (lifecycle_pool, lifecycle_run_id) {
            if let Err(persistence_error) = crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
                &pool,
                &meeting_id,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
                &lifecycle_run_id,
                crate::database::repositories::meeting::MEETING_STATUS_ERROR,
                Some(recording_duration),
                Some("Recording recovery failed"),
            )
            .await
            {
                log::warn!("Failed to persist recording recovery error state: {persistence_error}");
            }
        }
        Err(error)
    }

    pub async fn get(
        &self,
        session_id: SessionId,
    ) -> Result<RecordingSessionSnapshot, ContractError> {
        let mut store = self.inner.store.lock().await;
        let active_snapshot = store
            .active
            .as_mut()
            .filter(|active| active.snapshot.session_id == session_id)
            .map(|active| {
                Self::refresh_durations(active);
                active.snapshot.clone()
            });
        if let Some(snapshot) = active_snapshot {
            store.snapshots.insert(session_id.clone(), snapshot);
        }
        store.snapshots.get(&session_id).cloned().ok_or_else(|| {
            self.error(
                ContractErrorCode::NotFound,
                "Recording session not found",
                false,
                Some(session_id),
                None,
                None,
            )
        })
    }

    pub async fn get_active(&self) -> Option<RecordingSessionSnapshot> {
        let mut store = self.inner.store.lock().await;
        if let Some(active) = store.active.as_mut() {
            Self::refresh_durations(active);
        }
        store.active.as_ref().map(|active| {
            let _ = &active.identity;
            active.snapshot.clone()
        })
    }

    pub async fn shutdown_active(&self) -> Result<Option<RecordingSessionSnapshot>, ContractError> {
        let _action = self.inner.action_gate.lock().await;
        self.latch_shutdown();
        let session_id = {
            let store = self.inner.store.lock().await;
            store
                .active
                .as_ref()
                .map(|active| active.snapshot.session_id.clone())
        };
        match session_id {
            Some(session_id) => self.stop_locked(session_id).await.map(Some),
            None => Ok(None),
        }
    }

    fn refresh_durations(active: &mut ActiveSession) {
        let total = active.started.elapsed();
        let current_pause = active
            .pause_started
            .map(|started| started.elapsed())
            .unwrap_or_default();
        active.snapshot.recording_duration = total.as_secs_f64();
        active.snapshot.pause_duration = (active.pause_total + current_pause).as_secs_f64();
        active.snapshot.active_duration = total
            .saturating_sub(active.pause_total + current_pause)
            .as_secs_f64();
    }

    fn capture_permission_error(&self, error: AudioPermissionError) -> ContractError {
        match error {
            AudioPermissionError::MicrophoneDenied => self.error(
                ContractErrorCode::PermissionDenied,
                "Microphone permission denied",
                false,
                None,
                None,
                None,
            ),
            AudioPermissionError::MicrophoneUnavailable => self.error(
                ContractErrorCode::Internal,
                "Microphone permission state is unavailable",
                true,
                None,
                None,
                None,
            ),
            AudioPermissionError::MicrophoneRequestFailed(details) => self.error(
                ContractErrorCode::Internal,
                "Microphone permission request failed",
                true,
                None,
                None,
                Some(details),
            ),
        }
    }

    fn capture_error(
        &self,
        session_id: &str,
        state: RecordingSessionState,
        cause: CaptureCause,
    ) -> ContractError {
        let (code, retryable) = match &cause {
            CaptureCause::PermissionDenied(_) => (ContractErrorCode::PermissionDenied, false),
            CaptureCause::DeviceUnavailable(_) => (ContractErrorCode::DeviceUnavailable, true),
            CaptureCause::UnsupportedCapability(_) => {
                (ContractErrorCode::UnsupportedCapability, false)
            }
            _ => (ContractErrorCode::CaptureFailed, true),
        };
        self.error(
            code,
            "Recording capture failed",
            retryable,
            Some(session_id.into()),
            Some(state),
            Some(format!("{cause:?}")),
        )
    }

    fn error(
        &self,
        code: ContractErrorCode,
        message: &str,
        retryable: bool,
        session_id: Option<String>,
        current_state: Option<RecordingSessionState>,
        details: Option<String>,
    ) -> ContractError {
        ContractError {
            code,
            message: message.into(),
            retryable,
            session_id,
            job_id: None,
            current_state,
            details,
        }
    }
}

impl Default for RecordingSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::capture::backend::{
        CaptureBackend, CaptureCapabilities, CaptureRuntimeEvent, TrackSinks,
    };
    use super::*;
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::Row;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex as StdMutex;
    use tempfile::tempdir;

    static LIFECYCLE_MIGRATOR: Migrator = sqlx::migrate!("./migrations");

    struct IdentityTracking {
        next_stream_identity: Arc<AtomicUsize>,
        stream_identities: Arc<StdMutex<Vec<usize>>>,
        drop_count: Arc<AtomicUsize>,
    }

    struct OrderedFakeBackend {
        events: Arc<StdMutex<Vec<&'static str>>>,
        dropped: Arc<AtomicBool>,
        identity_tracking: Option<IdentityTracking>,
        sinks: Option<TrackSinks>,
        pause_fails: bool,
        pump_fails: bool,
        pump_delay: Duration,
        system_capable: bool,
    }

    impl Drop for OrderedFakeBackend {
        fn drop(&mut self) {
            if let Some(tracking) = &self.identity_tracking {
                tracking.drop_count.fetch_add(1, Ordering::AcqRel);
            }
            self.events.lock().unwrap().push("drop");
            self.dropped.store(true, Ordering::Release);
        }
    }

    impl CaptureBackend for OrderedFakeBackend {
        fn capabilities(&self) -> CaptureCapabilities {
            CaptureCapabilities {
                microphone: true,
                system: self.system_capable,
            }
        }

        fn start(
            &mut self,
            _request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<(), CaptureCause> {
            self.events.lock().unwrap().push("start");
            if let Some(tracking) = &self.identity_tracking {
                let stream_identity = tracking.next_stream_identity.fetch_add(1, Ordering::AcqRel);
                tracking
                    .stream_identities
                    .lock()
                    .unwrap()
                    .push(stream_identity);
            }
            let chunk = super::super::capture::backend::TrackChunk::contiguous(
                TrackKind::Microphone,
                vec![0.2; if self.system_capable { 28_800 } else { 4_800 }],
                48_000,
                1,
            );
            sinks.microphone.raw.try_send(chunk.clone())?;
            sinks.microphone.downstream.try_send(chunk)?;
            let mut sinks = sinks;
            sinks.system.take();
            self.sinks = Some(sinks);
            Ok(())
        }

        fn opened_sources(&self) -> BTreeSet<TrackKind> {
            BTreeSet::from([TrackKind::Microphone])
        }

        fn pause(&mut self) -> Result<(), CaptureCause> {
            self.events.lock().unwrap().push("pause");
            if self.pause_fails {
                Err(CaptureCause::CaptureFailed("fake pause".into()))
            } else {
                Ok(())
            }
        }

        fn resume(&mut self) -> Result<(), CaptureCause> {
            Ok(())
        }

        fn pump(
            &mut self,
        ) -> Result<Option<super::super::capture::backend::CaptureRuntimeEvent>, CaptureCause>
        {
            if !self.pump_delay.is_zero() {
                std::thread::sleep(self.pump_delay);
            }
            if self.pump_fails {
                Err(CaptureCause::CaptureFailed("fake pump".into()))
            } else {
                Ok(None)
            }
        }

        fn stop(&mut self) -> Result<super::super::capture::backend::CaptureStopped, CaptureCause> {
            self.events.lock().unwrap().push("stop");
            self.sinks.take();
            Ok(super::super::capture::backend::CaptureStopped {
                closed_sources: BTreeSet::from([TrackKind::Microphone]),
            })
        }
    }

    struct IdentityTrackingCaptureOwnerFactory {
        next_owner_identity: AtomicUsize,
        next_stream_identity: Arc<AtomicUsize>,
        owner_identities: Arc<StdMutex<Vec<usize>>>,
        stream_identities: Arc<StdMutex<Vec<usize>>>,
        drop_counts: Arc<StdMutex<Vec<Arc<AtomicUsize>>>>,
        join_counts: Arc<StdMutex<Vec<Arc<AtomicUsize>>>>,
    }

    impl CaptureOwnerFactory for IdentityTrackingCaptureOwnerFactory {
        fn spawn(
            &self,
            request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<CaptureThreadHandle, CaptureCause> {
            let owner_identity = self.next_owner_identity.fetch_add(1, Ordering::AcqRel);
            self.owner_identities.lock().unwrap().push(owner_identity);
            let drop_count = Arc::new(AtomicUsize::new(0));
            self.drop_counts.lock().unwrap().push(drop_count.clone());
            let join_count = Arc::new(AtomicUsize::new(0));
            self.join_counts.lock().unwrap().push(join_count.clone());
            let identity_tracking = IdentityTracking {
                next_stream_identity: self.next_stream_identity.clone(),
                stream_identities: self.stream_identities.clone(),
                drop_count,
            };
            let mut handle = CaptureThreadHandle::spawn_with_backend(request, sinks, move || {
                OrderedFakeBackend {
                    events: Arc::new(StdMutex::new(Vec::new())),
                    dropped: Arc::new(AtomicBool::new(false)),
                    identity_tracking: Some(identity_tracking),
                    sinks: None,
                    pause_fails: false,
                    pump_fails: false,
                    pump_delay: Duration::ZERO,
                    system_capable: false,
                }
            })?;
            handle.track_join_calls(join_count);
            Ok(handle)
        }
    }

    struct FakeCaptureOwnerFactory {
        events: Arc<StdMutex<Vec<&'static str>>>,
        dropped: Arc<AtomicBool>,
        pause_fails: bool,
        pump_fails: bool,
        pump_delay: Duration,
        system_capable: bool,
        requests: Arc<StdMutex<Vec<CaptureRequest>>>,
    }

    impl CaptureOwnerFactory for FakeCaptureOwnerFactory {
        fn spawn(
            &self,
            request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<CaptureThreadHandle, CaptureCause> {
            self.requests.lock().unwrap().push(request.clone());
            let events = self.events.clone();
            let dropped = self.dropped.clone();
            let pause_fails = self.pause_fails;
            let pump_fails = self.pump_fails;
            let pump_delay = self.pump_delay;
            let system_capable = self.system_capable;
            CaptureThreadHandle::spawn_with_backend(request, sinks, move || OrderedFakeBackend {
                events,
                dropped,
                identity_tracking: None,
                sinks: None,
                pause_fails,
                pump_fails,
                pump_delay,
                system_capable,
            })
        }
    }

    struct FailingCaptureOwnerFactory;

    impl CaptureOwnerFactory for FailingCaptureOwnerFactory {
        fn spawn(
            &self,
            _request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<CaptureThreadHandle, CaptureCause> {
            let chunk = super::super::capture::backend::TrackChunk::contiguous(
                TrackKind::Microphone,
                vec![0.25; 4_800],
                48_000,
                1,
            );
            sinks.microphone.raw.try_send(chunk.clone())?;
            sinks.microphone.downstream.try_send(chunk)?;
            Err(CaptureCause::DeviceUnavailable(
                "injected partial-start failure".into(),
            ))
        }
    }

    struct RuntimeEventCaptureOwnerFactory {
        runtime_event: Option<CaptureRuntimeEvent>,
        terminal_cause: Option<CaptureCause>,
    }

    struct RuntimeEventFakeBackend {
        runtime_event: Option<CaptureRuntimeEvent>,
        terminal_cause: Option<CaptureCause>,
        opened_sources: BTreeSet<TrackKind>,
        sinks: Option<TrackSinks>,
    }

    impl CaptureOwnerFactory for RuntimeEventCaptureOwnerFactory {
        fn spawn(
            &self,
            request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<CaptureThreadHandle, CaptureCause> {
            let runtime_event = self.runtime_event.clone();
            let terminal_cause = self.terminal_cause.clone();
            CaptureThreadHandle::spawn_with_backend(request, sinks, move || {
                RuntimeEventFakeBackend {
                    runtime_event,
                    terminal_cause,
                    opened_sources: BTreeSet::new(),
                    sinks: None,
                }
            })
        }
    }

    impl CaptureBackend for RuntimeEventFakeBackend {
        fn capabilities(&self) -> CaptureCapabilities {
            CaptureCapabilities {
                microphone: true,
                system: true,
            }
        }

        fn start(
            &mut self,
            _request: CaptureRequest,
            sinks: TrackSinks,
        ) -> Result<(), CaptureCause> {
            let microphone = super::super::capture::backend::TrackChunk::contiguous(
                TrackKind::Microphone,
                vec![0.2; 28_800],
                48_000,
                1,
            );
            sinks.microphone.raw.try_send(microphone.clone())?;
            sinks.microphone.downstream.try_send(microphone)?;
            self.opened_sources.insert(TrackKind::Microphone);
            if let Some(system) = &sinks.system {
                let chunk = super::super::capture::backend::TrackChunk::contiguous(
                    TrackKind::System,
                    vec![0.1; 28_800],
                    48_000,
                    1,
                );
                system.raw.try_send(chunk.clone())?;
                system.downstream.try_send(chunk)?;
                self.opened_sources.insert(TrackKind::System);
            }
            self.sinks = Some(sinks);
            Ok(())
        }

        fn opened_sources(&self) -> BTreeSet<TrackKind> {
            self.opened_sources.clone()
        }

        fn pause(&mut self) -> Result<(), CaptureCause> {
            Ok(())
        }

        fn resume(&mut self) -> Result<(), CaptureCause> {
            Ok(())
        }

        fn pump(&mut self) -> Result<Option<CaptureRuntimeEvent>, CaptureCause> {
            if let Some(event) = self.runtime_event.take() {
                return Ok(Some(event));
            }
            if let Some(cause) = self.terminal_cause.take() {
                return Err(cause);
            }
            Ok(None)
        }

        fn stop(&mut self) -> Result<super::super::capture::backend::CaptureStopped, CaptureCause> {
            self.sinks.take();
            Ok(super::super::capture::backend::CaptureStopped {
                closed_sources: self.opened_sources.clone(),
            })
        }
    }

    #[derive(Clone)]
    enum FakePermissionPreparation {
        PassThrough,
        Result(Result<bool, AudioPermissionError>),
        Blocked { started: Arc<tokio::sync::Notify> },
    }

    #[derive(Clone)]
    struct FakeSessionRuntime {
        preferences: super::super::recording_preferences::RecordingPreferences,
        model_error: Option<String>,
        notifications: Arc<StdMutex<Vec<RecordingSessionState>>>,
        capture_notifications: Arc<StdMutex<Vec<(SessionId, CaptureRuntimeEvent)>>>,
        permission_preparation: Arc<StdMutex<FakePermissionPreparation>>,
        permission_calls: Arc<AtomicUsize>,
        transcription_starts: Arc<AtomicUsize>,
        transcription_vocabularies: Arc<StdMutex<Vec<Option<String>>>>,
    }

    impl RecordingSessionRuntime for FakeSessionRuntime {
        fn load_preferences(
            &self,
        ) -> StartFuture<
            '_,
            anyhow::Result<super::super::recording_preferences::RecordingPreferences>,
        > {
            let preferences = self.preferences.clone();
            Box::pin(async move { Ok(preferences) })
        }

        fn validate_model(&self) -> StartFuture<'_, Result<(), String>> {
            let error = self.model_error.clone();
            Box::pin(async move { error.map_or(Ok(()), Err) })
        }

        fn start_transcription(
            &self,
            _transcript_history: Arc<StdMutex<Vec<TranscriptSegment>>>,
            _meeting_id: &str,
            meeting_vocabulary: Option<&str>,
        ) -> TranscriptionTask {
            self.transcription_starts.fetch_add(1, Ordering::AcqRel);
            self.transcription_vocabularies
                .lock()
                .unwrap()
                .push(meeting_vocabulary.map(str::to_owned));
            let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
            let join = tokio::spawn(async move { while receiver.recv().await.is_some() {} });
            (sender, join)
        }

        fn terminal_notifier(&self) -> Option<TerminalNotifier> {
            let notifications = self.notifications.clone();
            Some(Arc::new(move |snapshot| {
                notifications.lock().unwrap().push(snapshot.state);
            }))
        }

        fn prepare_capture_permissions(
            &self,
            requested_system: bool,
        ) -> StartFuture<'_, Result<bool, AudioPermissionError>> {
            self.permission_calls.fetch_add(1, Ordering::AcqRel);
            let preparation = self.permission_preparation.lock().unwrap().clone();
            Box::pin(async move {
                match preparation {
                    FakePermissionPreparation::PassThrough => Ok(requested_system),
                    FakePermissionPreparation::Result(result) => result,
                    FakePermissionPreparation::Blocked { started } => {
                        started.notify_one();
                        std::future::pending().await
                    }
                }
            })
        }

        fn capture_event_notifier(
            &self,
        ) -> Option<Arc<dyn Fn(&SessionId, &CaptureRuntimeEvent) + Send + Sync>> {
            let notifications = self.capture_notifications.clone();
            Some(Arc::new(move |session_id, event| {
                notifications
                    .lock()
                    .unwrap()
                    .push((session_id.clone(), event.clone()));
            }))
        }
    }

    fn fake_runtime(
        save_folder: std::path::PathBuf,
        auto_save: bool,
        preferred_microphone: Option<&str>,
    ) -> FakeSessionRuntime {
        let preferences = super::super::recording_preferences::RecordingPreferences {
            save_folder,
            auto_save,
            preferred_mic_device: preferred_microphone.map(str::to_owned),
            ..Default::default()
        };
        FakeSessionRuntime {
            preferences,
            model_error: None,
            notifications: Arc::new(StdMutex::new(Vec::new())),
            capture_notifications: Arc::new(StdMutex::new(Vec::new())),
            permission_preparation: Arc::new(StdMutex::new(FakePermissionPreparation::PassThrough)),
            permission_calls: Arc::new(AtomicUsize::new(0)),
            transcription_starts: Arc::new(AtomicUsize::new(0)),
            transcription_vocabularies: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    fn fake_session(requests: Arc<StdMutex<Vec<CaptureRequest>>>) -> RecordingSession {
        fake_session_with(requests, false, false)
    }

    fn fake_session_with(
        requests: Arc<StdMutex<Vec<CaptureRequest>>>,
        pause_fails: bool,
        pump_fails: bool,
    ) -> RecordingSession {
        RecordingSession::new_with_capture_factory(Arc::new(FakeCaptureOwnerFactory {
            events: Arc::new(StdMutex::new(Vec::new())),
            dropped: Arc::new(AtomicBool::new(false)),
            pause_fails,
            pump_fails,
            pump_delay: Duration::ZERO,
            system_capable: false,
            requests,
        }))
    }

    fn public_start_request(
        save_path: std::path::PathBuf,
        idempotency_key: &str,
        microphone: Option<&str>,
        auto_save: bool,
    ) -> StartRecordingRequest {
        StartRecordingRequest {
            meeting_id: format!("meeting-{idempotency_key}"),
            idempotency_key: idempotency_key.into(),
            microphone_device: microphone.map(str::to_owned),
            microphone_device_explicit: false,
            system_device: None,
            capture_system: true,
            meeting_name: "Public seam".into(),
            meeting_vocabulary: None,
            save_path: Some(save_path.to_string_lossy().into_owned()),
            auto_save: Some(auto_save),
        }
    }

    #[tokio::test]
    async fn public_start_auto_save_false_keeps_metadata_but_no_audio_namespace() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("meeting-no-audio");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests);
        let runtime = fake_runtime(temp.path().to_path_buf(), true, None);
        let mut request = public_start_request(folder.clone(), "no-audio", None, false);
        request.meeting_vocabulary = Some("Meeting term".into());

        let started = session.start(runtime, request).await.unwrap();
        assert_eq!(started.save_path, folder.to_string_lossy());
        assert!(folder.join("metadata.json").is_file());
        assert!(folder.join("transcripts.json").is_file());
        assert!(!folder.join(".tracks").exists());
        assert!(!folder.join(".checkpoints").exists());
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.join("metadata.json")).unwrap()).unwrap();
        let expected_created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(started.started_at_ms as i64)
                .unwrap()
                .to_rfc3339();
        assert_eq!(metadata["created_at"], expected_created_at);

        let stopped = session.stop(started.session_id).await.unwrap();
        assert_eq!(stopped.state, RecordingSessionState::Completed);
        assert_eq!(stopped.started_at_ms, started.started_at_ms);
        assert!(stopped.expected_tracks.is_empty());
        assert!(stopped.captured_tracks.is_none());
        assert!(!folder.join("audio.mp4").exists());
        assert!(!folder.join("microphone.mp4").exists());
        assert!(!folder.join("system.mp4").exists());

        let error = session.pause(stopped.session_id).await.unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::InvalidState));
        assert_eq!(error.current_state, Some(RecordingSessionState::Completed));
    }

    #[tokio::test]
    async fn invalid_oversized_vocabulary_is_rejected_before_capture_or_resources() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("meeting-invalid-vocabulary");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let runtime = fake_runtime(temp.path().to_path_buf(), true, None);
        let transcription_starts = runtime.transcription_starts.clone();
        let transcription_vocabularies = runtime.transcription_vocabularies.clone();
        let mut request = public_start_request(folder.clone(), "oversized-vocabulary", None, true);
        request.meeting_vocabulary = Some("x".repeat(1001));

        let error = session.start(runtime, request).await.unwrap_err();

        assert!(matches!(error.code, ContractErrorCode::InvalidRequest));
        assert_eq!(error.message, "Vocabulary is limited to 1000 characters");
        assert!(error.session_id.is_none());
        assert!(error.current_state.is_none());
        assert_eq!(transcription_starts.load(Ordering::Acquire), 0);
        assert!(transcription_vocabularies.lock().unwrap().is_empty());
        assert!(requests.lock().unwrap().is_empty());
        assert!(!folder.exists());
        let store = session.inner.store.lock().await;
        assert!(store.identities.is_empty());
        assert!(store.snapshots.is_empty());
        assert!(store.active.is_none());
    }

    #[tokio::test]
    async fn invalid_nul_vocabulary_is_rejected_before_capture_or_resources() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("meeting-nul-vocabulary");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let runtime = fake_runtime(temp.path().to_path_buf(), true, None);
        let transcription_starts = runtime.transcription_starts.clone();
        let mut request = public_start_request(folder.clone(), "nul-vocabulary", None, true);
        request.meeting_vocabulary = Some("valid\0term".into());

        let error = session.start(runtime, request).await.unwrap_err();

        assert!(matches!(error.code, ContractErrorCode::InvalidRequest));
        assert_eq!(error.message, "Vocabulary cannot contain a null character");
        assert_eq!(transcription_starts.load(Ordering::Acquire), 0);
        assert!(requests.lock().unwrap().is_empty());
        assert!(!folder.exists());
        let store = session.inner.store.lock().await;
        assert!(store.identities.is_empty());
        assert!(store.snapshots.is_empty());
        assert!(store.active.is_none());
    }

    #[tokio::test]
    async fn valid_vocabulary_is_normalized_before_live_start_and_survives_stop() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("meeting-normalized-vocabulary");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests);
        let runtime = fake_runtime(temp.path().to_path_buf(), false, None);
        let transcription_vocabularies = runtime.transcription_vocabularies.clone();
        let mut request = public_start_request(folder, "normalized-vocabulary", None, false);
        request.meeting_vocabulary = Some("  Xantoris, xantoris\n Neworon  ".into());

        let started = session.start(runtime, request).await.unwrap();

        assert_eq!(
            started.meeting_vocabulary.as_deref(),
            Some("Xantoris, Neworon")
        );
        assert_eq!(
            transcription_vocabularies.lock().unwrap().as_slice(),
            &[Some("Xantoris, Neworon".to_string())]
        );

        let stopped = session.stop(started.session_id.clone()).await.unwrap();
        assert_eq!(stopped.session_id, started.session_id);
        assert_eq!(stopped.meeting_id, started.meeting_id);
        assert_eq!(
            stopped.meeting_vocabulary.as_deref(),
            Some("Xantoris, Neworon")
        );
    }

    #[tokio::test]
    async fn get_refreshes_live_recording_pause_and_resume_durations() {
        let temp = tempdir().unwrap();
        let session = fake_session(Arc::new(StdMutex::new(Vec::new())));
        let started = session
            .start(
                fake_runtime(temp.path().to_path_buf(), false, None),
                public_start_request(
                    temp.path().join("live-durations"),
                    "live-durations",
                    None,
                    false,
                ),
            )
            .await
            .unwrap();

        let initial = session.get(started.session_id.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        let recording = session.get(started.session_id.clone()).await.unwrap();
        assert!(recording.recording_duration > initial.recording_duration + 0.02);
        assert!(recording.active_duration > initial.active_duration + 0.02);

        let paused = session.pause(started.session_id.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        let paused_read = session.get(started.session_id.clone()).await.unwrap();
        assert!(paused_read.recording_duration > paused.recording_duration + 0.02);
        assert!((paused_read.active_duration - paused.active_duration).abs() < 0.02);

        let resumed = session.resume(started.session_id.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        let resumed_read = session.get(started.session_id.clone()).await.unwrap();
        assert!(resumed_read.recording_duration > resumed.recording_duration + 0.02);
        assert!(resumed_read.active_duration > resumed.active_duration + 0.02);

        session.stop(started.session_id).await.unwrap();
    }

    #[tokio::test]
    async fn microphone_permission_denial_preflight_creates_no_session_or_artifacts() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("permission-denied");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let runtime = fake_runtime(temp.path().to_path_buf(), false, None);
        *runtime.permission_preparation.lock().unwrap() =
            FakePermissionPreparation::Result(Err(AudioPermissionError::MicrophoneDenied));

        let error = session
            .start(
                runtime,
                public_start_request(folder.clone(), "permission-denied", None, false),
            )
            .await
            .unwrap_err();

        assert!(matches!(error.code, ContractErrorCode::PermissionDenied));
        assert!(error.session_id.is_none());
        assert!(requests.lock().unwrap().is_empty());
        assert!(!folder.exists());
    }

    #[tokio::test]
    async fn system_permission_downgrade_preserves_original_identity_for_retry() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("system-downgrade");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let runtime = fake_runtime(temp.path().to_path_buf(), false, None);
        *runtime.permission_preparation.lock().unwrap() =
            FakePermissionPreparation::Result(Ok(false));
        let request = public_start_request(folder, "system-downgrade", None, false);

        let started = session
            .start(runtime.clone(), request.clone())
            .await
            .unwrap();
        assert!(!requests.lock().unwrap()[0].capture_system);
        assert_eq!(runtime.permission_calls.load(Ordering::Acquire), 1);

        let retried = session.start(runtime.clone(), request).await.unwrap();
        assert_eq!(retried.session_id, started.session_id);
        assert_eq!(runtime.permission_calls.load(Ordering::Acquire), 1);

        session.stop(started.session_id).await.unwrap();
    }

    #[tokio::test]
    async fn shutdown_during_permission_preflight_cancels_before_factory_or_artifacts() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("permission-cancelled");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let runtime = fake_runtime(temp.path().to_path_buf(), false, None);
        let preparation_started = Arc::new(tokio::sync::Notify::new());
        *runtime.permission_preparation.lock().unwrap() = FakePermissionPreparation::Blocked {
            started: preparation_started.clone(),
        };
        let starting = session.clone();
        let start = tokio::spawn(async move {
            starting
                .start(
                    runtime,
                    public_start_request(folder, "permission-cancelled", None, false),
                )
                .await
        });

        preparation_started.notified().await;
        assert!(requests.lock().unwrap().is_empty());
        assert!(!temp.path().join("permission-cancelled").exists());
        session.latch_shutdown();

        let error = tokio::time::timeout(Duration::from_secs(1), start)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::Cancelled));
        assert!(requests.lock().unwrap().is_empty());
        assert!(!temp.path().join("permission-cancelled").exists());
    }

    #[tokio::test]
    async fn public_start_without_override_uses_sanitized_timestamped_preference_folder() {
        let temp = tempdir().unwrap();
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests);
        let mut request =
            public_start_request(temp.path().join("ignored"), "default-folder", None, false);
        request.save_path = None;
        request.meeting_name = "Planning: Q3".into();

        let started = session
            .start(
                fake_runtime(temp.path().to_path_buf(), false, None),
                request,
            )
            .await
            .unwrap();
        let folder = std::path::Path::new(&started.save_path);
        assert_eq!(folder.parent(), Some(temp.path()));
        assert!(folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Planning_ Q3_20"));
        assert!(folder.join("metadata.json").is_file());
        session.stop(started.session_id).await.unwrap();
    }

    #[tokio::test]
    async fn system_start_failure_keeps_public_session_microphone_only() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("meeting-microphone-only");
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session =
            RecordingSession::new_with_capture_factory(Arc::new(FakeCaptureOwnerFactory {
                events: Arc::new(StdMutex::new(Vec::new())),
                dropped: Arc::new(AtomicBool::new(false)),
                pause_fails: false,
                pump_fails: false,
                pump_delay: Duration::ZERO,
                system_capable: true,
                requests: requests.clone(),
            }));
        let started = session
            .start(
                fake_runtime(temp.path().to_path_buf(), true, None),
                public_start_request(folder.clone(), "microphone-only", None, true),
            )
            .await
            .unwrap();

        assert_eq!(started.state, RecordingSessionState::Recording);
        assert_eq!(
            started.expected_tracks,
            BTreeSet::from([TrackKind::Microphone])
        );
        assert!(requests.lock().unwrap()[0].capture_system);
        assert!(!folder.join(".tracks/system").exists());

        let stopped = session.stop(started.session_id).await.unwrap();
        assert_eq!(stopped.state, RecordingSessionState::Completed);
        assert_eq!(
            stopped.expected_tracks,
            BTreeSet::from([TrackKind::Microphone])
        );
        assert_eq!(stopped.captured_tracks.as_ref().unwrap().len(), 1);
        assert_eq!(
            stopped.captured_tracks.as_ref().unwrap()[0].kind,
            TrackKind::Microphone
        );
        assert!(folder.join("audio.mp4").is_file());
        assert!(folder.join("microphone.mp4").is_file());
        assert!(!folder.join("system.mp4").exists());
        assert!(!folder.join(".tracks/microphone/.checkpoints").exists());
    }

    #[tokio::test]
    async fn microphone_switch_updates_active_snapshot_and_notifies_without_changing_preferences() {
        let temp = tempdir().unwrap();
        let session =
            RecordingSession::new_with_capture_factory(Arc::new(RuntimeEventCaptureOwnerFactory {
                runtime_event: Some(CaptureRuntimeEvent::MicrophoneSwitched {
                    previous_device: "Studio Microphone".into(),
                    current_device: "Built-in Microphone".into(),
                    reason: "active microphone became unavailable".into(),
                }),
                terminal_cause: None,
            }));
        let runtime = fake_runtime(temp.path().to_path_buf(), true, Some("Studio Microphone"));
        let capture_notifications = runtime.capture_notifications.clone();
        let started = session
            .start(
                runtime.clone(),
                public_start_request(
                    temp.path().join("microphone-switch"),
                    "microphone-switch",
                    None,
                    true,
                ),
            )
            .await
            .unwrap();

        let projected = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let snapshot = session.get(started.session_id.clone()).await.unwrap();
                if snapshot.microphone_device.as_deref() == Some("Built-in Microphone") {
                    break snapshot;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(projected.state, RecordingSessionState::Recording);
        assert_eq!(
            runtime.preferences.preferred_mic_device.as_deref(),
            Some("Studio Microphone")
        );
        assert_eq!(
            capture_notifications.lock().unwrap().as_slice(),
            &[(
                started.session_id.clone(),
                CaptureRuntimeEvent::MicrophoneSwitched {
                    previous_device: "Studio Microphone".into(),
                    current_device: "Built-in Microphone".into(),
                    reason: "active microphone became unavailable".into(),
                },
            )]
        );

        session.stop(started.session_id).await.unwrap();
    }

    #[tokio::test]
    async fn system_degradation_notifies_without_interrupting_the_session() {
        let temp = tempdir().unwrap();
        let session =
            RecordingSession::new_with_capture_factory(Arc::new(RuntimeEventCaptureOwnerFactory {
                runtime_event: Some(CaptureRuntimeEvent::SystemAudioDegraded {
                    reason: "system stream stopped".into(),
                }),
                terminal_cause: None,
            }));
        let runtime = fake_runtime(temp.path().to_path_buf(), true, None);
        let capture_notifications = runtime.capture_notifications.clone();
        let started = session
            .start(
                runtime,
                public_start_request(
                    temp.path().join("system-degradation"),
                    "system-degradation",
                    None,
                    true,
                ),
            )
            .await
            .unwrap();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if !capture_notifications.lock().unwrap().is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let projected = session.get(started.session_id.clone()).await.unwrap();
        assert_eq!(projected.state, RecordingSessionState::Recording);
        assert_eq!(
            projected.expected_tracks,
            BTreeSet::from([TrackKind::Microphone, TrackKind::System])
        );
        assert_eq!(
            capture_notifications.lock().unwrap().as_slice(),
            &[(
                started.session_id.clone(),
                CaptureRuntimeEvent::SystemAudioDegraded {
                    reason: "system stream stopped".into(),
                },
            )]
        );

        session.stop(started.session_id).await.unwrap();
    }

    #[tokio::test]
    async fn microphone_recovery_exhaustion_uses_recoverable_interruption() {
        let temp = tempdir().unwrap();
        let session =
            RecordingSession::new_with_capture_factory(Arc::new(RuntimeEventCaptureOwnerFactory {
                runtime_event: None,
                terminal_cause: Some(CaptureCause::DeviceUnavailable("Studio Microphone".into())),
            }));
        let mut request = public_start_request(
            temp.path().join("microphone-exhausted"),
            "microphone-exhausted",
            Some("Studio Microphone"),
            true,
        );
        request.capture_system = false;
        let started = session
            .start(fake_runtime(temp.path().to_path_buf(), true, None), request)
            .await
            .unwrap();

        let interrupted = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let snapshot = session.get(started.session_id.clone()).await.unwrap();
                if snapshot.state == RecordingSessionState::Interrupted {
                    break snapshot;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let error = interrupted.error.as_ref().unwrap();
        assert!(matches!(error.code, ContractErrorCode::DeviceUnavailable));
        assert!(error.retryable);
        let microphone_track = interrupted
            .captured_tracks
            .as_ref()
            .and_then(|tracks| {
                tracks
                    .iter()
                    .find(|track| track.kind == TrackKind::Microphone)
            })
            .expect("verified microphone Track must be retained");
        assert!(std::path::Path::new(&microphone_track.path).is_file());
    }

    #[tokio::test]
    async fn stop_then_start_uses_fresh_owner_streams_and_permission_decisions() {
        let temp = tempdir().unwrap();
        let owner_identities = Arc::new(StdMutex::new(Vec::new()));
        let stream_identities = Arc::new(StdMutex::new(Vec::new()));
        let drop_counts = Arc::new(StdMutex::new(Vec::new()));
        let join_counts = Arc::new(StdMutex::new(Vec::new()));
        let session = RecordingSession::new_with_capture_factory(Arc::new(
            IdentityTrackingCaptureOwnerFactory {
                next_owner_identity: AtomicUsize::new(1),
                next_stream_identity: Arc::new(AtomicUsize::new(101)),
                owner_identities: owner_identities.clone(),
                stream_identities: stream_identities.clone(),
                drop_counts: drop_counts.clone(),
                join_counts: join_counts.clone(),
            },
        ));
        let runtime = fake_runtime(temp.path().to_path_buf(), false, None);

        let first_snapshot = session
            .start(
                runtime.clone(),
                public_start_request(temp.path().join("session-a"), "session-a", None, false),
            )
            .await
            .unwrap();
        let first_owner_id = owner_identities.lock().unwrap()[0];
        let first_stream_id = stream_identities.lock().unwrap()[0];
        let first_drop_count = drop_counts.lock().unwrap()[0].clone();
        let first_join_count = join_counts.lock().unwrap()[0].clone();
        session.stop(first_snapshot.session_id).await.unwrap();
        assert_eq!(first_join_count.load(Ordering::Acquire), 1);
        assert_eq!(first_drop_count.load(Ordering::Acquire), 1);

        let second_snapshot = session
            .start(
                runtime,
                public_start_request(temp.path().join("session-b"), "session-b", None, false),
            )
            .await
            .unwrap();
        let second_owner_id = owner_identities.lock().unwrap()[1];
        let second_stream_id = stream_identities.lock().unwrap()[1];
        let second_state = second_snapshot.state;
        let second_error = second_snapshot.error.clone();
        session.stop(second_snapshot.session_id).await.unwrap();

        assert_ne!(first_owner_id, second_owner_id);
        assert_ne!(first_stream_id, second_stream_id);
        assert_eq!(first_drop_count.load(Ordering::Acquire), 1);
        assert_eq!(second_state, RecordingSessionState::Recording);
        assert!(second_error.is_none());
    }

    #[tokio::test]
    async fn public_start_preserves_persisted_microphone_fallback_provenance() {
        let temp = tempdir().unwrap();
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let runtime = fake_runtime(temp.path().to_path_buf(), false, Some("Stale saved mic"));
        let started = session
            .start(
                runtime,
                public_start_request(
                    temp.path().join("fallback"),
                    "fallback",
                    Some("Stale saved mic"),
                    false,
                ),
            )
            .await
            .unwrap();
        assert!(requests.lock().unwrap()[0].fallback_to_default_microphone);
        session.stop(started.session_id).await.unwrap();

        let mut explicit = public_start_request(
            temp.path().join("explicit"),
            "explicit",
            Some("Chosen mic"),
            false,
        );
        explicit.microphone_device_explicit = true;
        let started = session
            .start(
                fake_runtime(temp.path().to_path_buf(), false, Some("Stale saved mic")),
                explicit,
            )
            .await
            .unwrap();
        assert!(!requests.lock().unwrap()[1].fallback_to_default_microphone);
        session.stop(started.session_id).await.unwrap();
    }

    #[tokio::test]
    async fn public_start_is_rejected_after_action_gated_shutdown_latch() {
        let temp = tempdir().unwrap();
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        session.latch_shutdown();

        let error = session
            .start(
                fake_runtime(temp.path().to_path_buf(), false, None),
                public_start_request(temp.path().join("late"), "late", None, false),
            )
            .await
            .unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::Cancelled));
        assert!(requests.lock().unwrap().is_empty());
        assert!(!temp.path().join("late").exists());
    }

    #[tokio::test]
    async fn public_partial_start_failure_persists_failed_metadata_and_notifies_once() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("partial-start");
        let session =
            RecordingSession::new_with_capture_factory(Arc::new(FailingCaptureOwnerFactory));
        let runtime = fake_runtime(temp.path().to_path_buf(), false, None);
        let notifications = runtime.notifications.clone();
        let request = public_start_request(folder.clone(), "partial-start", None, true);

        let error = session.start(runtime, request.clone()).await.unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::DeviceUnavailable));
        assert_eq!(error.current_state, Some(RecordingSessionState::Failed));
        let failed = session
            .get(error.session_id.clone().unwrap())
            .await
            .unwrap();
        assert_eq!(failed.state, RecordingSessionState::Failed);
        assert_eq!(
            failed.expected_tracks,
            BTreeSet::from([TrackKind::Microphone, TrackKind::System])
        );
        let tracks = failed.captured_tracks.as_ref().unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].kind, TrackKind::Microphone);
        assert_eq!(tracks[0].sample_rate, 48_000);
        assert_eq!(tracks[0].channels, 1);
        assert_eq!(tracks[0].sample_count, 4_800);
        assert!(tracks[0].byte_length > 0);
        assert_eq!(tracks[0].sha256.len(), 64);
        assert_eq!(
            tracks[0].path,
            folder.join("microphone.mp4").to_string_lossy()
        );
        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            &[RecordingSessionState::Failed]
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.join("metadata.json")).unwrap()).unwrap();
        assert_eq!(metadata["status"], "failed");
        assert!(folder.join("microphone.mp4").is_file());
        assert!(folder.join(".tracks/microphone/track.json").is_file());

        let duplicate = session
            .start(fake_runtime(temp.path().to_path_buf(), true, None), request)
            .await
            .unwrap();
        assert_eq!(duplicate.session_id, failed.session_id);
        assert_eq!(duplicate.state, RecordingSessionState::Failed);
    }

    #[tokio::test]
    async fn public_start_capture_failure_interrupts_notifies_and_recovers() {
        let temp = tempdir().unwrap();
        let folder = temp.path().join("recoverable");
        let session = fake_session_with(Arc::new(StdMutex::new(Vec::new())), false, true);
        let runtime = fake_runtime(temp.path().to_path_buf(), true, None);
        let notifications = runtime.notifications.clone();
        let started = session
            .start(
                runtime,
                public_start_request(folder, "recoverable", None, true),
            )
            .await
            .unwrap();

        let interrupted = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let snapshot = session.get(started.session_id.clone()).await.unwrap();
                if !snapshot.state.is_nonterminal() {
                    break snapshot;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(interrupted.state, RecordingSessionState::Interrupted);
        assert!(interrupted.error.as_ref().unwrap().retryable);
        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            &[RecordingSessionState::Interrupted]
        );

        let recovered = session
            .recover(interrupted.session_id.clone())
            .await
            .unwrap();
        assert_eq!(recovered.state, RecordingSessionState::Completed);
        assert_eq!(
            session.recover(interrupted.session_id).await.unwrap().state,
            RecordingSessionState::Completed
        );
    }

    #[tokio::test]
    async fn shared_start_seam_opens_and_stops_the_production_owner_topology() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let factory: Arc<dyn CaptureOwnerFactory> = Arc::new(FakeCaptureOwnerFactory {
            events: events.clone(),
            dropped: dropped.clone(),
            pause_fails: false,
            pump_fails: false,
            pump_delay: Duration::ZERO,
            system_capable: false,
            requests: Arc::new(StdMutex::new(Vec::new())),
        });
        let (raw, _raw_rx) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_rx) = TrackSink::new(TrackKind::Microphone);
        let capture = spawn_capture_owner(
            factory,
            CaptureRequest {
                microphone_device: None,
                fallback_to_default_microphone: true,
                system_device: None,
                capture_system: false,
            },
            TrackSinks {
                microphone: SourceSinks::new(raw, downstream),
                system: None,
            },
        )
        .await
        .unwrap();

        let (acknowledged, joined) =
            tokio::task::spawn_blocking(move || stop_and_join_capture(capture))
                .await
                .unwrap();
        assert_eq!(
            acknowledged.unwrap().closed_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
        assert!(joined.is_ok());
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(*events.lock().unwrap(), vec!["start", "stop", "drop"]);
    }

    #[test]
    fn disconnected_downstream_does_not_invalidate_the_durable_raw_track() {
        let temp = tempdir().unwrap();
        let (raw, raw_receiver) = TrackSink::for_checkpoint_writer(TrackKind::Microphone);
        let (downstream, downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        drop(downstream_receiver);
        let sinks = SourceSinks::new(raw, downstream);
        let mut writer = TrackWriter::spawn(
            TrackKind::Microphone,
            raw_receiver,
            temp.path().to_path_buf(),
        )
        .unwrap();
        let sample_count = 48_000 * 30;
        let samples = vec![0.1; sample_count];
        sinks
            .send_pair(
                super::super::capture::backend::TrackChunk::contiguous(
                    TrackKind::Microphone,
                    samples.clone(),
                    48_000,
                    1,
                ),
                super::super::capture::backend::TrackChunk::contiguous(
                    TrackKind::Microphone,
                    samples,
                    48_000,
                    1,
                ),
            )
            .unwrap();
        drop(sinks);

        let saver = writer
            .join
            .take()
            .unwrap()
            .join()
            .unwrap()
            .unwrap()
            .unwrap();
        let finalized = saver.finalize().unwrap();
        assert_eq!(finalized.sample_count, sample_count as u64);
        assert_eq!(finalized.path, temp.path().join("microphone.mp4"));

        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(temp.path().join(".tracks/microphone/track.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["incomplete"], false);
        assert_eq!(manifest["sample_count"], sample_count as u64);
        assert_eq!(manifest["checkpoint_count"], 1);
        assert_eq!(
            manifest["checkpoints"][0]["input_sample_count"],
            sample_count as u64
        );
        assert!(temp.path().join("microphone.mp4").exists());
        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.sample_count, sample_count as u64);
    }

    #[test]
    fn checkpoint_write_error_freezes_writer_and_preserves_prefix_cause() {
        let temp = tempdir().unwrap();
        let (sink, receiver) = TrackSink::for_checkpoint_writer(TrackKind::Microphone);
        let mut writer =
            TrackWriter::spawn(TrackKind::Microphone, receiver, temp.path().to_path_buf()).unwrap();
        std::fs::write(temp.path().join(".force_checkpoint_write_error"), b"").unwrap();

        let sample_count = 48_000 * 30;
        sink.try_send(super::super::capture::backend::TrackChunk::contiguous(
            TrackKind::Microphone,
            vec![0.1; sample_count],
            48_000,
            1,
        ))
        .unwrap();
        let mut disconnected = false;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match sink.try_send(super::super::capture::backend::TrackChunk::contiguous(
                TrackKind::Microphone,
                vec![0.2; 1_024],
                48_000,
                1,
            )) {
                Ok(()) => {}
                Err(CaptureCause::CaptureOwnerDisconnected) => {
                    disconnected = true;
                    break;
                }
                Err(CaptureCause::TrackBackpressure(_)) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("unexpected writer error: {error:?}"),
            }
        }
        assert!(disconnected, "writer kept the raw queue open after failure");
        drop(sink);

        let saver = writer
            .join
            .take()
            .unwrap()
            .join()
            .unwrap()
            .unwrap()
            .unwrap();
        let error = match saver.finalize() {
            Err(SourceTrackFinalizationError::Persistence(error)) => error,
            Err(error) => panic!("unexpected finalization error: {error}"),
            Ok(track) => panic!("incomplete Track finalized successfully: {track:?}"),
        };
        assert!(
            error.contains("forced checkpoint write failure"),
            "root cause: {error}"
        );

        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(temp.path().join(".tracks/microphone/track.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["incomplete"], true);
        assert_eq!(manifest["sample_count"], sample_count as u64);
        assert_eq!(manifest["checkpoint_count"], 1);
        assert_eq!(
            manifest["checkpoints"][0]["input_sample_count"],
            sample_count as u64
        );
    }

    #[test]
    #[ignore = "35-second real-time checkpoint diagnostic with local FFmpeg"]
    fn live_track_writer_survives_first_checkpoint() {
        // Exercise the actual bounded capture queue and synchronous checkpoint
        // encoder without opening a microphone or touching user recordings.
        let temp = tempdir().unwrap();
        let (sink, receiver) = TrackSink::for_checkpoint_writer(TrackKind::Microphone);
        let mut writer =
            TrackWriter::spawn(TrackKind::Microphone, receiver, temp.path().to_path_buf()).unwrap();
        let started = Instant::now();
        let frames_per_chunk = 480_usize;
        let chunk_count = 3_500_usize;
        let mut sent = 0_usize;
        let mut failure = None;
        for index in 0..chunk_count {
            let deadline = started + Duration::from_millis(index as u64 * 10);
            std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            let samples = (0..frames_per_chunk * 2)
                .map(|offset| {
                    let sample = (index * frames_per_chunk * 2 + offset) as f32;
                    (sample * 0.071).sin() * 0.15 + (sample * 0.113).sin() * 0.05
                })
                .collect();
            if let Err(cause) =
                sink.try_send(super::super::capture::backend::TrackChunk::contiguous(
                    TrackKind::Microphone,
                    samples,
                    48_000,
                    2,
                ))
            {
                failure = Some((index, started.elapsed(), cause));
                break;
            }
            sent += 1;
        }
        drop(sink);
        // Join even after failure so no fixture writer escapes the test.
        let saver = writer
            .join
            .take()
            .unwrap()
            .join()
            .unwrap()
            .unwrap()
            .unwrap();
        let track = saver.finalize().unwrap();
        assert_eq!(track.sample_count, (sent * frames_per_chunk * 2) as u64);
        assert!(
            failure.is_none(),
            "capture queue failed during checkpoint: {failure:?}"
        );
        assert_eq!(sent, chunk_count);
    }

    #[test]
    fn auto_save_false_discards_audio_without_creating_a_track_namespace() {
        let temp = tempdir().unwrap();
        let meeting_folder = temp.path().join("must-not-exist");
        let (sink, receiver) = TrackSink::new(TrackKind::Microphone);
        let mut writer = spawn_source_writer(
            false,
            TrackKind::Microphone,
            receiver,
            meeting_folder.clone(),
        )
        .unwrap();
        sink.try_send(super::super::capture::backend::TrackChunk::contiguous(
            TrackKind::Microphone,
            vec![0.2; 64],
            48_000,
            1,
        ))
        .unwrap();
        drop(sink);
        assert!(writer
            .join
            .take()
            .unwrap()
            .join()
            .unwrap()
            .unwrap()
            .is_none());
        assert!(!meeting_folder.exists());
        assert!(
            durable_expected_tracks(false, &BTreeSet::from([TrackKind::Microphone])).is_empty()
        );
    }

    #[test]
    fn system_capture_can_close_as_microphone_only_when_no_frames_arrive() {
        let mut capture_sources = BTreeSet::from([TrackKind::Microphone, TrackKind::System]);
        let mut expected_tracks = capture_sources.clone();
        let closed_sources = BTreeSet::from([TrackKind::Microphone]);

        assert!(reconcile_closed_capture_sources(
            &mut capture_sources,
            &mut expected_tracks,
            &closed_sources,
        ));
        assert_eq!(capture_sources, closed_sources);
        assert_eq!(expected_tracks, BTreeSet::from([TrackKind::Microphone]));
    }

    #[test]
    fn unexpected_capture_source_closure_still_fails_reconciliation() {
        let mut capture_sources = BTreeSet::from([TrackKind::Microphone, TrackKind::System]);
        let mut expected_tracks = capture_sources.clone();
        let closed_sources = BTreeSet::new();

        assert!(!reconcile_closed_capture_sources(
            &mut capture_sources,
            &mut expected_tracks,
            &closed_sources,
        ));
        assert_eq!(
            expected_tracks,
            BTreeSet::from([TrackKind::Microphone, TrackKind::System])
        );
    }

    #[test]
    fn silent_optional_system_track_does_not_turn_a_valid_microphone_stop_into_failure() {
        let mut expected = BTreeSet::from([TrackKind::Microphone, TrackKind::System]);
        let captured = vec![CapturedTrack {
            kind: TrackKind::Microphone,
            sample_rate: 48_000,
            channels: 1,
            path: "/tmp/microphone.mp4".into(),
            byte_length: 1,
            sample_count: 1,
            sha256: "a".repeat(64),
        }];

        reconcile_optional_system_track(&mut expected, &captured);

        assert_eq!(expected, BTreeSet::from([TrackKind::Microphone]));
    }

    #[test]
    fn missing_microphone_track_is_not_hidden_by_optional_system_fallback() {
        let mut expected = BTreeSet::from([TrackKind::Microphone, TrackKind::System]);
        let captured = vec![CapturedTrack {
            kind: TrackKind::System,
            sample_rate: 48_000,
            channels: 2,
            path: "/tmp/system.mp4".into(),
            byte_length: 1,
            sample_count: 1,
            sha256: "b".repeat(64),
        }];

        reconcile_optional_system_track(&mut expected, &captured);

        assert_eq!(
            expected,
            BTreeSet::from([TrackKind::Microphone, TrackKind::System])
        );
    }

    #[test]
    fn production_stop_helper_acknowledges_then_drops_owner_before_returning_joined() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (raw, _raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let request = CaptureRequest {
            microphone_device: None,
            fallback_to_default_microphone: false,
            system_device: None,
            capture_system: false,
        };
        let sinks = TrackSinks {
            microphone: SourceSinks::new(raw, downstream),
            system: None,
        };
        let fake_events = events.clone();
        let fake_dropped = dropped.clone();
        let handle =
            CaptureThreadHandle::spawn_with_backend(request, sinks, move || OrderedFakeBackend {
                events: fake_events,
                dropped: fake_dropped,
                identity_tracking: None,
                sinks: None,
                pause_fails: false,
                pump_fails: false,
                pump_delay: Duration::ZERO,
                system_capable: false,
            })
            .unwrap();

        let (acknowledged, joined) = stop_and_join_capture(handle);
        events.lock().unwrap().push("joined");

        assert_eq!(
            acknowledged.unwrap().closed_sources,
            BTreeSet::from([TrackKind::Microphone])
        );
        assert!(joined.is_ok());
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(
            *events.lock().unwrap(),
            vec!["start", "stop", "drop", "joined"]
        );
    }

    async fn install_active_fake_session(
        events: Arc<StdMutex<Vec<&'static str>>>,
        dropped: Arc<AtomicBool>,
        pause_fails: bool,
        pump_fails: bool,
        pump_delay: Duration,
    ) -> (RecordingSession, tempfile::TempDir) {
        let session =
            RecordingSession::new_with_capture_factory(Arc::new(FakeCaptureOwnerFactory {
                events,
                dropped,
                pause_fails,
                pump_fails,
                pump_delay,
                system_capable: false,
                requests: Arc::new(StdMutex::new(Vec::new())),
            }));
        let temp = tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".checkpoints")).unwrap();
        let started = Instant::now();
        let pipeline_state = RecordingState::new();
        pipeline_state.start_recording().unwrap();
        let mut pipeline = AudioPipelineManager::new();
        let (transcription_tx, mut transcription_rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move { while transcription_rx.recv().await.is_some() {} });
        pipeline
            .start(
                pipeline_state.clone(),
                transcription_tx,
                500,
                48_000,
                None,
                "Test microphone".into(),
                super::super::device_detection::InputDeviceKind::Unknown,
                "Test system audio".into(),
                super::super::device_detection::InputDeviceKind::Unknown,
            )
            .unwrap();
        let (mixed_tx, mixed_writer) =
            IncrementalAudioSaver::start_mixed_writer(temp.path().to_path_buf(), 48_000).unwrap();
        // This microphone-only fixture uses the real downstream writer and saver.
        // Keep the consumer connected until stop_recording closes the sender, so
        // finalization checks captured chunks instead of unrelated seeded audio.
        pipeline_state.set_audio_sender(mixed_tx);

        let (microphone_raw_sink, microphone_rx) =
            TrackSink::for_checkpoint_writer(TrackKind::Microphone);
        let (microphone_downstream_sink, microphone_downstream_rx) =
            TrackSink::new(TrackKind::Microphone);
        let microphone_writer = TrackWriter::spawn(
            TrackKind::Microphone,
            microphone_rx,
            temp.path().to_path_buf(),
        )
        .unwrap();
        let microphone_downstream = DownstreamWriter::spawn(
            TrackKind::Microphone,
            microphone_downstream_rx,
            pipeline_state.clone(),
            started,
        )
        .unwrap();
        let mut capture = session
            .inner
            .capture_factory
            .spawn(
                CaptureRequest {
                    microphone_device: None,
                    fallback_to_default_microphone: false,
                    system_device: None,
                    capture_system: false,
                },
                TrackSinks {
                    microphone: SourceSinks::new(microphone_raw_sink, microphone_downstream_sink),
                    system: None,
                },
            )
            .unwrap();
        let owner_exit_rx = capture.take_owner_exit_receiver();
        let identity = StartIdentity {
            session_id: "session-a".into(),
            idempotency_key: "key-a".into(),
            fingerprint: "fingerprint-a".into(),
        };
        let snapshot = RecordingSessionSnapshot {
            session_id: identity.session_id.clone(),
            meeting_id: "meeting-a".into(),
            meeting_vocabulary: None,
            state: RecordingSessionState::Recording,
            started_at_ms: 1,
            recording_duration: 0.0,
            active_duration: 0.0,
            pause_duration: 0.0,
            microphone_device: None,
            system_device: None,
            expected_tracks: capture.opened_sources.clone(),
            captured_tracks: None,
            error: None,
            meeting_name: "Meeting A".into(),
            save_path: temp.path().to_string_lossy().into_owned(),
            auto_save: true,
        };
        {
            let mut store = session.inner.store.lock().await;
            store.identities.insert("key-a".into(), identity.clone());
            store.snapshots.insert("session-a".into(), snapshot.clone());
            let transcript_history = Arc::new(StdMutex::new(Vec::new()));
            store
                .transcript_histories
                .insert("session-a".into(), transcript_history.clone());
            store.active = Some(ActiveSession {
                snapshot,
                identity,
                capture: Some(capture),
                capture_sources: BTreeSet::from([TrackKind::Microphone]),
                microphone_writer,
                system_writer: None,
                microphone_downstream,
                system_downstream: None,
                pipeline,
                pipeline_state,
                transcription_join: Some(tokio::spawn(async {})),
                transcript_history,
                mixed_writer: Some(mixed_writer),
                started,
                pause_started: None,
                pause_total: Duration::ZERO,
                lifecycle_pool: None,
                lifecycle_run_id: None,
            });
        }
        if let Some(owner_exit_rx) = owner_exit_rx {
            session.monitor_capture_owner_exit("session-a".into(), owner_exit_rx);
        }
        (session, temp)
    }

    #[tokio::test]
    async fn wrong_session_stop_preserves_active_owner_then_a_stops_once() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (session, _temp) = install_active_fake_session(
            events.clone(),
            dropped.clone(),
            false,
            false,
            Duration::ZERO,
        )
        .await;

        assert!(session.stop("session-b".into()).await.is_err());
        assert_eq!(session.get_active().await.unwrap().session_id, "session-a");
        let stopped = session.stop("session-a".into()).await.unwrap();
        assert_eq!(stopped.state, RecordingSessionState::Completed);
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| **event == "stop")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn shutdown_waits_for_an_in_flight_finalizer_before_reporting_safe_exit() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (session, _temp) = install_active_fake_session(
            events.clone(),
            dropped.clone(),
            false,
            false,
            Duration::from_millis(200),
        )
        .await;

        let stopping = session.clone();
        let stop = tokio::spawn(async move { stopping.stop("session-a".into()).await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let exiting = session.clone();
        let shutdown = tokio::spawn(async move { exiting.shutdown_active().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!stop.is_finished());
        assert!(!shutdown.is_finished());

        assert_eq!(
            stop.await.unwrap().unwrap().state,
            RecordingSessionState::Completed
        );
        assert!(shutdown.await.unwrap().unwrap().is_none());
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| **event == "stop")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn control_failure_stops_and_joins_before_returning_session_error() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (session, _temp) = install_active_fake_session(
            events.clone(),
            dropped.clone(),
            true,
            false,
            Duration::ZERO,
        )
        .await;

        assert!(session.pause("session-a".into()).await.is_err());
        assert!(session.get_active().await.is_none());
        assert_eq!(
            session.get("session-a".into()).await.unwrap().state,
            RecordingSessionState::Interrupted
        );
        assert!(dropped.load(Ordering::Acquire));
        assert!(events
            .lock()
            .unwrap()
            .windows(3)
            .any(|events| { events == ["pause", "stop", "drop"] }));
    }

    #[tokio::test]
    async fn unexpected_owner_exit_uses_session_finalization_and_retains_verified_track() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (session, _temp) =
            install_active_fake_session(events, dropped.clone(), false, true, Duration::ZERO).await;

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = session.get("session-a".into()).await.unwrap();
            if snapshot.state == RecordingSessionState::Interrupted {
                assert_eq!(snapshot.captured_tracks.unwrap().len(), 1);
                assert!(snapshot.error.unwrap().retryable);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "owner failure did not terminalize session"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(session.get_active().await.is_none());
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn wrong_session_stop_preserves_active_owner() {
        let mut active = Some("session-a".to_string());
        assert_eq!(
            take_matching_active(&mut active, "session-b", String::as_str),
            None
        );
        assert_eq!(active.as_deref(), Some("session-a"));
        assert_eq!(
            take_matching_active(&mut active, "session-a", String::as_str).as_deref(),
            Some("session-a")
        );
        assert!(active.is_none());
    }

    #[test]
    fn full_control_queue_cannot_deadlock_stop_enqueue() {
        let (sender, _receiver) = mpsc::sync_channel::<CaptureControl>(1);
        let (reply, _) = mpsc::sync_channel(1);
        sender
            .try_send(CaptureControl::Pause { reply })
            .expect("first bounded control fits");
        let (reply, _) = mpsc::sync_channel(1);
        assert!(matches!(
            sender.try_send(CaptureControl::Stop { reply }),
            Err(mpsc::TrySendError::Full(_))
        ));
    }

    #[test]
    fn production_stop_helper_exits_when_control_queue_stays_full() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let dropped = Arc::new(AtomicBool::new(false));
        let (raw, _raw_receiver) = TrackSink::new(TrackKind::Microphone);
        let (downstream, _downstream_receiver) = TrackSink::new(TrackKind::Microphone);
        let fake_events = events.clone();
        let fake_dropped = dropped.clone();
        let handle = CaptureThreadHandle::spawn_with_backend(
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
            move || OrderedFakeBackend {
                events: fake_events,
                dropped: fake_dropped,
                identity_tracking: None,
                sinks: None,
                pause_fails: false,
                pump_fails: false,
                pump_delay: Duration::from_millis(100),
                system_capable: false,
            },
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(20));
        for _ in 0..super::super::capture::backend::CAPTURE_CONTROL_MAX_MESSAGES {
            let (reply, _receiver) = mpsc::sync_channel(1);
            handle
                .control_tx
                .try_send(CaptureControl::Pause { reply })
                .unwrap();
        }

        let started = Instant::now();
        let (acknowledged, joined) =
            stop_and_join_capture_with_timeout(handle, Duration::from_millis(25));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(matches!(
            acknowledged,
            Err(CaptureCause::CaptureReplyTimeout)
        ));
        assert!(joined.is_ok());
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| **event == "drop")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn failed_start_reuses_same_session_identity() {
        let session = RecordingSession::new();
        let request = StartRecordingRequest {
            meeting_id: "meeting".into(),
            idempotency_key: "same-key".into(),
            microphone_device: None,
            microphone_device_explicit: false,
            system_device: None,
            capture_system: true,
            meeting_name: "Meeting".into(),
            meeting_vocabulary: None,
            save_path: None,
            auto_save: None,
        };
        let identity = StartIdentity {
            session_id: "same-session".into(),
            idempotency_key: request.idempotency_key.clone(),
            fingerprint: request.fingerprint(),
        };
        session.insert_start_identity(identity, &request).await;
        let error = session.error(
            ContractErrorCode::ModelUnavailable,
            "model missing",
            true,
            Some("same-session".into()),
            Some(RecordingSessionState::Starting),
            None,
        );
        session.record_start_failure("same-session", error).await;
        let store = session.inner.store.lock().await;
        let existing = store.identities.get("same-key").unwrap();
        assert_eq!(existing.session_id, "same-session");
        assert_eq!(
            decide_start_identity(Some(existing), "same-key", &request.fingerprint()),
            StartIdentityDecision::Reuse("same-session".into())
        );
        assert_eq!(
            store.snapshots.get("same-session").unwrap().state,
            RecordingSessionState::Failed
        );
    }

    #[test]
    fn persisted_start_preferences_respect_explicit_overrides() {
        let persisted_folder = std::path::PathBuf::from("/persisted");
        let explicit_folder = Some("/explicit".to_string());
        let resolved_folder = explicit_folder
            .as_deref()
            .filter(|path| !path.is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| persisted_folder.join("meeting"));
        assert_eq!(resolved_folder, std::path::PathBuf::from("/explicit"));
    }

    #[test]
    fn artifact_publication_failure_reduces_finalizing_to_retryable_interrupted() {
        let failures = [(
            ContractErrorCode::PersistenceFailed,
            "injected metadata fsync failure".to_string(),
        )];
        let (state, code, retryable) =
            transition_failure(RecordingSessionState::Finalizing, &failures);
        assert_eq!(state, RecordingSessionState::Interrupted);
        assert!(matches!(code, ContractErrorCode::PersistenceFailed));
        assert!(retryable);
        assert!(reduce_transition(
            RecordingSessionState::Completed,
            RecordingSessionAction::Interrupted
        )
        .is_err());
    }

    #[tokio::test]
    async fn public_model_preflight_failure_creates_no_recording_resources() {
        let temp = tempdir().unwrap();
        let requests = Arc::new(StdMutex::new(Vec::new()));
        let session = fake_session(requests.clone());
        let mut runtime = fake_runtime(temp.path().to_path_buf(), true, None);
        runtime.model_error = Some("model unavailable".into());
        let notifications = runtime.notifications.clone();
        let transcription_starts = runtime.transcription_starts.clone();
        let mut request = public_start_request(temp.path().join("ignored"), "no-model", None, true);
        request.save_path = None;

        let error = session.start(runtime, request).await.unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::ModelUnavailable));
        assert_eq!(error.current_state, Some(RecordingSessionState::Failed));
        let failed = session.get(error.session_id.unwrap()).await.unwrap();
        assert_eq!(failed.state, RecordingSessionState::Failed);
        assert!(failed.save_path.is_empty());
        assert_eq!(
            notifications.lock().unwrap().as_slice(),
            &[RecordingSessionState::Failed]
        );
        assert_eq!(transcription_starts.load(Ordering::Acquire), 0);
        assert!(requests.lock().unwrap().is_empty());
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[test]
    fn control_failure_stops_and_joins_before_returning() {
        let source = include_str!("recording_session.rs");
        assert!(source.contains("shutdown_after_control_failure"));
        assert!(source.contains("finish_active_session"));
    }

    #[test]
    fn unexpected_owner_exit_uses_session_finalization() {
        let source = include_str!("recording_session.rs");
        assert!(source.contains("finalize_unexpected_owner_exit"));
        assert!(source.contains("finish_active_session"));
    }

    fn interrupted_snapshot(
        folder: &std::path::Path,
        expected: BTreeSet<TrackKind>,
    ) -> RecordingSessionSnapshot {
        RecordingSessionSnapshot {
            session_id: "session-recovery".into(),
            meeting_id: "meeting-recovery".into(),
            meeting_vocabulary: None,
            state: RecordingSessionState::Interrupted,
            started_at_ms: 1,
            recording_duration: 0.0,
            active_duration: 0.0,
            pause_duration: 0.0,
            microphone_device: Some("test microphone".into()),
            system_device: expected
                .contains(&TrackKind::System)
                .then(|| "test system".into()),
            expected_tracks: expected,
            captured_tracks: None,
            error: None,
            meeting_name: "Recovery".into(),
            save_path: folder.to_string_lossy().into_owned(),
            auto_save: true,
        }
    }

    async fn lifecycle_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        LIFECYCLE_MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn write_raw_fixture(folder: &std::path::Path, kind: TrackKind, sentinel: f32) {
        let source = match kind {
            TrackKind::Microphone => "microphone",
            TrackKind::System => "system",
        };
        let namespace = folder.join(".tracks").join(source);
        std::fs::create_dir_all(namespace.join(".checkpoints")).unwrap();
        let samples = vec![sentinel; 4_800];
        let checkpoint = namespace
            .join(".checkpoints")
            .join(format!("{source}_chunk_000.mp4"));
        super::super::encode::encode_single_audio(
            bytemuck::cast_slice(&samples),
            48_000,
            1,
            &checkpoint,
        )
        .unwrap();
        use sha2::{Digest, Sha256};
        let checkpoint_sha256 =
            format!("{:x}", Sha256::digest(std::fs::read(&checkpoint).unwrap()));
        std::fs::write(
            namespace.join("track.json"),
            serde_json::to_vec(&serde_json::json!({
                "sample_rate": 48_000,
                "channels": 1,
                "checkpoint_count": 1,
                "sample_count": samples.len(),
                "checkpoints": [{
                    "index": 0,
                    "file_name": format!("{source}_chunk_000.mp4"),
                    "input_sample_count": samples.len(),
                    "sha256": checkpoint_sha256,
                }],
                "final_sha256": serde_json::Value::Null,
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn interrupted_session_recovers_and_verifies_known_raw_tracks_only() {
        let temp = tempdir().unwrap();
        let folder = temp.path();
        std::fs::create_dir_all(folder.join(".checkpoints")).unwrap();
        std::fs::write(folder.join(".checkpoints/audio_chunk_000.mp4"), b"legacy").unwrap();
        write_raw_fixture(folder, TrackKind::Microphone, 0.2);
        write_raw_fixture(folder, TrackKind::System, -0.2);
        let session = RecordingSession::new();
        let snapshot = interrupted_snapshot(
            folder,
            BTreeSet::from([TrackKind::Microphone, TrackKind::System]),
        );
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot);

        let recovered = session.recover("session-recovery".into()).await.unwrap();
        assert_eq!(recovered.state, RecordingSessionState::Completed);
        assert_eq!(recovered.captured_tracks.unwrap().len(), 2);
        assert_eq!(
            std::fs::read(folder.join(".checkpoints/audio_chunk_000.mp4")).unwrap(),
            b"legacy"
        );
    }

    #[tokio::test]
    async fn recording_recovery_clears_failed_lifecycle_for_same_run() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let pool = lifecycle_pool().await;
        let meeting_id = "meeting-recording-recovery-db";
        let lifecycle_run_id =
            crate::database::repositories::meeting::MeetingsRepository::ensure_lifecycle_meeting(
                &pool,
                meeting_id,
                "Recoverable recording",
                Some(1),
                "suggested",
                crate::database::repositories::meeting::MEETING_STATUS_RECORDING,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
            )
            .await
            .unwrap();
        assert!(crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
            &pool,
            meeting_id,
            crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
            &lifecycle_run_id,
            crate::database::repositories::meeting::MEETING_STATUS_ERROR,
            Some(4.0),
            Some("stop failed"),
        )
        .await
        .unwrap());

        let session = RecordingSession::new();
        let mut snapshot =
            interrupted_snapshot(temp.path(), BTreeSet::from([TrackKind::Microphone]));
        snapshot.meeting_id = meeting_id.into();
        {
            let mut store = session.inner.store.lock().await;
            store
                .snapshots
                .insert(snapshot.session_id.clone(), snapshot);
            store
                .lifecycle_pools
                .insert("session-recovery".into(), pool.clone());
            store
                .lifecycle_run_ids
                .insert("session-recovery".into(), lifecycle_run_id.clone());
        }

        let recovered = session.recover("session-recovery".into()).await.unwrap();
        assert_eq!(recovered.state, RecordingSessionState::Completed);
        let row = sqlx::query(
            "SELECT status, lifecycle_error, lifecycle_operation, lifecycle_run_id
             FROM meetings WHERE id = ?",
        )
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.get::<String, _>("status"), "ready");
        assert!(row
            .try_get::<Option<String>, _>("lifecycle_error")
            .unwrap()
            .is_none());
        assert_eq!(
            row.get::<String, _>("lifecycle_operation"),
            crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING
        );
        assert_eq!(row.get::<String, _>("lifecycle_run_id"), lifecycle_run_id);
    }

    #[tokio::test]
    async fn recording_recovery_does_not_overwrite_a_newer_lifecycle_owner() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let pool = lifecycle_pool().await;
        let meeting_id = "meeting-recording-recovery-owner";
        let lifecycle_run_id =
            crate::database::repositories::meeting::MeetingsRepository::ensure_lifecycle_meeting(
                &pool,
                meeting_id,
                "Recovery ownership",
                Some(1),
                "suggested",
                crate::database::repositories::meeting::MEETING_STATUS_RECORDING,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
            )
            .await
            .unwrap();
        assert!(crate::database::repositories::meeting::MeetingsRepository::update_lifecycle_for_operation(
            &pool,
            meeting_id,
            crate::database::repositories::meeting::LIFECYCLE_OPERATION_RECORDING,
            &lifecycle_run_id,
            crate::database::repositories::meeting::MEETING_STATUS_ERROR,
            None,
            Some("stop failed"),
        )
        .await
        .unwrap());
        let newer_run_id =
            crate::database::repositories::meeting::MeetingsRepository::claim_lifecycle_operation(
                &pool,
                meeting_id,
                crate::database::repositories::meeting::LIFECYCLE_OPERATION_RETRANSCRIPTION,
            )
            .await
            .unwrap()
            .expect("new operation should claim the failed row");

        let session = RecordingSession::new();
        let mut snapshot =
            interrupted_snapshot(temp.path(), BTreeSet::from([TrackKind::Microphone]));
        snapshot.meeting_id = meeting_id.into();
        {
            let mut store = session.inner.store.lock().await;
            store
                .snapshots
                .insert(snapshot.session_id.clone(), snapshot);
            store
                .lifecycle_pools
                .insert("session-recovery".into(), pool.clone());
            store
                .lifecycle_run_ids
                .insert("session-recovery".into(), lifecycle_run_id.clone());
        }

        let recovered = session.recover("session-recovery".into()).await.unwrap();
        assert_eq!(recovered.state, RecordingSessionState::Completed);
        let row = sqlx::query(
            "SELECT status, lifecycle_error, lifecycle_operation, lifecycle_run_id
             FROM meetings WHERE id = ?",
        )
        .bind(meeting_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.get::<String, _>("status"), "processing");
        assert!(row
            .try_get::<Option<String>, _>("lifecycle_error")
            .unwrap()
            .is_none());
        assert_eq!(
            row.get::<String, _>("lifecycle_operation"),
            crate::database::repositories::meeting::LIFECYCLE_OPERATION_RETRANSCRIPTION
        );
        assert_eq!(row.get::<String, _>("lifecycle_run_id"), newer_run_id);
    }

    #[tokio::test]
    async fn two_source_recovery_retains_verified_microphone_when_system_fails() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let session = RecordingSession::new();
        let snapshot = interrupted_snapshot(
            temp.path(),
            BTreeSet::from([TrackKind::Microphone, TrackKind::System]),
        );
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot);

        let error = session
            .recover("session-recovery".into())
            .await
            .unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::CaptureFailed));
        let failed = session.get("session-recovery".into()).await.unwrap();
        assert_eq!(failed.state, RecordingSessionState::Interrupted);
        let partial = failed.captured_tracks.unwrap();
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].kind, TrackKind::Microphone);
    }

    #[tokio::test]
    async fn transient_recovery_failure_can_be_retried() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let session = RecordingSession::new();
        let snapshot = interrupted_snapshot(
            temp.path(),
            BTreeSet::from([TrackKind::Microphone, TrackKind::System]),
        );
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot);

        assert!(session.recover("session-recovery".into()).await.is_err());
        write_raw_fixture(temp.path(), TrackKind::System, -0.2);
        let recovered = session.recover("session-recovery".into()).await.unwrap();
        assert_eq!(recovered.state, RecordingSessionState::Completed);
        assert_eq!(recovered.captured_tracks.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn checkpoint_io_failure_keeps_session_interrupted_and_retryable() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let checkpoints = temp.path().join(".tracks/microphone/.checkpoints");
        std::fs::remove_dir_all(&checkpoints).unwrap();
        std::fs::write(&checkpoints, b"not a directory").unwrap();
        let session = RecordingSession::new();
        let snapshot = interrupted_snapshot(temp.path(), BTreeSet::from([TrackKind::Microphone]));
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot);

        let error = session
            .recover("session-recovery".into())
            .await
            .unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::PersistenceFailed));
        assert!(error.retryable);
        assert_eq!(
            error.current_state,
            Some(RecordingSessionState::Interrupted)
        );
        assert_eq!(
            session.get("session-recovery".into()).await.unwrap().state,
            RecordingSessionState::Interrupted
        );
    }

    #[tokio::test]
    async fn checkpoint_hash_mismatch_makes_session_failed_and_nonretryable() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let checkpoint = temp
            .path()
            .join(".tracks/microphone/.checkpoints/microphone_chunk_000.mp4");
        let mut bytes = std::fs::read(&checkpoint).unwrap();
        bytes[0] ^= 1;
        std::fs::write(&checkpoint, bytes).unwrap();
        let session = RecordingSession::new();
        let snapshot = interrupted_snapshot(temp.path(), BTreeSet::from([TrackKind::Microphone]));
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot);

        let error = session
            .recover("session-recovery".into())
            .await
            .unwrap_err();
        assert!(matches!(error.code, ContractErrorCode::IntegrityFailed));
        assert!(!error.retryable);
        assert_eq!(error.current_state, Some(RecordingSessionState::Failed));
        assert_eq!(
            session.get("session-recovery".into()).await.unwrap().state,
            RecordingSessionState::Failed
        );
    }

    #[tokio::test]
    async fn terminal_recovery_is_idempotent_without_republishing_tracks() {
        let temp = tempdir().unwrap();
        let session = RecordingSession::new();
        let mut snapshot =
            interrupted_snapshot(temp.path(), BTreeSet::from([TrackKind::Microphone]));
        snapshot.state = RecordingSessionState::Completed;
        snapshot.captured_tracks = Some(vec![CapturedTrack {
            kind: TrackKind::Microphone,
            sample_rate: 48_000,
            channels: 1,
            path: temp
                .path()
                .join("microphone.mp4")
                .to_string_lossy()
                .into_owned(),
            byte_length: 1,
            sample_count: 1,
            sha256: "0".repeat(64),
        }]);
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot.clone());

        let duplicate = session.recover(snapshot.session_id.clone()).await.unwrap();
        assert_eq!(duplicate.state, RecordingSessionState::Completed);
        assert_eq!(duplicate.captured_tracks.unwrap()[0].sha256, "0".repeat(64));
    }

    #[tokio::test]
    async fn failed_recovery_snapshot_is_terminal_even_when_error_was_retryable() {
        let temp = tempdir().unwrap();
        let session = RecordingSession::new();
        let mut snapshot =
            interrupted_snapshot(temp.path(), BTreeSet::from([TrackKind::Microphone]));
        snapshot.state = RecordingSessionState::Failed;
        snapshot.error = Some(session.error(
            ContractErrorCode::PersistenceFailed,
            "prior failure",
            true,
            Some(snapshot.session_id.clone()),
            Some(RecordingSessionState::Failed),
            None,
        ));
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot.clone());

        let returned = session.recover(snapshot.session_id.clone()).await.unwrap();
        assert_eq!(returned.state, RecordingSessionState::Failed);
        assert_eq!(returned.session_id, snapshot.session_id);
        assert_eq!(
            returned.error.as_ref().map(|error| error.message.as_str()),
            Some("prior failure")
        );
    }

    #[tokio::test]
    async fn recovery_retry_clears_partial_track_not_verified_by_current_attempt() {
        let temp = tempdir().unwrap();
        write_raw_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let session = RecordingSession::new();
        let snapshot = interrupted_snapshot(
            temp.path(),
            BTreeSet::from([TrackKind::Microphone, TrackKind::System]),
        );
        session
            .inner
            .store
            .lock()
            .await
            .snapshots
            .insert(snapshot.session_id.clone(), snapshot);
        assert!(session.recover("session-recovery".into()).await.is_err());
        assert_eq!(
            session
                .get("session-recovery".into())
                .await
                .unwrap()
                .captured_tracks
                .unwrap()
                .len(),
            1
        );

        std::fs::remove_file(temp.path().join("microphone.mp4")).unwrap();
        std::fs::remove_dir_all(temp.path().join(".tracks/microphone")).unwrap();
        assert!(session.recover("session-recovery".into()).await.is_err());
        assert!(session
            .get("session-recovery".into())
            .await
            .unwrap()
            .captured_tracks
            .is_none());
    }
}
