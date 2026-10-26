# Background absence proof plan v2 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** No actionable defect remains in this revised proposal. The zero-area inventory Important is resolved; this is permission to implement and verify the bounded test algorithm, not acceptance of an implemented background proof.

## Snapshot

Reviewed `docs/superpowers/plans/2026-10-04-theme-background-proof-v2.md`, SHA-256 `1a8cae8e3f81786a5100ec87ca809e7c25036e4d691223c722db89fdba9e2476`. The main semantic-theme plan remains the referenced `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`. V1 and its FAIL report remain preserved.

## Assessment

Line 19 now requires walking all DOM elements, including zero-size and off-scope owners. Shadows and generated pseudo paint are inspected before principal-fill area/intersection filtering. This prevents a zero-size white spread shadow or a positioned pseudo background from disappearing from inventory. Complete display-none/zero-opacity ancestry can establish absence of paint, while visibility-hidden ancestry alone cannot exclude a visibility-visible descendant. The corresponding zero-size/off-main/visible-descendant negatives are required at line 37.

Line 23's empty ordinary outer-shadow case is narrowly stated: all offsets, blur and spread must be zero, and the exception must not extend to inset shadows. Because ordinary outer shadow paint is clipped away inside the border box, that geometry has no remaining outer area even when the element background is transparent. The required transparent-background regression should distinguish this empty geometry from nonzero spread/offset/blur and inset cases. Every nonempty shadow remains conservatively global to main, including off-main sources; no assumed finite blur radius is introduced.

The ascending scalar bound and product-opacity treatment remain mathematically sound for the admitted ordinary source-over model. Line 31 records the shared Bernoulli-gate/positive-association proof, so nested opacity groups cannot be replaced by an unjustified exact flattening. Gradient maximum-channel and maximum-alpha bounds remain conditional on the admitted component-envelope interpolation behavior; unsupported syntax/spaces must reject. The checked-switch exception remains confined to the already admitted background, with its shadow still inventoried.

Implementation must supply the promised finite caps and a numerical error budget covering opacity multiplication as well as bound updates. Cap exhaustion, unsupported paint, malformed/nonfinite geometry or an inconclusive bound must fail. The actual implementation must show complete pseudo/shadow traversal and conservative cell-boundary inclusion rather than merely claim those properties from the plan. These are existing requirements, not additional product scope.

## Checks and limits

Reviewer performed read-only exact-plan inspection, SHA-256 pinning and comparison against the prior mathematical assessment/finding. No browser or paint experiment, algorithm implementation or test execution was performed. The proposal changes only the two already admitted test paths, keeps text/focus math unchanged, and authorizes no dependency, product or snapshot change.

`changed_paths`: this report only. `findings: []`. `blockers: []` for this planning gate. `next_action`: implement the separate background proof and specified fixtures, then review exact hashes and completed regressions before accepting any of the three real-route results. Full theme/native/release gates remain open.
