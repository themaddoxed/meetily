# FORK-043 Recording Root and Atomic Publication Implementation Plan

> Execute the existing task graph through the available mdtask/SDD workflow. Checkbox items are future implementation checks. Independent SPEC and QUALITY review remains mandatory at the explicit gates; historical skill names do not expand authorization.

**Goal:** make new recording, repeated start, audio import, concurrent save and crash recovery use one Rust-owned Recording Root contract where SQLite stores durable state and normalized relative paths, file publication is integrity-checked and no-clobber atomic, and retries never create a duplicate Meeting, Track, directory, or final file.

**Architecture:** one deep `recording_storage` module is the only module allowed to map an opaque `RecordingRootId` to an absolute path or perform managed recording-file operations. `RecordingSession` and import orchestration allocate a hidden/versioned Meeting aggregate, durable operation identity, deterministic relative destination and per-artifact publication journal through `MeetingRepository` before I/O. The journal binds a random temp name, ownership token, opened inode and pre-rename integrity across crashes; the storage module keeps directory/file descriptors through FFmpeg and no-clobber publication. SQLite remains lifecycle and operation authority; a folder, sidecar, IndexedDB row or UI snapshot is never evidence that capture completed.

**Tech stack:** Rust/Tauri 2, SQLx/SQLite migrations and transactions, macOS directory-relative filesystem calls, `renameatx_np(..., RENAME_EXCL)`, SHA-256, existing FFmpeg/Track encoders, TypeScript/React adapters, Node tests and Playwright.

## Global constraints

- Rust is the sole writer for Recording Session lifecycle, recording/import persistence transitions, Track publication state and operation identity.
- SQLite stores IDs, state, revision, root ID, normalized relative paths, canonical request fingerprint and integrity metadata. New rows never write an absolute recording path or audio blob.
- The durable root registry may store an absolute root outside Meeting rows. A binding is immutable: a different `(absolutePath, st_dev, st_ino)` always receives a new `RecordingRootId`; only the separate current-root pointer changes. Meeting/session/import rows and non-storage callers receive only `RecordingRootId` plus `RelativeRecordingPath`.
- Managed relative paths reject empty paths, absolute paths, `.`/`..`, platform prefixes, NUL, symlink traversal or escape, missing roots and mismatched root filesystem identity.
- Stable recording destination is allocated and persisted before capture. Use `recordings/<meetingId>/<sessionId>`; import uses `imports/<meetingId>/<importId>`. A display title never chooses identity or destination.
- Every retriable operation persists `(operationKind, idempotencyKey)` and a canonical fingerprint of every semantic input. Same key plus same fingerprint returns prior durable work. Same key plus any different semantic input returns non-retryable `idempotency_conflict` and creates nothing.
- Each Track/import artifact has a durable journal row before a writer receives its randomly suffixed sibling temp. The row binds operation, final and temp relative paths and an unguessable ownership token; exclusive temp creation then records `st_dev/st_ino`. Publication states are `allocated -> opened -> prepared -> renamed -> published`: finish bytes, `sync_all`, close, verify length/hash/media metadata, persist `prepared`, exclusive rename, persist `renamed`, parent sync, then persist `published`.
- On macOS, publication uses a small safe wrapper around directory-relative `renameatx_np` with `RENAME_EXCL = 0x00000004`; plain `std::fs::rename`, FFmpeg `-y` to a final path, check-then-rename and delete-before-rename do not satisfy the contract.
- Microphone and system Tracks are mandatory for every Recording Session, including when the legacy `auto_save` preference is false, and publish individually. The preference now controls optional compatibility projections only; UI copy must say so. A Recording Session reaches `completed` only when one SQLite transaction records every expected published Track and integrity, transcript rows, the completed session revision, and an optimistic Meeting revision/visibility transition.
- An existing final file is reused only when persisted operation identity, destination and integrity all match. Identity/destination mismatch is `idempotency_conflict`; integrity mismatch is `integrity_failed`; neither may overwrite, delete or repair the final.
- After publication the user owns the final file. Missing, moved or changed published media yields `storage_unavailable` or `integrity_failed`; retry/recovery never recreates or removes it.
- `metadata.json`, `transcripts.json`, mixed `audio.mp4` and folder existence are optional compatibility projections, not capture truth. Authoritative SQLite/Tracks complete first. Projection attempts have their own durable state/error and unique owned publication intent; failure is surfaced as a retryable projection error but never rolls back or falsifies Recording Session completion. A published projection is user-owned and is not recreated if later moved/changed.
- Legacy `meetings.folder_path` is read-only migration input. Do not rewrite, move or delete a legacy recording. New recording/import rows keep `folder_path = NULL` and use the new tables.
- Existing Intel CPU and arm64 behavior, separate raw Tracks, optional mixed compatibility audio, preferences UI and downstream processing semantics remain unchanged except for path resolution and persistence authority.
- No product commit is created until one frozen staged diff receives independent SPEC and QUALITY PASS with Critical 0 / Important 0. The product implementation is exactly one non-merge commit; local integration and SDD closeout are separate later steps.

## Current planning authority and execution gates — 2 October 2026

