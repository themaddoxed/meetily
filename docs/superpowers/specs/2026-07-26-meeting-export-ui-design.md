# Meeting Markdown Export UI Design

Date: 2026-07-26
Target: Meetily 0.4.1 Alpha release candidate
Status: Approved through the user's instruction to choose recommended options and proceed

## Purpose

Close the release-gate failure caused by the absence of a visible Export action. A reopened meeting must be exportable from Meeting Details without changing or deleting any persisted meeting data.

## Scope

- Add a visible `Export` action to the existing transcript action group.
- Export the complete meeting, not only the currently paginated transcript rows.
- Write UTF-8 Markdown containing meeting metadata, speaker-aware transcript text, and the saved summary when present.
- Use the native macOS save dialog and the existing atomic filesystem writer.
- Treat dialog cancellation as a normal result and filesystem failure as a visible error.

PDF, DOCX, batch export, and automatic cloud upload are out of scope for this alpha gate.

## Approaches Considered

### Selected: frontend serialization plus Rust-owned save dialog

The frontend already knows the displayed meeting and can fetch all transcript rows and summary content. A pure formatter creates deterministic Markdown. A focused Tauri command opens the native save dialog and delegates the final write to `export::write_export_atomically`.

This keeps formatting testable without a Tauri runtime while ensuring that cancellation, path selection, and atomic replacement remain desktop responsibilities.

### Rejected: browser download

A static Next.js browser download would not prove native packaged-app filesystem behavior and would bypass the existing atomic writer.

### Rejected: frontend dialog plugin plus arbitrary path command

This would spread path handling and dialog capabilities into the webview even though the Rust application already initializes `tauri-plugin-dialog` and can own the complete operation.

## User Experience

`Export` appears beside Copy and Recording in `TranscriptButtonGroup`.

1. The user selects Export.
2. The button shows a busy state while the app loads all transcript rows and builds Markdown.
3. A native save dialog suggests a sanitized meeting-title filename ending in `.md`.
4. Closing the dialog returns to the meeting with no error toast.
5. A successful write produces a success toast.
6. Serialization, database, permission, or filesystem failures produce a concise error toast and leave the source meeting unchanged.

The action is disabled only while its own export is running. Copy, playback, transcript navigation, and summary rendering stay unchanged.

## Data Contract

The Markdown formatter is a pure frontend function with inputs:

- meeting title and creation time;
- the complete ordered transcript collection;
- optional saved summary Markdown.

Speaker labels are emitted when present. Overlap rows use the persisted generic `Multiple speakers` label. Transcript text and summary text are escaped only where required to keep Markdown headings structurally valid; user content is otherwise preserved.

The Rust command receives only a suggested filename and finished UTF-8 content. It returns a tagged outcome:

- `saved`, optionally with the selected path for diagnostics;
- `cancelled`.

Internal filesystem details are logged locally. User-facing command errors are sanitized.

## Tauri Integration

The command uses Tauri 2's Rust `DialogExt` API and `blocking_save_file()`, where `None` is cancellation. The dialog filters Markdown files and normalizes the selected extension. The existing dialog plugin registration and default save permission are reused.

The file is written only through `export::write_export_atomically`; no partial destination file should survive a failed write.

## Tests and Acceptance Evidence

- A frontend unit test proves full ordered Markdown output with speakers, overlap, Unicode, and optional summary.
- A frontend unit test proves a meeting with no summary still exports a valid document.
- Rust tests prove filename sanitization, extension normalization, and that the command-independent writer preserves an existing destination when a write fails.
- A component test proves the visible Export control invokes the export flow and cancellation is silent.
- The complete repository check passes.
- Packaged-app smoke exports a reopened real meeting, opens the saved Markdown, and compares its transcript and summary with the source meeting.

## Release Boundary

Passing unit tests closes only the code path. The release row passes only after the action succeeds in the exact packaged candidate and the exported artifact is inspected.
