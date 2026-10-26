# Meeting Markdown Export UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Implement task-by-task with red-green-refactor and verify every commit.

**Goal:** Add a visible native Markdown export that writes the complete reopened meeting atomically.

**Architecture:** A pure TypeScript formatter and orchestrator fetch all transcript rows and construct Markdown. A small React hook owns per-click loading/toasts. A Rust Tauri command owns the native save dialog and delegates bytes to the existing atomic writer.

**Tech Stack:** React 18, TypeScript, Vitest, Tauri 2, Rust, `tauri-plugin-dialog`, existing export writer.

---

## Task 1: Pure complete-meeting Markdown serialization

**Files:**
- Create: `frontend/src/lib/meeting-export.ts`
- Create: `frontend/tests/lib/meeting-export.test.ts`
- Modify: `frontend/src/types/index.ts`

- [ ] Add RED tests for ordered timestamps, Unicode, speaker labels, generic overlap, missing summary, and heading-safe titles.
- [ ] Run `cd frontend && pnpm exec vitest run tests/lib/meeting-export.test.ts`; expect module-not-found failure.
- [ ] Add `formatMeetingMarkdown`, `formatRecordingTime`, and a typed export input. Preserve transcript/summary text and normalize only document structure.
- [ ] Add the existing backend speaker fields to `Transcript` so serialization is type-safe.
- [ ] Re-run the focused Vitest file; expect all cases pass.
- [ ] Run `cd frontend && pnpm typecheck`; expect pass.
- [ ] Commit `feat(export): serialize complete meetings as markdown`.

## Task 2: Native save command with cancellation semantics

**Files:**
- Modify: `frontend/src-tauri/src/export.rs`
- Modify: `frontend/src-tauri/src/lib.rs`
- Modify: `frontend/src-tauri/tests/export_integrity.rs`

- [ ] Add RED Rust tests for suggested-name sanitization, `.md` normalization, Unicode names, and unsafe empty/path-component names.
- [ ] Run `cd frontend/src-tauri && cargo test -p meetily --test export_integrity -- --nocapture`; expect the new helper tests to fail to compile.
- [ ] Add `ExportMeetingOutcome` and `export_meeting_markdown`. Use `DialogExt::file().add_filter(...).set_file_name(...).blocking_save_file()`; map `None` to `Cancelled`.
- [ ] Convert the selected `FilePath` to a local path, normalize `.md`, and call `write_export_atomically`. Log internal I/O errors and return one sanitized export error.
- [ ] Register the command in `generate_handler!` and do not remove the legacy `save_transcript` command.
- [ ] Re-run `export_integrity`; expect pass.
- [ ] Run focused `rustfmt --edition 2021 --check` on changed Rust files and `cargo check -p meetily --tests`; expect pass.
- [ ] Commit `feat(export): add native atomic markdown save command`.

## Task 3: Visible Export action and complete-data orchestration

**Files:**
- Create: `frontend/src/hooks/meeting-details/useMeetingExport.ts`
- Modify: `frontend/src/hooks/meeting-details/useCopyOperations.ts`
- Modify: `frontend/src/app/meeting-details/page-content.tsx`
- Modify: `frontend/src/components/MeetingDetails/TranscriptPanel.tsx`
- Modify: `frontend/src/components/MeetingDetails/TranscriptButtonGroup.tsx`
- Create: `frontend/tests/meeting-export-contract.test.mjs`

- [ ] First extract/export one `fetchAllMeetingTranscripts` helper from the copy flow and keep Copy behavior unchanged.
- [ ] Add a RED source-contract test requiring a visible Export label, `FileDown` icon, `export_meeting_markdown` invocation, disabled empty/busy state, and explicit handling of `cancelled` without an error toast.
- [ ] Run `cd frontend && node --test tests/meeting-export-contract.test.mjs`; expect failure because wiring is absent.
- [ ] Implement `useMeetingExport` as an event-driven action, not an Effect. Fetch all transcripts, obtain current summary Markdown through the existing BlockNote fallback path, format, invoke Rust, and guard stale completion with the current meeting id.
- [ ] Thread `onExportMeeting` and `isExporting` through `PageContent` and `TranscriptPanel` into `TranscriptButtonGroup`.
- [ ] Add the visible button next to Copy/Recording, track analytics, show success only for `saved`, and keep `cancelled` silent.
- [ ] Run the focused contract test, export Vitest tests, `pnpm lint`, and `pnpm typecheck`; expect pass.
- [ ] Commit `feat(export): expose native meeting export action`.

## Task 4: Export checkpoint

- [ ] Run `cd frontend/src-tauri && cargo test -p meetily --test export_integrity -- --nocapture`.
- [ ] Run `cd frontend && pnpm test:frontend && pnpm lint && pnpm typecheck`.
- [ ] Run repository-root `./scripts/check.sh --full`.
- [ ] Run `git diff --check` and inspect that no meeting persistence mutation was added to the export flow.
- [ ] Commit any narrowly required test/documentation correction as `test(export): verify packaged meeting export contract`.
