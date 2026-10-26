# FEAT-083: mute/gain and indicator contract review

Date: 2026-09-22
Result: source-contract review complete; native audio behavior remains `NOT VERIFIED`.

## Snapshots

- Product source: `.worktrees/feat071-audio-controls`, branch `themaddoxed/feat071-audio-controls`, commit `61e890f444f81718d8136beae6afc6f61a5f0e44`, tree `e95835dae2888dae6cba1652a1a9c72faa73dfdd`.
- Product worktree has unrelated dirty summary/testing changes. The audio-control source paths reviewed here are clean at that commit.
- Task document: `docs/prd/fork-features.md`, SHA-256 `f21acf5c490a47f6f6b2e80e55cd393c5615c81aed7d21ab146a5a86037329c6` in the dirty planning worktree.
- No product code changed and no tests or app were run for this review.

## Contract found in source

| Area | Implemented source behavior | Evidence |
|---|---|---|
| Initial control values | Microphone and system both default to `muted=false`, `gain=1.0`. The UI exposes gain `0..2` in steps of `0.05`; backend validates finite gain `0..4`. | `frontend/src/components/RecordingControls.tsx:47-50,206-224`; `frontend/src-tauri/src/audio/capture/backend.rs:223-249` |
| Control delivery | Mute invokes the Tauri command immediately. Gain is sent on slider blur. The command reaches `RecordingSession::set_source_control`, which sends the update to the capture owner and waits for its result. | `frontend/src/components/RecordingControls.tsx:165-233`; `frontend/src/services/recordingService.ts:12-23,110-115`; `frontend/src-tauri/src/audio/recording_commands.rs:138-145`; `frontend/src-tauri/src/audio/recording_session.rs:1788-1850` |
| Signal processing | Backend replaces downstream samples with zero while muted; otherwise it multiplies by gain and clamps to `[-1,1]`. macOS capture keeps a separate raw copy before this control. | `frontend/src-tauri/src/audio/capture/backend.rs:237-258,653-678,857-875`; `frontend/src-tauri/src/audio/capture/macos.rs:367-424` |
| Stored audio | When auto-save is enabled, source checkpoints/final files are stored separately under `.tracks/microphone` and `.tracks/system`. Downstream chunks feed the shared pipeline; the session identifies `audio.mp4` as the mixed playback artifact. | `frontend/src-tauri/src/audio/recording_session.rs:450-461,471-535,704-725,1287-1345,1360-1510,2320-2385`; `frontend/src-tauri/src/audio/incremental_saver.rs:126-130,381-490` |
| Recognition | The controlled downstream chunks feed the shared audio pipeline and transcription sender. This is a source-flow inference; recognition output was not observed on a native recording. | `frontend/src-tauri/src/audio/recording_session.rs:704-725,1287-1345` |
| Indicators | `DeviceSelection` listens to `audio-levels`, emitted by the separate preview monitor. It is not connected to `SourceControl` and does not report post-mute/gain recorder output. | `frontend/src/components/DeviceSelection.tsx:80-107,270-300`; `frontend/src/components/AudioLevelMeter.tsx:3-105`; `frontend/src-tauri/src/audio/level_monitor.rs:24-117,320-355`; `frontend/src-tauri/src/lib.rs:200-224` |

## Gaps and limits

- The UI slider is optimistic, and the UI/backend gain maxima differ (`2` versus `4`). The current UI remains within backend validation, but the effective setting is not projected back from a recording snapshot.
- `RecordingSessionSnapshot` and finalized track metadata do not retain the effective per-source mute/gain values, so later UI/recovery cannot display them as accepted session state.
- The meter is device-input preview, not an authoritative indicator for captured post-control audio.
- Existing unit/browser tests cover portions of the source contract, but were not run in this review and do not prove native capture. Actual system-track contents, signal separation, playback audibility, transcript effects, and pause/resume alignment remain open for FEAT-084–086.
- `set_recording_source_control` was left unchanged.

## Source file hashes

- `frontend/src/components/RecordingControls.tsx` — `50572f56584f680789441c8dce01289ba16a01a9eff98a77c87e77c262941cd2`
- `frontend/src/services/recordingService.ts` — `ae69c78cce9216bf7b1679a01443abfa69fd429c66f00fbb4f54d87fff8bbe22`
- `frontend/src-tauri/src/audio/recording_commands.rs` — `465c711df2e7888e7d9fce5a20d0e59cc80c4627b19db3ff8ee685b7233df210`
- `frontend/src-tauri/src/audio/recording_session.rs` — `7b1f66a2de0c2b7a9bd88c7a51973fd453a223968237058aa1539d7ee3a9e296`
- `frontend/src-tauri/src/audio/capture/backend.rs` — `251aefcf5370083076c452fd7bf70d38d72e2ab0336205c60eb10b5ebe8ff3a8`
- `frontend/src-tauri/src/audio/capture/macos.rs` — `d200e4fdf6c08bf519c8c32d2d911d4884aa2386aade92047c6229825c7d2c0d`
- `frontend/src-tauri/src/audio/incremental_saver.rs` — `de90b59e9ebee94bbf7b4c2796c45ad2855ff7b7eb2f12cdae769734f0da80ba`
- `frontend/src-tauri/src/audio/level_monitor.rs` — `7a341f7d887d23f08d91c3b52a50ef31e68d91f4b318a95d8a2e8f81393060f4`
- `frontend/tests/recording-controls.test.mjs` — `4e0f5cf0c5f4bdde2fc5e285dc1512e69a6fc1bce28b2ceaa5c33e47704c4ffd`
