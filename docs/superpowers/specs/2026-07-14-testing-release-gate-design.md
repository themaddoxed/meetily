# Meetily Testing System and Release Gate Design

**Date:** 2026-07-14
**Target branch:** `dev`
**Implementation branch:** `feature/testing-release-gate`
**Status:** Approved design, pending implementation

## Purpose

Meetily needs a release process that protects the meeting recording and transcript first. A successful compilation alone is not release evidence. The system described here gives a non-QA specialist a small set of commands, records what was actually checked, and refuses to call a release ready when required automatic or manual evidence is missing.

The primary invariant is:

> A failure in transcription, diarization, summary generation, model loading, export, or the interface must not delete or corrupt meeting data that was already persisted.

## Repository Truth and Scope

The supported product in `dev` is the Tauri 2 desktop application:

- Next.js 14 and React 18 render a statically exported frontend in `frontend/out`.
- Rust in `frontend/src-tauri` owns desktop commands, SQLite persistence, audio capture, STT, diarization, local summary engines, and bundling.
- SQLite migrations live in `frontend/src-tauri/migrations` and are applied through `sqlx::migrate!`.
- Browser IndexedDB and Rust audio checkpoints provide complementary interrupted-recording recovery paths.
- Whisper and Parakeet are local STT providers; local summary execution uses the packaged helper or an explicitly configured provider.
- The top-level Python/FastAPI `backend` is archived and is not part of the supported production application or release gate.
- Universal macOS packaging already has an offline-capable build path and verifies its application, FFmpeg, and helper Mach-O architectures.

The design does not claim that speaker editing, file export, real audio capture, or every local model path is complete merely because related modules or screenshots exist. A missing product capability is a failed or not-run gate item, never a mocked pass.

## Approaches Considered

### Selected: layered gate

A fast daily layer, a complete pre-build layer, and a macOS release layer share the same underlying checks. This keeps common feedback reasonably fast without weakening release evidence.

### Rejected: one monolithic command for every edit

Always building a Universal macOS application would be simple to explain but too slow and hardware-specific for daily use. Developers would be encouraged to skip it.

### Rejected: wrap only the existing tests

Existing tests are valuable but are not aggregated and do not establish a release order, migration matrix, production configuration policy, or manual hardware evidence.

## User-Facing Commands

The implementation will expose these repository-root commands:

```bash
./scripts/check.sh --quick
./scripts/check.sh
./scripts/release-candidate.sh
./scripts/release-gate.sh --smoke-report path/to/report.md
```

`check.sh --quick` is the everyday command. It omits only the expensive production frontend build and packaged application build.

`check.sh` is the single pre-build command. It runs repository policy validation, frontend formatting/lint/type checks and tests, a production static export, Rust formatting/lint/tests, SQLite migration tests, and Tauri production configuration checks. It stops on the first failed stage and prints the exact standalone command for diagnosis.

`release-candidate.sh` first runs the complete pre-build check and then builds a Universal macOS candidate using the existing offline artifact process. It verifies the candidate instead of treating the build command's exit code as sufficient proof.

The release-candidate command refuses a dirty source worktree, records the exact commit SHA, and keeps generated output outside the tracked source tree. This makes the candidate reproducible from a named clean revision rather than from unreported local edits.

`release-gate.sh` verifies automatic results, the candidate artifact, and a completed manual smoke report. It must not report `PASS` when required manual items are `FAIL`, `NOT RUN`, or absent.

No command will turn an unavailable check into an unconditional success. Platform-specific skips must be explicit in output and must remain required in the release smoke report.

## Gate Layers

### Layer 1: deterministic repository checks

- changed-source formatting;
- frontend lint;
- TypeScript `--noEmit` checking;
- frontend unit and contract tests;
- Next.js production static export;
- `cargo fmt --check`;
- `cargo clippy` with warnings denied for release-owned code;
- Rust unit and integration tests;
- real temporary-SQLite migration and persistence tests;
- parse and validate Tauri JSON configuration;
- reject production E2E/test bootstrap;
- reject tracked private keys, obvious credential literals, developer-home absolute paths, and unsafe production settings;
- verify required icons, templates, sidecar declarations, and build scripts exist.

The scanner is intentionally narrow and deterministic. It detects obvious accidental disclosures and configuration mistakes; it is not described as a complete security audit.

### Layer 2: production candidate verification

