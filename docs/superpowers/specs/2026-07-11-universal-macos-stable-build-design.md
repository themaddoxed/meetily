# Universal macOS Stable Build Design

Date: 2026-07-11
Status: approved for implementation planning

## Summary

Build one reliable ad-hoc macOS artifact for both Apple Silicon and Intel from the stable `v0.4.0` codebase. Integrate only current features that can be isolated, verified, and supported on the target machines. Experimental branches remain out of the first build rather than being merged behind unfinished UI.

## Target machines

- Mac mini 2018, Intel Core i5, 16 GB RAM, macOS 15.7.7.
- MacBook Pro 14-inch, M4, 16 GB RAM, 512 GB storage.
- MacBook Pro 14-inch, M1 Pro, 32 GB RAM, 512 GB storage.

The deliverable is one Universal macOS `.app` and, when bundling is stable, one Universal `.dmg`. The build uses ad-hoc signing for private testing and is not intended for public distribution or automatic updates.

## Stable baseline

Create the integration branch from `origin/main` at tag `v0.4.0`. Do not use the dirty local `main` checkout as an input. Use feature branches as patch sources and architectural references, not as merge targets.

`codex-port-v042` is not the baseline because it combines finished theme and Gemma work with an incomplete diarization foundation. The integration branch must contain only explicitly selected changes.

## Included scope

### Theme

- Light, dark, and system modes.
- Theme application before first paint.
- Semantic color tokens and migrated application surfaces.
- Persisted user preference with a safe fallback when storage is unavailable.
- Browser acceptance tests and a static theme-token audit.

Theme work must not alter recording, transcription, summary, database, or model-management behavior.

### Gemma 4 built-in summaries

- Keep the existing `builtin-ai` provider and model-management workflow.
- Add the verified Gemma 4 2B and 4B model entries and prompt support.
- Recommend the 2B model on 16 GB machines and the 4B model on the 32 GB machine.
- Keep Gemma 3 models as lower-resource and compatibility fallbacks.
- Preserve existing cloud, Ollama, and custom provider behavior.

### Reliability fixes

- Bundle a known FFmpeg binary instead of downloading it at runtime.
- Release microphone and recording resources when recording stops.
- Preserve provider endpoint, API key, and selected model across settings and provider switches.
- Avoid AVX-512-only assumptions on Intel hardware.

Each fix must be ported independently from its source branch and verified against the current `v0.4.0` implementation. Whole historical branch heads must not be merged.

### Crash recovery

- Detect interrupted meetings after relaunch.
- Recover persisted transcript segments.
- Recover audio from valid checkpoints when available.
- Allow recovery without audio when only transcript data survived.
- Avoid duplicate recovery dialogs and duplicate meeting records.

### Large transcript performance

- Load transcript records in bounded pages.
- Virtualize rendering for long meetings.
- Preserve full-copy behavior and correct ordering.
- Preserve navigation to a timestamp and avoid forced auto-scroll on the details page.

## Excluded scope

- Speaker diarization and its settings, database migration, or review UI.
- LFM Python sidecar and Liquid Audio dependencies.
- OTA updater and release signing/notarization.
- Experimental Whisper/Nut Whisper streaming pipeline.
- Queue mode that disables live transcription.
- Windows and Linux packaging changes.
- Unrelated refactors or UI redesigns.

Excluded features can be evaluated in later branches after the stable Universal build passes on all three target machines.

## Architecture

The integration remains a single Tauri application with the existing Next.js frontend and Rust command layer. New work is divided into bounded slices that can be built and tested independently:

1. Universal build and bundled-binary validation.
2. Theme foundation and UI migration.
3. Gemma 4 model registry and llama-helper compatibility.
4. Isolated reliability fixes.
5. Recovery flow.
6. Paginated transcript API and virtualized frontend.

No slice may depend on a later slice. A failing slice is fixed or reverted before the next slice begins.

## Universal binary strategy

