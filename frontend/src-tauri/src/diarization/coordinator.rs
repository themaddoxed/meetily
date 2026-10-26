use super::audio::{decode_audio_to_mono_16k, DiarizationAudioError};
use super::commands::{
    apply_diarization_segments_with_pool, ensure_post_call_run_allowed, ApplyDiarizationRequest,
    DiarizationRunKind,
};
use super::runtime::{
    normalize_engine_segments, DiarizationRuntime, RuntimeError, SherpaDiarizationRuntime,
    DIARIZATION_METHOD,
};
use crate::database::repositories::meeting::{MeetingRecordingResolution, MeetingsRepository};
use crate::state::AppState;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tracing::error;

const INVALID_REQUEST: &str = "invalid_request";
const MEETING_NOT_FOUND: &str = "meeting_not_found";
const RECORDING_ACTIVE: &str = "recording_active";
const MISSING_RECORDING: &str = "missing_recording";
const RECORDING_INCOMPLETE: &str = "recording_incomplete";
const INVALID_AUDIO: &str = "invalid_audio";
const MODEL_UNAVAILABLE: &str = "model_unavailable";
const MODEL_INVALID: &str = "model_invalid";
const INSUFFICIENT_SPEECH: &str = "insufficient_speech";
const INFERENCE_FAILED: &str = "inference_failed";
const INVALID_OUTPUT: &str = "invalid_output";
const ALREADY_RUNNING: &str = "already_running";
const PERSISTENCE_FAILED: &str = "persistence_failed";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiarizationRunResult {
    pub segment_count: usize,
    pub speaker_count: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunMeetingDiarizationRequest {
    pub meeting_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiarizationProgress {
    pub meeting_id: String,
    pub stage: String,
    pub progress: u8,
    pub message: String,
}

type ProgressCallback = Arc<dyn Fn(DiarizationProgress) + Send + Sync>;

#[derive(Default)]
pub struct DiarizationRunRegistry {
    active_meetings: Mutex<HashSet<String>>,
}

static GLOBAL_RUN_REGISTRY: Lazy<DiarizationRunRegistry> =
    Lazy::new(DiarizationRunRegistry::default);

pub fn global_run_registry() -> &'static DiarizationRunRegistry {
    &GLOBAL_RUN_REGISTRY
}

impl DiarizationRunRegistry {
    pub fn try_start(&self, meeting_id: &str) -> Result<DiarizationRunGuard<'_>, String> {
        let mut active_meetings = self
            .active_meetings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !active_meetings.insert(meeting_id.to_string()) {
            return Err(ALREADY_RUNNING.to_string());
        }
        Ok(DiarizationRunGuard {
            registry: self,
            meeting_id: meeting_id.to_string(),
        })
    }
}

pub struct DiarizationRunGuard<'a> {
    registry: &'a DiarizationRunRegistry,
    meeting_id: String,
}

impl Drop for DiarizationRunGuard<'_> {
    fn drop(&mut self) {
        self.registry
            .active_meetings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.meeting_id);
    }
}

#[derive(Debug)]
enum BlockingFailure {
    Audio(DiarizationAudioError),
    Runtime(RuntimeError),
}

enum RuntimeSource {
    Ready(Arc<dyn DiarizationRuntime>),
    Sherpa {
        resource_root: std::path::PathBuf,
        num_threads: i32,
    },
}

pub async fn run_meeting_diarization_with_pool(
    pool: &SqlitePool,
    meeting_id: &str,
    recording_active: bool,
    registry: &DiarizationRunRegistry,
    runtime: Arc<dyn DiarizationRuntime>,
) -> Result<DiarizationRunResult, String> {
    if !is_valid_meeting_id(meeting_id) {
        return Err(INVALID_REQUEST.to_string());
    }
    let run_guard = registry.try_start(meeting_id)?;
    run_started_diarization(
        pool,
        meeting_id,
        recording_active,
        run_guard,
        RuntimeSource::Ready(runtime),
        None,
    )
    .await
}

