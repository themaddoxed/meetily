# Speaker Labels and Rename Design

Date: 2026-07-26
Target: Meetily 0.4.1 Alpha release candidate
Status: Approved through the user's instruction to choose recommended options and proceed

## Purpose

Close the release-gate failure caused by the absence of visible speaker labels and a working speaker rename operation. The implementation completes the existing hidden diarization backend handoff without changing transcript text or timing.

## Scope

- Carry existing speaker metadata through frontend transcript types and pagination.
- Render speaker label, color, overlap, and final/provisional status in the virtualized transcript.
- Let the user rename a detected speaker inline from Meeting Details.
- Rename every occurrence of that speaker inside one meeting in one database transaction.
- Refresh the transcript after success and preserve the old UI/data after failure.

Merge, split, voice enrollment, cross-meeting identity, and automatic real-name inference remain outside this release slice. They stay follow-up review tools from the broader diarization design.

## Approaches Considered

### Selected: meeting-scoped speaker identity

The UI sends `meeting_id`, stable `speaker_id`, and the requested label. Rust validates the request, and the repository transaction updates raw speaker segments and aligned transcript rows for exactly that meeting and speaker.

This matches the existing data model, keeps speaker names as meeting metadata rather than biometric identity, and has a narrow rollback boundary.

### Rejected: edit each transcript row

Row-by-row edits can leave one speaker with inconsistent names and create partial state after a failure.

### Rejected: global speaker directory

Cross-meeting identities would introduce voice identity and privacy semantics not approved for 0.4.1 Alpha.

## User Experience

Each attributed transcript segment shows a compact colored speaker label above its text. Final labels appear without extra noise; provisional labels receive a subtle status marker. Overlap renders as localized `Multiple speakers` and cannot be renamed.

Selecting a normal speaker label opens a small inline editor:

- the current label is prefilled;
- Enter or Save submits;
- Escape or Cancel closes without mutation;
- blank, control-character, or overlong input is rejected locally and again in Rust;
- the control is busy while the transaction runs;
- success refetches the meeting transcript;
- failure keeps the editor and current label visible with a concise error.

Keyboard focus returns to the triggering label after cancellation or completion.

## Data and Command Contract

Frontend `Transcript` and `TranscriptSegmentData` retain:

- `speaker_id`;
- `speaker_label`;
- `speaker_color`;
- `is_overlap`;
- `diarization_status`;
- `diarization_confidence` when present.

Pagination and virtualization must not synthesize or drop these fields.

The rename command accepts a structured request. Validation rules are identical at UI and command boundaries:

- meeting and speaker identifiers must be nonblank;
- labels are trimmed;
- labels must contain 1 through 128 Unicode scalar values;
- control characters are rejected;
- the generic overlap identity cannot be renamed.

The repository transaction verifies the meeting/speaker pair exists, updates both `speaker_segments` and transcript denormalized labels, and reports not-found separately from storage failure. It does not alter transcript text, timing, summary, other speakers, or other meetings.

## Error Handling

Stable user-facing errors distinguish invalid label, missing speaker, and persistence failure. SQL details remain in local logs. A failed transaction rolls back all label changes.

## Tests and Acceptance Evidence

- Frontend mapping tests prove speaker metadata survives pagination.
- Transcript component tests cover normal, provisional, unknown, and overlap presentation.
- UI tests cover save, cancellation, keyboard behavior, busy state, and rejected names.
- Rust integration tests use a real temporary SQLite database, rename one speaker, reopen through a new pool, and prove persistence.
- Rollback/preservation tests prove text, timing, other meetings, and the previous label survive an injected failure.
- The complete repository check passes.
- Packaged-app smoke renames a speaker, restarts the app, and observes the same label in the reopened meeting.

## Release Boundary

The UI can render existing metadata before a real engine exists, but the release gate passes only when a real diarization run produces labels and the packaged-app rename/reopen scenario succeeds.
