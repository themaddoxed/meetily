# macOS Modular Monolith Contract

Related documents: [Meetily macOS Fork Domain Context](../../CONTEXT.md) and [ADR 0001](../adr/0001-rust-owned-modular-monolith.md).

## 1. Status, scope, and evidence boundary

**Status:** Accepted normative target for downstream implementation.

This document specifies the target architecture. Current source links show the verified migration starting point; they are not proof that the target behavior exists. Plain `FORK-NNN` references identify tasks in the separately maintained foundation-line backlog and are not links in this task baseline.

FORK-055 is documentation only. It does not change runtime behavior, configuration, identity, release state, or Git topology. Unless a statement is explicitly labeled **Current**, every obligation below is **Target** and unimplemented at this baseline.

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative.

## 2. Domain language

The linked domain context defines the ten canonical terms: Meeting, Recording Session, Processing Job, Track, Snapshot, Ordered Update, Recording Root, Relative Recording Path, Model Artifact, and Capability. Architecture interface names use code form, such as `RecordingSession`; prose uses the domain term Recording Session.

**Target.** One Recording Session is one Rust-owned capture attempt for one Meeting. One Processing Job is one Rust-owned `transcribe`, `diarize`, or `summarize` attempt. Snapshots are complete authoritative views, while Ordered Updates advance exactly one owner from a persisted sequence watermark.

## 3. Ownership and module seams

**Target.** The application is one in-process Rust modular monolith. Rust is the sole writer and source of truth for recording lifecycle, processing-job lifecycle, persistence transitions, model installation state, and sequence assignment. The UI issues queries and actions and renders snapshots and updates; it MUST NOT reconstruct authoritative lifecycle from browser storage or global events.

| Concern | Sole target owner | UI role |
| --- | --- | --- |
| Recording start/pause/resume/stop/recovery | `RecordingSession` in managed Rust state | Issue actions, render snapshot |
| Microphone/system capture | macOS `CaptureBackend` adapter selected by Rust | Select allowed input preferences |
| Transcribe/diarize/summarize jobs | Rust job coordinator plus the matching engine interface | Request/cancel, render ordered progress/result |
| Meeting/job persistence | `MeetingRepository` | Query and display |
| Model discovery/install/integrity/capability | `ModelRegistry` | Request install/select, render status |
| Sequence allocation | Rust session/job owner | Deduplicate and apply in order |
| Recording files | Rust while writing/finalizing; user after publication | Never write directly |

The logical target modules are `meeting`, `capture`, `processing`, `persistence`, `models`, and `ipc`. They expose the smallest useful set of interfaces, not exactly one interface per module. In particular, `processing` contains `TranscriptionEngine`, `DiarizationEngine`, and `SummaryEngine`. Concrete engines and backends are adapters at those seams. FORK-055 does not mandate a directory reorganization.

## 4. Recording-session lifecycle

**Target.** `RecordingSessionState` has the following values and only the following transitions:

```text
starting -> recording
recording -> paused
paused -> recording
starting|recording|paused -> stopping -> finalizing -> completed
starting|recording|paused|stopping|finalizing -> interrupted
interrupted -> finalizing -> completed
any nonterminal state -> failed
```

`completed` and `failed` are the only terminal Recording Session states. A crashed process cannot claim that capture continued. Recovery MAY finalize already durable Tracks, but it MUST follow the publication and completion protocol in section 9.

Duplicate `stop` or recovery actions MUST return the existing authoritative snapshot instead of creating a second Meeting, Track, or file. Only one nonterminal Recording Session MAY exist per Meeting, and only one application-wide capture session MAY own capture devices at a time. Rust MUST commit each lifecycle transition before exposing its snapshot or Ordered Update to the UI.

## 5. Processing-job lifecycle

**Target.** `JobKind = transcribe | diarize | summarize`. A persisted Processing Job has only these transitions:

```text
queued -> running -> succeeded
queued|running -> cancelled
running -> interrupted
queued|running|interrupted -> failed
interrupted -> queued
```

`succeeded`, `cancelled`, and `failed` are terminal Processing Job states. At most one active job MAY exist for a given `(meetingId, kind, inputRevision)`.

A repeated start with the same idempotency key MUST return the existing `jobId` and snapshot, including when that attempt is terminal. Retrying `interrupted` work MAY reuse the same `jobId` through `interrupted -> queued`. Retrying a terminal attempt requires a new idempotency key and a new `jobId`; the old attempt remains durable and queryable.

Diarization requires completed recording input. Summary requires a committed transcript revision. A downstream failure MUST NOT delete raw Tracks or a previously committed result. Rust MUST commit every transition before exposing it to the UI.

## 6. Tauri command and ordered Channel protocol

**Target.** Query commands are read-only. Action commands may change state. The semantic command surface is fixed below; downstream work MAY retain temporary legacy command names only through adapters.

