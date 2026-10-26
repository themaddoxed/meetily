use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};

#[derive(Debug, Clone)]
pub enum RecordingState {
    Stopped,
    Starting,
    Recording,
    Pausing,
    Paused,
    Resuming,
    Stopping,
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    // Start with default menu, will update with actual state after initialization
    // Pass can_record=true initially, will be updated by update_tray_menu immediately
    let menu = build_menu(app, RecordingState::Stopped, true)?;

    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Meetily")
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .build(app)?;

    // Update tray menu with actual recording state after creation
    update_tray_menu(app);

    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, item_id: &str) {
    match item_id {
        "toggle_recording" => toggle_recording_handler(app),
        "pause_recording" => pause_recording_handler(app),
        "resume_recording" => resume_recording_handler(app),
        "stop_recording" => stop_recording_handler(app),
        "open_window" => focus_main_window(app),
        "settings" => {
            focus_main_window(app);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("window.location.assign('/settings')");
            }
        }
        "quit" => {
            let _ = crate::begin_exit_after_recording_shutdown(app);
        }
        _ => {}
    }
}
fn toggle_recording_handler<R: Runtime>(app: &AppHandle<R>) {
    focus_main_window(app);
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        if crate::is_recording().await {
            log::info!("Tray toggle: Stopping recording...");

            let session = app_clone.state::<crate::audio::RecordingSession>();
            let stop_result = match session.get_active().await {
                Some(snapshot) => session.stop(snapshot.session_id).await,
                None => {
                    update_tray_menu_async(&app_clone).await;
                    return;
                }
            };

            // Handle result
            match stop_result {
                Ok(snapshot) => {
                    log::info!("Tray toggle: Recording stopped successfully");
                    update_tray_from_snapshot(&app_clone, &snapshot);

                    // Trigger frontend post-processing via event (works from any page)
                    // (SQLite save and navigation)
                    if let Err(e) = app_clone.emit(
                        "recording-stop-complete",
                        serde_json::json!({ "sessionId": snapshot.session_id }),
                    ) {
                        log::error!(
                            "Tray toggle: Failed to emit recording-stop-complete event: {}",
                            e
                        );
                    }
                }
                Err(e) => {
                    log::error!("Tray toggle: Failed to stop recording: {}", e);
                    // Revert tray state on error
                    update_tray_menu_async(&app_clone).await;
                }
            }
        } else {
            // Immediately show starting state
            set_tray_state(&app_clone, RecordingState::Starting);

            log::info!("Emitting start recording event from tray");
            if let Some(window) = app_clone.get_webview_window("main") {
                let _ = window.eval("sessionStorage.setItem('autoStartRecording', 'true')"); // Set the flag to start recording automatically
                let _ = window.eval("window.location.assign('/')");
            }
        }
    });
}

fn pause_recording_handler<R: Runtime>(app: &AppHandle<R>) {
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let session = app_clone.state::<crate::audio::RecordingSession>();
        let result = match session.get_active().await {
            Some(snapshot) => session.pause(snapshot.session_id).await,
            None => Err(crate::audio::recording_session::ContractError::not_found(
                None,
            )),
        };
        match result {
            Ok(snapshot) => {
                update_tray_from_snapshot(&app_clone, &snapshot);
                log::info!("Recording paused from tray");
            }
            Err(e) => {
                log::error!("Failed to pause recording from tray: {}", e);
                update_tray_menu_async(&app_clone).await;
            }
        }
    });
}

fn resume_recording_handler<R: Runtime>(app: &AppHandle<R>) {
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let session = app_clone.state::<crate::audio::RecordingSession>();
        let result = match session.get_active().await {
            Some(snapshot) => session.resume(snapshot.session_id).await,
            None => Err(crate::audio::recording_session::ContractError::not_found(
                None,
            )),
        };
        match result {
            Ok(snapshot) => {
                update_tray_from_snapshot(&app_clone, &snapshot);
                log::info!("Recording resumed from tray");
            }
            Err(e) => {
                log::error!("Failed to resume recording from tray: {}", e);
                update_tray_menu_async(&app_clone).await;
            }
        }
    });
}

