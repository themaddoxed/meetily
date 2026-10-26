# Background proof correction — independent SPEC review

**Bounded source corrections PASS; route acceptance OPEN. C0 / I0 / M0.** No actionable source defect remains in the reviewed corrections. The previous three Important findings are closed. This does not accept the complete background milestone: the exact real-route run has one unresolved failure.

## Snapshot

Reviewed 2026-10-04. Feature `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Before/after SHA-256:

- `frontend/tests/helpers/rendered-contrast.ts`: `eb1e22b0f3669f100add85587ea3646b1e989755c43b75cbd8bf011cdbd8e52f`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: `529c2c43163a33887845742577c5513c059b2b295c031922019c48cc8425b98e`.
- Root plan v4: `c61c921567f0d6e12f7760c6e0585bd7c529f51cb312585a7d00f08bf512a997`.

Review changed_paths: only this report; previous FAIL report preserved. No source, tests, plan or Git changes by this reviewer.

## Findings closure and canvas contract

- Previous I1: helper line 133 uses own-node `matches`; ordinary and overflowing descendants remain inventoried. Two negative regressions exercise those cases. The prior positive checked-switch fixture now has a dark descendant, removing the misleading broad exemption.
- Previous I2: helper line 26 rejects non-normal `background-blend-mode`; the screen-blend counterexample requires rejection. Normal source-over is no longer silently assumed for that paint.
- Previous I3: helper lines 126–129 reject background/image/shadow paint on first-line, first-letter and listed foreground-related pseudos independently of generated content. Line 10 rejects active modal/popover scenes before traversing ordinary ancestors. First-line, first-letter and actual modal-backdrop fixtures require rejection. Before/after conservative coverage remains intact.
- Canvas admission at lines 41–58 requires an opaque solid root, or transparent root plus the first BODY child's opaque solid background under the admitted display/visibility, opacity, containment, image and transform restrictions. It reports the provider and never substitutes an assumed browser color. Positive body propagation and nine unsupported cases are covered. Root/body duplicate computed solid colors cannot lower a conservative brightening bound; their already opaque admitted canvas color does not create a hidden light surface.

The per-channel threshold and original no-near-white condition remain intact; unsupported paint fails rather than becoming an exemption. No further source finding was established in this bounded correction review.

## Checks and evidence

Reviewer independently read numbered source/test definitions and the completed evidence below; these were implementer-owned executions, not reviewer-run tests.

- `/private/tmp/meetily-bg-diag/canvas-fix-fixtures.log`: terminal **45 passed (9.2s)**, including six former failure scenarios and canvas admission cases. SHA-256 `76041f59e740687ff998f11409d8a0a631fadd1fcf0ec16bb3a5285af2e4e03f`.
- `/private/tmp/meetily-bg-diag/routes-fixed.log`: parsed terminal JSON **2 expected / 1 unexpected / 0 skipped / 0 flaky**, 27.386s; SHA-256 `0bc9231ed5ce8833aac99d1daabb1f8efabd121af3770a79d8f4a98471461420`.
- Home ledger: BODY canvas `[10,10,10]`, 18 layers / 240 cells, maximum near-white upper **84.50425576325247**. Meeting Details: same canvas, 38 layers / 250 cells, maximum **33.9600000018**. Both remain below 245 over their recorded main scopes.
- Settings fails with `Unresolved background bound: transform matrix(0.940869, 0, 0, 1, -3.63654, 0)`. This is unsupported current geometry, not observed poor color. No accepted Settings ledger exists for this snapshot.

Typecheck/ESLint/diff-check success was reported by the implementer; terminal logs for those checks were not independently read in this review. No native or release verification is claimed.

## Blocker and next action

Settings route acceptance remains blocked by its in-flight scale transform. Establish that the real UI's finite entrance animation has settled before measuring, retain the transform rejection, and review the resulting exact test delta and terminal route evidence. A positive route after such a change must not erase this snapshot's failure. Full-theme, snapshots, native and release gates remain open.