| Kind | Command | Result |
| --- | --- | --- |
| Query | `getRecordingSession(sessionId)` | `RecordingSessionSnapshot` |
| Query | `getActiveRecordingSession()` | nullable `RecordingSessionSnapshot` |
| Query | `getJob(jobId)` | `JobSnapshot` |
| Query | `listMeetingJobs(meetingId)` | ordered job snapshots |
| Action | `startRecording(request)` | created/current `RecordingSessionSnapshot` |
| Action | `pauseRecording(sessionId)` | current `RecordingSessionSnapshot` |
| Action | `resumeRecording(sessionId)` | current `RecordingSessionSnapshot` |
| Action | `stopRecording(sessionId)` | current `RecordingSessionSnapshot` |
| Action | `startJob(request)` | created/current `JobSnapshot` |
| Action | `cancelJob(jobId)` | current `JobSnapshot` |
| Observe | `observeRecordingSession(sessionId, updates)` | `RecordingSessionSnapshot + watermark + subscriptionGeneration` |
| Observe | `observeJob(jobId, updates)` | `JobSnapshot + watermark + subscriptionGeneration` |
| Control | `ackObservation(ownerId, subscriptionGeneration, sequence)` | `ObservationAck` |

`updates` is a UI-created `tauri::ipc::Channel<SessionUpdate>` or `tauri::ipc::Channel<JobUpdate>`. The UI passes it only as an argument to the separate matching `observe*` invoke. Action commands MUST return snapshots and MUST NOT accept or return a Channel. `observe*` returns only the full authoritative snapshot, its `watermark`, and the active `subscriptionGeneration`; Rust never returns a Channel.

Every update envelope MUST contain `protocolVersion`, `subscriptionGeneration`, exactly one owner ID (`sessionId` or `jobId`), `sequence`, `kind`, and a typed payload. A job update MUST also contain `meetingId` and `jobKind`.

`ownerId` in `ackObservation` is a tagged `sessionId` or `jobId`. `ObservationAck` returns that owner, the `subscriptionGeneration`, the accepted `acknowledgedSequence`, and `resyncRequired`. The command is idempotent control-plane state and MUST NOT change Recording Session or Processing Job lifecycle.

Delivery rules:

1. `sequence` is a persisted unsigned 64-bit counter scoped to one `sessionId` or one `jobId`. It starts at 1 and strictly increases for every authoritative update whether or not an observation generation is active.
2. Coalescible progress MUST be coalesced before sequence allocation. Lifecycle transitions, results, failures, and terminal updates MUST NOT be coalesced. Once any update receives a `sequence`, it is authoritative and MUST NOT be silently dropped or replaced by another update.
3. One Rust owner or actor MUST always serialize sequence allocation and authoritative persistence of the transition or result together with its `sequence`, even when there is no active subscription. Only after that persistence, and only when a live active generation exists, does the owner retain the update in that generation's outbox and enqueue it on the Channel. With no active generation there is no outbox retention or Channel enqueue, but the persisted snapshot and `watermark` still advance. No other writer may allocate or publish updates for that owner.
4. Every active generation MUST have a fixed finite Rust outbox limit and a fixed finite UI buffer limit, each enforced by both message count and serialized bytes. These limits and the observation-watchdog timeout MUST be configuration constants. Downstream implementation MUST choose exact values and test the count, byte, and timeout boundaries; FORK-055 does not invent numeric values.
5. Rust MUST retain unacknowledged authoritative updates inside the bounded per-generation outbox. `Channel::send` success means only that Tauri accepted the message for IPC enqueue; it is not UI-consumer acknowledgement, delivery confirmation, or backpressure. Observation Channels MUST NOT carry audio bytes or audio chunks; audio flows only through the internal bounded `TrackSink`.
6. One active Channel generation preserves Rust enqueue order for its owner. There is no ordering promise between Channel delivery and resolution of the invoke Promise that returns the snapshot.
7. `startRecording` and `startJob` return snapshots without a Channel. The UI then creates a Channel and calls `observeRecordingSession(sessionId, updates)` or `observeJob(jobId, updates)`. The full authoritative observe snapshot represents every transition committed between the action and observation; the protocol does not depend on replaying those updates.
8. The UI Channel callback MUST buffer messages until the observe invoke Promise resolves, subject to both UI count and byte limits. It then accepts only messages whose `subscriptionGeneration` matches the returned generation, replaces projected state with the returned full snapshot, sets `lastApplied` to the returned `watermark`, ignores buffered duplicates with `sequence <= watermark`, and applies contiguous buffered or live messages with `sequence > watermark` in order. If either UI limit would be exceeded, the UI MUST discard that generation and immediately call `observe*` with a new Channel for a full snapshot.
9. `observe*` MUST run through the same owner or actor serialization as transitions. As one serialized operation, it first marks the previous subscription inactive so the owner has no active generation, then clears that generation's outbox, allocates a new `subscriptionGeneration`, registers the UI-created Channel, and captures the authoritative snapshot and `watermark`. In-flight messages from an old generation MAY arrive, but the UI MUST discard them.
10. After applying updates, the UI MUST call `ackObservation` with the highest contiguous `sequence` applied for that owner and generation. `ackObservation` MUST run through the same owner or actor serialization as transitions and `observe*`. In one atomic operation it validates that the supplied generation is the current live generation, monotonically advances the highest contiguous acknowledgement, and releases only outbox entries through that sequence. Duplicate or lower acknowledgements are idempotent. An acknowledgement beyond the highest enqueued sequence is rejected. A stale generation or an owner with no active generation returns `resyncRequired` and cannot advance or release any window.
11. During normal delivery the UI applies only `sequence == lastApplied + 1`, ignores duplicates with `sequence <= lastApplied`, and starts a new `observe*` generation on a gap. It MUST NOT guess, skip missing state, or request replay.
12. If retaining or serializing the next authoritative update would exceed either Rust outbox limit, or Channel enqueue fails, Rust MUST atomically mark the generation inactive before clearing its outbox, while leaving the authoritative lifecycle and update sequence persisted. The owner then has no active generation until the next successful `observe*`. It MUST NOT silently lose a lifecycle or terminal state. Rust SHOULD make a best-effort `resyncRequired` indication on the Channel or the next `ackObservation` response; a visible gap has the same meaning. Delivery of that advisory indication is not required for correctness because the UI watchdog MUST cause a full `observe*`.
13. The UI MUST run a finite per-generation observation watchdog. Expiry, `resyncRequired`, a gap, an acknowledgement failure, or local buffer overflow discards the generation and immediately starts a new `observe*`. The exact finite timeout and its tests belong to the downstream protocol implementation.
14. Reconnect has no retained-update replay and no replay-cursor parameter. The bounded outbox is generation-local delivery state, not a cross-generation replay log. After WebView reload, listener loss, sleep/wake, app relaunch, or any resync condition, the UI creates a new Channel and calls `observe*`. Rust returns a full authoritative persisted snapshot. That snapshot replaces UI state, its `watermark` becomes `lastApplied`, and the UI then applies only matching-`subscriptionGeneration` buffered messages above the watermark.
15. A dropped receiver, invalidated generation, or observation failure MUST NOT change Rust lifecycle. The full persisted snapshot remains the recovery authority.
16. Global Tauri events MAY remain for non-authoritative shell notifications, but MUST NOT carry audio chunks or serve as the authoritative recording or job stream.
17. Action errors and snapshots MUST echo the relevant stable `sessionId` or `jobId`. UI-generated timestamps, `localStorage`, `sessionStorage`, and IndexedDB MUST NOT be lifecycle authority.

