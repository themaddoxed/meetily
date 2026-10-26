> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# FORK-051 macOS Permissions and Microphone Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make macOS audio authorization truthful, keep system audio optional, automatically move an active recording to a usable default microphone after disconnect, and preserve recoverable audio when no replacement exists.

**Architecture:** A managed Rust permission coordinator owns native authorization reads and serialized requests. The existing capture-owner thread remains the only owner of CPAL and Core Audio streams; it reports nonterminal route/degradation events to `RecordingSession`, which updates the active snapshot and emits typed Tauri events. One frontend hook owns live permission state and refreshes it on focus, while the recording state provider owns device-change notifications.

**Tech Stack:** Rust 2021, Tauri 2.6.2, Tokio, CPAL 0.15.3, pinned cidre 0.11.3 (`a9587fa1d4ff6f0d5d082849ae7c62880fd739f7`), React 18, TypeScript 5.7, Vitest 4, Node test runner, Playwright.

## Global Constraints

- Product comparison base is exact commit `b7126ea82be887c38e8a8e8de30a48f9bb409e51` on `fork/macos-integration`; the approved-design commit is exact `4221f872ef7c3bc6419e1ee019e682c5d8bd4728`.
- Microphone access is required to start recording.
- System audio is optional at onboarding, recording start, and runtime; failure must continue microphone-only and notify the user.
- A disconnected microphone switches only the current session to an available default microphone; saved preferences do not change.
- When no microphone replacement opens within the bounded retry window, finalize the session as recoverable `interrupted` and retain verified checkpoints.
- Passive mount/focus refresh never prompts. Only an explicit user action may request macOS authorization.
- Never infer permission from device enumeration or infer denial from silence.
- Keep `RecordingSession` as the only lifecycle owner and `MacosCaptureBackend` as the only production capture adapter. Do not revive `AudioDeviceMonitor` or removed compatibility commands.
- Add no dependency. The pinned Tauri, cidre, CPAL, Tokio, Sonner, and existing test stack are sufficient.
- Preserve the current system-audio usage descriptions and audio-input entitlement; correct inaccurate UI copy without adding Screen Recording or virtual-device requirements.
- Physical `tccutil` resets and real permission toggles require a separate explicit user authorization immediately before execution.
- No push, pull request, release, Developer ID signing, notarization, or publication.
- Repository-wide gates must introduce no new failure. The comparison base currently has exactly four existing frontend lint errors in `RecordingControls.tsx` and `useRecordingStateSync.ts`, exactly ten strict-Clippy findings, and exactly four `audio::recording_session` workspace-test failures; any different count, path, diagnostic, or test name stops execution.
- Keep each implementation commit scoped to the task that produced it. Do not edit the approved design after implementation starts; record evidence in the final PRD closeout.

## File structure

### Create

- `frontend/src/lib/audioPermissions.ts`: frontend permission/access types and pure readiness/copy helpers.
- `frontend/tests/lib/audio-permissions.test.ts`: Vitest coverage for the pure frontend permission contract.
- `frontend/tests/macos-permissions-lifecycle.spec.ts`: Playwright first-run, optional-system-audio, focus-refresh, and recording-notification coverage.
- Ignored setup only, not committed: `frontend/node_modules` symlink to the clean integration-worktree installation.

### Modify

- `frontend/src-tauri/src/audio/permissions.rs`: native permission coordinator, snapshot types, commands, and focused unit tests.
- `frontend/src-tauri/src/audio/capture/core_audio.rs`: typed Core Audio setup errors through real stream start.
- `frontend/src-tauri/src/audio/capture/backend.rs`: bounded runtime-event transport owned by the capture thread.
- `frontend/src-tauri/src/audio/capture/macos.rs`: optional system capture, typed microphone disconnect, bounded replacement, and system-only degradation.
- `frontend/src-tauri/src/audio/recording_session.rs`: consume runtime events, update the current device, emit notifications, and retain recovery behavior.
- `frontend/src-tauri/src/lib.rs`: manage the permission coordinator and register the new commands.
- `frontend/src/hooks/usePermissionCheck.ts`: backend snapshot owner, generation-safe refresh, explicit requests, and focus refresh.
- `frontend/src/types/onboarding.ts`: remove independent onboarding permission storage and use the shared display state.
- `frontend/src/contexts/OnboardingContext.tsx`: remove stale permission state/setter ownership.
- `frontend/src/components/onboarding/steps/PermissionsStep.tsx`: consume the shared hook; require microphone only.
- `frontend/src/components/onboarding/shared/PermissionRow.tsx`: render authorized/available, denied/restricted, unknown, and unavailable states.
- `frontend/src/components/PermissionWarning.tsx`: blocking microphone copy and nonblocking Audio Capture copy.
- `frontend/src/app/_components/TranscriptPanel.tsx`: pass typed permission states to the warning.
- `frontend/src/app/page.tsx`: gate controls only on native microphone authorization or an active recording.
- `frontend/src/services/recordingService.ts`: typed route/degradation event listeners and message formatter.
- `frontend/src/contexts/RecordingStateContext.tsx`: listen for route/degradation events and show eight-second notifications.
- `frontend/src/testing/tauri-browser-mocks.ts`: deterministic permission commands and audio lifecycle events for Playwright.
- `frontend/scripts/release-policy.mjs`: protect native permission ownership, bounded capture events, and the absence of device-list permission inference.
- `frontend/tests/release-policy.test.mjs`: mutation tests for the new release-policy invariants.
- `frontend/tests/macos-audio-surface.test.mjs`: update the runtime-degradation contract and add policy mutation tests.

