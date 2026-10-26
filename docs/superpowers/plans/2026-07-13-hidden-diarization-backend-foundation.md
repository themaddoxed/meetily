# Hidden Diarization Backend Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate and harden the local speaker-diarization backend foundation without exposing a user-facing feature before a real diarization engine exists.

**Architecture:** Port the ordered v0.4.2 Rust domain, alignment, persistence, settings, and apply/handoff slices onto clean `dev`, treating every webview command as untrusted. SQLite is the single runtime authority; apply is transactional and rejects malformed or destructive input. Frontend settings, transcript presentation, copy, and rename are deferred until this backend is green.

**Tech Stack:** Rust, Tauri 2 commands, Tokio, SQLx/SQLite migrations, Serde, existing Meetily repository/API patterns.

---

## Fixed scope

- Port source commits `aa5e006`, code-only `d7b0c1c`, `49c3ab5`, code-only `c67308e`, semantic `a5a699b`, test-only `b971a58`, `7222852`, `4c02489`, and backend `6fe2a10`.
- Exclude docs staging, duplicate theme/Gemma, `c0201eb`, merge `8dceceb`, frontend settings/presentation, and rename.
- Do not touch `ConfigContext`, beta settings, `TranscriptSettings`, or visible settings panels.
- DB settings are authoritative. Apply requires `enabled=true` and a non-`off` mode compatible with the explicit run kind.
- Reject empty, cross-meeting, malformed, nonexistent-meeting, and no-alignable-transcript requests before any delete/update. There is no implicit clear command.
- Keep source-compatible generic overlap: `Multiple speakers`, `is_overlap=true`, no participating identities.
- Explicitly remove diarization rows/columns on meeting deletion and retranscription with SQLite foreign keys both enabled and disabled.
- Add no speaker-count cap. Log internal SQL failures locally and return stable, non-sensitive webview errors.

## Task 1: Decision record and scope guard

**Files:**
- Create: `docs/migration/diarization-backend-foundation.md`
- Create: `scripts/check-diarization-backend-migration.sh`
- Modify: `docs/migration/legacy-feature-registry.md`

- [ ] Write a failing guard that requires the decision record, requires the implementation registry row to be `migrating`, and rejects changes to `BetaSettings.tsx`, `TranscriptSettings.tsx`, `ConfigContext.tsx`, and `betaFeatures.ts` relative to base `a75082b`.
- [ ] Run `bash scripts/check-diarization-backend-migration.sh`; expect `missing hidden backend decision record`.
- [ ] Add the fixed-scope decisions above, selected/excluded commit lists, and the deferred frontend plan name `2026-07-13-hidden-diarization-frontend-handoff.md`.
- [ ] Run `bash scripts/check-diarization-backend-migration.sh && git diff --check`; expect `PASS: hidden diarization backend scope is preserved`.
- [ ] Commit `docs(migration): define hidden diarization backend contract`.

The visible-path check must use an exact array and quoted paths:

```bash
for path in "${VISIBLE_PATHS[@]}"; do
  git diff --quiet a75082b...HEAD -- "$path" || exit 1
done
```

## Task 2: Validated domain types and deterministic alignment

**Files:**
- Create: `frontend/src-tauri/src/diarization/{mod.rs,types.rs,alignment.rs}`
- Modify: `frontend/src-tauri/src/lib.rs`

- [ ] Add RED tests for status snake-case round trips, unknown/overlap constructors, and generic overlap identity.
- [ ] Add RED validation tests for blank/cross-meeting IDs; non-finite, negative, and reversed times; confidence outside `0.0..=1.0`; control characters; blank or over-128-character strings; and overlap segments carrying concrete speaker identity.
- [ ] Run `cd frontend/src-tauri && cargo test -p meetily diarization::types::tests -- --nocapture`; expect compile failure because the module is absent.
- [ ] Port `DiarizationStatus`, `SpeakerSegment`, `TranscriptWindow`, and `SpeakerAssignment` from `aa5e006` plus the constructor API from `d7b0c1c`.
- [ ] Add `SpeakerSegment::validate_for_meeting(&self, meeting_id: &str) -> Result<(), DiarizationInputError>` using this bounded error enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiarizationInputError {
    MeetingMismatch,
    InvalidTimeRange,
    InvalidConfidence,
    InvalidText,
    InvalidOverlapIdentity,
}
```

- [ ] Port alignment from `49c3ab5` plus threshold hardening from `c67308e`. Add RED/GREEN cases for largest overlap, exact threshold, no/zero/invalid transcript window, NaN/clamped threshold, and generic overlap.
- [ ] Run `cargo fmt --check`, the focused type tests, and focused alignment tests; expect all pass.
- [ ] Commit `feat(diarization): add validated domain and alignment`.

## Task 3: Constrained persistence and explicit privacy cleanup

**Files:**
- Create: `frontend/src-tauri/migrations/20260626000100_add_diarization_foundation.sql`
- Create: `frontend/src-tauri/src/database/repositories/diarization.rs`
- Create: `frontend/src-tauri/tests/diarization_migration.rs`
- Modify: `frontend/src-tauri/src/database/{models.rs,repositories/mod.rs,repositories/meeting.rs,repositories/transcript.rs}`
- Modify: `frontend/src-tauri/src/{api/api.rs,audio/common.rs,audio/import.rs,audio/retranscription.rs}`

- [ ] Add RED real-migrator tests for fresh DB, upgrade from the last pre-diarization migration, old-row defaults, tables/indexes, meeting deletion, and retranscription. Run cleanup cases with `PRAGMA foreign_keys=ON` and `OFF`.
- [ ] Run `cd frontend/src-tauri && cargo test -p meetily --test diarization_migration -- --nocapture`; expect failure because migration/tables are absent.
- [ ] Port semantic model/API/repository fields from `a5a699b` without its unrelated rustfmt churn, plus only the import fixture hunk from `b971a58`.
- [ ] Add SQL checks: booleans in `(0,1)`; statuses in the serialized enum set; confidence null or `0.0..=1.0`; `end_time > start_time`; mode in `live_plus_post_call`, `post_call_only`, `off`; overlap handling exactly `multiple_speakers`.
- [ ] Add one executor-generic cleanup helper and call it inside existing deletion/retranscription transactions:

```rust
pub async fn clear_meeting_diarization<'e, E>(executor: E, meeting_id: &str)
    -> Result<(), sqlx::Error>