### Operation identity and idempotency

**Target.** Every retriable operation has an explicit persisted identity. For keyed operations, an `idempotencyKey` is scoped by operation kind, so the stored identity is `(operationKind, idempotencyKey)`. Stop, recover, pause, resume, and cancel instead use the existing owner ID with operation-kind or desired-state semantics as listed below. Rust MUST persist the applicable identity with a deterministic canonical request fingerprint containing every semantic input and excluding transport metadata and UI-generated timestamps.

| Operation | Identity carrier | Canonical fingerprint obligation |
| --- | --- | --- |
| Start recording | `StartRecordingRequest.idempotencyKey` | Meeting identity, capture inputs/preferences, Recording Root identity, Relative Recording Path destination, and every other semantic request field. |
| Start job | `StartJobRequest.idempotencyKey` | `meetingId`, `jobKind`, `inputRevision`, model requirement, destination when applicable, and every other semantic job input. |
| Install model | `ModelInstallRequest.idempotencyKey` | Model identity/version/source/checksum, architecture requirement, destination, and every other semantic install input. |
| Future import | `ImportRequest.idempotencyKey` | Meeting/input identity, source integrity metadata, destination, and every other semantic import input. |
| Stop or recover recording | `sessionId` | The operation kind and existing Recording Session identity; no separate `idempotencyKey`. |
| Pause or resume recording | `sessionId` desired-state semantics | Repeating the desired state returns the current snapshot; it does not create a new operation. |
| Cancel job | `jobId` desired-state semantics | Repeating cancellation returns the current durable job snapshot. |

The same scoped identity and canonical fingerprint MUST return the prior snapshot or artifact, including a terminal result. Reusing the same scoped identity with a different Meeting, `inputRevision`, model, destination, or any other fingerprint field MUST return the stable non-retryable `idempotency_conflict` error and MUST NOT start work. Retrying a terminal Processing Job still requires a new `idempotencyKey` and therefore a new `jobId` as defined in section 5.

An existing final file MAY be reconciled only when the persisted operation identity, destination, and integrity metadata all match. A different identity or destination returns non-retryable `idempotency_conflict`; matching identity and destination with mismatched integrity returns `integrity_failed`. Neither case may overwrite the existing final file.

## 7. Rust module interfaces

**Target.** Each named interface has one responsibility and hides its implementation behind a small seam.