---

### Task 0: Reuse the verified local dependency installation

**Files:**
- Create ignored symlink only: `frontend/node_modules`

**Interfaces:**
- Consumes: clean installed dependency tree at `/Users/developer/Desktop/meeting-minutes/.worktrees/macos-fork-integration/frontend/node_modules`.
- Produces: local ignored `frontend/node_modules` link for deterministic offline frontend commands; no tracked change.

- [ ] **Step 1: Verify the source installation and destination**

```bash
test -d /Users/developer/Desktop/meeting-minutes/.worktrees/macos-fork-integration/frontend/node_modules
test ! -e /Users/developer/Desktop/meeting-minutes/.worktrees/fork-051-permissions-lifecycle/frontend/node_modules
```

Expected: both commands exit 0. If the destination already exists, verify it resolves to the same source before continuing; do not overwrite it.

- [ ] **Step 2: Create the ignored local link and verify Git remains clean**

```bash
ln -s /Users/developer/Desktop/meeting-minutes/.worktrees/macos-fork-integration/frontend/node_modules /Users/developer/Desktop/meeting-minutes/.worktrees/fork-051-permissions-lifecycle/frontend/node_modules
git status --short --branch
```

Expected: `node_modules` is available and Git shows no change from this setup.

---

### Task 1: Native permission snapshot and explicit request commands

**Files:**
- Modify: `frontend/src-tauri/src/audio/permissions.rs:1-153`
- Modify: `frontend/src-tauri/src/audio/capture/core_audio.rs:19-46,230-379`
- Modify: `frontend/src-tauri/src/lib.rs:390-475,547-559,655-659`

**Interfaces:**
- Consumes: `cidre::av::CaptureDevice`, `cidre::av::AuthorizationStatus`, `cidre::av::MediaType::audio()`, `CoreAudioCapture::new_for_preference_cause`.
- Produces:

```rust
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

pub struct AudioPermissionCoordinator {
    request_gate: tokio::sync::Mutex<()>,
    system_audio: std::sync::atomic::AtomicU8,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioPermissionKind {
    Microphone,
    SystemAudio,
}
```

- Produces Tauri commands `get_audio_permission_snapshot`, `request_microphone_permission`, `request_system_audio_permission`, and `open_audio_permission_settings(kind: AudioPermissionKind)` so each denied row opens its matching System Settings pane. The settings command maps `Microphone` to the existing `Privacy_Microphone` helper value and `SystemAudio` to `Privacy_ScreenCapture`; the physical TCC matrix must verify that the latter lands on Screen & System Audio Recording on the supported macOS version.
- Produces `CoreAudioCapture::stream_with_pause_cause(self, Arc<AtomicBool>) -> Result<CoreAudioStream, CoreAudioSetupError>` so the probe and production capture share typed setup behavior.

- [ ] **Step 1: Add failing native-state mapping tests**

Add these tests inside `audio/permissions.rs`:

```rust
#[test]
fn maps_every_avfoundation_microphone_state_without_device_enumeration() {
    assert_eq!(map_microphone_state(av::AuthorizationStatus::NotDetermined), MicrophonePermissionState::NotDetermined);
    assert_eq!(map_microphone_state(av::AuthorizationStatus::Authorized), MicrophonePermissionState::Authorized);
    assert_eq!(map_microphone_state(av::AuthorizationStatus::Denied), MicrophonePermissionState::Denied);
    assert_eq!(map_microphone_state(av::AuthorizationStatus::Restricted), MicrophonePermissionState::Restricted);
}

#[test]
fn system_audio_state_round_trip_keeps_unknown_distinct_from_denied() {
    let coordinator = AudioPermissionCoordinator::default();
    assert_eq!(coordinator.system_audio_state(), SystemAudioAccessState::Unknown);
    coordinator.store_system_audio_state(SystemAudioAccessState::Denied);
    assert_eq!(coordinator.system_audio_state(), SystemAudioAccessState::Denied);
}
```

- [ ] **Step 2: Run the focused tests and confirm RED**

