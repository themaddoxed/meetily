# Remaining macOS Alpha acceptance — 2026-10-02

This is the FORK-174 planning matrix, not a completed smoke report or release
verdict. Canonical task status and dependency edges remain in
[macos-fork.md](../prd/macos-fork.md) and
[fork-features.md](../prd/fork-features.md). Operator procedures are in
[testing.md](../testing.md).

## Current source and evidence boundary

Local main is `53f4415058fb01e38b8892a47ac036bfffa394ba`, tree
`bb8b8bdda6b53deaa926362f59b661410479ef78`. The integration report and full logs
are preserved locally under `.worktrees/integration-backups-2026-10-02/`.
They show 424 Node, 61 Vitest, 29 browser and 603 Rust tests (10 ignored),
all-target Clippy, typecheck, lint, production frontend build, and one FFmpeg
recovery scenario passing. The initial full script exited 1 with two failed
Rust stages; repaired all-target Clippy and workspace tests then exited 0.
Only three authorization/status documents changed after the tested code tree.

These are integration-stage results, not a successful `check.sh --full` run on
the final Alpha commit. No exact final Alpha app/DMG, completed native smoke,
Developer ID, Gatekeeper or notarization evidence is supplied by that report.
Older dirty/ad-hoc packages and tests remain historical evidence. Their hashes
or permission observations must not be attributed to this main commit.

## Source and product requirements

Every original FORK-053 blocker is retained below. Open rows are not considered
accepted because similar implementation or unit tests exist.

| Original blocker | Evidence still required | Artifact or environment | Current boundary |
| --- | --- | --- | --- |
| FORK-035 | Reviewed macOS CI plan, workflow implementation and actual CI/artifact readback (FORK-143–147) | Current workflow files; separately authorized CI run | Open; local tests do not prove hosted CI |
| FORK-036 | Final identity/channel, updater configuration and real signed update (FORK-157–159, FORK-173) | Old/new signed apps and updater manifest/signature | Open; owner decision and keys required |
| FORK-038 | Platform scanner with negative fixture and final source/package inspection (FORK-148–149) | Exact tracked source and Universal package | Open; FORK-035 remains a dependency |
| FORK-039 | Current theme plan/review, prepaint/hydration/focus acceptance (FORK-150–153) | Current frontend and real/browser UI per PRD | Open; existing browser suite is not full theme acceptance |
| FORK-040 | Downloader reuse verification and local Gemma/settings runtime (FORK-133–134, FORK-167) | Local verified model, Intel and arm64 app runs | Open; model unit tests are supplemental |
| FORK-042 | Visible diarization settings, persistence/failure behavior and native acceptance (FORK-140–141, FORK-170) | Local model, known multi-speaker recording and exact app | Open; export FEAT-089 still depends on this task |
| FORK-043 | Reviewed storage plan and start/import/restart/concurrency/recovery acceptance (FORK-126–131) | Isolated recording root and old/new schema control data | Open; existing SQLite/FFmpeg tests do not close all paths |
| FORK-044 | Preserve accepted short-phrase Whisper behavior | Current worker and meaningful regression evidence | Canonical task closed; integration tests retain coverage; live smoke still required |
| FORK-047 | System menus, commands and window behavior (FORK-162–164) | Exact native UI and keyboard/window scenarios | Open; FEAT-071 dependency retained |
| FORK-048 | Current main flows in agreed design (FORK-165–166, FORK-171) | Recording, editing, processing and export UI | Open; dependencies retained |
| FORK-050 | Benchmark-backed local-model roadmap (FORK-169) | Approved real corpus, Intel/arm64 measurements and roadmap | Open; FORK-049 dependency retained |
| FORK-051 | Exact artifact/test profile, TCC deny/recovery, physical device changes and stop/Quit/relaunch (FORK-119–122) | Isolated accounts, microphone/output, native UI | Open; test account/baseline and current artifact evidence missing |
| FORK-052 | Capture/recovery, packaged offline and actual WebView/SQLite upgrade (FORK-132, FORK-142, FORK-172) | Exact app and backed-up control profiles | Open; previous unit/browser results are supplemental |

## Final candidate and architecture gates