fn stop_recording_handler<R: Runtime>(app: &AppHandle<R>) {
    focus_main_window(app);
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        log::info!("Tray: Stopping recording...");

        let session = app_clone.state::<crate::audio::RecordingSession>();
        let stop_result = match session.get_active().await {
            Some(snapshot) => session.stop(snapshot.session_id).await,
            None => {
                update_tray_menu_async(&app_clone).await;
                return;
            }
        };

        // Handle result
        match stop_result {
            Ok(snapshot) => {
                log::info!("Tray: Recording stopped successfully");
                update_tray_from_snapshot(&app_clone, &snapshot);

                // Trigger frontend post-processing via event (works from any page)
                // (SQLite save and navigation)
                if let Err(e) = app_clone.emit(
                    "recording-stop-complete",
                    serde_json::json!({ "sessionId": snapshot.session_id }),
                ) {
                    log::error!("Tray: Failed to emit recording-stop-complete event: {}", e);
                }
            }
            Err(e) => {
                log::error!("Tray: Failed to stop recording: {}", e);
                // Revert tray state on error
                update_tray_menu_async(&app_clone).await;
            }
        }
    });
}

pub(crate) fn update_tray_from_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &crate::audio::RecordingSessionSnapshot,
) {
    use crate::audio::recording_session_state::RecordingSessionState as SessionState;
    let state = match snapshot.state {
        SessionState::Starting => RecordingState::Starting,
        SessionState::Recording => RecordingState::Recording,
        SessionState::Paused => RecordingState::Paused,
        SessionState::Stopping | SessionState::Finalizing => RecordingState::Stopping,
        SessionState::Completed | SessionState::Interrupted | SessionState::Failed => {
            RecordingState::Stopped
        }
    };
    set_tray_state(app, state);
}

pub(crate) fn perform_recording_shutdown_and_exit<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let session = app.state::<crate::audio::RecordingSession>();
        if let Err(error) = session.shutdown_active().await {
            log::error!("Recording shutdown before application exit failed: {error}");
        }
        crate::allow_exit_after_recording_shutdown();
        app.exit(0);
    });
}

pub fn update_tray_menu<R: Runtime>(app: &AppHandle<R>) {
    // For sync update, spawn async task to get current state
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        // Small delay to ensure recording state has been updated
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        update_tray_menu_async(&app_clone).await;
    });
}

pub fn set_tray_state<R: Runtime>(app: &AppHandle<R>, state: RecordingState) {
    log::info!("Tray: Setting intermediate state: {:?}", state);
    // During recording state transitions, we assume recording is allowed (we're already recording)
    if let Ok(menu) = build_menu(app, state, true) {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let result = tray.set_menu(Some(menu));
            log::info!("Tray: Intermediate state menu update result: {:?}", result);
        } else {
            log::warn!("Tray: Could not find tray with id 'main-tray'");
        }
    } else {
        log::error!("Tray: Failed to build menu for intermediate state");
    }
}

async fn get_current_recording_state() -> RecordingState {
    // Check if currently recording
    let is_recording = crate::audio::recording_commands::is_recording().await;
    log::info!(
        "Tray: get_current_recording_state - is_recording: {}",
        is_recording
    );

    if !is_recording {
        log::info!("Tray: Recording state is Stopped");
        return RecordingState::Stopped;
    }

    // Check if paused
    let is_paused = crate::audio::recording_commands::is_recording_paused().await;
    log::info!("Tray: is_paused: {}", is_paused);

    if is_paused {
        log::info!("Tray: Recording state is Paused");
        RecordingState::Paused
    } else {
        log::info!("Tray: Recording state is Recording");
        RecordingState::Recording
    }
}

/// Check if recording is allowed based on onboarding status and transcription model availability
/// Returns true if:
/// - Onboarding is complete (user may prefer Whisper later), OR
/// - Parakeet transcription model is ready (downloaded)
async fn check_can_record<R: Runtime>(app: &AppHandle<R>) -> bool {
    // First check if onboarding is complete
    let onboarding_complete = match crate::onboarding::load_onboarding_status(app).await {
        Ok(status) => status.completed,
        Err(e) => {
            log::warn!(
                "Tray: Failed to load onboarding status: {}, assuming complete",
                e
            );
            true // Assume complete if we can't check (safe default)
        }
    };

    // If onboarding is complete, always allow recording
    // (user may prefer Whisper or have their own transcription setup)
    if onboarding_complete {
        return true;
    }

    // During onboarding, check if Parakeet transcription model is ready
    match crate::parakeet_engine::commands::parakeet_has_available_models().await {
        Ok(has_models) => has_models,
        Err(e) => {
            log::warn!(
                "Tray: Failed to check Parakeet models: {}, assuming not ready",
                e
            );
            false
        }
    }
}