- require a clean tracked source worktree and record its commit SHA;
- build the static frontend and Tauri release application without `devUrl` at runtime;
- provision sidecars from the documented local artifact store, not from developer source directories;
- verify `meetily`, `llama-helper`, and `ffmpeg` are Mach-O Universal binaries with both arm64 and x86_64 slices;
- inspect the application bundle for required resources and metadata;
- verify product name, version, bundle identifier, icons, minimum macOS version, and hardened-runtime configuration;
- scan the bundle for known repository paths, developer home paths, test bootstrap markers, and obvious secrets;
- verify ad-hoc or Developer ID code signatures according to the selected build mode;
- run notarization and Gatekeeper checks only when a notarized artifact and credentials are actually available, otherwise report them as required manual/release-environment checks.

### Layer 3: manual macOS and hardware smoke

A versioned report template records operator name, date, commit, candidate checksum, Mac model, macOS version, installation state, audio devices, model selections, network state, and a result for every step. This is evidence for one specific artifact, not a reusable checkbox.

## Critical Product Path

| Stage | Input | Durable output | Representative failures | Data that must survive | Existing evidence | Evidence to add |
|---|---|---|---|---|---|---|
| Audio capture | macOS permissions, microphone and system device samples | incremental audio checkpoints and recording metadata | denied permission, missing/disconnected device, capture interruption, process crash | prior checkpoints and recoverable meeting identity | Rust incremental saver tests and recording modules | temp-directory checkpoint round-trip, corrupt/missing checkpoint isolation, manual permission/device smoke |
| Audio processing | asynchronous microphone/system buffers | mixed/resampled chunks for recording and STT | sample-rate mismatch, empty buffer, FFmpeg merge error | raw or previous checkpoints | Rust mixer/VAD tests and Universal FFmpeg validation | gate aggregation and failure diagnostics |
| Transcription | speech chunks and loaded local STT model | transcript rows/chunks plus recovery snapshot | missing model, provider error, empty result, worker stop | audio checkpoints, meeting row, every prior transcript | provider/worker tests and IndexedDB recovery | persisted-data invariant test around STT failure and empty transcript |
| Intermediate save | meeting id, transcript updates, audio checkpoints | SQLite/IndexedDB state marked with recovery status | database locked, partial write, app termination | last committed transcript and usable checkpoints | IndexedDB recovery Playwright tests | real SQLite checkpoint/reopen tests and crash-recovery contract |
| Finish meeting | active recording state | finalized recording and completed meeting metadata | finalize/merge error, app termination during stop | intermediate meeting, transcript, unmerged checkpoint files | recording stop/recovery code | failure-order tests and manual forced-quit smoke |
| Permanent storage | finalized metadata and transcripts | migrated SQLite database | fresh DB failure, incompatible legacy schema, repeated migration, corrupt row | existing database unchanged or recoverable backup | real diarization migration tests | full fresh/idempotent/previous-schema matrix and meeting round-trip with a new pool |
| Diarization | persisted transcript timings and local diarization result | speaker segments and transcript speaker fields | model failure, invalid payload, overlapping/partial updates | original transcript and previous speaker data | real atomic diarization integration tests | explicit rollback/preservation assertions and speaker rename/reopen test |
| Summary | persisted transcript and selected local/configured model | summary process/result | missing model, LLM timeout/error, malformed output | transcript always; previous summary on regeneration failure | summary backup/restore implementation | real SQLite failure-preservation integration test and empty-transcript contract |
| Export | reopened meeting, transcript, speakers, summary | user-selected export artifact | unsupported format/path, denied filesystem access, serialization error | source meeting and transcript | markdown conversion fallback tests; user-facing capability must be confirmed | serialization integrity test if a real path exists; otherwise release blocker and manual `FAIL` |
| Restart/reopen | application data directory after clean or forced exit | same meeting/transcript/summary visible after relaunch | corrupt unfinished meeting, stale recovery state | all healthy meetings; corrupt item isolated | browser recovery tests and SQLite repositories | new-connection DB test and manual packaged-app relaunch smoke |

## Persistence and Failure-Test Design

Persistence tests use a real temporary SQLite file and the production migration set. In-memory fakes are not acceptable for migration, transaction, restart, or durability assertions.

The migration matrix includes:

1. empty file to current schema;
2. all migrations applied a second time;
3. representative pre-diarization schema to current;
4. representative previous current-product schema to current;
5. preservation of meeting, transcript, speaker, and summary data during upgrade;
6. a malformed or unsupported legacy state failing without silently deleting user tables.

Critical failure tests establish state before injecting a failure, run the real repository transaction, reopen the same SQLite file through a new pool, and compare persisted state afterward. Mocks may stand in for heavy STT/diarization/LLM inference only at an existing service boundary; they may not replace SQLite, migrations, transaction code, filesystem checkpoints, or export serialization.

When a production defect is found, the implementation follows red-green-refactor: first add a failing regression test, then make the smallest production fix, then rerun the focused and aggregate gates.

## Crash Recovery Rules