Run from `frontend/src-tauri`:

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::permissions::tests::maps_every_avfoundation_microphone_state_without_device_enumeration -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::permissions::tests::system_audio_state_round_trip_keeps_unknown_distinct_from_denied -- --exact
```

Expected: compile failure because the enums, coordinator, and mapping function do not exist.

- [ ] **Step 3: Replace boolean permission helpers with the native coordinator**

Implement `Default` with `Unknown = 0`, keep conversion between `AtomicU8` and `SystemAudioAccessState` exhaustive, and read microphone status only through:

```rust
fn microphone_permission_state() -> MicrophonePermissionState {
    av::CaptureDevice::authorization_status_for_media_type(av::MediaType::audio())
        .map(map_microphone_state)
        .unwrap_or(MicrophonePermissionState::Unavailable)
}
```

Make request methods acquire `request_gate` and re-read the current state after acquiring it. `request_microphone_permission` calls cidre only for `NotDetermined`; authorized, denied, and restricted states return without prompting again. `request_system_audio_permission` short-circuits only an already `Available` state, so an explicit retry after denied/unavailable may perform a fresh operational probe. The probe uses `tokio::task::spawn_blocking`, opens the real Core Audio stream, keeps it alive for 250 ms, drops it, and maps only typed permission errors to `Denied`. It never examines sample amplitude.

- [ ] **Step 4: Preserve typed Core Audio errors through stream startup**

Add a stage-aware classifier that maps only `os::Error::from_be_bytes(*b"perm")` to `PermissionDenied`, maps missing/default output devices to `DeviceUnavailable`, and maps all other aggregate-device, IO-proc, format, and device-start errors to `CaptureFailed`. Make the existing `stream_with_pause` wrapper delegate to `stream_with_pause_cause` and erase the type only for compatibility callers.

- [ ] **Step 5: Register managed state and commands**

In `lib.rs`, add exactly one:

```rust
.manage(audio::permissions::AudioPermissionCoordinator::default())
```

Register the four new commands and remove the three obsolete screen-recording/boolean commands plus the root `trigger_microphone_permission` wrapper. Do not add a startup request.

- [ ] **Step 6: Add request serialization and error-classification tests**

Use a coordinator test seam that accepts a closure returning `SystemAudioAccessState`. Start two async requests against a barrier-controlled closure, assert the closure's maximum concurrent count is one, and assert both callers see the stored final state. Add a table test for the exact settings mapping (`Microphone -> Privacy_Microphone`, `SystemAudio -> Privacy_ScreenCapture`). Add `core_audio.rs` tests proving only four-byte `perm` maps to `PermissionDenied` and another `os::Error` maps to `CaptureFailed`.

- [ ] **Step 7: Run Task 1 GREEN tests**

Run:

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::permissions::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::core_audio::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo check -p meetily --tests
```

Expected: all focused tests pass and `cargo check` exits 0.

- [ ] **Step 8: Commit Task 1**

```bash
git add frontend/src-tauri/src/audio/permissions.rs frontend/src-tauri/src/audio/capture/core_audio.rs frontend/src-tauri/src/lib.rs
git diff --cached --check
git commit -m "feat(macos): model native audio permissions"
```

---

### Task 2: One frontend permission owner and truthful onboarding

**Files:**
- Create: `frontend/src/lib/audioPermissions.ts`
- Create: `frontend/tests/lib/audio-permissions.test.ts`
- Modify: `frontend/src/hooks/usePermissionCheck.ts:1-86`
- Modify: `frontend/src/types/onboarding.ts:1-34`
- Modify: `frontend/src/contexts/OnboardingContext.tsx:38-67,102-108,662-667,689-717`
- Modify: `frontend/src/components/onboarding/steps/PermissionsStep.tsx:1-168`
- Modify: `frontend/src/components/onboarding/shared/PermissionRow.tsx:1-80`
- Modify: `frontend/src/components/PermissionWarning.tsx:1-140`
- Modify: `frontend/src/app/_components/TranscriptPanel.tsx:1-105`
- Modify: `frontend/src/app/page.tsx:1-45,220-249`
- Modify: `frontend/src/testing/tauri-browser-mocks.ts`
- Create: `frontend/tests/macos-permissions-lifecycle.spec.ts`

**Interfaces:**
- Consumes: Task 1 command names and camel-case `AudioPermissionSnapshot`.
- Produces:

```ts
export type MicrophonePermissionState =
  | 'not_determined' | 'authorized' | 'denied' | 'restricted' | 'unavailable';
export type SystemAudioAccessState =
  | 'unknown' | 'available' | 'denied' | 'unavailable';
export interface AudioPermissionSnapshot {
  microphone: MicrophonePermissionState;
  systemAudio: SystemAudioAccessState;
}
export const microphoneReady = (value: AudioPermissionSnapshot) =>
  value.microphone === 'authorized';
export const systemAudioReady = (value: AudioPermissionSnapshot) =>
  value.systemAudio === 'available';
```

- `usePermissionCheck` produces `snapshot`, `hasMicrophone`, `hasSystemAudio`, `isChecking`, `error`, `checkPermissions`, `requestMicrophone`, `requestSystemAudio`, and `openSettings(kind)`.

- [ ] **Step 1: Write failing pure permission tests**

Create `audio-permissions.test.ts` with table tests asserting:

```ts
expect(microphoneReady({ microphone: 'authorized', systemAudio: 'unknown' })).toBe(true);
expect(microphoneReady({ microphone: 'denied', systemAudio: 'available' })).toBe(false);
expect(systemAudioReady({ microphone: 'authorized', systemAudio: 'available' })).toBe(true);
expect(systemAudioReady({ microphone: 'authorized', systemAudio: 'unknown' })).toBe(false);
expect(permissionAction('microphone', 'restricted')).toEqual({ label: 'Open Settings', blocking: true });
expect(permissionAction('systemAudio', 'unavailable')).toEqual({ label: 'Try Again', blocking: false });
```

- [ ] **Step 2: Run the Vitest file and confirm RED**

Run from `frontend` using the already installed dependency tree from the clean integration worktree if the new worktree has no `node_modules`:

