# Universal Post-call Diarization Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Implement task-by-task with red-green-refactor and verify every commit.

**Goal:** Run real local post-call speaker diarization from Meeting Details on Intel and Apple Silicon and atomically apply final labels.

**Architecture:** A small coordinator validates meeting/audio/model state, runs a replaceable `DiarizationRuntime` off the command thread, converts engine intervals to the existing validated apply contract, and emits bounded progress. The production runtime uses a pinned sherpa-onnx release and pinned offline models; tests use both a deterministic fake and a generated multi-voice macOS fixture.

**Tech Stack:** Rust, Tokio, Tauri 2, SQLx/SQLite, sherpa-onnx, Symphonia/FFmpeg, Node artifact scripts, React 18.

---

## Task 1: Bounded sherpa-onnx architecture spike

**Files:**
- Modify: `frontend/src-tauri/Cargo.toml`
- Modify: `Cargo.lock`
- Create: `frontend/src-tauri/src/diarization/sherpa_spike.rs` (temporary, deleted before task commit)
- Create: `artifacts/diarization-spike/README.md` (generated evidence, not committed unless repository policy permits)

- [ ] Add a temporary compile-only call to the official offline speaker-diarization Rust API using candidate `sherpa-onnx = "=1.13.2"` with static-link configuration.
- [ ] Run native `cargo check -p meetily --tests` and both target checks: `cargo check -p meetily --target x86_64-apple-darwin` and `cargo check -p meetily --target aarch64-apple-darwin`.
- [ ] Inspect dependency tree for duplicate incompatible ONNX Runtime/native symbols with `cargo tree -p meetily | rg 'sherpa|onnx|ort'`.
- [ ] If `1.13.2` cannot coexist with Meetily's pinned `ort`, test exactly one fallback integration: the sherpa C API as a separately linked native library. Do not write an inference algorithm from scratch.
- [ ] Record the selected API, crate/native version, link mode, architectures, and exact failure evidence for the rejected path in the runtime spec.
- [ ] Remove the temporary spike module. Keep only the dependency choice that passes both architecture checks.
- [ ] Commit `build(diarization): pin universal sherpa runtime`.

## Task 2: Deterministic model artifact manifest and licenses

**Files:**
- Create: `frontend/scripts/diarization-artifacts.js`
- Modify: `frontend/scripts/provision-universal-artifacts.js`
- Modify: `frontend/src-tauri/tauri.conf.json`
- Modify: `frontend/src-tauri/tauri.universal.conf.json`
- Create: `frontend/src-tauri/resources/diarization/NOTICE.md`
- Create: `frontend/tests/diarization-artifacts.test.mjs`
- Modify: `frontend/scripts/release-policy.mjs`

- [ ] Add RED Node tests requiring exact model URLs, SHA-256, expected size, license identifier/source, resource destinations, and both native architecture artifacts where applicable.
- [ ] Run `cd frontend && node --test tests/diarization-artifacts.test.mjs`; expect missing-manifest failure.
- [ ] Pin the pyannote segmentation and 3D-Speaker embedding artifacts. Verify upstream license files directly and record required attribution in NOTICE.
- [ ] Extend the existing local Universal artifact store `v1` manifest/seed flow; production build must fail on missing or mismatched assets and must not download them implicitly.
- [ ] Declare only the verified model/native resources in Tauri configs and extend release policy/candidate inspection to require them.
- [ ] Run artifact tests and `pnpm check:policy`; expect pass with the local store present and an explicit failure message when a copied fixture checksum is changed.
- [ ] Commit `build(diarization): provision pinned offline models`.

## Task 3: Runtime boundary and audio decoding

**Files:**
- Create: `frontend/src-tauri/src/diarization/runtime.rs`
- Create: `frontend/src-tauri/src/diarization/audio.rs`
- Modify: `frontend/src-tauri/src/diarization/mod.rs`
- Create: `frontend/src-tauri/tests/diarization_runtime.rs`

- [ ] Add RED unit tests for model checksum rejection, unreadable/unsupported/empty audio, non-finite engine intervals, negative/reversed intervals, stable neutral labels/colors, overlap normalization, and threshold clustering without a supplied participant count.
- [ ] Run `cd frontend/src-tauri && cargo test -p meetily --test diarization_runtime -- --nocapture`; expect missing runtime modules.
- [ ] Define `DiarizationRuntime` and a minimal owned `EngineSegment` DTO. Implement 16 kHz mono Float32 decode/resample by reusing existing Symphonia/resampling helpers; never modify the source recording.
- [ ] Implement `SherpaDiarizationRuntime` behind the trait. Resolve resources through Tauri/application paths passed into the runtime, verify SHA-256 before creating sessions, and use threshold clustering.
- [ ] Normalize output into valid `SpeakerSegment` values with stable generated ids/labels/colors and generic overlap.
- [ ] Re-run focused tests, Rust formatting, and `cargo check -p meetily --tests`; expect pass.
- [ ] Commit `feat(diarization): add verified offline inference runtime`.