| Interface | Responsibility | Required methods, inputs, and outputs |
| --- | --- | --- |
| `RecordingSession` | Own one recording lifecycle and coordinate persistence, capture, and publication. | `start(StartRecordingRequest) -> RecordingSessionSnapshot`; `pause(sessionId) -> RecordingSessionSnapshot`; `resume(sessionId) -> RecordingSessionSnapshot`; `stop(sessionId) -> RecordingSessionSnapshot`; `recover(sessionId) -> RecordingSessionSnapshot`; `get(sessionId) -> RecordingSessionSnapshot`; `getActive() -> optional RecordingSessionSnapshot` |
| `CaptureBackend` | Control one capability-selected macOS capture implementation. | `capabilities() -> CaptureCapabilities`; `start(CaptureRequest, TrackSink) -> CaptureHandle`; `pause(handle)`; `resume(handle)`; `stop(handle) -> CapturedTracks` |
| `TranscriptionEngine` | Produce one transcript artifact from committed input. | `capabilities() -> EngineCapabilities`; `transcribe(TranscriptionInput, ModelLease, Cancellation, ProgressSink) -> TranscriptArtifact` |
| `DiarizationEngine` | Produce one durable speaker-assignment artifact from completed recording input. | `capabilities() -> EngineCapabilities`; `diarize(DiarizationInput, ModelLease, Cancellation, ProgressSink) -> DiarizationArtifact` |
| `SummaryEngine` | Produce one summary artifact from a committed transcript revision. | `capabilities() -> EngineCapabilities`; `summarize(SummaryInput, ModelLease, Cancellation, ProgressSink) -> SummaryArtifact` |
| `MeetingRepository` | Load aggregates, commit optimistic revisions, and enumerate recoverable work. | `load(meetingId) -> optional MeetingAggregate`; `commit(expectedRevision, MeetingMutation) -> MeetingAggregate`; `listRecoverable() -> RecoverableWork` |
| `ModelRegistry` | Resolve, install, verify, and describe architecture-compatible Model Artifacts. | `resolve(ModelRequirement, RuntimeCapabilities) -> ModelLease`; `install(ModelInstallRequest, Cancellation, ProgressSink) -> ModelArtifact`; `get(modelId) -> ModelArtifactSnapshot` |

Tauri types, `AppHandle`, SQL rows, absolute paths, and concrete engine or backend types MUST NOT cross these core seams. `RecordingSession` and the Rust job coordinator consume the other interfaces; the UI calls only `ipc` adapters. `MeetingRepository.commit` MUST use optimistic `expectedRevision` so a stale writer cannot overwrite newer state.

`TrackSink`, `ProgressSink`, `Cancellation`, `ModelLease`, `CaptureHandle`, `ObservationAck`, request records, snapshot records, artifact records, and capability records are internal seam, wire, or value types, not additional top-level architecture interfaces. `TrackSink` MUST be bounded, propagate explicit backpressure to capture orchestration, and is the only listed sink that carries audio. `ProgressSink` MUST be bounded and MAY coalesce telemetry only before the Rust owner allocates `sequence`; it never owns lifecycle. `ObservationAck` acknowledges only the highest contiguous applied update under section 6 and is not a lifecycle result. `StartRecordingRequest`, `StartJobRequest`, `ModelInstallRequest`, and a future `ImportRequest` carry the explicit `idempotencyKey` defined in section 6. `Cancellation` is cooperative, MUST be checked at documented safe points, and cannot persist a transition; only the Rust owner does that. `ModelLease` pins one verified compatible Model Artifact for one engine run. None of these types may be replaced by global events.

## 8. Error contract

**Target.** Every command failure uses one wire envelope:

```text
ContractError {
  code,
  message,
  retryable,
  sessionId?,
  jobId?,
  currentState?,
  details?
}
```

| Code | Stable meaning |
| --- | --- |
| `invalid_state` | The requested transition is illegal from `currentState`. |
| `not_found` | The requested Meeting, Recording Session, Processing Job, model, or artifact does not exist. |
| `already_running` | A conflicting active capture or equivalent Processing Job already owns the resource. |
| `permission_denied` | macOS or user policy denied a required permission. |
| `device_unavailable` | A required capture device is absent or cannot be opened. |
| `capture_failed` | Capture started or ran but could not produce the expected Track data. |
| `model_unavailable` | No installed or installable model satisfies the request. |
| `model_incompatible` | The Model Artifact does not support the active architecture or required engine contract. |
| `storage_unavailable` | The Recording Root or required user-owned media is missing, inaccessible, or has the wrong identity. |
| `integrity_failed` | Size, checksum, or publication integrity verification failed. |
| `idempotency_conflict` | A scoped operation identity is already bound to a different canonical request fingerprint or destination; this error is non-retryable. |
| `persistence_conflict` | `expectedRevision` is stale or another authoritative commit won. |
| `persistence_failed` | SQLite or durable file metadata could not be committed. |
| `cancelled` | Cooperative cancellation reached a documented safe point. |
| `unsupported_capability` | No required CPU implementation exists for the requested mandatory behavior; this blocks Alpha readiness. |
| `internal` | An unexpected failure has no more specific stable code. |

`message` is diagnostic or user-facing text and MUST NOT be program logic. The UI branches on `code`, stable IDs, and `retryable`. An error MUST NOT advance lifecycle unless Rust persists the corresponding `failed` or `interrupted` transition.

## 9. Persistence and file ownership

**Target.** SQLite stores Meeting, Recording Session, and Processing Job IDs; lifecycle states; revisions; timestamps; job inputs and results; model references; the Recording Root identifier; normalized relative folder and file paths; and integrity metadata. It MUST NOT store an absolute recording path or an audio blob.

The configured external Recording Root maps its opaque identifier to an absolute path outside Meeting rows. Path resolution MUST join that root to a validated Relative Recording Path and reject an absolute path, `..`, symlink escape, a missing root, or a mismatched root identity.

Microphone and system audio MUST be written as distinct Tracks. Raw Tracks MUST be retained when transcription, diarization, or summary fails.

Rust MUST create the stable IDs and relative destination before capture, persist the nonterminal state, and write each output to a uniquely named temporary file in the same destination directory as its final file.

Publication of each Track is no-clobber and follows this order:

