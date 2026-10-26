#[cfg(target_os = "macos")]
use cidre::av;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicrophonePermissionState {
    NotDetermined,
    Authorized,
    Denied,
    Restricted,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemAudioAccessState {
    Unknown,
    Available,
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPermissionSnapshot {
    pub microphone: MicrophonePermissionState,
    pub system_audio: SystemAudioAccessState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AudioPermissionError {
    MicrophoneDenied,
    MicrophoneUnavailable,
    MicrophoneRequestFailed(String),
}

pub struct AudioPermissionCoordinator {
    request_gate: tokio::sync::Mutex<()>,
    system_audio: AtomicU8,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioPermissionKind {
    Microphone,
    SystemAudio,
}

impl Default for AudioPermissionCoordinator {
    fn default() -> Self {
        Self {
            request_gate: tokio::sync::Mutex::new(()),
            system_audio: AtomicU8::new(SystemAudioAccessState::Unknown as u8),
        }
    }
}

impl AudioPermissionCoordinator {
    pub fn snapshot(&self) -> AudioPermissionSnapshot {
        AudioPermissionSnapshot {
            microphone: microphone_permission_state(),
            system_audio: self.system_audio_state(),
        }
    }

    pub fn system_audio_state(&self) -> SystemAudioAccessState {
        match self.system_audio.load(Ordering::Acquire) {
            0 => SystemAudioAccessState::Unknown,
            1 => SystemAudioAccessState::Available,
            2 => SystemAudioAccessState::Denied,
            3 => SystemAudioAccessState::Unavailable,
            _ => SystemAudioAccessState::Unavailable,
        }
    }

    pub fn store_system_audio_state(&self, state: SystemAudioAccessState) {
        let value = match state {
            SystemAudioAccessState::Unknown => 0,
            SystemAudioAccessState::Available => 1,
            SystemAudioAccessState::Denied => 2,
            SystemAudioAccessState::Unavailable => 3,
        };
        self.system_audio.store(value, Ordering::Release);
    }

    pub async fn request_microphone_permission<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
    ) -> Result<AudioPermissionSnapshot, String> {
        let _request = self.request_gate.lock().await;
        self.request_microphone_permission_locked(&app).await?;

        Ok(self.snapshot())
    }

    pub async fn prepare_capture_permissions<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        requested_system: bool,
    ) -> Result<bool, AudioPermissionError> {
        let _request = self.request_gate.lock().await;
        self.request_microphone_permission_locked(app)
            .await
            .map_err(AudioPermissionError::MicrophoneRequestFailed)?;

        match microphone_permission_state() {
            MicrophonePermissionState::Authorized => {}
            MicrophonePermissionState::Denied | MicrophonePermissionState::Restricted => {
                return Err(AudioPermissionError::MicrophoneDenied);
            }
            MicrophonePermissionState::NotDetermined | MicrophonePermissionState::Unavailable => {
                return Err(AudioPermissionError::MicrophoneUnavailable);
            }
        }

        if !requested_system {
            return Ok(false);
        }

        // Do not probe CoreAudio on the recording start path. On some macOS
        // installations stream creation waits for the system audio permission
        // service for more than a minute. The capture owner now starts this
        // source asynchronously and reports a runtime degradation event when
        // the stream cannot be opened, so the microphone can begin promptly.
        // A previous successful stream also never proves current authorization.
        self.store_system_audio_state(SystemAudioAccessState::Unknown);
        Ok(system_audio_capture_can_be_attempted(
            SystemAudioAccessState::Unknown,
        ))
    }

    async fn request_microphone_permission_locked<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
    ) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        if microphone_permission_state() == MicrophonePermissionState::NotDetermined {
            request_microphone_access(app).await?;
        }

        Ok(())
    }

    pub async fn request_system_audio_permission(&self) -> AudioPermissionSnapshot {
        self.request_system_audio_permission_with_probe(probe_system_audio_access)
            .await
    }

    async fn request_system_audio_permission_with_probe<F>(
        &self,
        probe: F,
    ) -> AudioPermissionSnapshot
    where
        F: Fn() -> SystemAudioAccessState + Send + 'static,
    {
        self.request_system_audio_permission_with_probe_and_hooks(probe, || {}, || {})
            .await
    }

    #[cfg(test)]
    async fn request_system_audio_permission_with_probe_hooks<F, BeforeGate, AfterGate>(
        &self,
        probe: F,
        before_gate: BeforeGate,
        after_gate: AfterGate,
    ) -> AudioPermissionSnapshot
    where
        F: Fn() -> SystemAudioAccessState + Send + 'static,
        BeforeGate: FnOnce() + Send,
        AfterGate: FnOnce() + Send,
    {
        self.request_system_audio_permission_with_probe_and_hooks(probe, before_gate, after_gate)
            .await
    }

    async fn request_system_audio_permission_with_probe_and_hooks<F, BeforeGate, AfterGate>(
        &self,
        probe: F,
        before_gate: BeforeGate,
        after_gate: AfterGate,
    ) -> AudioPermissionSnapshot
    where
        F: Fn() -> SystemAudioAccessState + Send + 'static,
        BeforeGate: FnOnce() + Send,
        AfterGate: FnOnce() + Send,
    {
        before_gate();
        let _request = self.request_gate.lock().await;
        after_gate();
        let state = tokio::task::spawn_blocking(probe)
            .await
            .unwrap_or(SystemAudioAccessState::Unavailable);
        self.store_system_audio_state(state);
        self.snapshot()
    }
}

