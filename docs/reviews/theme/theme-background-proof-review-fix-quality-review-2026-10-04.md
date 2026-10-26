# Background proof correction — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded source QUALITY PASS: Critical 0 / Important 0 / Minor 0.** The three prior Important findings and the admitted canvas prerequisite are addressed. Real-route acceptance is still incomplete: Settings fails closed on a non-identity transform during this run.

## Exact snapshot

Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`:

- `frontend/tests/helpers/rendered-contrast.ts`: `eb1e22b0f3669f100add85587ea3646b1e989755c43b75cbd8bf011cdbd8e52f`
- `frontend/tests/theme-dark-surfaces.spec.ts`: `529c2c43163a33887845742577c5513c059b2b295c031922019c48cc8425b98e`
- Root plan v4: `c61c921567f0d6e12f7760c6e0585bd7c529f51cb312585a7d00f08bf512a997`

## Correction assessment

Helper line 26 now rejects non-normal background-blend-mode on principal and supported generated styles. The regression uses two sub-threshold gray layers with screen blending, which would exceed the admitted source-over model. Line 133 restricts the checked-switch exception to the node itself; both static and overflowing white descendants remain inventoried and have negative fixtures. Shadows are still handled before the exception.

Line 10 rejects active modal/popover top-layer scenes before ordinary opacity/inventory traversal. Lines 126–128 reject unsupported background/image/shadow paint on the listed foreground-related pseudo elements; first-line/first-letter negative fixtures no longer rely on content being none or normal. Before/after paint remains conservatively supported across the entire scope. The prior gaps are closed without expanding the accepted paint model.

Lines 41–58 implement the v4 canvas choice: opaque HTML or transparent HTML with its first body child's opaque solid background under the specified opacity, image, containment, visibility, ordinary-display and transform restrictions. The provider is recorded in the returned ledger. Partial-alpha root, translucent/absent provider, containment, translated body, display-contents, hidden body and image-backed body have fail-closed fixtures. Both root and body branches keep the existing effect validation. This admits the observed standard body canvas without changing product CSS.

The component-wise math, closed-cell coverage, global nonempty-shadow bound and finite caps remain as previously inspected. In-memory removal of the new background function reproduces accepted helper hash `009af7c626199e6e94d56b2565d721f6c4823e12adb36581ab2f2e0ed2652b79`, confirming byte preservation of the previous text/focus and shadow-proof implementation.

## Observed checks and limits

Reviewer actions: read-only exact-source/fixture inspection, SHA-256 checks, in-memory byte comparison and parsing completed logs/attachments. No browser or test process was launched by this reviewer; owner-reported typecheck/ESLint success is not a reviewer-run check.

- `/private/tmp/meetily-bg-diag/canvas-fix-fixtures.log`, SHA `76041f59e740687ff998f11409d8a0a631fadd1fcf0ec16bb3a5285af2e4e03f`: 45 fixture passes (9.2s), including corrected guard/exception/pseudo/canvas cases.
- `/private/tmp/meetily-bg-diag/canvas-red.log`, SHA `4ccc0975561d29c941ccc2d2304f0a51349aa5108ff45800ac5b65f40377d943`: retained canvas RED evidence from the preceding source.
- `/private/tmp/meetily-bg-diag/routes-fixed.log`, SHA `0bc9231ed5ce8833aac99d1daabb1f8efabd121af3770a79d8f4a98471461420`: completed JSON reports expected 2, unexpected 1, duration 27.39s. Home records BODY canvas `[10,10,10]`, 18 layers/240 cells, maximum near-white upper bound 84.50425576325247. Meeting Details records the same provider, 38 layers/250 cells, bound 33.9600000018. Settings throws for transform `matrix(0.940869, 0, 0, 1, -3.63654, 0)`; this is unresolved transient geometry, not detected near-white paint and not a successful route.

The Settings failure does not justify relaxing the transform guard. A scoped settled-state verification may establish an admissible scene, but it needs its own source-bound run. No full theme, native WebView, packaged app or release acceptance follows from this report. Prior FAIL reports remain preserved.

`changed_paths`: this report only. `findings: []`. `blockers`: Settings runtime gate remains incomplete; no blocker for the bounded source correction. `next_action`: resolve and verify the settled Settings scenario without weakening paint rejection, then continue the already required wider checks.
