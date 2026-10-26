# Background proof final wait delta — independent SPEC review

**PASS for the bounded wait-only correction. C0 / I0 / M0.** No actionable findings. Settings now has terminal positive evidence. This is not a full-theme or release verdict.

## Snapshot and scope

Reviewed 2026-10-04 in `.worktrees/semantic-theme-2026-10-03`. SHA-256 before/after:

- Helper `frontend/tests/helpers/rendered-contrast.ts`: `eb1e22b0f3669f100add85587ea3646b1e989755c43b75cbd8bf011cdbd8e52f` (unchanged from accepted correction).
- `frontend/tests/theme-dark-surfaces.spec.ts`: `31d8afbe359dfef17a4f4b02f7dbbc01e828478a26e9fff1c37e3265ee69757f`.

Scope is the added readiness wait at test lines 402–413 and its Settings evidence, against accepted v4 plan `c61c921567f0d6e12f7760c6e0585bd7c529f51cb312585a7d00f08bf512a997`. Prior source correction review and its 2 PASS / 1 FAIL record remain preserved. Reviewer changed_paths: only this report; no product/test/Git mutations.

## Assessment

The bounded five-second poll reads actual computed transforms on visible positive-area nodes and permits only none or finite 2D identity-linear matrices with translation. It neither rewrites CSS nor disables animation. Unsupported or unsettled transforms remain a timeout failure. The same strict background resolver immediately follows the wait and still independently rejects unsupported paint and transforms; the readiness filter does not remove elements from its inventory. Therefore omitted nodes in the readiness poll cannot silently become omitted background layers.

This addresses the documented transient scale matrix on Settings without weakening the accepted near-white threshold, complete-main proof, checked-switch exception or exact text/focus resolver. It measures the representative route after its entrance animation; it does not substitute for the separate prepaint gate or claim that every animation frame was checked.

## Evidence and limits

Reviewer actions: numbered source read, compare added wait with prior reviewed route code, SHA-256 before/after, parse completed Settings JSON and decode its ledger. No reviewer-run test is claimed.

`/private/tmp/meetily-bg-diag/settings-settled.log` SHA-256 `7d36acc7c083437427d68829dd873d950925736ae457dbe54087ab0430ff9630`: **1 expected / 0 unexpected / 0 skipped / 0 flaky**, duration 13.851s; no top-level errors. Settings uses BODY canvas `[10,10,10]`, 29 layers / 390 cells / 19,059 operations, one recorded checked-switch exception, and maximum near-white upper **133.2131968042615 < 245**.

Prior Home and Meeting Details PASS evidence belongs to the preceding test snapshot with the same helper; they were not rerun by this reviewer and are not relabeled as an all-three-route run on the latest test hash. Their preserved provenance is in `theme-background-proof-review-fix-spec-review-2026-10-04.md`. The broader five-suite run was still in progress at assignment and is not a PASS claim in this bounded review. Latest typecheck/ESLint success was parent-reported, not independently established here from terminal logs.

## Next action

No blocker for this wait delta. Consume and review the broader suite's terminal results and remaining required theme evidence before any full-theme closeout. Native and exact-artifact release gates remain separate and open.