- Recording must not rely exclusively on the normal Finish action.
- Audio checkpoints are written atomically or remain individually recoverable.
- Transcript recovery state is updated during recording, not only at finalization.
- Recovery enumeration must isolate malformed or incomplete meetings so one item cannot prevent application startup.
- Finalization may delete checkpoints only after permanent storage is confirmed.
- Recovery tests must distinguish a process crash from the currently active browser session.
- No storage format is changed without a migration and a preservation test.

If audit evidence shows these rules are not currently met, the implementation may add a narrowly scoped fix guarded by a failing test. It must not replace production storage with a test-only path.

## CI and Release Ordering

A new CI workflow runs on pull requests and pushes to `dev` and `main`:

1. frontend/config/security checks on a fast runner;
2. Rust, SQLite, and macOS-specific tests on a macOS runner;
3. production candidate build on macOS where required artifacts are provisioned;
4. a final required job depending on every preceding job.

Jobs receive only the permissions they need. Build artifacts are uploaded for inspection; no release is published by CI.

The existing manual release workflow is reordered:

1. validate explicit manual-smoke attestation;
2. run or verify the automatic gate for the exact commit;
3. build signed platform artifacts;
4. create/update a draft release only after the gate passes;
5. require human review before publication.

`needs` dependencies enforce order. A failed or skipped required job prevents draft creation. Signing and notarization secrets remain GitHub secrets and are never added to the repository.

## Offline and Privacy Validation

- Automatic tests run local fixtures and do not upload meeting data.
- The gate rejects unexpected production endpoints and documents the explicitly user-selected external providers already present in the product.
- The packaged app is smoke-tested with networking disabled for local recording, local STT, reopen, and local summary where the required model is installed.
- Missing STT or LLM models must produce a user-readable error without modifying persisted transcript data.
- Analytics tests continue to verify that raw identifying browser data is not serialized.

## Manual Smoke Scenario

The required packaged-app scenario is:

1. install or launch the exact candidate artifact;
2. grant or verify microphone and screen/audio permissions;
3. select real microphone and system-audio sources;
4. create a uniquely named meeting;
5. record speech and system audio long enough to create checkpoints;
6. observe a real-time transcript;
7. force-quit once and verify recovery;
8. resume or create a second meeting and finish normally;
9. verify permanent save and speaker operations;
10. run diarization and local summary, including one missing-model/error case;
11. export through the actual product UI and inspect the artifact;
12. disable networking and repeat local reopen/summary checks;
13. restart the packaged app and reopen the meeting;
14. verify older healthy meetings still open;
15. attach logs, report, and artifact checksum.

Any unavailable product action is recorded as `FAIL`, not silently skipped. Clean-install permissions and upgrade-from-previous-version are separate required smoke variants because they cannot be inferred from a developer-machine launch.

## Diagnostics and Operator Experience

`docs/testing.md` will explain each command in plain language, expected duration, success output, and first troubleshooting action. It will include:

- where frontend, Rust, application, recovery, and build logs live;
- how to preserve the entire diagnostics folder without exposing secrets;
- how to rerun one failing test;
- how to check disk space, Xcode selection, models, permissions, and sidecars;
- what to attach to a bug: commit, app version, artifact checksum, macOS/hardware, exact step, expected/actual result, logs, and whether data was recovered;
- defects that block release.

Release-blocking defects include data loss/corruption, failed migration, unrecoverable interrupted recording, inability to reopen a saved meeting, privacy regression, unsigned/unnotarized artifact when signing is required, missing bundled resources, or any required gate item not run.

## Required Project Policy

The branch will add a short `AGENTS.md` release section (or a direct link from it) that says an agent must:

- run the automatic checks and production build before claiming release readiness;
- report exact commands and factual outcomes;
- list manual/macOS/hardware items separately;
- never claim clean-install permissions, audio capture, offline model execution, signing, or notarization without direct evidence;
- never bypass a failure without explaining its impact and leaving the release blocked.

## Acceptance Evidence

The testing-system goal is complete only when repository state proves all of the following:

- one documented pre-build command exists and actually runs;
- critical real-storage and recovery tests exist and pass, or a discovered production defect remains an explicit release blocker;
- migration tests cover fresh, repeated, and previous-schema upgrades;
- summary/diarization/STT failure tests prove persisted transcript preservation at the strongest available boundary;
- a production candidate is built from the isolated feature worktree and verified;
- CI covers pull requests and primary development/release branches;
- the release workflow cannot create a draft before its automated gate;
- the manual smoke template and parser enforce complete evidence;
- documentation is understandable without QA terminology;
- the final report separates passed, failed, not-run, and hardware-only checks;
- the release is not described as ready unless every mandatory gate is passed.