- Rebound planning base: `main@53f4415058fb01e38b8892a47ac036bfffa394ba`, tree `bb8b8bdda6b53deaa926362f59b661410479ef78`. The current checkout contains the uncommitted plan/scanner overlay; it is not an admitted clean implementation or release base.
- FORK-034/057 are closed in the canonical PRD and their source is integrated. The old STOP that waited for the Round 7 patch to be committed is obsolete. Do not reapply that patch or use the old `d7ca2990` foundation line as execution authority.
- Original plan SHA-256: `2bda589971d6e6cf6bfbbc4e170325afc1b34db2b64a058263be880886620689`. The original ignored plan is preserved at `.worktrees/macos-fork-foundation/.superpowers/sdd/macos-fork/fork-043-plan.md`; this tracked revision replaces its old base/roots/path assumptions, not its storage acceptance contract.
- FORK-126 produces this rebind/census only. FORK-127 must independently review these exact bytes with SPEC and QUALITY, Critical 0/Important 0. FORK-063/043 and their implementation children remain open. No product storage edit is performed or authorized by this document's checked planning status.
- Before implementation, select and verify a clean product checkout/base against current main and the project's agreed dev/feature workflow. Record its exact commit/tree and replace `$STORAGE_TASK_ROOT` with that absolute task root. A changed source/path/contract or a dirty/moving destination repeats census and independent review; preserve unrelated overlays.
- Commit, integration, push, PR, signing, packaging/native launch and release need their own applicable authorization. A proposed future commit command is not authorization. Historical `fork/macos-integration`/foundation refs are preservation sources, not current destinations.

The source census now includes the reviewed, uncommitted FEAT-100 overlay copied
into main: the new selected-meeting context reader and tests plus API command
registration. Commit/tree above remain the immutable base; current source hashes
and differing base hashes are explicit in the census. This is still a dirty
planning snapshot, never an admitted clean product/release checkout.

## Verified current and future-state gaps

- FORK-043 PRD requires new recording, repeat start, import, concurrent save and recovery; SQLite relative path/state; atomic rename; and no duplicate on retry (see FORK-043 in `docs/prd/macos-fork.md`).
- The accepted architecture requires persisted operation identities/fingerprints (architecture contract sections 6-8), small Rust seams without absolute paths, root-relative SQLite and exact publication/recovery ordering, and the root-relative/atomic/idempotent invariants.
- Current `RecordingSession` still owns an in-memory `SessionStore`; `StartRecordingRequest.save_path`, `RecordingSessionSnapshot.save_path` and `CapturedTrack.path` are absolute strings. Restart loses operation identity and recovery authority.
- Current storage uses minute-resolution title folders through `create_dir_all`, fixed JSON and recovery temporary names, replace-capable rename, FFmpeg `-y`, and in-memory completion. Two equal titles in a minute, import versus recording, concurrent finalizers or two recoveries can collide.
- Current SQLite `meetings.folder_path`, transcript save, summary, diarization, retranscription and Finder commands interpret an absolute folder directly. UI stop and IndexedDB recovery remain parallel persistence/path authorities.
- Current import ignores the selected Recording Root, copies directly to a final filename, commits SQLite separately, treats sidecar write failures as warnings, and may recursively remove a shared collided directory on cancellation.

## Rebind findings and cross-feature compatibility

The [census](2026-10-02-recording-storage-census.json) records every matched path,
line and hash, all 62 proposed paths, existing migration order and source equality.
It is a bounded source inventory, not proof that the future storage contract is
implemented. Current Meeting lifecycle/provenance and search are already present;
retain `status`, `started_at_ms`, `title_source`, `lifecycle_operation` and
`lifecycle_run_id`. New visibility and optimistic revisions extend these facts;
recording/import completion must not steal summary/retranscription ownership.

1. The next migration is proposed as `20261002000000_recording_root_storage.sql`,
   after existing `20260926000000_add_meeting_search_index.sql`. Do not insert the
   old August migration before already-applied migrations or rewrite their bytes.
   Run every existing SQLx prefix preservation fixture plus the new prefix;
   preserve legacy folder_path, notes, summaries, speaker data and search output.
2. An immutable registry binding identifies the full `(absolute_path, st_dev,
   st_ino)` tuple. Use a UNIQUE tuple, not UNIQUE absolute_path or UNIQUE inode:
   both historical constraints contradicted new-ID-on-path/inode-drift. Preserve
   old referenced roots; same path with replaced inode gets a new root ID while
   old sessions fail closed against the old identity. Test same inode at a moved
   path, path replacement, symlinks and concurrent selection in real temp storage.
3. Search is an ordinary user-visible query. New hidden recording/import rows
   must be excluded from index rebuild and search, then become visible only with
   successful revision-checked completion. Characterize title/transcript/summary
   search, rollback and close/reopen; this adds paths 54-55. Existing visible-text
   filtering and atomic accepted-summary indexing remain intact.
4. Summary metadata and Markdown now have active file writers not covered by the
   old 53-path plan. Paths 56-58 make that seam explicit. Keep FEAT-186–188 accepted
   SQLite summary/owner/export-pending semantics and per-meeting language behavior.
   Immutable capture Tracks/projections remain no-clobber and user-owned after
   publication. Derived results use the immutable version/SQLite pointer contract
   below; current mutable `summary.md`/metadata writers are baseline gaps, not a
   waiver. Exports remain automatic and pending until verified publication; a
   NULL legacy folder does not mean export is disabled.
5. Model directories, app database pickers, migration adapters and incidental
   folder tokens are classified separately. The ID-only Finder frontend already
   delegates to Rust; retain it. UI may display the selected root's absolute path,
   but new per-artifact managed paths stay root-relative and opaque across IPC.

6. Encoder/decoder paths 59-60 explicitly cover the current FFmpeg descriptor boundary. The decoder
   currently creates scratch beside its absolute input; a `/dev/fd` locator is not
   automatically compatible with that implementation. Classify unmanaged scratch
   separately, use the verified input/sink contract, and reject unsupported descriptor codecs without a pathname fallback.
   Before/after inode checks cannot prevent a child from writing through a swapped
   symlink; they cannot admit a writer. `RENAME_EXCL=0x00000004` and
   the local SDK `renameatx_np` declaration were checked in `sys/stdio.h`; executable
   race, fsync, descriptor lifetime and supported-codec evidence remain future gates.

The steps below preserve the complete original registry, revisions, journal,
no-clobber, cancellation, inode, retry/recovery, SQLite and legacy no-move
requirements. Any conflict found while resolving the summary seam requires plan
amendment and fresh review, never dropping a storage invariant to make a gate pass.

