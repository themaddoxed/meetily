use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::capture::backend::{SourceControl, TrackKind};
use super::recording_session::{
    ContractError, RecordingSession, RecordingSessionSnapshot, StartRecordingRequest,
};
pub use super::transcription::TranscriptUpdate;

#[derive(Debug, Deserialize)]
pub struct RecordingArgs {
    pub save_path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TranscriptionStatus {
    pub chunks_in_queue: usize,
    pub is_processing: bool,
    pub last_activity_ms: u64,
}

fn start_request(
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
    save_path: Option<String>,
) -> StartRecordingRequest {
    let microphone_device_explicit = mic_device_name.is_some();
    let meeting_name = meeting_name.unwrap_or_else(|| {
        format!(
            "Meeting {}",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        )
    });
    StartRecordingRequest {
        meeting_id: uuid::Uuid::new_v4().to_string(),
        idempotency_key: uuid::Uuid::new_v4().to_string(),
        microphone_device: mic_device_name,
        microphone_device_explicit,
        system_device: system_device_name,
        capture_system: true,
        meeting_name,
        meeting_vocabulary: None,
        save_path,
        auto_save: None,
    }
}

pub async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
) -> Result<RecordingSessionSnapshot, ContractError> {
    start_recording_with_meeting_name(app, None).await
}

pub async fn start_recording_with_meeting_name<R: Runtime>(
    app: AppHandle<R>,
    meeting_name: Option<String>,
) -> Result<RecordingSessionSnapshot, ContractError> {
    start_recording_with_devices_and_meeting(app, None, None, meeting_name).await
}

pub async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<RecordingSessionSnapshot, ContractError> {
    start_recording_with_devices_and_meeting(app, mic_device_name, system_device_name, None).await
}

pub async fn start_recording_with_devices_and_meeting<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<RecordingSessionSnapshot, ContractError> {
    let session = app.state::<RecordingSession>();
    let snapshot = session
        .start(
            app.clone(),
            start_request(mic_device_name, system_device_name, meeting_name, None),
        )
        .await?;
    let _ = app.emit(
        "recording-started",
        serde_json::json!({ "sessionId": snapshot.session_id }),
    );
    crate::tray::update_tray_menu(&app);
    Ok(snapshot)
}

pub async fn stop_recording<R: Runtime>(
    app: AppHandle<R>,
    args: RecordingArgs,
) -> Result<RecordingSessionSnapshot, ContractError> {
    let session = app.state::<RecordingSession>();
    let active = session
        .get_active()
        .await
        .ok_or_else(|| ContractError::not_found(None))?;
    let _ = app.emit(
        "recording-processing",
        serde_json::json!({
            "sessionId": active.session_id,
            "meetingId": active.meeting_id,
        }),
    );
    let snapshot = session.stop(active.session_id.clone()).await?;
    let _ = app.emit(
        "recording-stopped",
        serde_json::json!({
            "sessionId": snapshot.session_id,
            "folder_path": snapshot.save_path,
            "meeting_name": snapshot.meeting_name,
            "requested_save_path": args.save_path,
        }),
    );
    crate::tray::update_tray_menu(&app);
    Ok(snapshot)
}

#[tauri::command]
pub async fn pause_recording(
    session: State<'_, RecordingSession>,
    session_id: String,
) -> Result<RecordingSessionSnapshot, ContractError> {
    session.pause(session_id).await
}

#[tauri::command]
pub async fn resume_recording(
    session: State<'_, RecordingSession>,
    session_id: String,
) -> Result<RecordingSessionSnapshot, ContractError> {
    session.resume(session_id).await
}

#[tauri::command]
pub async fn set_recording_source_control(
    session: State<'_, RecordingSession>,
    session_id: String,
    kind: TrackKind,
    control: SourceControl,
) -> Result<RecordingSessionSnapshot, ContractError> {
    session.set_source_control(session_id, kind, control).await
}

#[tauri::command]
pub async fn get_recording_session(
    session: State<'_, RecordingSession>,
    session_id: String,
) -> Result<RecordingSessionSnapshot, ContractError> {
    session.get(session_id).await
}

#[tauri::command]
pub async fn recover_recording_session(
    session: State<'_, RecordingSession>,
    session_id: String,
) -> Result<RecordingSessionSnapshot, ContractError> {
    session.recover(session_id).await
}

#[tauri::command]
pub async fn get_active_recording_session(
    session: State<'_, RecordingSession>,
) -> Result<Option<RecordingSessionSnapshot>, ContractError> {
    Ok(session.get_active().await)
}

pub async fn is_recording() -> bool {
    RecordingSession::projected_active()
        .await
        .is_some_and(|snapshot| snapshot.state.is_nonterminal())
}

pub async fn get_transcription_status() -> TranscriptionStatus {
    TranscriptionStatus {
        chunks_in_queue: 0,
        is_processing: is_recording().await,
        last_activity_ms: 0,
    }
}

#[tauri::command]
pub async fn is_recording_paused() -> bool {
    RecordingSession::projected_active()
        .await
        .is_some_and(|snapshot| {
            snapshot.state == super::recording_session_state::RecordingSessionState::Paused
        })
}

#[tauri::command]
pub async fn get_recording_state() -> serde_json::Value {
    match RecordingSession::projected_active().await {
        Some(snapshot) => serde_json::json!({
            "is_recording": snapshot.state.is_nonterminal(),
            "is_paused": snapshot.state == super::recording_session_state::RecordingSessionState::Paused,
            "is_active": snapshot.state == super::recording_session_state::RecordingSessionState::Recording,
            "recording_duration": snapshot.recording_duration,
            "active_duration": snapshot.active_duration,
            "total_pause_duration": snapshot.pause_duration,
            "current_pause_duration": null,
        }),
        None => serde_json::json!({
            "is_recording": false, "is_paused": false, "is_active": false,
            "recording_duration": null, "active_duration": null,
            "total_pause_duration": 0.0, "current_pause_duration": null,
        }),
    }
}

#[tauri::command]
pub async fn get_meeting_folder_path() -> Result<Option<String>, String> {
    Ok(RecordingSession::projected_active()
        .await
        .map(|snapshot| snapshot.save_path))
}

#[tauri::command]
pub async fn get_transcript_history(
) -> Result<Vec<super::recording_saver::TranscriptSegment>, String> {
    Ok(RecordingSession::projected_transcript_history().await)
}

#[tauri::command]
pub async fn get_recording_meeting_name() -> Result<Option<String>, String> {
    Ok(RecordingSession::projected_active()
        .await
        .map(|snapshot| snapshot.meeting_name))
}

#[tauri::command]
pub async fn get_active_audio_output() -> Result<super::playback_monitor::AudioOutputInfo, String> {
    super::playback_monitor::get_active_audio_output()
        .await
        .map_err(|error| error.to_string())
}
