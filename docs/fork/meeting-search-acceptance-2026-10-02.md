# Integrated meeting search acceptance — 2026-10-02

FEAT-076 acceptance is verified against
`main@53f4415058fb01e38b8892a47ac036bfffa394ba` using the existing integration
candidate, whose inspected product and test files match main byte-for-byte.
The [census](evidence/meeting-search-acceptance-2026-10-02.json) contains their
SHA-256 values and the actual focused test results.

No donor patch was transferred in this step. FEAT-094–096 are already integrated;
this verifies their parent acceptance. It does not declare release readiness.

| Original criterion | Evidence | Result |
|---|---|---|
| Local search finds titles, transcripts and visible summaries | `search_covers_visible_title_transcript_and_summary_only` runs the production repositories against temporary SQLite with all migrations | PASS |
| Internal JSON and hidden translations are excluded | The same test rejects hidden translation, stale Markdown and internal-data terms; the Markdown test excludes link URLs and formatting | PASS |
| Result shows meeting, time and source excerpt | Backend test asserts meeting ID, transcript ID, time 42.5 and matching excerpt; Sidebar renders source label, formatted time and `matchContext`; Node verifies link construction | PASS |
| Navigation opens the correct meeting and transcript time | Three browser cases cover title/summary/transcript; transcript case verifies `transcriptId`, `at=600`, active location, viewport visibility, a page offset greater than zero and actual scroll | PASS |
| Removed meetings do not return after reopen | `search_index_tracks_changes_and_deleted_meetings_after_reopen` updates title/transcript, rejects old terms, deletes the meeting, reopens SQLite, rebuilds the index and verifies absence | PASS |
| Accepted summaries and their index stay consistent | Two additional Rust tests cover a late old generation and rollback when index refresh fails | PASS |

Focused runs, all exit 0:

- `cargo test --locked -p meetily --test meeting_search -- --nocapture`: 5 passed.
- `node --test tests/lib/meeting-search.test.mjs`: 3 passed.
- `playwright test tests/meeting-search.spec.ts --workers=1`: 3 passed,
  no skipped or flaky cases. The existing Chromium cache was used.

Rust tests use real production repository code and an isolated temporary
database. Browser tests exercise the real frontend with fixture Tauri IPC;
they do not exercise the installed application's native IPC or user profile.
No real user data was accessed or deleted. Installed `.app` acceptance,
packaged permissions/audio, offline, signing and notarization remain separate
open gates. No product code or dependencies changed in this verification.