```bash
pnpm exec vitest run tests/lib/audio-permissions.test.ts
```

Expected: FAIL because `src/lib/audioPermissions.ts` does not exist.

- [ ] **Step 3: Implement the pure frontend contract**

Create the exact types and readiness functions above. Implement `permissionAction(kind, state)` as an exhaustive switch returning `{ label, blocking }`; microphone states other than `authorized` are blocking, while every system-audio state is nonblocking. Throw from the `never` branch so a new backend state cannot silently receive stale UI copy.

- [ ] **Step 4: Replace device-list inference with generation-safe command state**

In `usePermissionCheck.ts`, call only `get_audio_permission_snapshot` for passive checks. Protect overlapping refreshes with a monotonically increasing generation ref, matching the established recording-state pattern. Add one `window.focus` listener whose handler calls `checkPermissions`; remove it on unmount. Request functions invoke their specific request command and set the returned snapshot. They do not call `get_audio_devices` and do not use timeouts.

- [ ] **Step 5: Remove independent onboarding permission storage**

Delete `permissions`, `permissionsSkipped`, and `setPermissionStatus` from `OnboardingContext`. Remove `OnboardingPermissions` and the obsolete `screenRecording` field. Keep onboarding download/database state untouched.

- [ ] **Step 6: Make onboarding require only the microphone**

`PermissionsStep` consumes `usePermissionCheck`. The Finish button uses:

```ts
const canFinish = snapshot.microphone === 'authorized';
```

The system-audio row title becomes `System Audio (Optional)`. A denied/unavailable system row keeps an enabled retry/settings action but never disables Finish. Keep the existing reload after successful onboarding completion because the root layout owns the onboarding/main-app boundary; remove only copy that tells the user to restart after changing permission. The hook's `openSettings(kind)` invokes `open_audio_permission_settings` with `{ kind: 'microphone' }` for denied/restricted microphone states and `{ kind: 'systemAudio' }` for denied system audio.

- [ ] **Step 7: Correct warnings and page gating**

`PermissionWarning` receives the two typed states. Microphone denial/restriction is blocking. System audio uses `Audio Capture`, says recording will continue microphone-only, and never mentions BlackHole, virtual devices, Screen Recording, or restarting Meetily. `page.tsx` continues to show controls only when `hasMicrophone || isRecording`.

- [ ] **Step 8: Add deterministic browser scenarios and RED/GREEN acceptance**

Extend `tauri-browser-mocks.ts` with exact handlers for the four Task 1 commands and mutable per-scenario permission state. In `macos-permissions-lifecycle.spec.ts`, add tests named:

- `microphone authorization enables Finish while system audio remains optional`
- `returning from settings refreshes permission state without reload`
- `system audio denial shows microphone-only guidance and keeps recording controls available`

Assert command-call counts, the enabled Finish button, the absence of restart/Screen Recording/BlackHole copy, and that a synthetic `window.focus` changes the displayed microphone state.

- [ ] **Step 9: Run Task 2 GREEN tests**

Run:

```bash
pnpm exec vitest run tests/lib/audio-permissions.test.ts
pnpm exec playwright test tests/macos-permissions-lifecycle.spec.ts
pnpm typecheck
```

Expected: focused Vitest and Playwright tests pass; typecheck exits 0.

- [ ] **Step 10: Commit Task 2**

```bash
git add frontend/src/lib/audioPermissions.ts frontend/tests/lib/audio-permissions.test.ts frontend/src/hooks/usePermissionCheck.ts frontend/src/types/onboarding.ts frontend/src/contexts/OnboardingContext.tsx frontend/src/components/onboarding/steps/PermissionsStep.tsx frontend/src/components/onboarding/shared/PermissionRow.tsx frontend/src/components/PermissionWarning.tsx frontend/src/app/_components/TranscriptPanel.tsx frontend/src/app/page.tsx frontend/src/testing/tauri-browser-mocks.ts frontend/tests/macos-permissions-lifecycle.spec.ts
git diff --cached --check
git commit -m "feat(macos): make system audio optional in onboarding"
```

---

### Task 3: Degrade system-audio failures without ending microphone capture

**Files:**
- Modify: `frontend/src-tauri/src/audio/capture/backend.rs:14-336`
- Modify: `frontend/src-tauri/src/audio/capture/macos.rs:11-350`
- Modify: `frontend/src-tauri/src/audio/recording_session.rs:80-98,1245-1384`
- Test: inline Rust tests in the same three files

**Interfaces:**
- Consumes: Task 1 typed `CoreAudioSetupError` and `stream_with_pause_cause`.
- Produces:

```rust
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

pub trait CaptureBackend {
    fn pump(&mut self) -> Result<Option<CaptureRuntimeEvent>, CaptureCause>;
}
```

- `CaptureThreadHandle::take_runtime_event_receiver(&mut self) -> Option<Receiver<CaptureRuntimeEvent>>` returns a bounded receiver separate from terminal owner exit.

- [ ] **Step 1: Write failing owner/degradation tests**

Add backend fake modes and tests asserting:

```rust
assert_eq!(runtime_event_rx.recv_timeout(CAPTURE_REPLY_TIMEOUT).unwrap(), CaptureRuntimeEvent::SystemAudioDegraded {
    reason: "system stream stopped".into(),
});
assert!(handle.join.is_some());
```