| Task | Required evidence | Environment | Current status |
| --- | --- | --- | --- |
| FORK-175 | All original blockers closed; authorized clean reviewed exact commit; complete `./scripts/check.sh --full` exit 0 | Final Alpha source, dependencies and build toolchain | Not ready: original blockers remain open |
| FORK-176 | `./scripts/release-candidate.sh`; candidate-report.json, exact app/DMG hashes, Universal slices, resources and minimum macOS 14.2, normal clean provenance | Final commit with Intel CPU and arm64 acceleration routing | NOT_RUN for final Alpha; old diagnostic packages are not substitutes |
| FORK-177 | Developer ID, Gatekeeper and notarization; final hashes frozen for smoke | Separately authorized signing/trust operations on exact app/DMG | NOT_RUN; ad-hoc is not PASS |
| FORK-178 | All release smoke rows with concrete evidence | Real Intel Mac, physical audio, test account and verified local models | NOT_RUN for final Alpha |
| FORK-179 | Same Universal artifact; all release smoke rows with concrete evidence, including model/acceleration behavior | Real Apple Silicon Mac and equivalent test inputs | NOT_RUN; Intel evidence cannot satisfy arm64 |
| FORK-180 | Run on actual minimum macOS 14.2; both architecture reports and exact-artifact release gate verdict | Minimum supported OS plus Intel/arm64 evidence | NOT_RUN; Info.plist deployment target alone is insufficient |

## Exact-artifact smoke coverage

Use one completed [release smoke report](../release/smoke-report-template.md)
per tested architecture, with the same final commit and Universal DMG hash.
Every row below is still `NOT_RUN` for the final Alpha. For every row retain
operator, OS, architecture, artifact identity and concrete visible/file/log
readback; a copied unit-test result cannot satisfy it.

| Smoke requirement | Owning tasks | Proof to record | Artifact/device | Final Alpha status |
| --- | --- | --- | --- | --- |
| SMOKE-LAUNCH | FORK-176, FORK-178, FORK-179 | DMG-installed app, startup log and visible window | Exact Universal app on Intel and arm64 | NOT_RUN |
| SMOKE-CLEAN-PERMISSIONS | FORK-119, FORK-120, FORK-178, FORK-179 | Clean test-account TCC baseline, choices and recovery after Settings | Test accounts on Intel and arm64; microphone and system source | NOT_RUN |
| SMOKE-AUDIO-SOURCES | FEAT-084, FORK-178, FORK-179 | Visible selected sources and requested/actual source metadata | Microphone and supported system output on each architecture | NOT_RUN |
| SMOKE-MIC-CAPTURE | FORK-051, FORK-178, FORK-179 | Unique phrase, level/transcript, saved microphone playback | Physical microphone on each architecture | NOT_RUN |
| SMOKE-SYSTEM-CAPTURE | FEAT-084, FEAT-085, FEAT-086, FORK-178, FORK-179 | Unique system phrase, saved system.mp4, mute/gain and pause/resume timing | Physical system output on each architecture | NOT_RUN |
| SMOKE-LIVE-TRANSCRIPT | FORK-044, FORK-178, FORK-179 | Two exact live phrases and timestamps | Local STT model and real audio on each architecture | NOT_RUN |
| SMOKE-NORMAL-FINISH | FORK-122, FORK-178, FORK-179 | Stop completes once; earlier transcript and tracks survive | Same recording on each architecture | NOT_RUN |
| SMOKE-FORCED-QUIT-RECOVERY | FORK-132, FORK-178, FORK-179 | Interrupted recording recovered with exact phrases, checkpoints and playback | Backed-up test data on each architecture | NOT_RUN |
| SMOKE-PERMANENT-SAVE | FORK-043, FORK-178, FORK-179 | Title, transcript count, relative/audio paths after reopening | Isolated recording root on each architecture | NOT_RUN |
| SMOKE-SPEAKER-RENAME | FORK-042, FORK-170, FORK-178, FORK-179 | Visible rename survives reopening without changing transcript text | Saved multi-speaker fixture on each architecture | NOT_RUN |
| SMOKE-DIARIZATION-SUCCESS | FORK-042, FORK-170, FORK-178, FORK-179 | Assignments and unchanged original text; settings and model recorded | Local diarization model and multi-speaker fixture on each architecture | NOT_RUN |
| SMOKE-DIARIZATION-FAILURE | FORK-042, FORK-170, FORK-178, FORK-179 | Safe failure and unchanged canonical transcript/assignments | Controlled model/error fixture on each architecture | NOT_RUN |
| SMOKE-SUMMARY-SUCCESS | FORK-040, FORK-134, FORK-167, FORK-178, FORK-179 | Local model/settings, visible heading and persisted accepted summary | Compatible local Gemma model on Intel and arm64 | NOT_RUN |
| SMOKE-SUMMARY-FAILURE | FEAT-186, FEAT-187, FEAT-188, FORK-178, FORK-179 | Missing/broken model error; old transcript and accepted summary survive | Controlled model failure on each architecture | NOT_RUN |
| SMOKE-EXPORT-UI | FORK-048, FORK-171, FORK-178, FORK-179 | Actual export control, destination and cancel behavior | Native save dialog in the same app on each architecture | NOT_RUN |
| SMOKE-EXPORT-ARTIFACT | FORK-048, FORK-171, FORK-178, FORK-179 | Opened export contains known phrase and summary heading; checksum | Real exported file from each architecture | NOT_RUN |
| SMOKE-OFFLINE | FEAT-088, FORK-142, FORK-178, FORK-179 | Network disabled; local launch/record/summary/reopen and observed egress | Isolated test session on each architecture; locally present models | NOT_RUN |
| SMOKE-RESTART-REOPEN | FORK-122, FORK-132, FORK-178, FORK-179 | Normal restart retains transcript, speakers, accepted summary and audio | Same test profile and artifact on each architecture | NOT_RUN |
| SMOKE-UPGRADE-PREVIOUS | FORK-123, FORK-124, FORK-125, FORK-172, FORK-178, FORK-179 | Previous-version test profile, WebView export/import, SQLite/WAL backup, known rows | Backed-up control profile on each architecture | NOT_RUN |
| SMOKE-OLD-DATA | FORK-124, FORK-172, FORK-178, FORK-179 | Older healthy title/phrase/audio verified after upgrade | Control profile with older meetings on each architecture | NOT_RUN |
| SMOKE-LOGS | FEAT-082, FEAT-083, FORK-178, FORK-179 | Log path/time and useful errors without transcript/audio contents or secrets | Same packaged app/session on each architecture | NOT_RUN |
| SMOKE-CHECKSUM | FORK-176, FORK-177, FORK-178, FORK-179 | DMG SHA-256 exactly equals candidate-report artifact hash | The same distributed Universal DMG on both machines | NOT_RUN |
| SMOKE-SIGNATURE | FORK-177 | codesign verification, Developer ID authority and final app/DMG hashes | Final exact app/DMG; separately authorized identity | NOT_RUN |
| SMOKE-GATEKEEPER | FORK-177 | spctl assessment of installed exact app | Real macOS trust assessment | NOT_RUN |
| SMOKE-NOTARIZATION | FORK-177 | stapler validation and notarization result for exact artifact | Final exact app; separately authorized notarization | NOT_RUN |

