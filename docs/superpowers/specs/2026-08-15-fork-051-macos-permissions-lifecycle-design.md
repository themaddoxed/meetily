# FORK-051 macOS permissions and microphone lifecycle design

**Date:** 2026-08-15
**Task:** FORK-051
**Product base:** `b7126ea82be887c38e8a8e8de30a48f9bb409e51` (`fork/macos-integration`)
**Status:** Approved by the user; implementation not started

## Goal

Make first-run audio permissions and the recording device lifecycle truthful and recoverable on macOS. A recording must require a working microphone, treat system audio as optional, survive a microphone disconnect by switching to an available default microphone, and preserve recoverable recording data when no microphone remains.

## User decisions

- Microphone access is required to start a recording.
- System-audio access is optional. When it is denied or unavailable, Meetily starts microphone-only recording and shows a warning.
- When the active microphone disconnects, Meetily automatically switches the current recording to an available default microphone and notifies the user.
- An automatic fallback applies only to the current recording. It does not overwrite the user's saved microphone preference.
- If no replacement microphone can be opened, Meetily ends the active capture safely, retains completed checkpoints, and exposes the session as recoverable.

## Current problems

- `usePermissionCheck` infers permission from the device list, which confuses device availability with macOS authorization.
- The existing system-audio check returns success after constructing a tap even though its own comments acknowledge that this does not prove capture authorization.
- Onboarding stores permission state only after a button click and does not refresh it after the user returns from System Settings.
- Onboarding requires both permissions even though the main UI says microphone-only recording is supported.
- A Core Audio permission denial currently fails the whole start request, while another system-audio setup failure falls back to microphone-only.
- `AudioDeviceMonitor` is not connected to the current recording owner. Runtime microphone stream errors currently terminate the owner instead of attempting a route change.

## Platform contract

### Microphone permission

Use the already pinned `cidre` AVFoundation bindings to query `AVCaptureDevice.authorizationStatus(for: .audio)` and request access only after an explicit user action. Map the native result into these product states:

- `not_determined`
- `authorized`
- `denied`
- `restricted`
- `unavailable` for a platform or API failure that is not an authorization decision

Do not infer microphone permission from device enumeration or from a CPAL stream attempt. A real recording start still opens the CPAL stream and remains the final operational check.

### System-audio permission

Core Audio does not provide the same AVFoundation status query for process taps. Do not infer authorization from output-device enumeration, tap construction alone, or whether the current machine happens to be producing non-silent audio.

An explicit system-audio request must create and start the short-lived aggregate-device capture path that the product uses. Use a separate system-audio state because this is an operational probe rather than an AVFoundation authorization query:

- `unknown` before a probe has produced a trustworthy operational result in the current process;
- `available` when the real capture path starts successfully without a permission error;
- `denied` when the Core Audio result is classified as a permission denial;
- `unavailable` for unsupported devices or other setup failures.

`available` means that the production capture path opened; it is not a claim that Meetily read the private TCC database. Silence alone must never change the state to `denied`, because legitimate system output may be silent. Physical TCC verification must confirm how the supported macOS version reports an actual denial. If denial produces a successful but silent stream with no public status or error, retain `unknown` and show honest guidance instead of claiming authorization.

The first production recording performs the same operational check again. A system-audio denial or unavailable device degrades that recording to microphone-only.

### Command surface

Expose one serializable snapshot rather than unrelated booleans:

```text
AudioPermissionSnapshot {
  microphone: MicrophonePermissionState,
  systemAudio: SystemAudioAccessState,
}
```

Use narrow commands to read the snapshot, request microphone permission, request system-audio permission, and open the matching System Settings pane. Request commands are single-flight so repeated button clicks cannot create concurrent prompts or probe streams.

No permission prompt runs automatically during application setup or a passive permission refresh.

## First-run and focus lifecycle

- Onboarding reads the backend snapshot on mount.
- The main window refreshes the snapshot whenever it regains focus, covering a return from System Settings.
- A refresh never opens a prompt.
- The onboarding action button requests only the selected permission and then reloads the snapshot.
- Microphone authorization enables completion. System audio is labeled optional and never blocks completion.
- If the user skips onboarding, recording start still enforces the microphone requirement and reports the typed denial.
- The shared permission hook is the only frontend owner of live permission state; onboarding and the warning component consume that contract instead of maintaining independent guesses.

## Recording start behavior

1. Resolve the selected or preferred microphone under the existing explicit-versus-default fallback rules.
2. Open and start the microphone stream. A permission failure maps to `permission_denied`; no usable device maps to `device_unavailable`.
3. If system audio was requested, open and start it.
4. A system-audio permission denial, unavailable device, or unsupported capture capability removes the system source for this session, discards its unused writer namespace, and emits a warning. It does not roll back the microphone stream.
5. Only sources that actually opened become expected recording tracks.

## Automatic microphone switching

The capture-owner thread remains the sole owner of CPAL streams. Add a bounded replace-microphone control to that owner rather than starting a second recording session or mutating capture from an async task.

When CPAL reports that the active input device is unavailable:

