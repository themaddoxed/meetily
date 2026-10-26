# Background absence proof plan — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**QUALITY FAIL: Critical 0 / Important 1 / Minor 0.** The scalar/order/group-opacity upper bound is mathematically defensible, but the proposed inventory excludes paint-capable zero-area elements.

## Finding

**Important — inventory shadow and pseudo paint independently of border-box area.** `docs/superpowers/plans/2026-10-04-theme-background-proof.md:19` limits collection to positive-area rendered DOM elements. A zero-width/height element can still paint a nonzero spread box-shadow; its generated pseudo-element can also have positive dimensions and a white background. For example, a zero-size element inside main with `box-shadow: 0 0 0 20px white` creates visible white paint despite having no positive-area border box. Excluding the originating node prevents line 23's otherwise conservative shadow/pseudo treatment from running, allowing a false absence PASS.

Collect/validate shadow and pseudo paint regardless of originating element border-box area; only a proven-empty background fill may be omitted for zero geometry. `display:none` with no rendered descendants is different from zero size or `display:contents`. Add negative fixtures for a zero-size white spread shadow and zero-size origin with a positive-size white pseudo background, plus any relevant display-contents traversal case. Do not add an exception or minimum-size threshold.

## Exact scope

Reviewed root plan `docs/superpowers/plans/2026-10-04-theme-background-proof.md`, SHA-256 `29cbe1742d16e779b21b71285b3a95bb9bc8438db6048b0a5c49bff1185fc222`. It supplements semantic-theme plan `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41` and restricts future implementation to the same helper/spec test paths. No implementation or product mutation is accepted here.

## Mathematical assessment

For supported ordinary source-over paint, the ascending-maxima bound is sound, including nested opacity groups and external siblings. One useful proof represents each layer alpha and each group opacity as independent Bernoulli visibility gates. The expected source-over channel equals the expected selected source channel; group gates are shared by all their descendants. For any realization, the actual selected channel cannot exceed the largest active source maximum or canvas maximum.

A leaf's marginal activation probability is its own alpha times all ancestor opacity gates. Events that particular leaves are inactive are decreasing functions of independent gates and therefore positively associated. For each threshold above the canvas bound, the probability that all above-threshold leaves are inactive is at least the product of their marginal inactivity probabilities. Replacing shared group gates with independent gates at those marginal probabilities therefore increases the distribution of the maximum. Compositing scalar maxima in ascending order, skipping darkening contributions, computes the expected maximum for these independent gates. This proves an upper bound, not an exact flattening of group compositing. Additional nested gates and an external sibling preserve the argument.

The simple two-layer interchange check agrees: putting the larger source maximum later increases the bound by the nonnegative product of both alphas and their source difference when both can raise the starting bound. The max-with-zero update handles sources below the current bound.

For gradients, independent maximum channel and maximum alpha are conservative provided the admitted interpolation actually stays within the source component envelope in the tested browser. The implementation must establish the supported sRGB interpolation grammar/behavior and reject unsupported spaces. Grouping fill and gradient as separate possible paints overestimates coverage, which is acceptable. The ledger must label an excessive bound as inconclusive rather than claiming an actual white island.

Closed conservative rectangle inclusion, whole-scope shadow coverage, no alpha/size sampling, explicit finite caps and upward rounding margins support the mathematical argument. Node/style/opacity parsing must fail on nonfinite or out-of-domain values. The actual margin and operation cap need concrete derivation and edge tests in the implementation; simply adding an unspecified epsilon would not discharge line 33.

## Verification and remaining gates

Reviewer actions: read-only exact-plan inspection, SHA-256 pinning and mathematical reasoning. No browser test, paint experiment or numerical implementation was executed. The existing background failures are unresolved geometry, not proof of near-white paint. The proposed separate background function correctly preserves exact text/focus acceptance and prohibits product, dependency and snapshot expansion.

`changed_paths`: this report only. `blockers`: zero-area paint inventory omission. `next_action`: amend inventory and fixtures, repin and re-review before implementation. Actual route evidence and all wider theme/native/release gates remain open. Prior source reviews are not superseded by this planning review.