## Inputs and execution order

Finish independently runnable plan/code cards against the current product base;
retain their canonical dependencies. In particular FORK-143, FORK-148,
FORK-126 and FORK-150 follow the already closed FORK-057, while FEAT-089 waits
for FORK-042. FORK-123 can make code progress but cannot close its user WebView
acceptance from a Rust-only test.

Before native acceptance, select the exact permitted artifact and isolated test
profile; record its TCC baseline without resetting the owner's profile. The
current computer-use API explicitly disables native app control. A real human
operator or another authorized native-control route is required for visible
app/device/TCC evidence; unavailable control is not an application FAIL.

Final ordering is the unchanged FORK-175 → FORK-176 → FORK-177 →
FORK-178/FORK-179 → FORK-180 dependency chain. Identity/channel, Developer ID,
updater key and notarization authorization, approved benchmark corpus, Intel,
Apple Silicon and minimum-OS access must be obtained before their relevant
steps. Their availability is not inferred from earlier sessions.

No rebuild or repeat of green checks is required solely to write this matrix.
Repeat affected checks after source/input changes; run the mandatory complete
gate before creating the final package. Packaging/smoke/signing evidence must
refer to the same final bytes. Only all-PASS exact-artifact evidence can satisfy
`./scripts/release-gate.sh --smoke-report <report>`. The separate
`./scripts/test-plan-gate.sh` and its
[packaged test-plan template](../release/test-plan-smoke-report-template.md)
exclude Apple distribution trust and do not close FORK-177 or prove release
readiness. Push, publication and final handoff stay separately authorized.