fn system_audio_capture_can_be_attempted(state: SystemAudioAccessState) -> bool {
    matches!(
        state,
        SystemAudioAccessState::Unknown | SystemAudioAccessState::Available
    )
}

#[cfg(target_os = "macos")]
fn map_microphone_state(state: av::AuthorizationStatus) -> MicrophonePermissionState {
    match state {
        av::AuthorizationStatus::NotDetermined => MicrophonePermissionState::NotDetermined,
        av::AuthorizationStatus::Authorized => MicrophonePermissionState::Authorized,
        av::AuthorizationStatus::Denied => MicrophonePermissionState::Denied,
        av::AuthorizationStatus::Restricted => MicrophonePermissionState::Restricted,
    }
}

#[cfg(target_os = "macos")]
fn microphone_permission_state() -> MicrophonePermissionState {
    av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio())
        .map(map_microphone_state)
        .unwrap_or(MicrophonePermissionState::Unavailable)
}

#[cfg(not(target_os = "macos"))]
fn microphone_permission_state() -> MicrophonePermissionState {
    MicrophonePermissionState::Unavailable
}

#[cfg(target_os = "macos")]
async fn request_microphone_access<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let error_sender = sender.clone();
        let mut completion = cidre::blocks::SendBlock::new1(move |_| {
            let _ = sender.send(Ok(()));
        });
        if let Err(error) = av::CaptureDevice::request_access_for_media_type_ch(
            av::MediaType::audio(),
            &mut completion,
        ) {
            let _ = error_sender.send(Err(format!(
                "Failed to request microphone permission: {error:?}"
            )));
        }
    })
    .map_err(|error| format!("Failed to request microphone permission: {error}"))?;

    tokio::task::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| format!("Microphone permission task failed: {error}"))?
        .map_err(|error| format!("Microphone permission callback failed: {error}"))?
}

#[cfg(target_os = "macos")]
fn probe_system_audio_access() -> SystemAudioAccessState {
    use std::sync::{atomic::AtomicBool, Arc};
    use std::time::Duration;

    let result = crate::audio::capture::CoreAudioCapture::new_for_preference_cause(None)
        .and_then(|capture| capture.stream_with_pause_cause(Arc::new(AtomicBool::new(false))))
        .map(|stream| {
            std::thread::sleep(Duration::from_millis(250));
            drop(stream);
        });
    system_audio_probe_state(result)
}

