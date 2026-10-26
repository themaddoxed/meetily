# Theme focus implementation — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL: Critical 0 / Important 1 / Minor 0.** The current untransformed Appearance run is green, but the expanded geometry oracle can silently accept unsupported transformed geometry.

## Finding

**Important — reject unsupported transforms before mixing coordinate spaces.** `frontend/tests/helpers/rendered-contrast.ts:150–181` obtains viewport-space rectangles but uses untransformed computed border widths, outline widths and corner radii. Neither the ancestor loop nor `validate` rejects scale/rotate/skew (or their individual transform properties). The analytical proof at lines 187–205 therefore does not describe the painted boundary under such transforms and can produce a false PASS, rather than the required unresolved result.

Concrete source-level counterexample: take the existing radius-0 negative fixture (200×50 border-box parent, 1px border, 12px radius, 4px padding, 90×40 button, 2px outline) and scale its parent uniformly by 2. Relative to the transformed parent origin, the oracle places the measured corner at (8,8) and parent circle center at (12,12), so the corner-distance check passes against radius 11 and bypasses the zero-radius rejection. The actual outer outline corner is (6,6), while the inner parent circle center is (24,24), radius 22; its distance is approximately 25.46, outside that background. This calculation is derived from source geometry, not a reviewer-run browser reproduction.

Fix by rejecting unsupported transforms across the relevant ancestor chain (including individual scale/rotate and perspective where applicable), or explicitly translating all measurements into one supported coordinate system. Translation-only/identity matrices may be supported if proven safe. Add a transformed escaping-corner negative fixture so this remains fail-closed. Do not weaken the existing intersection/contrast assertions.

## Reviewed snapshot

Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`; bounded plan `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`.

Exact candidate hashes (all paths relative to task `frontend/`):

- `tests/helpers/rendered-contrast.ts`: `4ecbf66441738267048a871af3070d3b9edb021e93ef4ec11576bac5dc012e25`
- `tests/theme-dark-surfaces.spec.ts`: `a761285bfa020419ff35dac56be42e43eb7738de650672ba9cdc230b2cd52e2e`
- `tests/theme-foundation.spec.ts`: `18528fc93dfe8bfbe635c9a845748e5b83a38654a3870219b15719b5547b58e7`
- `src/components/PreferenceSettings.tsx`: `7078398c24cf894a1ba676ecf16c57a5ac5aecb1cc83be82e5d26281d5163972`

For the admitted untransformed circular geometry, the arc maximum uses both endpoints plus the outward radial extremum when inside the quarter. It is not sparse sampling. Radius order TL/TR/BL/BR agrees with the loop. An initial review message incorrectly questioned that order; it is withdrawn and is not a finding. Strict pixel radii, conservative overlap rejection, uniform parent borders and elliptical-radius rejection remain effective. Symmetric 8px-positive/1px-escaping/0px-negative fixtures provide useful coverage; asymmetric corner fixtures would further protect index correspondence.

Production classes keep a semantic solid 2px zero-offset indicator, scope shadow suppression to the Appearance selector and add only the admitted gap. `focus-within` also applies to pointer/programmatic focus, consistent with the selector mechanism; it is not exclusively a keyboard predicate. The test uses real Tab, requires all three buttons in each mode, waits for all button shadows to settle to none, measures both surfaces, checks viewport overflow and attaches screenshots.

## Checks and evidence

Reviewer performed read-only source inspection, SHA-256 checks, mathematical analysis and parsing of existing logs; no browser or test process was launched by this reviewer.

- `/private/tmp/meetily-theme-focus-owned.log`, SHA-256 `2cb6101cbaf8e7a002f0acb4bc09412f12870a6445dc5c1be1ec69e4ee849c2f`: ten fixture passes plus the earlier real-focus failure. This is not an all-green run.
- `/private/tmp/meetily-theme-focus-gap.log`, SHA-256 `fd5725c369827fa771b39322e5641f9081bc3e7d3fd8b33f85e5d34d12f4219f`: complete JSON reports expected 1, unexpected/skipped/flaky 0, duration 20.66s. Its six-entry focus ledger reports minimum adjacent-surface ratios 7.014688543268176 in light and 4.974482213656899 in dark. This is inspection of retained runtime output, not independent process-exit observation.

`changed_paths`: this report only. `blockers`: the transform fail-closed gap. `next_action`: correct the oracle boundary, add the targeted regression, repin and re-review. Full theme, packaged/native and release acceptance remain NOT VERIFIED; passing current Appearance geometry does not discharge the helper defect.

Post-review pin: helper, foundation and PreferenceSettings still match the hashes above. The dark-surface test changed concurrently to `0bf89eef3f18b686374ad4659f5549a15667754feacd9256f585dd67cdbe200a`; that newer test snapshot is **NOT REVIEWED** here. The finding remains bound to the unchanged reviewed helper. No acceptance of the current four-file set is implied.