1. Finish writing bytes.
2. Flush and durably sync the file.
3. Close the file.
4. Verify expected size and checksum when applicable.
5. Use a platform-equivalent atomic rename that fails if the final name already exists.
6. Durably sync parent-directory metadata using the platform-supported equivalent.
7. Only then treat that Track as published.

A plain rename that may replace an existing final file does not satisfy this contract. Existing final files MUST NOT be silently overwritten.

The microphone and system Track renames are individually atomic but are not cross-file atomic. A Recording Session reaches `completed` only after every expected microphone and system Track is published and both final Relative Recording Paths and integrity records are committed together in one SQLite transaction.

Publication and reconciliation MUST use the operation identities and canonical fingerprints in section 6. A final file may be reused only when identity, destination, and integrity metadata match; identity or destination mismatch returns `idempotency_conflict`, integrity mismatch returns `integrity_failed`, and neither case may overwrite the file. Recovery MUST reconcile persisted expectations against the complete or partial set of temporary and final Tracks. It MAY finish a valid missing publication or commit an already published deterministic artifact, but it MUST NOT overwrite a final path and MUST NOT mark the Recording Session `completed` until the full expected Track set is recorded by the single completion transaction.

After final publication, the user owns recording files and may move or delete them. The application MUST report `storage_unavailable` or missing media and MUST NOT silently recreate, relocate, or delete user files. App-managed Model Artifacts remain owned by `ModelRegistry` and MUST use the same temporary-file, integrity-check, no-clobber atomic-rename, and durable-parent-sync discipline.

**Current.** Meeting rows expose an absolute `folder_path`; existing recovery accepts direct folder paths. This is migration input only and is not the target contract.

## 10. Universal macOS capability matrix

**Target.** The platform floor is:

```text
Build target: universal-apple-darwin
Required slices: x86_64 and arm64 in the app and every bundled executable/native library
Minimum OS declaration: bundle.macOS.minimumSystemVersion = 14.2
First-party deployment target: 14.2 for the app and helper targets
Intel rule: every required Alpha function has a CPU execution path
Apple Silicon rule: Metal/CoreML/other acceleration is optional, capability-probed, and falls back to CPU
```

| Capability area | `x86_64` required behavior | `arm64` required behavior | Optional acceleration | Fallback or failure rule |
| --- | --- | --- | --- | --- |
| App/Tauri shell | Run the complete Alpha shell on macOS 14.2 through the Intel slice. | Run the same shell through the arm64 slice. | None required. | Missing slice or invalid `minos` fails packaging. |
| Microphone capture | Capture and publish the microphone Track without ML acceleration. | Identical capture and publication behavior. | Native device optimizations MAY vary. | Permission or device loss uses the stable error and recovery contract; no architecture-only waiver. |
| System-audio capture | Capture and publish the system Track without ML acceleration. | Identical capture and publication behavior. | Native capture optimizations MAY vary. | Failure reports the stable capture error and MUST NOT claim completed publication. |
| FFmpeg/media processing | A bundled `x86_64` CPU path processes required media. | A bundled `arm64` CPU path processes the same media. | Verified hardware media acceleration MAY be selected. | Acceleration failure falls back to CPU; a missing required slice blocks Alpha readiness. |
| Transcription | Whisper and Parakeet/ONNX have mandatory Intel CPU paths. | The same required behavior has a CPU path. | Metal, CoreML, or another verified arm64 adapter MAY run. | Acceleration failure MUST fall back to CPU; missing required CPU support returns `unsupported_capability` and blocks Alpha readiness. |
| Diarization | Required diarization has an Intel CPU path. | The same required diarization has a CPU path. | Metal, CoreML, or another verified arm64 adapter MAY run. | Acceleration failure MUST fall back to CPU; missing required CPU support returns `unsupported_capability` and blocks Alpha readiness. |
| Local summary | Required local summary has an Intel CPU path. | The same required local summary has a CPU path. | Metal, CoreML, or another verified arm64 adapter MAY run. | Acceleration failure MUST fall back to CPU; missing required CPU support returns `unsupported_capability` and blocks Alpha readiness. |
| Model installation/integrity | Install, checksum, and activate only Intel-compatible artifacts. | Install, checksum, and activate only arm64-compatible artifacts. | Architecture-specific optimized artifacts MAY be selected. | Compatibility is checked before activation; an incompatible artifact cannot become selected or ready. |
| SQLite/recovery | Persist and recover without ML acceleration. | Identical persistence and recovery behavior. | None. | Storage or persistence failure uses the stable error contract and preserves raw data. |
| Remote-provider adapters | Expose the same interface and state behavior on Intel. | Expose the same interface and state behavior on arm64. | Provider-specific remote execution MAY be used. | Remote providers do not remove the required local CPU fallback for mandatory Alpha functions. |

Capture and persistence MUST work on both architectures without ML acceleration. Whisper, Parakeet/ONNX, diarization, and local summary MUST have Intel CPU paths. `unsupported_capability` is allowed only when no required CPU implementation exists; that condition blocks Alpha readiness rather than becoming an accepted runtime mode.

Universal packaging MUST inspect every embedded Mach-O and require `x86_64` and `arm64` slices in the app and every bundled executable or native library. Every first-party app and helper slice MUST report `LC_BUILD_VERSION/minos = 14.2`. A bundled third-party FFmpeg or native-library slice MAY report `minos <= 14.2`; any embedded slice with `minos > 14.2` fails the gate.

