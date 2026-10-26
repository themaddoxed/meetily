# Universal Post-call Diarization Runtime Design

Date: 2026-07-26
Target: Meetily 0.4.1 Alpha release candidate
Status: Approved through the user's instruction to choose recommended options and proceed

## Purpose

Add the missing real producer and visible launch action above the existing validated diarization persistence foundation. For this alpha, the supported runtime is an explicit local post-call pass over a completed recording on both Intel and Apple Silicon macOS.

This is a release-focused delta to `2026-06-26-speaker-diarization-design.md`. It does not claim the broader live/provisional, separate-stem, merge, or split roadmap is complete.

## Constraints

- The current product ships a Universal macOS application and must not make Intel a decorative architecture slice.
- Diarization remains local and does not upload meeting audio.
- The user does not supply a participant count; no visible speaker cap is introduced.
- A failed run must preserve transcript text, timing, summaries, and the previous complete speaker assignment.
- The low-level `apply_diarization_segments` command is not itself a user-facing run implementation.
- Models and native libraries used by the candidate require pinned versions, recorded checksums, redistributable licenses, and both macOS architectures.

## Approaches Considered

### Selected: sherpa-onnx offline runtime in the Rust application

Use the pinned `sherpa-onnx` Rust API and its offline speaker diarization pipeline with a pyannote segmentation model plus a 3D-Speaker embedding model. The official project supports speaker diarization, Rust, macOS x64, and macOS arm64. Unknown speaker count is handled by threshold clustering rather than a hard participant count.

The 0.4.1 line should pin a release at or after `1.13.1`, because that release contains speaker-diarization bounds hardening. The exact crate/native artifact revision is locked only after both architecture builds and the focused fixture test pass.

The architecture spike selected `sherpa-onnx = "=1.13.2"` and locks
`sherpa-onnx-sys` to `1.13.2` in `Cargo.lock`. The default static link mode is used.
The public Rust API is imported from the crate root: `OfflineSpeakerDiarizationConfig`
combines pyannote segmentation, speaker embedding, and `FastClusteringConfig`;
`OfflineSpeakerDiarization::create` and `process` return a result whose ordered
segments are read with `sort_by_start_time`.

Both explicit target checks and link-only unit-test builds passed from the same
source with full Xcode selected per command:

- `x86_64-apple-darwin`: compile and link PASS; static archive SHA-256
  `aba062391bd847bae6b839fd474f444442d1f45071f80ca0c429ec0fb23f9c52`;
- `aarch64-apple-darwin`: compile and link PASS; static archive SHA-256
  `e2d704b01c392970ee7fb90d7e74fd854528a172de0381228e987b79ac479f8e`.

`lipo -info` reports the corresponding `libsherpa-onnx-c-api.a` files as
non-fat `x86_64` and `arm64` archives. The dependency tree contains Meetily's
existing `ort 2.0.0-rc.10` plus sherpa's native ONNX Runtime, but both target
link-only builds pass without duplicate-symbol errors. Therefore the C-API
fallback was not exercised. The implicit network download performed by the
crate build script is spike-only; production builds must use the verified local
artifact store through `SHERPA_ONNX_ARCHIVE_DIR` or `SHERPA_ONNX_LIB_DIR`.

The selected models are packaged or provisioned through the existing deterministic local-artifact process, with license notices and SHA-256 checks. Production diarization must not depend on a first-run network download.

### Rejected for this Universal alpha: FluidAudio/Core ML

FluidAudio remains a strong Apple Silicon candidate for the broader hybrid roadmap, but its documented offline manager requires macOS 14 or later and is optimized for the Apple Neural Engine. It does not provide adequate Intel release evidence for this Universal artifact.

### Rejected: visible wrapper over precomputed segments

Exposing `apply_diarization_segments` would allow tests or callers to inject prepared labels but would not analyze the user's recording. It cannot satisfy the manual smoke requirement.

## Runtime Architecture

Add a `DiarizationRuntime` boundary with a production sherpa implementation and a deterministic fake used only by coordinator tests.

The production flow is:

1. The frontend invokes `run_meeting_diarization(meeting_id)` from Meeting Details.
2. Rust validates persisted diarization settings, meeting existence, completed recording state, and a readable local audio path.
3. The coordinator resolves verified native runtime/model artifacts from application resources or the documented local model store.
4. Audio is decoded and converted to 16 kHz mono Float32 without modifying the source recording.
5. The sherpa runtime returns timestamped speaker intervals using threshold clustering when the participant count is unknown.
6. The coordinator normalizes, validates, and maps intervals to the existing `DiarizationSegmentInput` contract.
7. The existing atomic apply transaction replaces speaker intervals and aligned transcript metadata only after the whole result passes validation.
8. The frontend refetches transcript data and renders final speaker labels.