## Domain decisions and interfaces

Canonical terms retain the accepted architecture meaning: Recording Root is a registered user-selected root; Relative Recording Path is a validated path below it; Recording Session is one capture attempt; Track is one durable microphone/system artifact; import is a separate keyed operation. A Meeting folder is only a destination namespace. It does not prove devices opened, bytes arrived, expected Tracks closed, integrity passed or SQLite completion committed.

The production interface to add in `frontend/src-tauri/src/audio/recording_storage.rs` is deliberately small:

```rust
pub struct RecordingRootId(String);
pub struct RelativeRecordingPath(PathBuf);
pub struct RootBinding { pub id: RecordingRootId, /* absolute path and dev/inode stay private */ }
pub struct Integrity { pub byte_length: u64, pub sha256: String }
pub struct PublishedArtifact { pub relative_path: RelativeRecordingPath, pub integrity: Integrity }
pub struct ArtifactIntentId(String);

impl RelativeRecordingPath {
    pub fn parse(value: &str) -> Result<Self, StorageError>;
}

impl RecordingStorage {
    pub fn resolve(&self, root: &RecordingRootId, relative: &RelativeRecordingPath)
        -> Result<ResolvedPath, StorageError>;
    pub fn open_owned_temp(&self, intent: &ArtifactIntent)
        -> Result<OwnedTemp, StorageError>;
    pub fn prepare(&self, temp: OwnedTemp)
        -> Result<PreparedArtifact, StorageError>;
    pub fn publish_no_clobber(&self, prepared: PreparedArtifact)
        -> Result<PublishedArtifact, StorageError>;
    pub fn reconcile(&self, expected: &ExpectedArtifact)
        -> Result<ReconciledArtifact, StorageError>;
}
```

`ResolvedPath` and absolute `PathBuf` fields are private implementation details and may be exposed only to the Finder adapter. Directory traversal uses owned directory descriptors, `openat`/`mkdirat` with `O_NOFOLLOW`, `fstat` identity checks, `unlinkat` and directory-relative `renameatx_np`; validation followed by an unrelated string-path operation is not sufficient. The FFmpeg adapter receives inherited, already-open input/output descriptors and explicit formats, clears `FD_CLOEXEC` only for the enumerated child descriptors, addresses them as `/dev/fd/<n>` on macOS, closes them after `wait`, and never receives a managed absolute pathname. For a codec/demuxer that rejects descriptor I/O, return typed
`unsupported_descriptor_codec` before allowing a pathname writer; preserve
SQLite pending state and owned input bytes. There is no pathname output fallback.
If seeking is required, use an already-open seekable owned file descriptor.
Tests swap the original temp name to a symlink during child execution and assert
that the unrelated victim's bytes/inode remain unchanged, not merely that a
post-exit integrity check rejects publication. Real codec/FFmpeg descriptor
compatibility remains a future executable gate; no native PASS is asserted.

## Derived-output versions and accepted-summary ownership

This contract applies to summary Markdown and its language/metadata exports,
separately from immutable capture Tracks and capture compatibility projections.
SQLite remains the accepted content/owner authority. Every accepted automatic
summary, explicit manual summary edit and per-meeting language change commits
its existing FEAT-186–188 ownership checks and accepted content in the same
transaction as a durable immutable export version and pending publication intent.
No successful acceptance may clear pending simply because `folder_path` is NULL.

Add `meeting_derived_versions` with `version_id` primary key, `meeting_id` foreign
key, `derived_kind`, `accepted_revision`, `lifecycle_run_id` when applicable,
`payload_json`, `payload_sha256`, `recording_root_id`, `relative_directory`,
`summary_intent_id`, `metadata_intent_id`, `state`, `error_code`, and `revision`.
UNIQUE `(meeting_id, derived_kind, accepted_revision)` and UNIQUE
`(recording_root_id, relative_directory)` make retries return the same version.
Add `meeting_derived_heads` keyed by `(meeting_id, derived_kind)` with accepted
version ID, last published version ID and revision. Foreign keys protect versions
and their journal intents; deleting a Meeting never deletes its published files.
Persist the complete accepted Markdown and language metadata snapshot in SQLite
so restart needs no current mutable source file. Payload changes under the same
version ID are `idempotency_conflict`, never a rewritten export.

`MeetingRepository::accept_derived(expected_owner, expected_meeting_revision,
payload)` performs the existing ownership/revision checks, updates accepted SQLite
summary/language and search index, inserts the version/intents and advances the
accepted-head revision atomically. Recording/import completion cannot claim that
owner. Export I/O occurs after commit. Manual edits use their explicit edit revision,
not a summary lifecycle token. Losing/stale workers cannot accept content or mark
another version exported. A language-only change creates a new snapshot without
rerunning inference or discarding accepted summary text.

`RecordingStorage::publish_derived(version_id)` resolves persisted root ID plus
relative directory and publishes distinct immutable
`derived/<meeting_id>/<version_id>/summary.md` and `metadata.json` through the same
journal, descriptor, integrity, fsync and exclusive-rename protocol as other owned
artifacts. It never overwrites a published version, legacy summary/metadata, or
capture projection. The current version is a SQLite pointer, not a mutable
`latest` file or symlink. Finder/export adapters resolve that version through Rust;
UI content continues to read accepted SQLite text. Legacy rows with folder paths
use one read-only legacy locator and registered binding for a distinct derived
version subdirectory, without moving/replacing legacy files. New NULL-folder rows
use their persisted recording/import root locator. A legacy Meeting without a
valid locator records `storage_unavailable` and stays pending while its accepted
SQLite summary remains usable; it does not silently succeed or choose a new root.