1. Classify the stream error as a microphone-device failure instead of a generic capture failure.
2. Resolve the current default input and reject the same unavailable device.
3. Build the replacement stream against the existing microphone sinks.
4. Pause the old stream, start the replacement, then replace and drop the old handle. If replacement startup fails while the old stream remains usable, resume the old stream.
5. Retry only within a short bounded window to tolerate a transient Bluetooth route change. There is no unbounded reconnection loop.
6. On success, keep the same session, writers, meeting folder, transcript pipeline, and recovery namespace.

The recording session receives a typed route-change result and emits a frontend event containing the session ID, previous device name, new device name, and reason. The UI shows a toast for at least eight seconds stating that recording is continuing on the new microphone. The saved preference is unchanged.

If the bounded fallback cannot open another microphone, the existing unexpected-owner finalization path moves the session to `interrupted`, joins all writers, and retains verified checkpoints for the existing recovery flow.

## System-audio failure during recording

A runtime failure isolated to system audio must stop and drop only the system stream, keep microphone capture active, and emit a degraded-recording warning. The system track remains part of the session only if it was opened and may contain valid partial audio. Microphone failure follows the automatic-switch contract above.

## Stop and restart

- Stop waits for the capture owner to acknowledge shutdown and join before returning.
- All microphone and system stream handles are dropped before a later start can open devices.
- Permission probes use their own short-lived streams and release them before returning.
- Repeated `start -> stop -> start` must not reuse a stale stream, owner, fallback decision, or permission result.

## UI behavior

- Replace boolean permission labels with the backend states and distinguish denial, restriction, and device unavailability.
- Remove instructions that require a virtual audio device or Screen Recording when the product uses Core Audio Audio Capture.
- Do not tell the user to restart Meetily after granting permission; focus refresh handles the normal return path.
- Show microphone denial as blocking.
- Show system-audio denial or failure as a non-blocking microphone-only warning.
- Show automatic microphone replacement with both device names.
- Show the existing recoverable-interruption action only when no replacement microphone can be opened.

## Verification design

### Automated

- Native microphone-status mapping for every AVFoundation authorization state.
- Single-flight request behavior and passive refresh without prompting.
- System-audio request classifications without using silence as an authorization signal.
- First-run onboarding state, focus refresh, microphone-required completion, and optional system audio.
- Start with system permission denied or system device unavailable continues microphone-only.
- Capture-owner replacement keeps the same sinks and session and drops the previous stream.
- Transient replacement retry is bounded.
- Replacement success emits the route-change payload and does not mutate saved preferences.
- No replacement moves the session to recoverable `interrupted` with verified partial tracks.
- Runtime system-audio failure keeps microphone capture active.
- Consecutive `start -> stop -> start` creates and releases distinct streams cleanly.

### macOS integration evidence

- Rust unit and integration tests, frontend unit tests, type checking, lint baseline comparison, strict Clippy baseline comparison, workspace tests, policy validation, and a Tauri build.
- A packaged-app smoke check verifies the bundled usage descriptions and audio-input entitlement.

### Physical TCC evidence

The following changes real macOS privacy state and therefore requires separate explicit authorization immediately before execution:

- reset Microphone and Audio Capture TCC entries for Meetily;
- verify first prompt, denial, return-from-Settings refresh, grant, stop/start, microphone disconnect/fallback notification, and no-device recovery on a physical Mac.

Automated evidence and owner-observed physical evidence must be reported separately. The task must not claim physical TCC coverage if this step is not authorized or not run.

## Non-goals

- Persisting an automatically selected fallback as the user's preference.
- Seamlessly moving historical samples between different physical clocks beyond the existing resampling pipeline.
- Changing transcription models, storage format, database behavior, release workflows, signing, notarization, or publication.
- Broad refactoring of legacy audio helpers that are outside the active recording path.
- Adding a new dependency when the pinned `cidre`, CPAL, Tauri, and existing event system cover the contract.

## Acceptance criteria

- Permission state comes from macOS authorization or an actual capture attempt, never device-list inference.
- Microphone is required; system audio is optional at onboarding, start, and runtime.
- Returning from System Settings refreshes state without an application restart.
- A disconnected microphone switches automatically to a usable default and visibly notifies the user without changing their saved preference.
- Failure to find a replacement preserves recoverable session data.
- Stop/start releases old streams and starts cleanly.
- New focused tests pass, and repository-wide gates introduce no new failures beyond explicitly recorded baseline-equivalent failures.
- No push, pull request, release, signing, notarization, publication, or TCC reset occurs without its own authorization.

## References

- Apple, “Requesting Authorization for Media Capture on macOS”: <https://developer.apple.com/documentation/bundleresources/requesting-authorization-for-media-capture-on-macos>
- Apple, “Capturing system audio with Core Audio taps”: <https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps>
- Tauri 2 state and window-event guidance: <https://v2.tauri.app/develop/state-management/>
- Pinned libraries: `tauri 2.6.2`, `cpal 0.15.3`, `cidre 0.11.3` at revision `a9587fa1d4ff6f0d5d082849ae7c62880fd739f7`
