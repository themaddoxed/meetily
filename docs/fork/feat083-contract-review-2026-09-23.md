# FEAT-083: mute/gain and indicator contract revalidation

Date: 2026-09-23
Result: source-contract review is accurate after the corrections below; native audio behavior remains `NOT VERIFIED`.

## Prior FEAT-071 snapshot and document-history references

- Product: `.worktrees/feat071-audio-controls`, branch `themaddoxed/feat071-audio-controls`, HEAD `61e890f444f81718d8136beae6afc6f61a5f0e44`, tree `e95835dae2888dae6cba1652a1a9c72faa73dfdd`, dirty diff-vs-HEAD SHA-256 `9c835e454df467b4be8b92d1c05c7706e26a6fcaf9578f5192bde5f4b4a7d3b0`.
- The dirty product tree contains the FEAT-082 privacy removal plus unrelated summary/testing edits. The audio-control paths reviewed below were compared with HEAD; the privacy-only changes to `RecordingControls.tsx`, `DeviceSelection.tsx`, `lib.rs`, and `recording-controls.test.mjs` remove analytics calls/mocks and do not change mute/gain routing.
- Planning worktree: `.worktrees/task-decomposition-publish`, branch `themaddoxed/task-decomposition-plan`, HEAD `605021c229de15f144c7d8033533f4abf9fea605`. Its whole-worktree dirty diff SHA-256 before the FEAT-083/FORK-119 document corrections was `2045a6827256731901f1a97a052552777ea4be2c4b5eea11852bf7c5d189bb0e`; that value is historical and does not fingerprint the post-correction planning tree. The reviewer handoff binds the exact current report and task-document file hashes.
- Historical report `feat083-contract-review-2026-09-22.md` is retained as written (SHA-256 before this revalidation: `d2f9111393c238605796b4ed6cdb9bb23a650eab6df342ab6be177255c3e3907`). Its `incremental_saver.rs` hash was wrong even at the cited commit, and its indicator and final-track-location descriptions are superseded by this revalidation.

## Contract cross-checked against the combined candidate

| Area | Current source behavior | Evidence |
|---|---|---|
| Initial values and UI range | Microphone and system begin `muted=false`, `gain=1.0`. UI gain range is `0..2` in `0.05` steps; backend accepts finite gain `0..4`. | `RecordingControls.tsx`; `capture/backend.rs` |
| Control delivery | Mute sends immediately. Gain is sent on slider blur. The command reaches `RecordingSession::set_source_control` and waits for the capture owner response. | `RecordingControls.tsx`; `recordingService.ts`; `recording_commands.rs`; `recording_session.rs` |
| Signal path | Mute zeros the downstream copy; otherwise gain is applied and samples are clamped to `[-1,1]`. macOS capture retains a separate raw copy before source controls. | `capture/backend.rs`; `capture/macos.rs` |
| Saved tracks | Checkpoints and `track.json` use `.tracks/microphone` and `.tracks/system`. Each captured nonempty source finalizes to a root-level raw file (`microphone.mp4` or `system.mp4`); `system.mp4` is absent when the optional system source produces no samples. Recovery uses the same root paths. The controlled downstream signal feeds the mixed `audio.mp4`. | `incremental_saver.rs`; `recording_session.rs` |
| Recognition | Controlled chunks that are accepted by the downstream queue feed the shared pipeline and transcription sender. Delivery is best effort: after raw enqueue, a failed downstream `try_send` is intentionally ignored. This is a source-flow inference, not a native observation. | `capture/backend.rs`; `recording_session.rs`; `pipeline.rs` |
| Indicators | `DeviceSelection` contains an `audio-levels` listener and meter rendering, but its meter-start button is commented out. The registered Rust command calls `simple_level_monitor`, which emits synthetic sine-wave values; it does not read device input or post-control capture. The meter is therefore not evidence of real input or recorder output. | `DeviceSelection.tsx`; `AudioLevelMeter.tsx`; `lib.rs`; `simple_level_monitor.rs` |

## Gaps and limits