After both files are verified published, one revision/accepted-version checked
transaction advances the published-head pointer and clears only that version's
pending flag. A newer accepted version cannot be cleared by an older finisher;
an older valid exported version may remain in history without becoming current.
Crash after either rename resumes only its matching journal/token/inode and
payload hash; crash after both publications before pointer commit performs the
same conditional transaction on reopen. No file operation rolls back accepted
SQLite content or the accepted search index.

For a published version, changed/moved/deleted files are user-owned: reconciliation
records `integrity_failed`/`storage_unavailable`, leaves pending/conflict visible,
and never overwrites or recreates them. Retry of that version only verifies existing
files and retries an incomplete owned unpublished intent. A user explicitly
requesting a new export/version allocates a new ID/path; that is not recovery of
a deleted version. Preserve both existing bytes and accepted content. Surface
export-pending/conflict through the existing summary export outcome, without
reporting model success as export success or mutating another lifecycle run.

Task 7 must add real temporary-storage/SQLite close-reopen tests for automatic
NULL-folder export, stale owner/CAS, manual edit and language-only versions,
external edits/deletion, duplicate retry and crash at each file/pointer boundary.
Assert user/victim bytes unchanged, old pointer correctness, accepted summary and
search preserved, and pending cleared only for the current fully published version.
These tests are future admission gates, not results of this planning amendment.

The following schema sketch names required fields/constraints; it is not an executable migration. The migration must use valid CREATE/ALTER statements and be tested against every existing prefix:

```sql
ALTER TABLE meetings ADD COLUMN lifecycle_visibility TEXT NOT NULL DEFAULT 'visible';
ALTER TABLE meetings ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
recording_roots(
  recording_root_id PRIMARY KEY, absolute_path, st_dev, st_ino,
  binding_kind, created_at, UNIQUE(absolute_path, st_dev, st_ino)
)
recording_root_preference(singleton_id PRIMARY KEY CHECK(singleton_id = 1), recording_root_id, revision)
recording_sessions(
  session_id PRIMARY KEY, meeting_id REFERENCES meetings(id) ON DELETE CASCADE,
  operation_kind, idempotency_key, UNIQUE(operation_kind, idempotency_key),
  request_fingerprint, recording_root_id, relative_folder_path,
  state, revision, meeting_created_by_operation, expected_tracks_json, created_at, updated_at,
  UNIQUE(recording_root_id, relative_folder_path)
)
recording_tracks(
  session_id REFERENCES recording_sessions(session_id) ON DELETE CASCADE,
  track_kind, relative_file_path, publication_state,
  byte_length, sample_count, sample_rate, channels, sha256,
  PRIMARY KEY(session_id, track_kind)
)
recording_imports(
  import_id PRIMARY KEY, meeting_id REFERENCES meetings(id) ON DELETE CASCADE,
  operation_kind, idempotency_key, UNIQUE(operation_kind, idempotency_key),
  request_fingerprint, recording_root_id, relative_folder_path,
  relative_file_path, state, revision, meeting_created_by_operation, source_byte_length, source_sha256,
  published_byte_length, published_sha256, created_at, updated_at,
  UNIQUE(recording_root_id, relative_folder_path)
)
recording_artifact_intents(
  intent_id PRIMARY KEY, operation_kind, operation_id, artifact_kind,
  recording_root_id REFERENCES recording_roots(recording_root_id) ON DELETE RESTRICT,
  temp_relative_path UNIQUE, final_relative_path, ownership_token UNIQUE,
  temp_st_dev, temp_st_ino, stage, byte_length, sha256, media_metadata_json, revision,
  UNIQUE(recording_root_id, final_relative_path),
  UNIQUE(operation_kind, operation_id, artifact_kind)
)
recording_projections(operation_kind, operation_id, projection_kind, state, error_code, revision,
  PRIMARY KEY(operation_kind, operation_id, projection_kind))
```

All state/visibility/kind values receive SQL `CHECK` constraints matching Rust reducers. Existing Meetings migrate to `visible, revision=0`; a newly created Meeting begins `hidden`, while an operation on an existing Meeting preserves its visibility. Ordinary list/detail/history queries return only `visible`; recovery queries join hidden Meetings through their nonterminal operation. `MeetingsRepository` owns `begin_recording`, `begin_import`, artifact-intent transitions, `list_recoverable`, and `complete_recording(expected_session_revision, expected_meeting_revision, ...)`. That one transaction verifies every intent is `published`, upserts the exact Track set and transcripts, moves the session to `completed`, increments both revisions, and changes a new hidden Meeting to visible. A stale revision rolls back all writes. Repository deletion refuses any Meeting with a nonterminal session/import; after that guard, Meeting deletion cascades session/import/Track/intent/projection rows but never deletes published user files. Recording Roots are `RESTRICT` and cannot be removed while referenced.

## Reconciled product allowlist: 62 unique paths

Create only:

1. `frontend/src-tauri/migrations/20261002000000_recording_root_storage.sql`
2. `frontend/src-tauri/src/audio/recording_storage.rs`
3. `frontend/src-tauri/tests/recording_storage.rs`

Modify only:

4. `frontend/scripts/release-policy.mjs`
5. `frontend/tests/release-policy.test.mjs`
6. `frontend/src-tauri/src/audio/mod.rs`
7. `frontend/src-tauri/src/audio/audio_processing.rs`
8. `frontend/src-tauri/src/audio/common.rs`
9. `frontend/src-tauri/src/audio/incremental_saver.rs`
10. `frontend/src-tauri/src/audio/recording_saver.rs`
11. `frontend/src-tauri/src/audio/recording_session.rs`
12. `frontend/src-tauri/src/audio/recording_commands.rs`
13. `frontend/src-tauri/src/audio/recording_preferences.rs`
14. `frontend/src-tauri/src/audio/import.rs`
15. `frontend/src-tauri/src/audio/retranscription.rs`
16. `frontend/src-tauri/src/database/models.rs`
17. `frontend/src-tauri/src/database/repositories/meeting.rs`
18. `frontend/src-tauri/src/database/repositories/transcript.rs`
19. `frontend/src-tauri/src/database/setup.rs`
20. `frontend/src-tauri/src/state.rs`
21. `frontend/src-tauri/src/api/api.rs`
22. `frontend/src-tauri/src/summary/commands.rs`
23. `frontend/src-tauri/src/summary/service.rs`
24. `frontend/src-tauri/src/diarization/coordinator.rs`
25. `frontend/src-tauri/src/lib.rs`
26. `frontend/src-tauri/tests/data_integrity.rs`
27. `frontend/src-tauri/tests/recording_recovery.rs`
28. `frontend/src-tauri/tests/diarization_runtime.rs`
29. `frontend/src/services/recordingService.ts`
30. `frontend/src/services/storageService.ts`
31. `frontend/src/services/indexedDBService.ts`
32. `frontend/src/hooks/useRecordingStart.ts`
33. `frontend/src/hooks/useRecordingStop.ts`
34. `frontend/src/hooks/useImportAudio.ts`
35. `frontend/src/hooks/useTranscriptRecovery.ts`
36. `frontend/src/contexts/TranscriptContext.tsx`
37. `frontend/src/app/meeting-details/page.tsx`
38. `frontend/src/app/meeting-details/page-content.tsx`
39. `frontend/src/components/MeetingDetails/TranscriptButtonGroup.tsx`
40. `frontend/src/components/MeetingDetails/TranscriptPanel.tsx`
41. `frontend/src/components/MeetingDetails/RetranscribeDialog.tsx`
42. `frontend/src/components/TranscriptRecovery/TranscriptRecovery.tsx`
43. `frontend/src/types/index.ts`
44. `frontend/src/testing/tauri-browser-mocks.ts`
45. `frontend/tests/lib/storage-service.test.mjs`
46. `frontend/tests/lib/import-audio-hook.test.mjs`
47. `frontend/tests/lib/transcript-recovery-hook.test.mjs`
48. `frontend/tests/lib/indexeddb-service.test.mjs`
49. `frontend/tests/react-hooks-lifecycle.test.mjs`
50. `frontend/tests/interrupted-recovery.spec.ts`
51. `frontend/tests/indexeddb-transcript-contract.spec.ts`
52. `frontend/tests/recording-folder-selection.spec.ts`
53. `frontend/src/components/RecordingSettings.tsx`

54. `frontend/src-tauri/src/database/repositories/search.rs` — Ordinary search must not expose newly hidden Meetings; rebuild/query and completion transition are one visibility contract.

55. `frontend/src-tauri/tests/meeting_search.rs` — Prove hidden recording/import does not appear by title/transcript/summary until authoritative completion; rollback and reopen preserve visibility.

56. `frontend/src-tauri/src/summary/metadata.rs` — Summary preference/metadata file IO must not bypass registered-root resolution or silently overwrite user-owned capture projections.

57. `frontend/src-tauri/src/summary/persistence.rs` — Existing explicit summary export/replacement behavior needs a reviewed derived-output seam, distinct from immutable Track publication.

58. `frontend/src-tauri/src/database/repositories/summary.rs` — Existing lifecycle_run_id summary ownership must survive new visibility/revision and export resolution; characterize existing writes before any change.


59. `frontend/src-tauri/src/audio/encode.rs` — Current FFmpeg encoder receives an absolute output path and owns child descriptors; admit only verified storage-sink adaptation.

60. `frontend/src-tauri/src/audio/decoder.rs` — Current decoder creates scratch beside its input and reopens an absolute path; verified descriptors cannot use this path-only API unchanged.
61. `frontend/src-tauri/src/database/repositories/meeting_context.rs` — FEAT-100 selected-meeting reader must reject hidden rows; preserve single-meeting transaction and stable source links.
62. `frontend/src-tauri/tests/meeting_context.rs` — Add hidden selection rejection and visibility-after-completion/rollback/close-reopen characterization without weakening existing isolation tests.

The census reconciles this list to current main; it remains proposed mutation scope until FORK-127 accepts these exact bytes and Task 0 admits a clean implementation base. No Cargo dependency/lockfile, capture backend, model, updater, identity, signing, broad UI or unrelated cleanup path may be added implicitly. If a genuinely required caller exists on the admitted base, amend the list, freeze a new plan SHA and repeat both plan reviews before product editing.

## Canonical execution roots and Task 0: admit current source

- `$MEETILY_REPO_ROOT` is the canonical task/plan checkout; `$STORAGE_TASK_ROOT` is the separately admitted clean implementation checkout; `$ADMITTED_INTEGRATION_ROOT` is the separately agreed integration destination. Set each to an absolute path and quote it in shell commands. No historical checkout is selected automatically.
- Git, pnpm, Node and Cargo commands must use those exact roots; do not mix generated state with root's stale dependencies. Dependencies must match the selected checkout's lockfile. Do not run Next build concurrently with Playwright.
- Re-read FORK-043/063/126/127 and all implementation children in `docs/prd`; revalidate the PRD dependency graph. FORK-126 completed means census/rebind only, not SPEC/QUALITY approval.
- Verify main's full commit/tree and current schema, clean/idle admitted source and destination, source equality for all census paths, and exact scope; do not stash/reset unrelated work. Recompute this plan/census hashes for the review snapshot.
- Repeat the full storage token census and SQL/registered-command caller census recorded in [the census JSON](2026-10-02-recording-storage-census.json). Reconcile every discovered path as owner, consumer, legacy migration input, test or unrelated resource; any required new path stops implementation for amendment/review.
- Require fresh independent SPEC and QUALITY PASS on this plan and the reconciled 62 paths, including the new search/summary seams below. Resolve every explicit design stop before product changes. Only then select/create the product checkout from the exact admitted base.