#[cfg(target_os = "macos")]
fn system_audio_probe_state(
    result: Result<(), crate::audio::capture::core_audio::CoreAudioSetupError>,
) -> SystemAudioAccessState {
    match result {
        // A created stream may contain silence when permission is denied.
        // Neither stream creation nor silence proves authorization.
        Ok(()) => SystemAudioAccessState::Unknown,
        Err(crate::audio::capture::core_audio::CoreAudioSetupError::PermissionDenied(_)) => {
            SystemAudioAccessState::Denied
        }
        Err(_) => SystemAudioAccessState::Unavailable,
    }
}

#[cfg(not(target_os = "macos"))]
fn probe_system_audio_access() -> SystemAudioAccessState {
    SystemAudioAccessState::Unavailable
}

fn settings_pane(kind: AudioPermissionKind) -> &'static str {
    match kind {
        AudioPermissionKind::Microphone => "Privacy_Microphone",
        AudioPermissionKind::SystemAudio => "Privacy_ScreenCapture",
    }
}

#[tauri::command]
pub fn get_audio_permission_snapshot(
    coordinator: tauri::State<'_, AudioPermissionCoordinator>,
) -> AudioPermissionSnapshot {
    coordinator.snapshot()
}

#[tauri::command]
pub async fn request_microphone_permission(
    app: tauri::AppHandle,
    coordinator: tauri::State<'_, AudioPermissionCoordinator>,
) -> Result<AudioPermissionSnapshot, String> {
    coordinator.request_microphone_permission(app).await
}