- The source-level gain maximum differs between UI (`2`) and backend validation (`4`); the UI remains inside the accepted range. The displayed UI control state is optimistic, and the effective source values are not stored in `RecordingSessionSnapshot` or finalized track metadata.
- `audio-levels` is synthetic in the currently wired command path. The source does not establish a functioning live device-input meter or a post-mute/gain capture indicator.
- FEAT-084–086 still need native evidence for actual system-track contents, signal separation, playback, transcription effects, raw-track reprocessing, and pause/resume timing. No source inspection or passing unit test proves those behaviors on the packaged app.
- No app was launched and no native recording was made in this revalidation.

## Current file hashes

These hashes describe the earlier FEAT-071 worktree at HEAD `61e890f444f81718d8136beae6afc6f61a5f0e44`; they are retained as historical review evidence and do not identify the combined candidate.

| Path | SHA-256 |
|---|---|
| `frontend/src/components/RecordingControls.tsx` | `759740e5ffefe5acd4f5fe997e69b2052c770323298efe16ae12aded9b48e8b4` |
| `frontend/src/services/recordingService.ts` | `ae69c78cce9216bf7b1679a01443abfa69fd429c66f00fbb4f54d87fff8bbe22` |
| `frontend/src-tauri/src/audio/recording_commands.rs` | `465c711df2e7888e7d9fce5a20d0e59cc80c4627b19db3ff8ee685b7233df210` |
| `frontend/src-tauri/src/audio/recording_session.rs` | `7b1f66a2de0c2b7a9bd88c7a51973fd453a223968237058aa1539d7ee3a9e296` |
| `frontend/src-tauri/src/audio/capture/backend.rs` | `251aefcf5370083076c452fd7bf70d38d72e2ab0336205c60eb10b5ebe8ff3a8` |
| `frontend/src-tauri/src/audio/capture/macos.rs` | `d200e4fdf6c08bf519c8c32d2d911d4884aa2386aade92047c6229825c7d2c0d` |
| `frontend/src-tauri/src/audio/incremental_saver.rs` | `2748945595a7ba2160442c4b93a636bf5797011827b36c6db41e4419e0d00b3d` |
| `frontend/src-tauri/src/audio/level_monitor.rs` | `7a341f7d887d23f08d91c3b52a50ef31e68d91f4b318a95d8a2e8f81393060f4` |
| `frontend/src-tauri/src/audio/simple_level_monitor.rs` | `340b7d9ddbd26933f51f3944b3081c29018c387a18b9815767a63fde79bcd659` |
| `frontend/src-tauri/src/audio/pipeline.rs` | `7e8a0f3c0cb94c4a62bf6ae3395d22604ee937c6559f73ea6e8f9feffc5f5150` |
| `frontend/src-tauri/src/lib.rs` | `085076e349686a5d43d4d385a0f9ba4d4f0c5a8797cbea44f7d8a02b78fc90ae` |
| `frontend/src/components/DeviceSelection.tsx` | `f1d762e2842e59a0171365fe304812e87cb0ea6b13aae212e19fa8d798da737a` |
| `frontend/src/components/AudioLevelMeter.tsx` | `72b77f365e0cc6844ea41e53a7fe6546da11fd1701a98ba6dbcfa863b6be90cc` |
| `frontend/tests/recording-controls.test.mjs` | `d077ef85793d71d6267a8b051c956ff63c0a958b12830bb00211bfadc6048e4d` |

## Combined candidate revalidation — 2026-09-23

The independent source revalidation examined `.worktrees/feat071-fork051-integration-local`, detached HEAD `61e890f444f81718d8136beae6afc6f61a5f0e44`, MERGE_HEAD `f35d864370cbdafd088447c16f78b24e3f07c06a`, and the complete dirty diff SHA-256 `0609ae4c707c6d4e6df899a1cecfcf9c1bbc0b98d5f656e22d51e474c782df07`. The diff was unchanged at review exit. It confirmed the contract above and identified two qualifications now included in this report: downstream queue delivery can fail after raw persistence, and the optional system track may have no finalized `system.mp4` when it produced no samples.

Current source hashes for the 13 reviewed audio-contract files, relative to that combined candidate worktree:

