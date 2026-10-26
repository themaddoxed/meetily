# Background proof v4 amendment — independent SPEC review

**PASS for the bounded plan delta. C0 / I0 / M0.** No actionable plan findings remain. The implementation defects remain open until corrected code and meaningful green regressions are reviewed.

## Snapshot and checks

Reviewed 2026-10-04. Plan `docs/superpowers/plans/2026-10-04-theme-background-proof-v4.md` SHA-256 before/after: `c61c921567f0d6e12f7760c6e0585bd7c529f51cb312585a7d00f08bf512a997`. Compared its complete diff against accepted v3 `3da22e4b231f5cfabe5b37e09ed56c3ad684971a88d3ccd9e67a2de5ef91fe05`. Scope is only this amendment; no approval of changed test bytes or canvas implementation is implied.

Reviewer actions: read full v3-to-v4 diff; independently consult primary CSS background propagation rules; read terminal RED log; rehash plan and log. No reviewer-run browser test. `/private/tmp/meetily-bg-diag/review-red.log` ends with six failed tests: background blend, two checked-descendant cases, first-line, first-letter and active top-layer backdrop. SHA-256 `36908df5e858b760b0b9af551ea14ab799ecd39e59f9dfa68900c391a745e996`. These are regression evidence against the old implementation, not acceptance evidence.

## Contract assessment

Line 19's new canvas admission is standards-based and sufficiently restrictive. When an HTML root has a transparent background and no background image, the first BODY child's background propagates to the canvas; containment disables this special handling. The specification also describes the propagated body's used background becoming initial rather than a second independent canvas. See [CSS Backgrounds 3 §2.11–2.11.2](https://www.w3.org/TR/css-backgrounds-3/#body-background). Requiring opaque solid body paint, opacity 1, ordinary visible display, no containment, no transforms/images/unsupported effects and explicit provider reporting avoids assuming a browser-default backdrop. The positive and negative canvas fixtures are mandatory implementation evidence. Product CSS need not change to accommodate a test's former explicit-root restriction.

Line 9's own-node `matches` requirement closes the exception ambiguity. Line 25 explicitly rejects background blending in addition to mix blending. Line 23 requires rejecting painted first-line/first-letter and unsupported foreground-related pseudos independently of generated-content values, and rejects active native modal/popover scenes rather than assuming their backdrop and ancestor-opacity behavior. These are appropriate corrective obligations for the three previously reported implementation findings; white-descendant and active-backdrop regressions preserve the original scope instead of adding exemptions.

V3's component-wise upper bound, per-cell threshold, complete-main coverage, zero-size/off-main inventory, checked-switch-only exception, opacity proof obligations and numerical caps remain unchanged. Text/focus measurement remains separate. No palette, dependency, snapshot or additional source path is authorized.

## Blockers and next action

No blocker to implementing this bounded amendment. Review changed_paths: only this report. Correct the implementation, run the canvas/paint negatives and actual three-route cases, and review exact source hashes plus terminal evidence. Previous implementation FAIL and real-route canvas errors remain preserved; this plan PASS does not supersede them. Full-theme, native and release gates remain open.