## Task 1: make the storage and schema contracts executable (RED)

**Files:** create paths 1-3; modify paths 4-5, 16-18 and 26.

- [ ] First add a compile-safe `recording_storage` module exporting the exact value/interface names above and returning typed `StorageError::NotImplemented`; register it. In `release-policy.mjs`, export a side-effect-free `validateRecordingStoragePolicy(root)` behind a correct direct-CLI guard; its scaffold returns an empty finding list. Add the final migration names but no caller. These sentinels make RED behavioral, never import/compile failure.
- [ ] Add exact tests `relative_recording_path_rejects_every_escape`, `root_identity_rejects_replacement_and_symlink_escape`, `recording_schema_hides_incomplete_meeting_and_never_persists_absolute_managed_path`, `recording_completion_is_one_meeting_revision_transaction`, `identical_recording_identity_returns_durable_operation`, `changed_recording_fingerprint_conflicts_without_side_effects`, and import equivalents.
- [ ] Add policy mutations for absolute managed writes, arbitrary path recovery/cleanup IPC, final FFmpeg `-y`, replace-capable rename and unmanaged allocation. Allow only the root-registry absolute field and legacy read adapter; comment/string/dead-function decoys and a caller outside the configured product root must fail.
- [ ] Run each RED by absolute root, for example `cargo test --manifest-path $STORAGE_TASK_ROOT/Cargo.toml --test recording_storage relative_recording_path_rejects_every_escape -- --exact --nocapture` and the analogous exact names above; run `node --test --test-name-pattern='^recording storage policy rejects managed absolute path$' $STORAGE_TASK_ROOT/frontend/tests/release-policy.test.mjs`. Require exactly one selected test and its named `NotImplemented`/policy assertion. A compile, import, toolchain or zero-test result is `NOT_COMPLETED`.

## Task 2: implement root identity, relative values and durable repository (GREEN)

**Files:** paths 1-3, 11, 13, 16-20, 25-26 and 54-55.

- [ ] Implement immutable `RecordingRootId` bindings and a versioned singleton current-root pointer in SQLite. Reselecting the same verified `(path,dev,inode)` reuses the ID; any changed identity gets a new ID. The transaction that changes the pointer commits before the UI reports success; JSON `save_folder` becomes a compatibility mirror, never authority. Migration validates the existing default/custom folder into the first binding without moving bytes. Retain every referenced old binding; missing/inaccessible/replaced roots fail closed.
- [ ] Apply the concrete Meeting visibility/revision, root, operation, Track, intent and projection schema above with all checks/FKs. Begin inserts a hidden Meeting only when needed, records whether this operation created it, and persists operation/destination before filesystem work. Existing populated `folder_path` bytes remain unchanged and existing Meetings stay visible.
- [ ] Implement canonical fingerprints with deterministic typed serialization, not delimiter-concatenated raw strings. Recording includes Meeting, capture preferences, root ID, relative destination and auto-save; import includes Meeting, source size/SHA-256, root ID, relative destination, provider/model/language and every other semantic input. Exclude timestamps, progress and transport-only source path.
- [ ] Implement optimistic repository transitions and ordinary-query visibility. Same key/fingerprint returns the persisted owner across process restart; changed fingerprint creates nothing. Completion with expected Meeting/session revisions atomically records transcripts/full Tracks, advances both revisions and reveals a newly created Meeting. Stale revision and injected SQL failure leave no partial rows. Close/reopen fixtures prove hidden interrupted work is recoverable but absent from ordinary history.
- [ ] Initialize `RecordingSession` only after database/root adapters exist in Tauri setup, and hydrate durable snapshots/list-recoverable state instead of treating its BTreeMap as relaunch authority. Test the real public repository on a closed/reopened SQLite pool.
- [ ] Run every exact Task 1 command GREEN plus `cargo test --manifest-path $STORAGE_TASK_ROOT/Cargo.toml --test data_integrity -- --nocapture`. Add A→B root switching with active/completed A work, pointer-write rollback, same-path inode replacement, concurrent select/start, corrupted registry and relaunch.

## Task 3: implement exclusive durable file publication (RED then GREEN)

**Files:** paths 2-3, 7-10, 27 and 59-60.

- [ ] Before production changes, add close/reopen fault tests at journal allocation, owner-manifest/temp create, opened-inode commit, write, file sync, close, verification, `prepared` commit, exclusive rename, `renamed` commit, parent sync and `published` commit. Add two publishers, existing finals, post-completion mutation and directory/temp/concat-input swaps before/during FFmpeg. Prove recovery identifies only the exact journal+token+inode temp and never cleans by glob.
- [ ] Verify RED: existing final bytes change or the sentinel returns `NotImplemented`; exactly-one-winner, stable error and owned-temp cleanup assertions must fail for the intended reason.
- [ ] Generate and persist random temp relative name plus ownership token before creation. Create temp and owner manifest exclusively, sync their parent, persist opened `dev/inode`, and only then hand out the file. Persist verified integrity/media metadata in `prepared` before rename. Recovery accepts only matching journal/token/inode, handles both valid temp and valid final at every stage idempotently, and never cleans a mismatched name/object.
- [ ] Implement descriptor-relative no-follow traversal and reviewed `renameatx_np(RENAME_EXCL)`. FFmpeg consumes inherited `/dev/fd` inputs/output with explicit format and finite descriptor lifetime; reject unsupported descriptor codecs before any pathname writer. No managed input is reopened from a string path. Test apostrophes/non-UTF8, CLOEXEC, child failure, symlink/inode swaps and exact error mapping.
- [ ] Retrofit source Track finalization/recovery and compatibility JSON helpers through the storage module. Preserve separate microphone/system Tracks. A mixed `audio.mp4` remains a compatibility projection and cannot substitute for expected Track integrity.
- [ ] Run absolute-root exact tests `publication_journal_recovers_every_crash_boundary`, `ffmpeg_uses_verified_descriptors_without_path_reopen`, and `concurrent_no_clobber_publish_has_exactly_one_winner` with `-- --exact --nocapture`, then the full `recording_storage`, library storage and `recording_recovery` targets. Zero selected tests are failure.

