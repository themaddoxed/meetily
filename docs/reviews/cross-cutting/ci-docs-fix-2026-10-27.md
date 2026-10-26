# Public source CI and documentation corrections — 2026-10-27

Author: Meetily Contributors. Base: anonymized public root `d25a861`.
This report covers source corrections, not application release acceptance.

## Observed failures and corrections

The [initial CI run](https://github.com/themaddoxed/meetily/actions/runs/40405330536)
failed in three independent places:

- The initial-push all-zero `before` SHA was treated as a real whitespace base.
  Root and new-branch pushes now check the complete committed tree against the
  empty tree; an invalid nonzero ref still fails. Six temporary-repository tests
  cover root, new branch, invalid ref and ordinary ranges.
- Playwright 1.61.0 could not discover tests under the pinned Node 22.17.0 runtime:
  its synchronous loader assumed `context.conditions` was an array. The exact
  runtime reproduced the failure; the minimal pinned 1.61.1 update discovers the
  same tests successfully. See the
  [upstream patch release](https://github.com/microsoft/playwright/releases/tag/v1.61.1).
- The capture saturation test drained its event channel after an arbitrary
  150 ms sleep. Under CI scheduling this happened before the fake backend had
  finished producing events, exposing a lower-priority event before the retained
  microphone route. A test-only acknowledgement now establishes that all fake
  events traversed the owner loop before draining. Production capture code is
  unchanged.

A local browser run additionally exposed slow first-route Next dev compilation.
The browser setup prepares `/` and `/meeting-details` before per-test budgets,
rejecting HTTP errors and bounded preparation failures. Existing test timeouts,
fixtures and assertions are unchanged. HTTP preparation does not execute browser
JavaScript and is not proof of native, offline or complete UI readiness.

## Whitespace and existing databases

The whole-tree check exposed existing trailing whitespace and excess empty EOF
lines. UI changes are whitespace-only. One already-applied SQLx migration must
remain byte-identical: changing even trailing spaces invalidates its stored
checksum and can prevent existing databases from reopening.

Only `20250920155811_add_openrouter_api_key.sql` disables blank-at-EOL lint through
an exact `.gitattributes` entry. All other whitespace rules remain active. Its
unchanged SHA-256 is enforced by a regression test:
`96839c8dd47fa1de98595deff59080efcb850fd5222cfb2bd659aabd71f381d5`.
No migration or storage format was changed.

## Documentation reconciled with code

Active README, contributor/build/platform/GPU/workflow guides, architecture,
privacy policy and agent handoff now describe the current macOS/Tauri source.
They correct legacy Windows/Linux product-build instructions, universal-bundle
and GPU assumptions, obsolete ScreenCaptureKit/BlackHole defaults, historical
backend diagrams, wrong model paths, analytics claims, offline guarantees and
claims equating a source bootstrap or ad-hoc app with an accepted release.
Historical evidence remains historical; old refs must not be restored into the
independent public graph. Remote provider and model/download network boundaries
are explicit. Local-only capture and local inference do not imply every product
flow is offline.

## Public source protection

CI now checks text (including decoded gzip), personal home paths, email contacts,
Git identities, symlinks and accidental runtime data. Diagnostics redact matched
values. Neutral synthetic examples, known public noreply identities and actual
retina asset references have narrow allowances; synthetic test source is excluded
from identity-text checks. Existing secret policy still scans test source.
Two independent-review findings (numeric-domain email bypass and home-directory
paths without a child slash) were reproduced and fixed with regression tests.
The final 12 privacy regression tests pass; bounded independent re-review found
no remaining actionable Critical or Important defect in the reviewed changes.

A separate Gitleaks scan covers public source and decoded gzip. These checks
reduce accidental disclosure; they do not claim exhaustive detection of arbitrary
personal data or text embedded in binary images. Images were not changed.

## Evidence and limits

- Pinned Node 22.17.0 frozen dependency install: PASS.
- Production source policy, frontend lint and TypeScript: PASS.
- Full frontend baseline after the initial fixes: 468 Node tests and 61 Vitest
  tests passed. Additional focused regressions cover subsequent privacy changes.
- Whitespace/check-command regressions, migration byte hash, scoped PRD validation,
  index validation and Rust format: PASS.
- Critical browser recovery/contracts on Node 22.17.0: 29/29 PASS after listener
  readiness and explicit HTTP route preparation. Superseded local cold-start
  attempts timed out before preparation; assertions were not weakened.
- Production export and remote suites are recorded by the subsequent exact commit
  CI run; successful local stages do not replace remote readback.
- Rust compilation, Clippy, workspace/data/recovery suites and macOS production
  packaging must be read from the exact commit CI run, not inferred from format
  checks or the initial run.

The commit containing this report and its GitHub checks are the correction's
identity. Raw local traces/logs are kept outside public source. No native capture,
installed-artifact smoke, hardware/model acceptance, signing, Gatekeeper or
notarization acceptance is claimed. The mandatory release gate remains separate.