`bundle.macOS.minimumSystemVersion = 14.2` and a first-party build deployment-target setting of 14.2 are required configuration. They express intent but are not binary evidence. Slice and `LC_BUILD_VERSION/minos` inspection is the acceptance evidence.

The same interfaces, states, errors, persistence rules, and wire protocol MUST apply on both architectures; only the capability-selected adapter may vary.

**Current.** `Meetily`, `meetily`, and `com.meetily.ai` names remain temporary compatibility labels. FORK-055 performs no rename, signing or updater setup, endpoint choice, or identity publication.

## 11. Current-to-target map

Each **Current** statement below records a verified fact at baseline `45bebcbf184da4d2b16c48a0415050019becff98`. Each **Target** statement is an obligation, not an implementation claim.

| Current source path | Current fact | Target obligation |
| --- | --- | --- |
| [`frontend/src-tauri/src/lib.rs`](../../frontend/src-tauri/src/lib.rs) | **Current.** Registers many commands and several separately managed states. | **Target.** One managed Rust application state owns session and job coordinators and adapters. |
| [`frontend/src-tauri/src/state.rs`](../../frontend/src-tauri/src/state.rs) | **Current.** `AppState` holds only `DatabaseManager`. | **Target.** Expand the composition root without globals as lifecycle authority. |
| [`frontend/src-tauri/src/audio/recording_commands.rs`](../../frontend/src-tauri/src/audio/recording_commands.rs) | **Current.** Uses global `IS_RECORDING`, `RECORDING_MANAGER`, task and listener slots, emits global events, and delegates Meeting persistence after stop to the UI. | **Target.** `RecordingSession` owns lifecycle, IDs, persistence, and ordered updates. |
| [`frontend/src-tauri/src/audio/recording_manager.rs`](../../frontend/src-tauri/src/audio/recording_manager.rs) and [`frontend/src-tauri/src/audio/recording_state.rs`](../../frontend/src-tauri/src/audio/recording_state.rs) | **Current.** Rust capture control and partial state machinery exist. | **Target.** Move them behind `RecordingSession` and `CaptureBackend` seams while preserving live behavior during migration. |
| [`frontend/src-tauri/src/audio/capture/`](../../frontend/src-tauri/src/audio/capture/) | **Current.** Capture implementations and adapters exist. | **Target.** Select one macOS `CaptureBackend` by Capability. |
| [`frontend/src-tauri/src/audio/transcription/provider.rs`](../../frontend/src-tauri/src/audio/transcription/provider.rs) and [`frontend/src-tauri/src/audio/transcription/engine.rs`](../../frontend/src-tauri/src/audio/transcription/engine.rs) | **Current.** A provider trait and engine selection exist. | **Target.** Conform behind `TranscriptionEngine` and Rust job ownership. |
| [`frontend/src-tauri/src/diarization/coordinator.rs`](../../frontend/src-tauri/src/diarization/coordinator.rs) | **Current.** Has a per-Meeting run registry and emits `meeting-diarization-progress` global events. | **Target.** Put `DiarizationEngine` behind a persisted ordered job. |
| [`frontend/src-tauri/src/summary/service.rs`](../../frontend/src-tauri/src/summary/service.rs) and [`frontend/src-tauri/src/summary/summary_engine/`](../../frontend/src-tauri/src/summary/summary_engine/) | **Current.** Summary lifecycle and model logic are split across modules. | **Target.** Coordinate `SummaryEngine` and `ModelRegistry` through persisted jobs. |
| [`frontend/src-tauri/src/database/manager.rs`](../../frontend/src-tauri/src/database/manager.rs), [`frontend/src-tauri/src/database/models.rs`](../../frontend/src-tauri/src/database/models.rs), and [`frontend/src-tauri/src/database/repositories/`](../../frontend/src-tauri/src/database/repositories/) | **Current.** SQLite manager, models, and repositories exist; Meeting rows expose `folder_path`. | **Target.** `MeetingRepository` owns persisted revisions and states, with a root ID and Relative Recording Paths. |
| [`frontend/src-tauri/src/audio/recording_saver.rs`](../../frontend/src-tauri/src/audio/recording_saver.rs) and [`frontend/src-tauri/src/audio/incremental_saver.rs`](../../frontend/src-tauri/src/audio/incremental_saver.rs) | **Current.** Some metadata and transcript writes use temporary files and rename; recovery still accepts direct folder paths. | **Target.** Use one uniform atomic publication and recovery contract. |
| [`frontend/src/services/recordingService.ts`](../../frontend/src/services/recordingService.ts) and [`frontend/src/services/transcriptService.ts`](../../frontend/src/services/transcriptService.ts) | **Current.** The UI invokes commands and listens to global recording and transcript events. | **Target.** Use thin query/action adapters and owner-scoped ordered Channels. |
| [`frontend/src/hooks/useRecordingStop.ts`](../../frontend/src/hooks/useRecordingStop.ts), [`frontend/src/hooks/useProcessingProgress.ts`](../../frontend/src/hooks/useProcessingProgress.ts), and [`frontend/src/services/indexedDBService.ts`](../../frontend/src/services/indexedDBService.ts) | **Current.** The UI saves Meeting data after stop and keeps recovery or progress authority in browser stores. | **Target.** The UI is only a cache and projection of Rust snapshots. |
| [`frontend/scripts/build-universal-macos.js`](../../frontend/scripts/build-universal-macos.js) and [`frontend/src-tauri/tauri.universal.conf.json`](../../frontend/src-tauri/tauri.universal.conf.json) | **Current.** The Universal target and slices are present; the deployment floor is not declared in Tauri configuration. | **Target.** Enforce both slices for every embedded Mach-O, `minimumSystemVersion: "14.2"`, first-party app and helper deployment target 14.2, exact first-party `minos`, and third-party `minos <= 14.2`. |
| [`frontend/src-tauri/Cargo.toml`](../../frontend/src-tauri/Cargo.toml) and [`frontend/src-tauri/src/whisper_engine/acceleration.rs`](../../frontend/src-tauri/src/whisper_engine/acceleration.rs) | **Current.** macOS dependencies enable Metal and CoreML broadly while runtime code special-cases Intel CPU. | **Target.** Select optional arm64 acceleration by Capability and retain mandatory Intel CPU fallback. |
| [`frontend/src-tauri/tauri.conf.json`](../../frontend/src-tauri/tauri.conf.json) | **Current.** Legacy Meetily identity, updater, and local endpoints remain. | **Target.** Keep identity temporary; FORK-056 and FORK-057 remove forbidden legacy runtime surface. |