Add a session test where the fake capture opens only `TrackKind::Microphone` despite `capture_system: true`; assert public start reaches `Recording`, `expected_tracks` contains only microphone, and the system writer namespace is absent.

- [ ] **Step 2: Run the focused tests and confirm RED**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::backend::tests::system_degradation_event_keeps_owner_running -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::system_start_failure_keeps_public_session_microphone_only -- --exact
```

Expected: compile failure because `CaptureRuntimeEvent` and its receiver do not exist.

- [ ] **Step 3: Add bounded runtime-event transport**

Change only the existing `CaptureBackend::pump` method from `Result<(), CaptureCause>` to `Result<Option<CaptureRuntimeEvent>, CaptureCause>`; keep every other trait method unchanged. Use `sync_channel(CAPTURE_CONTROL_MAX_MESSAGES)` for nonterminal events. The capture-owner loop sends an event returned by `pump()` with `try_send`; a full event channel must not block or terminate capture. Retain the returned event in one `pending_runtime_event` slot and retry it before the next pump. If another event arrives while the slot is occupied, preserve `MicrophoneSwitched` over `SystemAudioDegraded`, because route identity must reach the session snapshot. Add a saturation test proving the owner keeps running and eventually forwards the retained event. Keep the existing owner-exit channel solely for terminal failure.

- [ ] **Step 4: Make every system setup failure optional**

In `MacosCaptureBackend::start`, keep the microphone stream installed first. For every `CoreAudioSetupError` variant, log its typed reason, leave `system_stream` and `system_sinks` empty, store one `SystemAudioDegraded` event in a private `pending_start_event`, and return success with only `TrackKind::Microphone` opened. The first later `pump()` returns that pending event so the session can warn the user. Never convert a system permission denial into `CaptureCause::PermissionDenied` for the whole recording. Extend the focused start-failure test to receive this event while the capture owner remains alive.

- [ ] **Step 5: Isolate runtime system termination**

When `CoreAudioStream::should_terminate()` is true, drop `system_stream` and `system_sinks`, keep `TrackKind::System` in the historical opened/closed source set, and return `SystemAudioDegraded`. Later pumps continue processing the microphone. Pause, resume, and stop accept the missing current system stream and still acknowledge the originally opened source set.

- [ ] **Step 6: Run Task 3 GREEN tests**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::backend::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::macos::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::system_start_failure_keeps_public_session_microphone_only -- --exact
```

Expected: all focused tests pass and no recording owner exits on the nonterminal system event.

- [ ] **Step 7: Commit Task 3**

```bash
git add frontend/src-tauri/src/audio/capture/backend.rs frontend/src-tauri/src/audio/capture/macos.rs frontend/src-tauri/src/audio/recording_session.rs
git diff --cached --check
git commit -m "fix(audio): keep recording after system capture failure"
```

---

### Task 4: Automatically replace a disconnected microphone inside the capture owner

**Files:**
- Modify: `frontend/src-tauri/src/audio/capture/macos.rs:11-350`
- Modify: `frontend/src-tauri/src/audio/capture/backend.rs:338-900` tests

**Interfaces:**
- Consumes: Task 3 `CaptureRuntimeEvent::MicrophoneSwitched` and bounded runtime-event channel.
- Produces constants `MICROPHONE_RECOVERY_MAX_ATTEMPTS: u8 = 3` and `MICROPHONE_RECOVERY_WINDOW: Duration = Duration::from_secs(3)`.
- Produces private state `MicrophoneRecovery { previous_device, attempts, deadline, next_attempt }` owned only by `MacosCaptureBackend`.

- [ ] **Step 1: Write failing CPAL mapping and retry-policy tests**

Add tests named:

- `device_not_available_stream_error_starts_microphone_recovery`
- `microphone_recovery_rejects_the_same_device_name`
- `microphone_recovery_stops_after_three_attempts_or_three_seconds`
- `microphone_switch_event_preserves_the_capture_owner`

Use a pure `MicrophoneRecovery::may_attempt(now)` test with controlled `Instant` values. In the owner fake, emit `MicrophoneSwitched` from `pump`, read it from the runtime receiver, then send `CaptureControl::Stop` and assert the owner joins normally.

- [ ] **Step 2: Run focused tests and confirm RED**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::macos::tests::device_not_available_stream_error_starts_microphone_recovery -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::backend::tests::microphone_switch_event_preserves_the_capture_owner -- --exact
```

Expected: compile failure because the recovery state and event path are absent.

- [ ] **Step 3: Preserve microphone sinks and current route in the backend**

Derive `Clone` for `SourceSinks`. After successful start, store a clone in `MacosCaptureBackend`, plus the actual device name returned by CPAL. Do not write recording preferences. Map CPAL's exact `StreamError::DeviceNotAvailable` to `CaptureCause::DeviceUnavailable(current_name)`; keep other stream errors as `CaptureFailed`.

- [ ] **Step 4: Implement nonblocking bounded recovery**

On the first device-unavailable fault, drop the unusable stream and create recovery state. Each later `pump()` may make at most one attempt when `next_attempt <= now`, while continuing to drain system audio. Resolve `default_input_device`, require its name to differ from `previous_device`, build it against the existing sinks, start it, and install it. Space attempts at 0 ms, 500 ms, and 1500 ms; the hard deadline is 3 seconds.

On success, clear recovery and return:

```rust
CaptureRuntimeEvent::MicrophoneSwitched {
    previous_device,
    current_device,
    reason: "active microphone became unavailable".into(),
}
```

After the third failure or deadline, return `CaptureCause::DeviceUnavailable(previous_device)` so the existing terminal path finalizes recoverable data. There is no background polling thread and no unbounded retry.

- [ ] **Step 5: Verify stop and pause during recovery**

Add tests proving Stop cancels recovery and joins, and Pause during recovery never installs a playing replacement behind a paused session. When replacement succeeds while paused, create it paused and let Resume call `play()`.

- [ ] **Step 6: Run Task 4 GREEN tests**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::macos::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::capture::backend::tests -- --nocapture
```