Use Tauri's `universal-apple-darwin` target. Install both Rust targets and build the application for both architectures before bundling:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

Every executable bundled inside the app must also support both architectures. Validate the application binary, `llama-helper`, and FFmpeg with `lipo -info`. If a dependency cannot be made Universal, bundle architecture-specific copies and select the correct executable at runtime; do not ship an artifact that launches only on one architecture.

Use ad-hoc signing after all binaries are assembled. Verify the final bundle with `codesign --verify --deep --strict` and test first launch through Finder on each target machine.

## Data and control flow

### Summary generation

The frontend saves a built-in model selection through the existing settings path. Rust resolves the selected registry entry and starts `llama-helper`. The helper loads the matching GGUF and returns the generated summary through the existing command/event contract. Missing or corrupt models return a user-facing recovery action and must not crash the app.

### Crash recovery

During recording, transcript and audio checkpoint persistence remains incremental. On relaunch, the recovery coordinator finds incomplete meetings, validates available artifacts, and offers one recovery action. Recovery writes through the existing meeting repository and removes temporary recovery state only after the recovered meeting is durable.

### Transcript pagination

The details page requests metadata separately from transcript pages. The Rust repository returns ordered pages and total-count information. The frontend appends de-duplicated pages and renders them through the existing virtualized transcript component.

## Error handling

- Unsupported or wrong-architecture sidecars fail during build validation, not at user runtime.
- Gemma download, validation, and load errors retain the existing model and expose retry/delete actions.
- Failure to recover audio does not discard a recoverable transcript.
- Pagination failures retain already loaded segments and expose a retry path.
- Theme storage failures fall back to system mode without blocking startup.
- Recording resource cleanup is idempotent so tray and UI stop paths cannot double-release or deadlock.

## Integration order and commit policy

Integrate one coherent slice per commit:

1. Universal build foundation.
2. Theme.
3. Gemma 4.
4. FFmpeg and CPU compatibility.
5. Recording-resource cleanup and settings persistence.
6. Crash recovery.
7. Transcript pagination and virtualization.
8. Release-candidate fixes found by hardware testing.

Commits should contain only files required by their slice. Historical feature branches with shared ancestry must be mined for unique patches instead of merged wholesale.

## Verification gates

### Per-slice automated gates

- Frontend lint and production build.
- Rust formatting and tests.
- Relevant unit or browser tests for the slice.
- Tauri development launch on the build machine when the slice changes runtime behavior.

### Universal artifact gates

- Both Rust targets installed.
- Universal Tauri application build succeeds.
- `lipo -info` reports `x86_64` and `arm64` for every bundled executable.
- Ad-hoc signature verifies after bundling.
- The app launches without Rosetta on Apple Silicon.
- The same artifact launches natively on the Intel Mac.

### Hardware acceptance matrix

Run the same checklist on all three machines:

1. Fresh install and first launch.
2. Microphone and screen-recording permission flow.
3. Five-minute microphone-only recording.
4. Five-minute microphone and system-audio recording.
5. Stop from the UI and verify the microphone is released.
6. Live transcription and saved transcript playback.
7. Import and retranscribe an existing audio file.
8. Download the recommended Gemma model and generate a summary.
9. Force-quit during recording and recover the meeting after relaunch.
10. Open, scroll, search, copy, and summarize a long transcript.
11. Switch light, dark, and system themes and relaunch.

The build is accepted only when all core checks pass on Intel, M4, and M1 Pro. Machine-specific performance differences are acceptable; crashes, missing binaries, data loss, stuck recording state, and architecture fallbacks are not.

## Success criteria

- One ad-hoc Universal artifact runs natively on all three target Macs.
- Core v0.4.0 recording and transcription behavior remains intact.
- Every included feature has an automated regression gate and a hardware acceptance check.
- Excluded experiments are absent from the UI and runtime initialization path.
- No branch is merged solely to preserve code; every integrated line supports the agreed build.