where E: sqlx::Executor<'e, Database = sqlx::Sqlite>;
```

- [ ] The helper deletes `speaker_segments` and `meeting_diarization_status`, then clears transcript diarization columns without changing legacy `transcripts.speaker`.
- [ ] Run migration, repository, deletion, retranscription, and import tests; expect all pass with FK on/off.
- [ ] Commit `feat(diarization): add constrained persistence and cleanup`.

## Task 4: DB-authoritative validated settings

**Files:**
- Modify: `frontend/src-tauri/src/database/repositories/diarization.rs`
- Create: `frontend/src-tauri/src/diarization/commands.rs`
- Modify: `frontend/src-tauri/src/diarization/mod.rs`
- Modify: `frontend/src-tauri/src/lib.rs`

- [ ] Add RED tests for disabled defaults, valid round trip, concurrent saves, invalid mode/overlap values, and `enabled=true` with `mode=off`.
- [ ] Run `cargo test -p meetily diarization::commands::tests::settings -- --nocapture`; expect compile failure.
- [ ] Use typed command-boundary enums rather than arbitrary strings:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationMode { LivePlusPostCall, PostCallOnly, Off }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlapHandling { MultipleSpeakers }
```

- [ ] Port get/save repository behavior from `7222852`; persist only enum database values. Register only backend Tauri commands.
- [ ] Map internal failures to fixed `Unable to load diarization settings` or `Unable to save diarization settings`; never return SQL text.
- [ ] Re-run focused tests and the hidden-scope guard; expect pass and no frontend-visible diff.
- [ ] Commit `feat(diarization): add validated backend settings`.

## Task 5: Validated atomic apply and backend handoff

**Files:**
- Modify: `frontend/src-tauri/src/database/repositories/diarization.rs`
- Modify: `frontend/src-tauri/src/diarization/commands.rs`
- Modify: `frontend/src-tauri/src/lib.rs`
- Create: `frontend/src-tauri/tests/diarization_apply.rs`

- [ ] Add RED tests for disabled/off settings, incompatible run kind, nonexistent meeting, empty input, any cross-meeting segment, every malformed segment field, no alignable transcript, rollback on insert/update/status failure, replacement success, deterministic segment IDs, status precedence, and preservation of existing data after every rejected request.
- [ ] Run `cargo test -p meetily --test diarization_apply -- --nocapture`; expect failure because apply is absent.
- [ ] Add an explicit run kind; never infer live/post-call behavior from a method substring:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationRunKind { Live, PostCall }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyDiarizationRequest {
    pub meeting_id: String,
    pub method: String,
    pub run_kind: DiarizationRunKind,
    pub segments: Vec<SpeakerSegment>,
}
```

- [ ] Validate before opening a transaction: settings/run kind, meeting existence, bounded/control-free strings, non-empty same-meeting segments, every segment validator, and at least one finite positive transcript window.
- [ ] Return only stable codes/messages: `feature_disabled`, `invalid_request`, `meeting_not_found`, `no_alignable_transcripts`, `persistence_failed`.
- [ ] Inside one transaction replace segment rows, upsert meeting status from the explicit run kind, assign transcript windows, and commit. Any failure rolls back. Rejected input exits before any `DELETE`.
- [ ] Run apply integration tests and all focused diarization tests; expect pass.
- [ ] Commit `feat(diarization): add validated atomic application`.

## Task 6: Backend checkpoint verification

**Files:**
- Modify: `docs/migration/diarization-backend-foundation.md`
- Modify: `docs/migration/legacy-feature-registry.md`
- Modify: `scripts/check-diarization-backend-migration.sh`

- [ ] Run `cargo fmt --check` and `cargo check -p meetily --tests`.
- [ ] Run `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer cargo test -p meetily`; expect the full Rust suite pass.
- [ ] Run the backend scope guard and `git diff --check`; expect pass and no visible frontend settings paths.
- [ ] Record destination commits for every selected source slice. Explicitly defer `dea4578`, `29b24fc`, `28e1bb0`, `d2db258`, `2576ba8`, frontend `6fe2a10`, and `0160d84` to `2026-07-13-hidden-diarization-frontend-handoff.md`.
- [ ] Keep the combined implementation registry row `migrating`. Record only the backend checkpoint as verified, state that no engine or visible UI exists, and do not claim complete diarization.
- [ ] Commit `docs(migration): verify hidden diarization backend checkpoint`.

## Self-review

- Every approved backend policy maps to Tasks 1–6; frontend presentation/rename is explicitly deferred.
- No placeholder steps remain; every code task names exact files, RED/GREEN commands, expected outcomes, and commit boundary.
- `DiarizationMode`, `OverlapHandling`, `DiarizationRunKind`, `SpeakerSegment`, and error names are consistent across tasks.
- `ConfigContext`, beta settings, visible settings, semantic theme, and recording-folder code remain outside the plan.
- Each task has focused verification; Task 6 has the full Rust gate.