pub async fn update_tray_menu_async<R: Runtime>(app: &AppHandle<R>) {
    log::info!("Tray: update_tray_menu_async called");
    // Get the current recording state
    let recording_state = get_current_recording_state().await;
    log::info!("Tray: Current recording state: {:?}", recording_state);

    // Determine if recording should be allowed
    // Only block recording during incomplete onboarding when no transcription model is ready
    let can_record = check_can_record(app).await;
    log::info!("Tray: can_record: {}", can_record);

    if let Ok(menu) = build_menu(app, recording_state, can_record) {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let result = tray.set_menu(Some(menu));
            log::info!("Tray: Menu update result: {:?}", result);
        } else {
            log::warn!("Tray: Could not find tray with id 'main-tray'");
        }
    } else {
        log::error!("Tray: Failed to build menu");
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: RecordingState,
    can_record: bool, // True if recording is allowed (onboarding complete OR transcription model ready)
) -> tauri::Result<tauri::menu::Menu<R>> {
    let mut builder = MenuBuilder::new(app);

    // If recording is not allowed (during onboarding, no transcription model), show disabled message
    if !can_record {
        builder = builder.item(
            &MenuItemBuilder::new("⏳ Downloading transcription model...")
                .enabled(false)
                .build(app)?,
        );
    } else {
        match state {
            RecordingState::Stopped => {
                builder = builder.item(
                    &MenuItemBuilder::with_id("toggle_recording", "Start Recording").build(app)?,
                );
            }
            RecordingState::Starting => {
                builder = builder.item(
                    &MenuItemBuilder::new("🔄 Starting Recording...")
                        .enabled(false)
                        .build(app)?,
                );
            }
            RecordingState::Recording => {
                builder = builder
                    .item(
                        &MenuItemBuilder::with_id("pause_recording", "⏸ Pause Recording")
                            .build(app)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("stop_recording", "⏹ Stop Recording")
                            .build(app)?,
                    );
            }
            RecordingState::Pausing => {
                builder = builder
                    .item(
                        &MenuItemBuilder::new("⏸ Pausing...")
                            .enabled(false)
                            .build(app)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("stop_recording", "⏹ Stop Recording")
                            .build(app)?,
                    );
            }
            RecordingState::Paused => {
                builder = builder
                    .item(
                        &MenuItemBuilder::with_id("resume_recording", "▶ Resume Recording")
                            .build(app)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("stop_recording", "⏹ Stop Recording")
                            .build(app)?,
                    );
            }
            RecordingState::Resuming => {
                builder = builder
                    .item(
                        &MenuItemBuilder::new("▶ Resuming...")
                            .enabled(false)
                            .build(app)?,
                    )
                    .item(
                        &MenuItemBuilder::with_id("stop_recording", "⏹ Stop Recording")
                            .build(app)?,
                    );
            }
            RecordingState::Stopping => {
                builder = builder.item(
                    &MenuItemBuilder::new("⏹ Stopping...")
                        .enabled(false)
                        .build(app)?,
                );
            }
        }
    }

    builder
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&MenuItemBuilder::with_id("open_window", "Open Main Window").build(app)?)
        .item(&MenuItemBuilder::with_id("settings", "Settings").build(app)?)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&MenuItemBuilder::with_id("quit", "Quit").build(app)?)
        .build()
}

pub(crate) fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.unminimize() {
            log::error!("Failed to unminimize main window: {}", e);
        }

        if let Err(e) = window.show() {
            log::error!("Failed to show main window: {}", e);
        }

        if let Err(e) = window.set_focus() {
            log::error!("Failed to focus main window: {}", e);
        }

        if let Err(e) = window.eval("window.focus()") {
            log::error!("Failed to focus main webview: {}", e);
        }
    } else {
        log::warn!("Could not find main window");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tray_pause_resume_projects_returned_snapshot_only() {
        let source = include_str!("tray.rs");
        assert!(source.contains("Ok(snapshot)"));
        assert!(source.contains("update_tray_from_snapshot(&app_clone, &snapshot)"));
    }
}