#[tauri::command]
pub async fn run_meeting_diarization(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: RunMeetingDiarizationRequest,
) -> Result<DiarizationRunResult, String> {
    if !is_valid_meeting_id(&request.meeting_id) {
        return Err(INVALID_REQUEST.to_string());
    }
    let run_guard = global_run_registry().try_start(&request.meeting_id)?;
    let resource_root = app.path().resource_dir().map_err(|path_error| {
        error!("Unable to resolve application resources for diarization: {path_error}");
        MODEL_UNAVAILABLE.to_string()
    })?;
    let num_threads = std::thread::available_parallelism()
        .map(|count| count.get().min(4) as i32)
        .unwrap_or(2);

    let event_app = app.clone();
    let progress_callback: ProgressCallback = Arc::new(move |progress| {
        if let Err(emit_error) = event_app.emit("meeting-diarization-progress", progress) {
            error!("Unable to emit meeting diarization progress: {emit_error}");
        }
    });
    report_progress(
        Some(&progress_callback),
        &request.meeting_id,
        "preparing",
        5,
        "Preparing speaker detection",
    );

    let recording_active = crate::audio::recording_commands::is_recording().await;
    let result = run_started_diarization(
        state.db_manager.pool(),
        &request.meeting_id,
        recording_active,
        run_guard,
        RuntimeSource::Sherpa {
            resource_root,
            num_threads,
        },
        Some(progress_callback.clone()),
    )
    .await?;
    report_progress(
        Some(&progress_callback),
        &request.meeting_id,
        "complete",
        100,
        "Speaker labels are ready",
    );
    Ok(result)
}

async fn run_started_diarization(
    pool: &SqlitePool,
    meeting_id: &str,
    recording_active: bool,
    _run_guard: DiarizationRunGuard<'_>,
    runtime_source: RuntimeSource,
    progress: Option<ProgressCallback>,
) -> Result<DiarizationRunResult, String> {
    report_progress(
        progress.as_ref(),
        meeting_id,
        "validating",
        15,
        "Checking meeting and recording",
    );

    let audio_path =
        match MeetingsRepository::resolve_completed_recording(pool, meeting_id, recording_active)
            .await
            .map_err(|database_error| {
                error!("Unable to resolve meeting recording for diarization: {database_error}");
                PERSISTENCE_FAILED.to_string()
            })? {
            MeetingRecordingResolution::Ready(path) => path,
            MeetingRecordingResolution::MeetingNotFound => {
                return Err(MEETING_NOT_FOUND.to_string())
            }
            MeetingRecordingResolution::RecordingActive => return Err(RECORDING_ACTIVE.to_string()),
            MeetingRecordingResolution::RecordingIncomplete => {
                return Err(RECORDING_INCOMPLETE.to_string())
            }
            MeetingRecordingResolution::FolderUnavailable
            | MeetingRecordingResolution::RecordingUnavailable => {
                return Err(MISSING_RECORDING.to_string())
            }
        };

    ensure_post_call_run_allowed(pool).await?;

    let blocking_progress = progress.clone();
    let blocking_meeting_id = meeting_id.to_string();
    let engine_segments = tauri::async_runtime::spawn_blocking(move || {
        report_progress(
            blocking_progress.as_ref(),
            &blocking_meeting_id,
            "decoding",
            25,
            "Preparing meeting audio",
        );
        let audio = decode_audio_to_mono_16k(&audio_path).map_err(BlockingFailure::Audio)?;
        let runtime: Arc<dyn DiarizationRuntime> = match runtime_source {
            RuntimeSource::Ready(runtime) => runtime,
            RuntimeSource::Sherpa {
                resource_root,
                num_threads,
            } => {
                report_progress(
                    blocking_progress.as_ref(),
                    &blocking_meeting_id,
                    "preparing_models",
                    35,
                    "Verifying offline speaker models",
                );
                Arc::new(
                    SherpaDiarizationRuntime::from_resource_root(&resource_root, num_threads)
                        .map_err(BlockingFailure::Runtime)?,
                )
            }
        };
        report_progress(
            blocking_progress.as_ref(),
            &blocking_meeting_id,
            "detecting_speakers",
            40,
            "Detecting speakers locally",
        );
        let segments = runtime.diarize(&audio).map_err(BlockingFailure::Runtime)?;
        report_progress(
            blocking_progress.as_ref(),
            &blocking_meeting_id,
            "normalizing",
            80,
            "Preparing speaker labels",
        );
        Ok::<_, BlockingFailure>(segments)
    })
    .await
    .map_err(|join_error| {
        error!("Post-call diarization worker failed to join: {join_error}");
        INFERENCE_FAILED.to_string()
    })?
    .map_err(map_blocking_failure)?;

    let segments =
        normalize_engine_segments(meeting_id, engine_segments).map_err(|runtime_error| {
            error!("Post-call diarization output validation failed: {runtime_error}");
            map_runtime_error(runtime_error)
        })?;
    let speaker_count = segments
        .iter()
        .filter_map(|segment| segment.speaker_id.as_deref())
        .collect::<BTreeSet<_>>()
        .len();
    let segment_count = segments.len();

    report_progress(
        progress.as_ref(),
        meeting_id,
        "applying",
        90,
        "Updating the transcript",
    );
    apply_diarization_segments_with_pool(
        pool,
        ApplyDiarizationRequest {
            meeting_id: meeting_id.to_string(),
            method: DIARIZATION_METHOD.to_string(),
            run_kind: DiarizationRunKind::PostCall,
            segments,
        },
    )
    .await
    .map_err(|apply_error| {
        error!("Unable to atomically apply post-call diarization: {apply_error}");
        match apply_error.as_str() {
            "feature_disabled" | "meeting_not_found" => apply_error,
            _ => PERSISTENCE_FAILED.to_string(),
        }
    })?;

    Ok(DiarizationRunResult {
        segment_count,
        speaker_count,
    })
}