Expected: all capture tests pass, including owner stop/join and bounded retry assertions.

- [ ] **Step 7: Commit Task 4**

```bash
git add frontend/src-tauri/src/audio/capture/macos.rs frontend/src-tauri/src/audio/capture/backend.rs
git diff --cached --check
git commit -m "feat(audio): switch disconnected microphones automatically"
```

---

### Task 5: Project route changes into the session and notify the user

**Files:**
- Modify: `frontend/src-tauri/src/audio/recording_session.rs:34-78,188-228,1361-1419`
- Modify: `frontend/src/services/recordingService.ts:1-131`
- Modify: `frontend/src/contexts/RecordingStateContext.tsx:1-155`
- Modify: `frontend/src/testing/tauri-browser-mocks.ts`
- Modify: `frontend/tests/macos-permissions-lifecycle.spec.ts`
- Test: inline Rust tests in `recording_session.rs`

**Interfaces:**
- Consumes: Task 3/4 `CaptureRuntimeEvent` receiver.
- Produces Tauri events:

```ts
export interface RecordingAudioRouteChanged {
  sessionId: string;
  previousDevice: string;
  currentDevice: string;
  reason: string;
}

export interface RecordingAudioDegraded {
  sessionId: string;
  source: 'system';
  reason: string;
}
```

- Event names are exactly `recording-audio-route-changed` and `recording-audio-degraded`.

- [ ] **Step 1: Write failing session projection tests**

Add an optional default method to the fake runtime seam that records runtime notifications. Write tests named:

- `microphone_switch_updates_active_snapshot_and_notifies_without_changing_preferences`
- `system_degradation_notifies_without_interrupting_the_session`
- `microphone_recovery_exhaustion_uses_recoverable_interruption`

For the switch test, assert `snapshot.microphone_device == Some("Built-in Microphone")`, state remains `Recording`, and the originally loaded preferred microphone remains unchanged in the fake runtime.

- [ ] **Step 2: Run the focused session tests and confirm RED**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::microphone_switch_updates_active_snapshot_and_notifies_without_changing_preferences -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::system_degradation_notifies_without_interrupting_the_session -- --exact
```

Expected: compile failure because runtime-event monitoring and notification do not exist.

- [ ] **Step 3: Monitor nonterminal capture events with session identity**

Add a default `capture_event_notifier` method to `RecordingSessionRuntime` returning `None`, so existing test runtimes remain source-compatible. The `AppHandle` implementation emits the two exact Tauri events. Start one monitor task per active capture runtime receiver. For `MicrophoneSwitched`, take the session store lock briefly, verify the active session ID, update only `snapshot.microphone_device`, persist the snapshot clone, release the lock, then notify. For system degradation, do not remove a previously opened expected track or alter session state.

- [ ] **Step 4: Add typed frontend listeners and exact copy**

In `recordingService.ts`, add `onAudioRouteChanged` and `onAudioDegraded`. Export pure message functions with exact results:

```ts
export const routeChangeMessage = (event: RecordingAudioRouteChanged) =>
  `Microphone disconnected. Recording continues on ${event.currentDevice}.`;

export const degradationMessage = () =>
  'System audio stopped. Recording continues with the microphone.';
```

Do not include the previous device in analytics or persistent preferences; it is available only to the local event/UI.

- [ ] **Step 5: Show eight-second notifications from the recording state provider**

Register both listeners beside the existing start/stop/pause/resume listeners. Use `toast.warning(message, { duration: 8000 })`. On route change, refresh the matching snapshot so the UI projects the new current microphone. Dispose both listeners on unmount.

- [ ] **Step 6: Add browser acceptance for both notifications**

Extend the browser mock event bus with deterministic helpers for the two events. In the Playwright spec, install `page.clock` before navigation, emit the route event after the app loads, call `page.clock.fastForward(7_999)`, and assert the route toast still contains the replacement name. Also assert the selected saved preference in mock state is unchanged. Assert the system-degraded toast says microphone capture continues and recording controls remain active.

- [ ] **Step 7: Run Task 5 GREEN tests**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::microphone_switch_updates_active_snapshot_and_notifies_without_changing_preferences -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::system_degradation_notifies_without_interrupting_the_session -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::microphone_recovery_exhaustion_uses_recoverable_interruption -- --exact
pnpm exec playwright test tests/macos-permissions-lifecycle.spec.ts
pnpm typecheck
```

Expected: all focused Rust and browser tests pass; typecheck exits 0.