## 12. Downstream traceability

**Target.** Each downstream task consumes the listed part of this contract.

| Task | Contract used by that task |
| --- | --- |
| `FORK-033` | Remove non-macOS Rust surface while retaining the mandatory Intel CPU matrix. |
| `FORK-034` | Implement the single `RecordingSession` interface and macOS `CaptureBackend`; retain separate microphone and system Tracks. |
| `FORK-035` | Enforce Universal slices and the macOS 14.2 deployment floor in CI and release. |
| `FORK-036` | A new updater must wait for final identity and respect Universal architecture and signature checks. |
| `FORK-037` | Preserve SQLite, recordings, models, and settings across the later bundle-ID and data-location migration without violating root-relative ownership. |
| `FORK-038` | The absence gate checks forbidden platforms and legacy runtime against the target module map. |
| `FORK-040` | Implement `ModelRegistry`, integrity, atomic install, architecture compatibility, and local-summary Capability. |
| `FORK-041` | Implement `DiarizationEngine`, job lifecycle, ordered progress, and durable speaker result. |
| `FORK-043` | Implement Recording Root, relative SQLite metadata, scoped idempotency identities and fingerprints, conflict-safe reconciliation, atomic rename, and folder recovery. |
| `FORK-045` | Implement `TranscriptionEngine`, bounded sinks and queues, finite count/byte observation windows, highest-contiguous acknowledgements, resync watchdog, monotonic `sequence`, `sessionId` and `jobId`, and ordered non-audio Channel delivery instead of events. |
| `FORK-051` | Implement permission and device errors plus recording start, stop, restart, and recovery invariants. |
| `FORK-052` | Implement the persisted Rust source of truth, operation fingerprints, snapshot and full-resync behavior, crash and relaunch recovery, and no duplicate or raw-data loss. |
| `FORK-056` | Keep the legacy updater disabled until a separate FORK-036 is implemented and verified after the identity decision. |
| `FORK-057` | Remove Python, FastAPI, and local HTTP product orchestration after callers move behind Rust interfaces. |

A downstream task MAY strengthen implementation checks, but it MUST NOT weaken these ownership, storage, ordering, recovery, or Capability invariants without a superseding ADR.

## 13. Invariants

**Target.** These invariants apply across all modules and adapters.

| Area | Normative invariant |
| --- | --- |
| Stable IDs | One Recording Session keeps one `sessionId`; one Processing Job keeps one `jobId`; retries follow the lifecycle and idempotency rules instead of silently replacing identities. |
| Legal transitions | Rust MUST persist only the Recording Session and Processing Job transitions listed in sections 4 and 5. |
| One writer | Exactly one Rust owner always allocates sequences and commits authoritative lifecycle for each session or job regardless of subscription state; observe, acknowledgement, generation invalidation, and conditional enqueue run through that same serialization; the UI and adapters MUST NOT become parallel writers. |
| One active capture | At most one nonterminal Recording Session exists per Meeting and at most one application-wide capture session owns devices. |
| One active equivalent job | At most one active Processing Job exists per `(meetingId, kind, inputRevision)`. |
| Separate raw Tracks | Microphone and system audio remain separate durable Tracks until an explicit downstream operation combines them; downstream failure MUST NOT delete them. |
| Root-relative storage | SQLite MUST NOT hold absolute or escaping recording paths; resolution MUST reject absolute paths, `..`, symlink escape, missing roots, and mismatched root identity. |
| Atomic publication | Every Track and Model Artifact uses integrity-checked, no-clobber atomic publication and durable parent sync; reconciliation requires matching identity, destination, and integrity metadata; Recording Session completion uses one transaction for the full expected Track set. |
| Monotonic ordering | Coalescible progress is reduced before allocation; each authoritative update then has a persisted, strictly increasing `sequence` regardless of subscription state, and the UI applies only contiguous matching-generation updates after a full snapshot watermark. |
| Bounded observation | Rust outboxes and UI buffers have finite count and serialized-byte limits; `Channel::send` is enqueue only; an atomic current-generation acknowledgement releases the highest-contiguous window; invalidation marks no active generation before clearing the outbox; overflow or enqueue failure leaves no active generation until full observe without changing persisted lifecycle. |
| Idempotent actions | Explicit keys are scoped by operation kind and persisted with canonical semantic fingerprints; an identical retry returns prior durable work, while identity reuse with a different fingerprint returns non-retryable `idempotency_conflict` and MUST NOT create or overwrite anything. |
| Architecture capability | No mandatory Alpha behavior may require an Apple-Silicon-only Capability; Intel CPU execution is mandatory and optional acceleration falls back to CPU. |

