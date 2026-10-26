# Meetily agent policy

## Navigation and work records

Start with [INDEX.md](INDEX.md). For folder ownership, historical evidence,
worktree provenance and where to save a result, read
[repository-map.md](docs/development/repository-map.md).
For the current execution order, read
[execution-order.md](docs/development/execution-order.md).
The task source is `docs/prd/`; use `mdtask --path docs/prd`.
Manual acceptance is the final stage; implementation stages use automated
tests and agent-run checks. This does not waive any release gate below.
Do not use Firecrawl or Graphify.

## Mandatory release gate

The authoritative operator guide is [`docs/testing.md`](docs/testing.md). An
agent must not call a Meetily release ready merely because code compiles or an
application bundle exists.

Before claiming release readiness for one exact commit, the agent must:

1. run `./scripts/check.sh --full` and report its actual exit status and every
   failed or skipped stage;
2. run `./scripts/release-candidate.sh` from a completely clean worktree and report
   the candidate commit, artifact SHA-256, and verifier result;
3. require a fully completed copy of
   `docs/release/smoke-report-template.md` for that exact commit and artifact;
4. run `./scripts/release-gate.sh --smoke-report <report>` and require exit 0;
5. separately list every manual, macOS, hardware, offline, model, signing, and
   notarization check that was not actually performed.

Do not bypass, remove, allowlist, or describe a failing check as passing without
an explicit root-cause explanation. `--diagnostic-build` always creates an
`UNGATED` artifact under a unique
`diagnostic[-dirty]/<commit>/run-*` evidence path and is never release
evidence. A dirty diagnostic build must record `WORKTREE_CLEAN=false`, a unique
`RUN_ID`, and a `SOURCE_FINGERPRINT`. A dirty normal preflight is recorded under
`preflight-dirty/<commit>/run-*` and cannot reach the build.

Public release provenance must be inside `candidate-report.json`, not inferred
from its directory or `status.env`. The public gate requires
`provenance.mode=normal`, `provenance.worktreeClean=true`, and
`release.eligible=true`; diagnostic or dirty provenance can never be `PASS`
even when bundle, Developer ID, Gatekeeper, and notarization checks succeed.
The candidate report must be published only after a post-verifier source
fingerprint recheck, and the public gate must recompute the current source
fingerprint before accepting it.

If a Developer ID identity is unavailable, use only `--diagnostic-build` for
packaging diagnosis. Record Developer ID signing as `NOT_AVAILABLE` and
notarization as `NOT_RUN`; the public release gate remains blocked. An ad-hoc
signature does not satisfy Developer ID, Gatekeeper, or notarization checks.

Never claim that clean-install permissions, microphone capture, system-audio
capture, audio quality, offline behavior, upgrade safety, Developer ID signing,
Gatekeeper, or notarization passed unless they were performed on the exact
reported artifact. An unavailable product action is a failed/not-run smoke
item, not a mocked pass.

The browser `desktop-local-safety` contract observes only WebView HTTP/WebSocket
egress and mocked updater/model failures; it is not native or packaged offline
evidence. The opt-in `smoke-local-models.sh` and `smoke-macos-audio.sh` scripts
run ignored hardware/model tests as supplemental evidence only. They do not
turn a manual smoke row into `PASS` without the exact installed artifact and
the visible user flow required by that row. Any failed system-audio subcheck
makes `smoke-macos-audio.sh` `OVERALL FAIL` (exit 1), while microphone remains
`NOT_RUN`. If both system subchecks pass, the script remains overall `NOT_RUN`
(exit 2) until microphone and UI evidence exists.

Data safety has priority over summary, model, and UI success. Do not delete or
rewrite user data during release diagnosis. Storage-format changes require a
migration, a real temporary-storage preservation test, and an upgrade/reopen
check.