## Task 4: make RecordingSession durable from start through completion

**Files:** paths 6-13, 16-20, 25, 27, 29, 32-36, 44, 49 and 53.

- [ ] RED tests cover new recording under default/selected root, response-loss/process restart repeat, changed fingerprint, concurrent start, crash after each Track stage and before/during/after the completion transaction. `auto_save=false_still_publishes_required_raw_tracks` must prove the expected microphone/system set is unchanged; compatibility projection failure is separately durable/retryable and does not change terminal session truth.
- [ ] Replace `StartRecordingRequest.save_path` with root/destination identity resolved in Rust. Snapshots expose `recordingRootId`, `relativeFolderPath` and relative captured Track references; no managed absolute artifact path crosses the RecordingSession interface or IPC.
- [ ] Persist begin-recording before initializing artifacts/capture. Use the stored destination on every retry. Folder creation uses descriptor-relative exclusive creation/reconciliation; title is metadata only.
- [ ] Stop drains capture/writers, publishes every mandatory raw Track, then calls the one revision-checked repository completion transaction. Any file/SQL fault leaves the hidden/new or already-visible Meeting consistent and the operation recoverable. Only after authoritative completion schedule optional sidecar/mixed projections; persist and surface projection failure without changing session completion.
- [ ] UI start preserves one transport idempotency key while the same start action is unresolved/retried; Rust remains authority. UI stop only renders the terminal snapshot/refetches/navigates. Remove the second completion writer and new IndexedDB absolute-folder writes. Change the existing `auto_save` label/help to state that disabling it disables only compatibility exports, never mandatory separate raw Tracks; do not add a new preference or redesign Settings.
- [ ] Run focused RecordingSession public-seam tests, `node --test frontend/tests/react-hooks-lifecycle.test.mjs`, and `pnpm --dir frontend exec playwright test tests/indexeddb-transcript-contract.spec.ts` to GREEN.

## Task 5: route import through the same identity and publisher

**Files:** paths 2, 7-10, 13-14, 16-18, 21, 25, 29-30, 34, 44-46.

- [ ] RED tests prove selected-root import, source immutability, durable allocation, hidden Meeting visibility, response-loss/relaunch retry and collision safety. Add `import_hook_reuses_key_and_cancels_exact_import_id`, source rename/symlink/mutation during hash/copy, wrong/stale cancel ID, and import/recording concurrency.
- [ ] `useImportAudio` creates one key per unresolved user action, reuses it for transport retry, stores the returned `importId`, and cancels only that ID. Rust opens the selected external source once with no-follow, derives fingerprint and copies/hashes from that same descriptor, compares initial/final `fstat` plus final hash, and never mutates it. Persist destination before copy; the external pathname remains transient IPC input only.
- [ ] Delete duplicate Meeting insertion and use the same aggregate transaction/journal. Cancellation or failure persists typed state and removes only a journal-proven unpublished temp. Authoritative import completion is not rolled back by optional projection failure; that failure is persisted and returned separately.
- [ ] Run absolute-root Rust import targets plus `node --test --test-name-pattern='^import hook reuses key and cancels exact import id$' $STORAGE_TASK_ROOT/frontend/tests/lib/import-audio-hook.test.mjs` GREEN.

## Task 6: replace path-authority recovery with persisted reconciliation

**Files:** paths 2, 9, 11-12, 16-20, 25, 27, 31, 34-35, 41, 43 and 45-49.

- [ ] RED restart matrix reconstructs from a closed/reopened database: no files, owned temp only, microphone final only, both finals before DB commit, completed DB commit, gapped/malformed checkpoint set, matching final, mismatched final, missing/moved user-owned final and two concurrent recovery calls. Assert raw inputs remain until the one completion transaction succeeds.
- [ ] Make recovery accept only `sessionId` for new sessions, load persisted root/destination/expected Track integrity, and serialize through the RecordingSession action gate plus optimistic revision. `list_recoverable` comes from SQLite. Duplicate/concurrent recovery returns the existing authoritative snapshot and never creates another operation or file.
- [ ] Remove public arbitrary-path recovery/check/cleanup commands. Cleanup takes persisted owner ID and removes only owned unpublished checkpoints after durable completion. Folder presence, `metadata.json`, mixed audio or an IndexedDB row is not completion evidence.
- [ ] Retain one legacy adapter. For each existing valid no-symlink historical locator, transactionally create/reuse an immutable `binding_kind='legacy'` root for its actual historical parent `(path,dev,inode)` and a normalized relative child, regardless of the current root. Never rewrite `folder_path` or move bytes. Missing/moved media stays a legacy pending locator with typed availability; malformed/symlink locators fail closed. Test old default, two previous custom roots, current-root change, repeated adoption, missing/moved files and rollback.
- [ ] Run the absolute-manifest `recording_recovery` target, absolute Node recovery/IndexedDB paths, and `pnpm --dir $STORAGE_TASK_ROOT/frontend exec playwright test tests/interrupted-recovery.spec.ts tests/indexeddb-transcript-contract.spec.ts`.

## Task 7: route every reader through the resolver and remove bypasses

Also cover paths 54-62: verified FFmpeg descriptors/scratch, hidden-row search and the immutable derived-output version/SQLite pointer contract above; add matching rollback/reopen and summary-owner/export regression evidence.

**Files:** paths 4-5, 12, 15-25, 28-31, 34, 36-43, 46-50 and 54-62.