fn report_progress(
    callback: Option<&ProgressCallback>,
    meeting_id: &str,
    stage: &str,
    progress: u8,
    message: &str,
) {
    if let Some(callback) = callback {
        callback(DiarizationProgress {
            meeting_id: meeting_id.to_string(),
            stage: stage.to_string(),
            progress: progress.min(100),
            message: message.to_string(),
        });
    }
}

fn map_blocking_failure(failure: BlockingFailure) -> String {
    match failure {
        BlockingFailure::Audio(audio_error) => {
            error!("Unable to decode meeting audio for diarization: {audio_error}");
            match audio_error {
                DiarizationAudioError::Unavailable => MISSING_RECORDING.to_string(),
                DiarizationAudioError::Empty => RECORDING_INCOMPLETE.to_string(),
                DiarizationAudioError::Unsupported | DiarizationAudioError::InvalidSamples => {
                    INVALID_AUDIO.to_string()
                }
            }
        }
        BlockingFailure::Runtime(runtime_error) => {
            error!("Post-call diarization inference failed: {runtime_error}");
            map_runtime_error(runtime_error)
        }
    }
}

fn map_runtime_error(error: RuntimeError) -> String {
    match error {
        RuntimeError::ModelUnavailable => MODEL_UNAVAILABLE,
        RuntimeError::ModelInvalid => MODEL_INVALID,
        RuntimeError::NoSpeech => INSUFFICIENT_SPEECH,
        RuntimeError::InvalidOutput => INVALID_OUTPUT,
        RuntimeError::InvalidAudio => INVALID_AUDIO,
        RuntimeError::InitializationFailed | RuntimeError::InferenceFailed => INFERENCE_FAILED,
    }
    .to_string()
}

fn is_valid_meeting_id(meeting_id: &str) -> bool {
    meeting_id == meeting_id.trim()
        && !meeting_id.is_empty()
        && meeting_id.chars().count() <= 128
        && !meeting_id.chars().any(char::is_control)
}