#[tauri::command]
pub async fn request_system_audio_permission(
    coordinator: tauri::State<'_, AudioPermissionCoordinator>,
) -> Result<AudioPermissionSnapshot, String> {
    Ok(coordinator.request_system_audio_permission().await)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn open_audio_permission_settings(kind: AudioPermissionKind) -> Result<(), String> {
    crate::utils::open_system_settings(settings_pane(kind).to_owned()).await
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn open_audio_permission_settings(_kind: AudioPermissionKind) -> Result<(), String> {
    Err("Audio permission settings are only available on macOS".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    use cidre::av;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    #[test]
    #[cfg(target_os = "macos")]
    fn created_stream_is_unconfirmed_and_only_explicit_permission_errors_are_denied() {
        use crate::audio::capture::core_audio::CoreAudioSetupError;
        assert_eq!(
            system_audio_probe_state(Ok(())),
            SystemAudioAccessState::Unknown
        );
        assert_eq!(
            system_audio_probe_state(Err(CoreAudioSetupError::PermissionDenied("denied".into()))),
            SystemAudioAccessState::Denied
        );
        assert_eq!(
            system_audio_probe_state(Err(CoreAudioSetupError::CaptureFailed("failed".into()))),
            SystemAudioAccessState::Unavailable
        );
    }

    #[test]
    fn unconfirmed_audio_can_be_attempted_but_known_failures_use_microphone_only() {
        for (state, expected) in [
            (SystemAudioAccessState::Unknown, true),
            (SystemAudioAccessState::Available, true),
            (SystemAudioAccessState::Denied, false),
            (SystemAudioAccessState::Unavailable, false),
        ] {
            assert_eq!(system_audio_capture_can_be_attempted(state), expected);
        }
    }

    #[tokio::test]
    async fn retry_does_not_trust_a_previously_available_stream() {
        let coordinator = AudioPermissionCoordinator::default();
        coordinator.store_system_audio_state(SystemAudioAccessState::Available);
        let snapshot = coordinator
            .request_system_audio_permission_with_probe(|| SystemAudioAccessState::Unknown)
            .await;
        assert_eq!(snapshot.system_audio, SystemAudioAccessState::Unknown);
        let snapshot = coordinator
            .request_system_audio_permission_with_probe(|| SystemAudioAccessState::Denied)
            .await;
        assert_eq!(snapshot.system_audio, SystemAudioAccessState::Denied);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn maps_every_avfoundation_microphone_state_without_device_enumeration() {
        assert_eq!(
            map_microphone_state(av::AuthorizationStatus::NotDetermined),
            MicrophonePermissionState::NotDetermined
        );
        assert_eq!(
            map_microphone_state(av::AuthorizationStatus::Authorized),
            MicrophonePermissionState::Authorized
        );
        assert_eq!(
            map_microphone_state(av::AuthorizationStatus::Denied),
            MicrophonePermissionState::Denied
        );
        assert_eq!(
            map_microphone_state(av::AuthorizationStatus::Restricted),
            MicrophonePermissionState::Restricted
        );
    }

    #[test]
    fn system_audio_state_round_trip_keeps_unknown_distinct_from_denied() {
        let coordinator = AudioPermissionCoordinator::default();
        assert_eq!(
            coordinator.system_audio_state(),
            SystemAudioAccessState::Unknown
        );
        coordinator.store_system_audio_state(SystemAudioAccessState::Denied);
        assert_eq!(
            coordinator.system_audio_state(),
            SystemAudioAccessState::Denied
        );
    }

    #[test]
    fn maps_each_permission_kind_to_its_system_settings_pane() {
        assert_eq!(
            settings_pane(AudioPermissionKind::Microphone),
            "Privacy_Microphone"
        );
        assert_eq!(
            settings_pane(AudioPermissionKind::SystemAudio),
            "Privacy_ScreenCapture"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn serializes_system_audio_probes_and_returns_the_final_state() {
        let coordinator = Arc::new(AudioPermissionCoordinator::default());
        let first_probe_started = Arc::new(Barrier::new(2));
        let release_first_probe = Arc::new(Barrier::new(2));
        let second_attempting_gate = Arc::new(Barrier::new(2));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));

        let probe = {
            let first_probe_started = first_probe_started.clone();
            let release_first_probe = release_first_probe.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            let calls = calls.clone();
            move || {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    first_probe_started.wait();
                    release_first_probe.wait();
                }
                active.fetch_sub(1, Ordering::SeqCst);
                SystemAudioAccessState::Denied
            }
        };

        let first = tokio::spawn({
            let coordinator = coordinator.clone();
            let probe = probe.clone();
            async move {
                coordinator
                    .request_system_audio_permission_with_probe(probe)
                    .await
            }
        });

        tokio::task::spawn_blocking({
            let first_probe_started = first_probe_started.clone();
            move || first_probe_started.wait()
        })
        .await
        .unwrap();

        let (second_acquired_gate, mut second_acquired_gate_rx) = tokio::sync::oneshot::channel();
        let second = tokio::spawn({
            let coordinator = coordinator.clone();
            let second_attempting_gate = second_attempting_gate.clone();
            async move {
                coordinator
                    .request_system_audio_permission_with_probe_hooks(
                        probe,
                        move || {
                            second_attempting_gate.wait();
                        },
                        move || {
                            let _ = second_acquired_gate.send(());
                        },
                    )
                    .await
            }
        });

        tokio::task::spawn_blocking(move || second_attempting_gate.wait())
            .await
            .unwrap();
        let second_acquired_gate_early =
            tokio::time::timeout(Duration::from_millis(100), &mut second_acquired_gate_rx)
                .await
                .is_ok();

        release_first_probe.wait();
        let first = first.await.unwrap();
        let second = second.await.unwrap();

        assert!(
            !second_acquired_gate_early,
            "second request acquired the gate before the first denied probe completed"
        );
        assert_eq!(max_active.load(Ordering::SeqCst), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(first.system_audio, SystemAudioAccessState::Denied);
        assert_eq!(second.system_audio, SystemAudioAccessState::Denied);
    }
}
