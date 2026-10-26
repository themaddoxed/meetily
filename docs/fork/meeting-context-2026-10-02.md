# Selected meeting context — FEAT-100, 2026-10-02

The five reviewed files were copied without byte changes from
`.worktrees/main-features-2026-10-02` into the main working tree based on
`53f4415058fb01e38b8892a47ac036bfffa394ba`. Independent SPEC and QUALITY
review passed with Critical 0 / Important 0 / Minor 0. These local changes are
not committed. Exact hashes and checks are in the
[census](evidence/meeting-context-2026-10-02.json).

## Source applicability

Read the pinned ActuallyFree source at
[`4e1527d`](https://raw.githubusercontent.com/TylerBuza/Meetily-ActuallyFree/4e1527dca64339f9bb21acbc191ebc21db99af2d/frontend/src-tauri/src/live_assistant.rs),
its API, command registration and architecture. GitHub API returned 403;
reading exact raw files succeeded without credentials. The local repository
does not contain that donor commit object.

The donor live assistant accepts frontend-supplied transcript text and permits
general-knowledge answers. Its person assistant instead loads attributed data
in Rust. Neither is a direct single-meeting context transfer for this task.
The prepared implementation follows FEAT-100's selected-meeting constraint
through the current SQL schema; no donor provider, person mapping or prompt
policy is copied. Provider selection and insufficient-evidence behavior remain
FEAT-101/102 work, not proof delivered by this context module.

## Reviewable implementation

- `MeetingContextRepository::read` requires a nonblank meeting ID and reads its
  title and full transcript rows in one SQLite transaction. Both queries bind
  that ID; there is no global search, caller-supplied corpus or fallback meeting.
- Each nonblank source retains its stored transcript ID, text and timestamp.
  Its link uses URL query encoding for meeting/transcript IDs and the existing
  meeting-details transcript anchor. Invalid audio time is omitted rather than
  represented as a made-up time. Text is not paginated or silently truncated.
- `api_get_meeting_question_context` is registered as a read-only Tauri command.
  It prepares context only. It does not call a model, send a network request,
  modify the schema, persist a conversation or change credentials.

The four real temporary-SQLite tests passed: a selected meeting with 160 rows
excludes its neighbor; unusual IDs round-trip in stable links; missing/deleted/
blank selection never falls back; empty text and unusable time do not invent
evidence. `cargo fmt --all --check` and Clippy for the library and context test
with `-D warnings` also exited 0. No installed native command invocation was run.

## Accepted scope and remaining work

Independent review inspected the five exact candidate files, original FEAT-100
scope and four integration tests, including snapshot consistency, selected-ID
isolation, link compatibility, deletion and empty context. Future FEAT-101 must apply the
selected model's context budget explicitly; FEAT-102 must verify answer/source
isolation and stale-context handling. Full Q&A UI and model inference have not
been implemented or accepted here.
