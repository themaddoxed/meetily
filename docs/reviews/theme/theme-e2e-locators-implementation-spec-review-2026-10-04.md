# Two e2e locators — independent implementation SPEC review

**PASS for the bounded correction. C0 / I0 / M0.** No actionable findings. Both targeted cases have terminal PASS evidence. Broader required gates remain pending and are not implied by this result.

## Snapshot and scope

Reviewed 2026-10-04 in `.worktrees/semantic-theme-2026-10-03`, against accepted plan SHA-256 `432617704a37aec8e2139b7828bdf4c461a7ec2afcdda7068076822fac1fed36`. Exact source SHA-256 before/after:

- `frontend/tests/interface-polish-acceptance.spec.ts`: `ea3ae619a0d814bf377840e5d0b9777dae5b516faeae95faf0d15ccc98e56755`.
- `frontend/tests/summary-template-selection.spec.ts`: `36325558b7d1ebedd4364f6bce48a489f6a2004b836d7fe50c6f20453b43ef25`.

Read-only Git diff confirms only the admitted edits in these two files. Review changed_paths: only this report. No product, test, configuration or Git mutations by this reviewer.

## Behavior and findings

`interface-polish-acceptance.spec.ts:584,603` now targets exact Settings and Home buttons within app-sidebar. The original URL, hydration, probe removal/reappearance and listener add/remove/active-count assertions remain intact. This resolves the demonstrated substring ambiguity without changing what navigation and cleanup must do.

`summary-template-selection.spec.ts:108–116` scopes the exact existing Section title 1 textbox to Create summary template. It retains visibility, sequential Decisions input and exact resulting value, and adds retained-focus verification. No force clicks, timeouts, product labels, fixtures or handlers were changed.

Both changes implement the accepted narrow plan and preserve the original behavior under test. No scope expansion or weakened assertion was found.

## Verification and limitations

Reviewer actions: inspect exact diff and surrounding contract inherited from the immediately preceding plan review, hash both sources before/after, read completed owner-run log. No reviewer-run test is claimed.

`/private/tmp/meetily-theme-e2e-locators-fix-2026-10-04.log` ends **2 passed (24.4s)**, Chromium, one worker. Both requested test names are present, including retained-focus and reduced-motion cleanup. SHA-256 `97752b25272315416d6a2bddedd07c1645d9f6e25bee129f1dfdc52baa004f46`.

The prior full e2e 210 PASS / 6 FAIL and isolated 4 PASS / 2 FAIL remain historical failures; this focused success does not erase them. Both entire spec files, full frontend e2e and canonical `check.sh --full` still require their planned terminal evidence. Native, ignored hardware/model tests and exact-artifact release gates are not established here.

## Next action

No blocker for this bounded correction. Complete and inspect the planned wider test runs on the same source snapshot before claiming the corresponding gates passed. Task closure, integration and release readiness remain separate.