- [ ] **Step 8: Commit Task 5**

```bash
git add frontend/src-tauri/src/audio/recording_session.rs frontend/src/services/recordingService.ts frontend/src/contexts/RecordingStateContext.tsx frontend/src/testing/tauri-browser-mocks.ts frontend/tests/macos-permissions-lifecycle.spec.ts
git diff --cached --check
git commit -m "feat(audio): notify microphone fallback and degradation"
```

---

### Task 6: Stop/start regression and structural policy

**Files:**
- Modify: `frontend/src-tauri/src/audio/recording_session.rs` tests
- Modify: `frontend/src-tauri/src/audio/capture/backend.rs` tests
- Modify: `frontend/scripts/release-policy.mjs:1385-1455`
- Modify: `frontend/tests/macos-audio-surface.test.mjs:39-67,142-159,223-229`
- Modify: `frontend/tests/release-policy.test.mjs`

**Interfaces:**
- Consumes: completed Task 1-5 contracts.
- Produces policy findings `audio.permission-device-inference`, `audio.unbounded-runtime-event`, `audio.missing-microphone-recovery-bound`, and `audio.system-failure-blocks-recording`.

- [ ] **Step 1: Write the failing stop/start regression**

Use a fake backend factory that records owner and stream identities. Start session A, stop and join it, start session B, then assert:

```rust
assert_ne!(first_owner_id, second_owner_id);
assert_eq!(first_drop_count.load(Ordering::Acquire), 1);
assert_eq!(second_snapshot.state, RecordingSessionState::Recording);
assert!(second_snapshot.error.is_none());
```

Name the test `stop_then_start_uses_fresh_owner_streams_and_permission_decisions`.

- [ ] **Step 2: Run the regression and confirm RED**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::stop_then_start_uses_fresh_owner_streams_and_permission_decisions -- --exact
```

Expected: FAIL until the fake records distinct owner/stream identities and the lifecycle releases the first owner before returning.

- [ ] **Step 3: Make the minimum lifecycle correction**

If the test exposes a stale handle, move only the necessary drop/join before the stop return. Do not refactor the finalization pipeline. Keep this invariant explicit: `stop_and_join_capture` must acknowledge Stop, call `join_once`, and drop the handle before a new start can pass the action gate.

- [ ] **Step 4: Add structural policy checks and mutation tests**

Extend `realSurface()` with `audio/permissions.rs` and `usePermissionCheck.ts`. The validator must report:

- `audio.permission-device-inference` if the shared permission hook contains `get_audio_devices`;
- `audio.unbounded-runtime-event` unless runtime events use `sync_channel(CAPTURE_CONTROL_MAX_MESSAGES)`;
- `audio.missing-microphone-recovery-bound` unless exact constants `3` attempts and `Duration::from_secs(3)` are present;
- `audio.system-failure-blocks-recording` if `CoreAudioSetupError::PermissionDenied` returns whole-capture `CaptureCause::PermissionDenied` from the macOS adapter.

Add one mutation test per finding to `release-policy.test.mjs`, plus a real-tree PASS assertion in `macos-audio-surface.test.mjs`. Update the old source assertion that system termination must surface `CaptureFailed`; it must now require `SystemAudioDegraded` while microphone capture remains.

- [ ] **Step 5: Run Task 6 GREEN tests**

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily audio::recording_session::tests::stop_then_start_uses_fresh_owner_streams_and_permission_decisions -- --exact
pnpm test:node
pnpm check:policy
```

Expected: the focused Rust test passes, Node tests pass, and policy prints PASS with no findings.

- [ ] **Step 6: Commit Task 6**

```bash
git add frontend/src-tauri/src/audio/recording_session.rs frontend/src-tauri/src/audio/capture/backend.rs frontend/scripts/release-policy.mjs frontend/tests/macos-audio-surface.test.mjs frontend/tests/release-policy.test.mjs
git diff --cached --check
git commit -m "test(audio): guard permission and device recovery lifecycle"
```

---

### Task 7: Full local verification and physical TCC gate

**Files:**
- Verify only; do not change product files unless a new regression is found.

**Interfaces:**
- Consumes: complete branch diff from `b7126ea82be887c38e8a8e8de30a48f9bb409e51`.
- Produces: exact automated evidence, packaged-app evidence, and separately labeled physical/owner evidence.

- [ ] **Step 1: Confirm scope and clean history**

```bash
git status --short --branch
git log --oneline --decorate b7126ea82be887c38e8a8e8de30a48f9bb409e51..HEAD
git diff --name-status b7126ea82be887c38e8a8e8de30a48f9bb409e51..HEAD
git diff --check b7126ea82be887c38e8a8e8de30a48f9bb409e51..HEAD
```

Expected: only the approved design, plan, and Task 1-6 files; no unstaged or untracked product changes.

- [ ] **Step 2: Run all frontend gates**

From `frontend`:

```bash
pnpm typecheck
pnpm test:node
pnpm test:unit
pnpm exec playwright test
pnpm build
pnpm check:policy
```

Expected: all commands exit 0. Then run `pnpm lint` separately. Accept only the exact four base diagnostics in `RecordingControls.tsx` and `useRecordingStateSync.ts`; any changed diagnostic stops execution.