| Path | SHA-256 |
|---|---|
| `frontend/src/components/RecordingControls.tsx` | `759740e5ffefe5acd4f5fe997e69b2052c770323298efe16ae12aded9b48e8b4` |
| `frontend/src/services/recordingService.ts` | `ae69c78cce9216bf7b1679a01443abfa69fd429c66f00fbb4f54d87fff8bbe22` |
| `frontend/src-tauri/src/audio/recording_commands.rs` | `465c711df2e7888e7d9fce5a20d0e59cc80c4627b19db3ff8ee685b7233df210` |
| `frontend/src-tauri/src/audio/recording_session.rs` | `5451fc9f84e13744facf456555740a80d400c291f2ffda4fe557c6f5b8605dbc` |
| `frontend/src-tauri/src/audio/capture/backend.rs` | `79a152038627f964139c166de896e72ba1b1e4d0b64f8d2cabec8ba55fe33c82` |
| `frontend/src-tauri/src/audio/capture/macos.rs` | `87f4f8e978f1374ff8c4852ac24589c8bb16176375fffe690cd64b5f78e84ac9` |
| `frontend/src-tauri/src/audio/incremental_saver.rs` | `7fcdef272ff8b0d0b6b182920fd34a9c8aa30d610997439a2aa55d41537915dd` |
| `frontend/src-tauri/src/audio/level_monitor.rs` | `7a341f7d887d23f08d91c3b52a50ef31e68d91f4b318a95d8a2e8f81393060f4` |
| `frontend/src-tauri/src/audio/simple_level_monitor.rs` | `340b7d9ddbd26933f51f3944b3081c29018c387a18b9815767a63fde79bcd659` |
| `frontend/src-tauri/src/audio/pipeline.rs` | `7e8a0f3c0cb94c4a62bf6ae3395d22604ee937c6559f73ea6e8f9feffc5f5150` |
| `frontend/src/components/DeviceSelection.tsx` | `f1d762e2842e59a0171365fe304812e87cb0ea6b13aae212e19fa8d798da737a` |
| `frontend/src/components/AudioLevelMeter.tsx` | `72b77f365e0cc6844ea41e53a7fe6546da11fd1701a98ba6dbcfa863b6be90cc` |
| `frontend/src-tauri/src/lib.rs` | `085076e349686a5d43d4d385a0f9ba4d4f0c5a8797cbea44f7d8a02b78fc90ae` |

The exact combined `.app` was opened through macOS Launch Services (`open -a` exited `0`); process `84731` was then confirmed running from that candidate bundle. This establishes process-level launch only. No UI state or microphone/system capture was verified. The build/artifact identity and remaining FORK-119 blockers are recorded in [the FORK-119 report](fork119-artifact-selection-2026-09-23.md).

## Checks and review

- Independent source review of the prior report returned three documentation findings: synthetic rather than device-input levels; final raw-track location; and the `incremental_saver.rs` hash mismatch. The reviewed files and exact snapshot are listed above. Privacy edits did not alter the mute/gain behavior.
- The earlier FEAT-071 snapshot ran `pnpm --dir frontend test:node` (406 passed), `pnpm --dir frontend typecheck`, and `cargo check --manifest-path frontend/src-tauri/Cargo.toml --offline`; those results are bound to its `9c835e...` diff, not to combined candidate diff `0609ae...`.
- The combined candidate is bound to source diff SHA-256 `0609ae4c707c6d4e6df899a1cecfcf9c1bbc0b98d5f656e22d51e474c782df07`. On that worktree, focused automated checks passed: Node audio/lifecycle `31/31`; Rust `audio::capture::macos::tests` `28/28`, `audio::capture::backend::tests` `25/25`, and `audio::recording_session::tests` `50 passed, 1 ignored`; analytics startup tests `9/9`. Rust checks ran offline with Xcode 26.3 selected by `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`; the default developer directory remains Command Line Tools. These unit/source tests use controlled seams and do not prove physical microphone/system samples, permissions, UI behavior, playback, or transcription. The separate `.app` launch did not exercise audio capture.
- The source hash for `incremental_saver.rs` is `27489455…` both on disk and at HEAD `61e890f`; the old report's `de90b59e…` value is not the reviewed source and is not accepted as evidence.

## Status

FEAT-083 is a completed source-contract review after this correction. The combined `.app` process was launched, but that is not evidence of a working indicator, microphone/system samples, playback, transcription outcomes, or audio behavior. FEAT-084–086 remain open and native acceptance is `NOT VERIFIED`.