## 14. Verification and TDD decision

FORK-055 changes only architecture documentation. There is no runtime behavior to drive through a failing executable test, so no RED/GREEN product test is added. A Markdown word-search test would verify spelling rather than the contract. Verification therefore uses diff hygiene, local-link/path validation, an explicit human contract checklist, and the existing release-policy regression test. Runtime tests belong to the downstream implementation tasks traced above.

Reviewer checklist:

- [ ] `CONTEXT.md` is a glossary only and uses all ten canonical terms consistently.
- [ ] ADR status is `Accepted`, explains the real alternatives/trade-offs, and says docs-only.
- [ ] Neither product document links to absent `docs/prd/macos-fork.md`; plain FORK IDs are identified as traceability to the separately maintained foundation-line backlog.
- [ ] Rust alone owns recording lifecycle, job lifecycle, persistence transitions, and sequence allocation.
- [ ] Query/action command semantics and owner-scoped ordered Channel semantics are separate and explicit.
- [ ] Start actions accept no Channel and return snapshots only; UI then calls separate `observeRecordingSession(sessionId, updates)` or `observeJob(jobId, updates)` with a UI-created Channel.
- [ ] Observe returns a full snapshot, `watermark`, and `subscriptionGeneration`; owner/actor serialization, buffering until the observe Promise resolves, old-generation invalidation, duplicate/gap handling, and the lack of Channel-vs-invoke-response ordering are explicit.
- [ ] The owner/actor always serializes `sequence` allocation and authoritative persistence even without a subscription; outbox retention and Channel enqueue require a live active generation; `observe*`, highest-contiguous `ackObservation`, and invalidation use the same serialization; invalidation marks no active generation before clearing the outbox, and stale/no-active acknowledgements return `resyncRequired`.
- [ ] Rust outboxes and UI buffers have finite count and serialized-byte limits; progress coalesces before `sequence`; `Channel::send` is enqueue only; Channels carry no audio; overflow, failure, or watchdog expiry leaves no active generation and forces a full snapshot without silently losing persisted lifecycle or terminal state.
- [ ] Reconnect/gap handling replaces UI state from a full authoritative snapshot, with no retained-update replay or replay cursor; transitions between action and observe are covered by the observe snapshot.
- [ ] `sessionId`, `jobId`, `sequence`, `watermark`, and `subscriptionGeneration` appear in every relevant wire rule.
- [ ] All seven required top-level interfaces have responsibilities, inputs, outputs, and no leaked Tauri/SQL/concrete adapter types; processing correctly exposes three engine interfaces.
- [ ] `TrackSink`, `ProgressSink`, `Cancellation`, `ModelLease`, `ObservationAck`, and related helper records are classified as internal seam/wire/value types with bounded/backpressure/cancellation/acknowledgement semantics.
- [ ] Recording/job states, exact terminal states, interrupted retry, terminal retry, error envelope/codes, and idempotency rules are complete; explicit operation keys are scoped by kind and bound to canonical fingerprints, and conflicting reuse returns non-retryable `idempotency_conflict`.
- [ ] SQLite relative metadata, external Recording Root ownership, no-clobber publication, durable file/parent sync, non-atomic two-track publication, one completion transaction, and partial-set recovery are complete.
- [ ] Universal `x86_64` + `arm64` is checked for every embedded Mach-O; configuration fixes `minimumSystemVersion` and first-party app/helper deployment target at 14.2; binary evidence requires first-party `minos = 14.2`, permits third-party `minos <= 14.2`, and rejects every slice with `minos > 14.2`.
- [ ] Optional acceleration failure MUST fall back to CPU for mandatory Alpha functions; any missing required CPU implementation produces `unsupported_capability` and blocks Alpha readiness.
- [ ] Legacy Meetily identity is temporary and the document makes no rename/updater/signing/publication claim.
- [ ] Current-to-target rows link exact current code paths and never describe target behavior as current.
- [ ] Traceability contains exactly `FORK-033`, `FORK-034`, `FORK-035`, `FORK-036`, `FORK-037`, `FORK-038`, `FORK-040`, `FORK-041`, `FORK-043`, `FORK-045`, `FORK-051`, `FORK-052`, `FORK-056`, and `FORK-057`.
- [ ] FORK-056 keeps the legacy updater disabled until separate FORK-036 implementation and verification after the identity decision.
- [ ] The TDD decision explicitly rejects a Markdown word-search test and assigns runtime tests to downstream tasks.
- [ ] No tracked path outside the exact three-output set changed.
