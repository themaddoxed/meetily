# Speaker Labels and Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Implement task-by-task with red-green-refactor and verify every commit.

**Goal:** Render persisted speaker metadata and provide an atomic, meeting-scoped rename action that survives restart.

**Architecture:** Existing API speaker fields flow unchanged through pagination into virtualized rows. A reusable label editor invokes a validated Rust command. SQL updates transcript and raw speaker-segment labels in one transaction.

**Tech Stack:** React 18, TypeScript, Vitest/source-contract tests, Tauri 2, Rust, SQLx/SQLite.

---

## Task 1: Preserve speaker metadata through frontend mapping

**Files:**
- Modify: `frontend/src/types/index.ts`
- Modify: `frontend/src/hooks/usePaginatedTranscripts.ts`
- Modify: `frontend/src/components/MeetingDetails/TranscriptPanel.tsx`
- Create: `frontend/tests/lib/transcript-speakers.test.ts`

- [ ] Export `convertTranscriptsToSegments` and add RED tests proving every speaker/diarization field survives both paginated and non-paginated conversion.
- [ ] Run `cd frontend && pnpm exec vitest run tests/lib/transcript-speakers.test.ts`; expect missing fields or inaccessible mapper failure.
- [ ] Add nullable speaker fields to `TranscriptSegmentData` and copy them without synthesis in both conversion sites.
- [ ] Re-run the focused tests and `pnpm typecheck`; expect pass.
- [ ] Commit `feat(speakers): preserve transcript attribution metadata`.

## Task 2: Atomic meeting-scoped rename repository

**Files:**
- Modify: `frontend/src-tauri/src/database/repositories/diarization.rs`
- Create: `frontend/src-tauri/tests/speaker_rename.rs`

- [ ] Add RED real-SQLite tests: successful rename updates transcript and speaker-segment rows; other speakers/meetings and transcript text/timing stay unchanged; missing speaker updates zero rows; injected second-update failure rolls back the first update; reopened pool sees the new label.
- [ ] Run `cd frontend/src-tauri && cargo test -p meetily --test speaker_rename -- --nocapture`; expect failure because repository API is absent.
- [ ] Implement `DiarizationRepository::rename_speaker` in one transaction. Count matching transcript and raw segment rows before mutation, reject overlap/null identity, update both tables, and commit only when at least one matching row exists.
- [ ] Re-run the integration test; expect pass.
- [ ] Commit `feat(speakers): rename one meeting speaker atomically`.

## Task 3: Validated Tauri rename command

**Files:**
- Modify: `frontend/src-tauri/src/diarization/commands.rs`
- Modify: `frontend/src-tauri/src/lib.rs`
- Modify: `frontend/src-tauri/tests/speaker_rename.rs`

- [ ] Add RED tests for blank meeting/speaker ids, blank label, more than 128 Unicode scalar values, control characters, generic overlap identity, missing speaker, and sanitized persistence error.
- [ ] Run the focused integration test; expect validation cases to fail to compile.
- [ ] Add camelCase `RenameSpeakerRequest`, one shared `normalize_speaker_label`, and `rename_speaker_with_pool` for integration tests. Return stable `invalid_request`, `speaker_not_found`, or `persistence_failed` messages without SQL text.
- [ ] Register `rename_speaker` in the Tauri handler.
- [ ] Re-run focused tests, focused Rust formatting, and `cargo check -p meetily --tests`; expect pass.
- [ ] Commit `feat(speakers): expose validated rename command`.

## Task 4: Transcript label presentation and inline editor

**Files:**
- Create: `frontend/src/components/MeetingDetails/SpeakerLabelEditor.tsx`
- Modify: `frontend/src/components/VirtualizedTranscriptView.tsx`
- Modify: `frontend/src/components/MeetingDetails/TranscriptPanel.tsx`
- Create: `frontend/tests/speaker-label-ui-contract.test.mjs`

- [ ] Add a RED source-contract test for colored labels in both simple and virtualized rows, provisional marker, generic non-editable overlap, Enter/Save, Escape/Cancel, 128-character validation, busy state, `rename_speaker`, refetch, and failure toast.
- [ ] Run `cd frontend && node --test tests/speaker-label-ui-contract.test.mjs`; expect failure because the editor is absent.
- [ ] Implement `SpeakerLabelEditor` with event-handler-owned async state. Reset draft when speaker/meeting changes and ignore late completion after a meeting change.
- [ ] Pass one `onSpeakerRenamed` callback into `VirtualizedTranscriptView`; render the same memoized segment component in both list modes so behavior cannot drift.
- [ ] Do not allow rename for overlap or missing `speaker_id`; keep keyboard focus behavior explicit.
- [ ] Re-run the contract and mapping tests, `pnpm lint`, and `pnpm typecheck`; expect pass.
- [ ] Commit `feat(speakers): render and rename transcript speakers`.

## Task 5: Speaker checkpoint

- [ ] Run all focused frontend and `speaker_rename` tests.
- [ ] Run `./scripts/check.sh --full` from repository root.
- [ ] Run `git diff --check` and inspect the SQL transaction plus both virtualized/non-virtualized render paths.
- [ ] Use a temporary database to rename, reopen, and compare transcript text/timing before and after.
- [ ] Commit only a narrowly required correction as `test(speakers): verify rename persistence and presentation`.
