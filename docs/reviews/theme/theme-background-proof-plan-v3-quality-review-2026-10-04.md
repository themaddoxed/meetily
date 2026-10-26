# Background absence proof plan v3 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** No actionable defect in the component-wise amendment or the retained planning constraints. This accepts the proposed algorithm for implementation review, not an implemented proof or actual route result.

## Exact snapshot and delta

Reviewed `docs/superpowers/plans/2026-10-04-theme-background-proof-v3.md`, SHA-256 `3da22e4b231f5cfabe5b37e09ed56c3ad684971a88d3ccd9e67a2de5ef91fe05`, including a direct diff against preserved v2. Changes replace the scalar maximum-channel bound with three independently calculated channel bounds and add saturated opaque red/yellow positive fixtures. The two-path scope, prior atomic text baseline, zero-size/off-scope shadow/pseudo coverage, hidden-descendant rules, unsupported-paint failures, numerical caps/margins and unchanged exact text/focus behavior remain intact.

## Logical assessment

For each cell, if its actual components satisfy `R <= uR`, `G <= uG`, `B <= uB`, then any one bound strictly below 245 proves that the actual color cannot have all three channels at least 245. Applying that condition to every conservatively covered cell proves the original no-near-white criterion. The channel furnishing the certificate may differ between cells. `max_cells(min(uR,uG,uB)) < 245` is the correct aggregate criterion; neither a per-route sampled minimum nor a global maximum of one channel would express the same property.

Each channel may use a different ascending source-component order. Such orders need not coexist in one physical stacking arrangement: each independently maximizes that channel over a superset of possible arrangements, yielding simultaneous upper bounds on the actual scene. The shared-opacity Bernoulli-gate/positive-association argument from v2 applies separately to each component without relying on correlation between RGB values. Nested groups and external siblings therefore remain bounded under the stated ordinary source-over assumptions.

A dark canvas plus one opaque yellow `[251,217,80]` layer or red `[248,114,114]` layer can now certify absence through a lower green/blue bound despite its red bound exceeding 245. That removes the unnecessary scalar restriction without lowering the original white threshold or creating a new exception. Conversely, per-channel maxima may combine extrema from different layers or gradient positions and become inconclusive even without an actual near-white pixel; such a result must remain FAIL/unknown, as specified.

The complementary-color mixture regression is still essential: testing each source color against the all-channel threshold before compositing would be unsound. Gradient bounds must use separate per-channel extrema and the alpha maximum, not one selected stop. Numerical upward margins must cover opacity products and all three bound update sequences, with finite operation/cell caps. These remain implementation evidence requirements.

## Checks and limits

Reviewer actions: read-only SHA-256 checks, v2/v3 diff and complete changed-section inspection, plus mathematical assessment. No browser, inventory generation, numerical implementation or test execution was performed. Reported actual red/yellow inventory is motivation; the mathematical assessment does not rely on accepting that inventory as reviewer-generated evidence.

`changed_paths`: this report only. `findings: []`. `blockers: []` for the bounded planning gate. `next_action`: implement within the admitted helper/spec paths, run the required exact-color/opacity/geometry/unsupported-paint regressions, and independently review source hashes and completed real-route ledgers. Full theme/native/release acceptance remains open; earlier versions and reports remain preserved.