- [ ] **Step 3: Run all Rust gates**

From `frontend/src-tauri`:

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo fmt --all -- --check
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p llama-helper
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::permissions::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::capture::backend::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::capture::macos::tests -- --nocapture
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::recording_session::tests::system_start_failure_keeps_public_session_microphone_only -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::recording_session::tests::microphone_switch_updates_active_snapshot_and_notifies_without_changing_preferences -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::recording_session::tests::system_degradation_notifies_without_interrupting_the_session -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::recording_session::tests::microphone_recovery_exhaustion_uses_recoverable_interruption -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test -p meetily --lib audio::recording_session::tests::stop_then_start_uses_fresh_owner_streams_and_permission_decisions -- --exact
CARGO_TARGET_DIR=../../target/fork-051 cargo test --workspace
```

Expected: focused tests pass. The workspace command may contain only the exact four base `audio::recording_session` failures and three existing ignored hardware tests. Run strict Clippy separately and accept only the exact ten base findings; any new or different finding stops execution:

```bash
CARGO_TARGET_DIR=../../target/fork-051 cargo clippy --workspace --all-targets --all-features -- -D warnings
```

- [ ] **Step 4: Build and inspect the packaged application**

```bash
CARGO_TARGET_DIR=../target/fork-051 pnpm tauri build --bundles app
plutil -p ../target/fork-051/release/bundle/macos/meetily.app/Contents/Info.plist
codesign -d --entitlements :- ../target/fork-051/release/bundle/macos/meetily.app
codesign --verify --deep --strict ../target/fork-051/release/bundle/macos/meetily.app
```

Expected: build and verification pass; bundled plist contains nonempty `NSMicrophoneUsageDescription` and `NSAudioCaptureUsageDescription`; entitlements contain `com.apple.security.device.audio-input = true`. Record ad hoc identity as local build evidence, not Developer ID evidence.

- [ ] **Step 5: Stop for explicit physical TCC authorization**

Report automated results and ask permission before any `tccutil reset Microphone com.meetily.ai` or Audio Capture reset. Do not infer authorization from the general implementation request.

- [ ] **Step 6: If authorized, execute and record the physical matrix**

On the physical Mac, verify each row separately: first microphone prompt; deny; return from Settings without restart; grant; first Audio Capture prompt; deny with microphone-only start; stop/start; disconnect selected microphone; automatic default fallback plus eight-second notification; disconnect all microphones; recoverable interruption. Record observed macOS version, app bundle identity, expected result, actual result, and PASS/FAIL. Never label this automated.

- [ ] **Step 7: If physical authorization is withheld, keep FORK-051 open**

Mark physical TCC rows `NOT_RUN (authorization not granted)` and do not close the canonical task. Continue only after the user explicitly authorizes or explicitly changes the task's acceptance boundary.

---

### Task 8: Independent review, integration, and canonical closeout

**Files:**
- Modify after product integration: `/Users/developer/Desktop/meeting-minutes/.worktrees/macos-fork-foundation/docs/prd/macos-fork.md`

**Interfaces:**
- Consumes: clean feature branch, complete automated evidence, and physical TCC PASS evidence.
- Produces: independent SPEC and QUALITY verdicts with Critical 0 / Important 0, fast-forward integration, and a separate docs-only FORK-051 closeout commit.

- [ ] **Step 1: Freeze review identity**

```bash
git rev-parse HEAD
git rev-parse HEAD^{tree}
git diff --binary b7126ea82be887c38e8a8e8de30a48f9bb409e51..HEAD | shasum -a 256
git status --porcelain=v2 --branch
```

Expected: clean branch and one stable branch-diff hash.

- [ ] **Step 2: Request independent SPEC and QUALITY reviews**

Give each reviewer the exact base, tip, tree, diff hash, approved design, implementation plan, physical matrix, test evidence, and known baseline exceptions. Require explicit Critical/Important/Minor counts. Any Critical or Important finding returns to the failing task, adds a focused regression first, and reruns every affected gate plus final identity.

- [ ] **Step 3: Fast-forward the clean integration worktree**

Verify `/Users/developer/Desktop/meeting-minutes/.worktrees/macos-fork-integration` is clean and still at exact expected ancestor. Then run only:

```bash
git merge --ff-only fork/fork-051-permissions-lifecycle
```

Do not push. Re-run the focused permission, capture, session, browser, and policy tests from the integration worktree.

- [ ] **Step 4: Close FORK-051 in the canonical PRD**

From the clean foundation worktree, run:

```bash
mdtask view FORK-051 --path docs/prd/macos-fork.md
```

Change only FORK-051's checkbox/status and append factual implementation evidence: product commits, integration tip, exact branch-diff hash, test counts, review verdicts, physical TCC matrix, and explicit non-publication boundaries. Preserve FORK-061 and every later task byte-for-byte.

- [ ] **Step 5: Validate and commit the docs-only closeout**

```bash
mdtask validate --path docs/prd/macos-fork.md
bash scripts/check-fork-migration-registry.sh validate
git diff --check
git add docs/prd/macos-fork.md
git diff --cached --check
git commit -m "docs(macos): close FORK-051 permission lifecycle"
```

Expected: validators pass; commit changes only `docs/prd/macos-fork.md`; foundation worktree returns clean. Do not push or publish.