The inference work runs off the Tauri command thread. Only one diarization run per meeting may be active. A process restart cannot leave a partially applied result because persistence occurs once, after inference completes.

## Audio and Overlap Semantics

The alpha pass uses the completed mixed recording already associated with the meeting. Mixed-only input is recorded as a quality limitation; separate microphone/system stems remain a later enhancement.

Segments retain start/end time, stable generated speaker ID, neutral label, color, confidence where available, and overlap state. Simultaneous segments are collapsed to the generic `Multiple speakers` representation for transcript alignment; the first release does not attach one transcript row to multiple identities.

Transcript text and timestamps are immutable inputs to alignment.

## Model and Artifact Policy

- Pin the sherpa crate/native release after an architecture spike; do not float a semver range in release builds.
- Pin the segmentation and embedding model URLs, license files, sizes, and SHA-256 values in the artifact manifest.
- Prefer the smaller verified quantized segmentation model only if the fixture quality threshold is not worse than the full model beyond the accepted tolerance.
- Bundle required license/NOTICE material in the application resources.
- Build and inspect both `x86_64-apple-darwin` and `aarch64-apple-darwin`; the Universal merge must contain all required native slices.
- Runtime startup verifies model checksums before inference and returns a stable `models unavailable or invalid` error on mismatch.

The currently selected sources are the sherpa-onnx pyannote segmentation export and Apache-2.0 3D-Speaker embedding family. Redistribution remains a release-gate check, not an assumption inferred only from source-code licenses.

## User Experience

Add `Определить спикеров` / localized equivalent to `TranscriptButtonGroup` for completed meetings.

- The action is disabled while recording or while another run for the same meeting is active.
- Running shows a persistent progress state and prevents a duplicate launch.
- Success reports the number of resulting speakers and refreshes the transcript.
- Missing recording, unavailable/corrupt models, unsupported audio, insufficient speech, inference failure, and persistence failure have distinct sanitized messages.
- Failure keeps the previous complete labels and leaves transcript/summary content untouched.

An explicit rerun is allowed and atomically replaces the prior successful result. Cancellation is optional for 0.4.1 only if the underlying runtime cannot safely interrupt; closing the page must not corrupt state.

## Tests and Qualification

### Red-first automated tests

- Coordinator tests reject missing meetings, active recordings, unreadable audio, duplicate runs, malformed engine output, and empty speech before mutation.
- A deterministic runtime fixture proves successful interval normalization and application through the real SQLite transaction.
- Failure injection proves the previous speaker assignment survives engine and persistence failures after reopening the database.
- A short licensed/generated multi-speaker WAV fixture runs through the production sherpa runtime and produces at least two ordered speaker intervals without a supplied speaker count.
- Transcript alignment tests preserve text/timing and mark overlap generically.
- Resource-manifest tests verify pinned hashes and license files.

### Architecture and packaged-app evidence

- Focused inference test passes natively on Intel and Apple Silicon.
- The candidate app, sherpa native runtime, FFmpeg, and helper executables all contain the required architecture slices.
- Offline packaged-app smoke launches diarization on a real completed meeting, renders labels, renames one speaker, restarts, and observes preserved labels.
- A missing/corrupt-model smoke produces a readable error and leaves prior transcript/speaker data intact.

## Release Boundary

This implementation can close the product's visible diarization failure, but READY requires the exact candidate to pass the full automatic gate and manual hardware report. Developer ID signing, notarization, Gatekeeper acceptance, clean-install permissions, real audio capture, and both-architecture runtime evidence remain mandatory and cannot be marked PASS from unit tests.

## Primary References

- Tauri 2 dialog documentation: https://v2.tauri.app/plugin/dialog/
- sherpa-onnx project and platform matrix: https://github.com/k2-fsa/sherpa-onnx
- sherpa-onnx offline diarization API: https://k2-fsa.github.io/sherpa/onnx/c-api/html/speaker_diarization.html
- sherpa-onnx clustering configuration: https://k2-fsa.github.io/sherpa/onnx/c-api/html/structSherpaOnnxFastClusteringConfig.html
- FluidAudio API constraints: https://github.com/FluidInference/FluidAudio/blob/main/Documentation/API.md
- 3D-Speaker license: https://github.com/modelscope/3D-Speaker
