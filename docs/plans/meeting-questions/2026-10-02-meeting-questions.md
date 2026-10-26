# FEAT-101 — Selected-meeting questions implementation plan

Base: main 53f4415 plus exact reviewed FEAT-100 source overlay. Implement in
`.worktrees/main-features-2026-10-02`; guard root owned bytes before transfer.
No actual network inference, credentials inspection, install, commit or push.
Parent FEAT-078 and verification FEAT-102 remain separate acceptance gates.

## Required result

Ask a question on the selected meeting. Show the actual selected model/provider
and local/network transport. Return source links to stored transcript rows;
explicitly show insufficient evidence. Never supply other meetings or global
search, accept caller-supplied corpus, silently truncate, or persist this chat.

## Owned files and route

- New `frontend/src-tauri/src/meeting_questions.rs` and `tests/meeting_questions.rs`.
  Native configuration loads SettingsRepository; IPC carries meeting ID, question,
  request ID, native configuration fingerprint and explicit network consent, never a key,
  endpoint or transcript. A sanitized native config command returns model/provider,
  transport and an opaque fingerprint over provider/model/normalized endpoint and
  generation parameters (no credential material). Consent binds that fingerprint,
  so same provider/model with a changed endpoint is rejected before dispatch.
  Read one Settings row/custom JSON snapshot; revalidate config before dispatch.
- Modify `frontend/src-tauri/src/lib.rs` for module and command registration only.
- Modify `frontend/src-tauri/src/summary/summary_engine/client.rs` only to add a
  dedicated built-in generation entry point sharing prompt/model integrity logic.
  Existing summary entry point keeps its global manager behavior. Q&A creates its
  own SidecarManager; cancellation/deadline/drop shuts down that instance, never
  global SIDECAR_MANAGER. Do not add a second model downloader or weaker checksum.
- Also own `summary/summary_engine/sidecar.rs`: add an isolated-only constructor
  mode and bounded force-shutdown method, preserving existing summary methods.
  Isolated children use kill_on_drop and stderr null. Shutdown sets the internal
  stop flag, takes only its child, start_kill then bounded wait/reap; never tries
  a potentially blocking stdin shutdown write first. Release pipe/model handles
  after killing. An owned async guard shuts down on success and every error;
  Drop schedules that same bounded cleanup as cancellation fallback. Prevent new
  isolated startup after shutdown was requested. Keep the global summary manager
  untouched. Tests use real temporary helper processes (no model/network) and
  assert PID termination/reaping for success/error/startup failure/cancel/deadline/
  dropped future, plus a separate summary fixture remaining alive.
- New `frontend/src/components/MeetingDetails/MeetingQuestions.tsx`; connect in
  `frontend/src/app/meeting-details/page-content.tsx`; add en/ru keys in
  `frontend/src/lib/i18n/catalog.ts`. No auto-generation or transcript/key logging.
- New focused browser contract `frontend/tests/meeting-questions.spec.ts` using
  existing real meeting-details fixture and mocked IPC; no new dependency.

## Native contract

Read fresh FEAT-100 snapshot from SQLite. Question length bounded; the full JSON
corpus is budgeted before inference, never partly dropped. Built-in byte budget
uses selected catalog context_size minus output and template reserves as a
conservative upper token bound. Network uses an explicit app request ceiling,
not an invented model context claim; provider rejection is surfaced as failure.
Empty transcript returns insufficient evidence without inference.

Prompt treats transcript text as untrusted data, not instructions. Require strict
JSON with insufficient_evidence or a bounded list of answer claims, each carrying
transcript IDs and exact quoted spans. Reject unknown IDs, missing/empty/invalid
quotes or inconsistent insufficient/claims output. Server creates links from the
snapshot; model URLs/links are never trusted/rendered. Quote matching validates
attribution, not semantic entailment: show evidence alongside the generated
answer, and do not claim genuine model faithfulness without FEAT-102 inference.

Retain request-scoped cancellation tokens with RAII cleanup and duplicate-ID
rejection. Await the generation future to finish cancellation cleanup; a timeout
cancels the same token rather than dropping a shared process request. Each
request uses the existing LLM HTTP adapter; external errors returned to the UI
are sanitized because response bodies can contain provider echoes. Never log
question, corpus, response, auth or custom endpoint. Built-in errors sanitized too.

After generation reread exactly this meeting and compare deterministic serialized
snapshot/hash; deletion or edits reject stale answer. Return actual provider/model,
transport and validated source objects. Concurrent UI requests cannot replace a
new selection's answer; unmount/selection change cancels only its request. No
shared summary cancellation registry or summary persistence/lifecycle mutation.

Local means built-in or an explicitly parsed loopback endpoint; remote Ollama,
custom endpoints and cloud APIs are network. UI requires an explicit consent
checkbox before network invocation and describes what is sent. Native enforces
consent too. Configuration and endpoints come from existing SQLite settings.

## Evidence and admission

Write focused RED tests for neighboring meeting exclusion, config mismatch,
empty/no-answer behavior, prompt data boundaries, full-context overflow, malformed
model JSON/unknown IDs/invalid quotes, stale/deleted meeting, request collision
and cancellation/deadline cleanup. Use real temporary SQLite for isolation and
staleness; fake generator for deterministic replies, no secrets or real inference.
Browser checks provider/transport, consent, answer/insufficient/error, stale request
routing, cancel and real transcript anchor navigation beyond page one. Run fmt,
focused Rust+Clippy, frontend typecheck/lint and the focused browser contract in
candidate exact dependencies. Independent SPEC/QUALITY must inspect exact diff.
Only then guard and copy owned files to root. These tests do not establish actual
model accuracy, native packaged inference or release readiness; FEAT-102 stays open.

QUALITY round1 identified process cleanup and consent-binding gaps; both are
addressed above. Exact amended plan requires fresh independent acceptance.


## Integration harness amendment — 2026-10-04

Explicitly add `frontend/tests/react-hooks-lifecycle.test.mjs` to owned integration
scope: add only the `MeetingQuestions` dependency Symbol beside the existing
`TranscriptPanel`/`SummaryPanel` mocks in `loadPageContent`. The five unchanged
auto-summary scenarios failed at import before assertions on the 14-path QA
transfer preview; targeted RED was 5 failed, and the one-line harness correction
made those same five PASS without skips or assertion changes. Retain the first
full gate failure and run the required full gate on the corrected 15-path
candidate after the current isolated initial run reaches terminal state.
[Evidence](../../fork/evidence/qa-autosummary-harness-fix-2026-10-04.json).
No product source, timeout, runner configuration or acceptance oracle changes.