## Task 4: Coordinator and failure-preserving application

**Files:**
- Create: `frontend/src-tauri/src/diarization/coordinator.rs`
- Modify: `frontend/src-tauri/src/diarization/commands.rs`
- Modify: `frontend/src-tauri/src/diarization/mod.rs`
- Modify: `frontend/src-tauri/src/database/repositories/meeting.rs`
- Modify: `frontend/src-tauri/tests/diarization_runtime.rs`

- [ ] Add RED real-SQLite tests for missing meeting/folder/audio, active or incomplete recording, disabled/off settings, duplicate in-flight run, no speech, engine failure, malformed output, apply failure, success, and rerun replacement. Reopen the pool and compare previous transcript/speaker data after every failure.
- [ ] Run the focused integration test; expect missing coordinator API.
- [ ] Add one repository query that resolves the completed meeting and supported recording candidate (`audio.mp4` first, documented WAV fallback) without inventing paths outside persisted `folder_path`.
- [ ] Add an in-memory per-meeting run registry with RAII cleanup so panic/error cannot leave a permanent busy state.
- [ ] Run inference inside `tauri::async_runtime::spawn_blocking`; validate all output before calling the existing atomic apply helper with explicit `PostCall` kind.
- [ ] Return stable codes for missing recording, model invalid, insufficient speech, already running, inference failed, and persistence failed. Log internal causes locally.
- [ ] Re-run coordinator/runtime/apply tests; expect pass.
- [ ] Commit `feat(diarization): orchestrate post-call speaker detection`.

## Task 5: Visible launch action and progress

**Files:**
- Modify: `frontend/src-tauri/src/lib.rs`
- Create: `frontend/src/hooks/meeting-details/useDiarizationRun.ts`
- Modify: `frontend/src/app/meeting-details/page-content.tsx`
- Modify: `frontend/src/components/MeetingDetails/TranscriptPanel.tsx`
- Modify: `frontend/src/components/MeetingDetails/TranscriptButtonGroup.tsx`
- Create: `frontend/tests/diarization-action-contract.test.mjs`

- [ ] Add a RED source-contract test for visible `Определить спикеров`/localized label, disabled recording/empty/busy states, one command invocation, progress/busy feedback, success refetch, and distinct error copy.
- [ ] Run the contract test; expect failure because the action is absent.
- [ ] Register `run_meeting_diarization` and bounded progress events. Do not register a production fake or direct engine-output injection command.
- [ ] Implement an event-handler-driven hook. Keep run state scoped to meeting id, unsubscribe from Tauri events on change/unmount, and ignore stale completions per React 18 cleanup guidance.
- [ ] Thread the action into the existing transcript button group and disable duplicate launch while keeping Copy/Recording/Export behavior independent.
- [ ] Re-run focused frontend tests, `pnpm lint`, and `pnpm typecheck`; expect pass.
- [ ] Commit `feat(diarization): expose post-call speaker detection`.

## Task 6: Production inference fixture and architecture qualification

**Files:**
- Create: `scripts/generate-diarization-fixture.sh`
- Modify: `frontend/src-tauri/tests/diarization_runtime.rs`
- Modify: `frontend/tests/universal-build-config.spec.ts`
- Modify: `frontend/scripts/verify-release-candidate.mjs`
- Modify: `docs/testing.md`

- [ ] Generate a local fixture on macOS with two built-in `say` voices, alternating separated utterances, and FFmpeg normalization to 16 kHz mono. Record generation commands; do not commit personal recordings.
- [ ] Run the production sherpa runtime on the fixture without supplying speaker count; require at least two ordered clusters and valid coverage, not exact labels.
- [ ] Run the focused native test on Intel. Run the same frozen source/artifact test on Apple Silicon or leave the architecture row blocked; never infer one architecture from the other.
- [ ] Extend candidate verification to inspect every added native library/resource and reject a missing slice or checksum mismatch.
- [ ] Run `./scripts/check.sh --full`, both target builds, and Universal candidate verification.
- [ ] Document model location, offline behavior, first troubleshooting step, and expected runtime.
- [ ] Commit `test(diarization): qualify universal offline runtime`.

## Task 7: Diarization checkpoint

- [ ] Run all diarization type/alignment/migration/apply/rename/runtime tests.
- [ ] Run frontend action/label tests, lint, and typecheck.
- [ ] Run `git diff --check` and inspect that no test-only runtime command is registered.
- [ ] Run full repository check from a clean local commit.
- [ ] Record Intel/Apple Silicon evidence separately; do not claim Universal runtime PASS until both exist.