- [ ] Extend `MeetingContextRepository::read` to select only visible meetings in its existing snapshot; selecting a hidden ID fails like missing, never reveals capture/import text or falls back. Test hidden selection, successful visibility transition, rollback and reopen, preserving all four FEAT-100 scenarios.
- [ ] Replace direct `folder_path` interpretation in Finder open, summary metadata, diarization recording resolution, retranscription and meeting details with one repository locator plus storage resolution. New records return typed availability/integrity facts or a boolean capability, never an absolute managed folder to the WebView.
- [ ] Retranscription accepts `meetingId`, not `meetingFolderPath`. Recording details/buttons use `hasRecording`/typed availability. Finder open resolves internally by Meeting ID. Keep legacy absolute resolution inside the single repository adapter only.
- [ ] Preserve recording-root settings display and folder picker: showing the configured root path is allowed, while individual managed artifact paths are not. A root-selection save remains serialized and rollback-safe; concurrent preference saves cannot roll back a winner.
- [ ] Extend policy and mutation tests to prove no new SQL write to `folder_path`, no `savePath`/`folderPath` managed IPC, no arbitrary recovery cleanup path, no direct reader `PathBuf::from(folder_path)`, and no capture-completion inference from folder/JSON existence. Include comment/string/dead-code decoys.
- [ ] Run Node with the four absolute test paths, Playwright with the absolute frontend `--dir`, affected absolute-manifest Rust targets, and `pnpm --dir $STORAGE_TASK_ROOT/frontend check:policy`.

## Task 8: full verification, staged reviews and one product commit

- [ ] Run sequentially with only exact roots: `git -C $STORAGE_TASK_ROOT diff --check`; Cargo fmt/full named targets via the absolute manifest from the canonical-roots section; Node tests by absolute file path; every pnpm lint/typecheck/unit/build/policy/Playwright command via the absolute frontend `--dir`. Repeat every exact RED/GREEN name with `-- --exact` or anchored Node pattern and require count 1 before the full target.
- [ ] Run `$STORAGE_TASK_ROOT/scripts/check.sh --quick` and `--full` separately. If native dependencies/Xcode/FFmpeg block a gate, preserve exact output and mark `NOT_COMPLETED`; never substitute search evidence.
- [ ] Run a fresh static census proving new SQLite rows and wire snapshots contain no absolute managed paths, only the single legacy adapter reads `meetings.folder_path`, Track publication has no replace-capable final rename/FFmpeg `-y`, and arbitrary path cleanup commands are absent.
- [ ] Record evidence separately for unit repository behavior, real SQLite close/reopen, real macOS no-clobber race, injected crash boundaries, import, legacy compatibility, UI browser projections and packaged FFmpeg recovery. Tests do not prove physical disk power-loss durability; label that limitation honestly.
- [ ] Stage exactly the reconciled allowlist, freeze `git diff --cached --binary` SHA/count/status, run `git diff --cached --check`, and obtain independent staged SPEC and QUALITY reviews on those exact bytes. Review must focus path traversal/TOCTOU, unsafe FFI, fsync/rename semantics, ownership cleanup, SQL transaction order, idempotency fingerprints, restart/concurrency and legacy no-move compatibility.
- [ ] After both reviews PASS Critical 0 / Important 0 and explicit commit authorization, create exactly one non-merge product commit with subject `feat(macos): make recording storage root-relative and atomic`. Do not amend, merge, push, publish or close the PRD.

## Task 9: local fast-forward integration and separate SDD closeout

- [ ] Confirm integration still equals the admitted base and is clean. After explicit local integration authorization, fast-forward only to the reviewed commit. Replace the task-worktree root in every Task 8 command with `$ADMITTED_INTEGRATION_ROOT`, then repeat diff identity, scope, exact focused counts, full gates and topology.
- [ ] In a separate documentation change at canonical `$MEETILY_REPO_ROOT/docs/prd`, run `mdtask --path "$MEETILY_REPO_ROOT/docs/prd" view FORK-043`, mark complete only what fresh evidence proves, add concise Russian `Implemented` bullets and update the feature description. Review and commit documentation separately. Never claim physical power-loss, broad upgrade or FORK-052 end-to-end recovery evidence unless it actually ran.

## Principal risks and controls

- **Wrong filesystem primitive:** ordinary rename replaces on macOS. Use reviewed `renameatx_np` `RENAME_EXCL`, exact `EEXIST` handling and an executable two-publisher race.
- **Path validation TOCTOU:** canonicalize-then-open can escape after validation. Keep root/destination directory descriptors and perform no-follow, directory-relative operations through the storage module.
- **FFmpeg path reopen:** keep verified input/output descriptors through the child and use `/dev/fd`; pathname output fallback is forbidden, including before/after inode revalidation. Unsupported descriptor codecs fail closed before any pathname writer.
- **Crash between files and SQLite:** individual Track publication is intentionally not cross-file atomic. Persist expectations first; reconcile partial finals; expose completed only from one full-set SQL transaction.
- **Unowned random temp after restart:** persist temp name/token before create, opened inode before write, and integrity before rename; cleanup requires every recorded identity to match.
- **Idempotency that only exists in memory:** prove identical retry after pool/process reconstruction and reject changed fingerprints before any filesystem action.
- **Deleting user or neighbor data:** operation ownership is required for cleanup. Never recursively delete a title/timestamp folder or published final.
- **Legacy compatibility becoming a bypass:** legacy absolute paths are read-only and isolated to one adapter; new writes and managed IPC cannot use them.
- **Import drift:** import must use the selected root, the same publisher/repository and the same retry rules; its global in-process guard is not durable identity.
- **Folder-as-truth regression:** all completion/recovery assertions use persisted expected Tracks plus integrity. Directory, sidecar or mixed-audio existence alone is never sufficient.
- **Scope collision with FORK-052:** FORK-043 implements durable storage identity/publication and the required folder recovery cases. Bounded ordered UI observation and full processing-job relaunch remain FORK-045/FORK-052 and must not be invented here.
