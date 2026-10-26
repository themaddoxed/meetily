# Background absence amendment — independent SPEC review

**FAIL pending one bounded clarification. C0 / I1 / M0.** Review is of the proposal, not an implementation or full-theme gate.

## Important finding

**I1 — Inventory selection can omit visible paint from zero-area sources (partial coverage).** `docs/superpowers/plans/2026-10-04-theme-background-proof.md:19` says “Collect every positive-area rendered DOM element whose background can intersect the main rectangle.” That selection conflicts with line 23's “Parse every box-shadow” and pseudo-element coverage. A zero-width/height positioned element can have a white nonzero spread shadow; a zero-area host can contain a positioned, positive-area white `::before`. Filtering the host first omits visible background paint and can incorrectly certify “no near-white app background island” (line 9). Likewise, a source border box outside main cannot be discarded before its shadow/pseudo effects are inventoried.

**Required correction:** traverse rendered source elements independently of border-box area/intersection; validate and inventory shadows and pseudo paint before pruning ordinary background fills. Only proven zero-paint sources may be omitted. Add zero-area-host white spread-shadow and positioned-pseudo negatives, plus an outside-main source whose shadow overlaps main. The existing tiny-background fixture does not cover zero-area sources. This is a plan ambiguity with a concrete unsafe implementation route, not an observed product defect.

## Soundness and scope

The separate upper-bound API is compatible with criterion 6 (`2026-10-02-semantic-theme.md:211`) and keeps exact text/focus rules separate. A scalar maximum below 245 is stronger than the original all-channels-at-least-245 predicate; inconclusive upper bounds correctly remain FAIL. Whole-main cells, no minimum paint size, offscreen coverage, bounded gradient grammar, global shadow coverage, fail-closed effects, explicit checked-switch-node accounting and non-exempt shadows are appropriate constraints. The exception must stay on the existing checked switch's own background, not descendants or unrelated paint.

The group-opacity simplification has a defensible proof route: interpret alpha as activation probability and upper-bound actual source-over output by the maximum active source color or canvas. Sorting ascending computes this maximum expectation for independent effective-alpha layers. Shared opacity gates correlate descendant activation; replacing a shared gate by independent per-leaf gates increases the probability that at least one color above any threshold is active: `g*(1-product(1-p_i)) <= 1-product(1-g*p_i)`. Apply recursively for nested groups and combine sibling groups, then integrate over color thresholds. This supports the proposed bound for supported normal source-over paint, not unsupported CSS effects. Keep the plan's requirement for implementation-level proof, finite arithmetic bounds, and exact independently calculated nested-group/sibling fixtures.

## Snapshot and checks

Plan SHA-256 before/after: `29cbe1742d16e779b21b71285b3a95bb9bc8438db6048b0a5c49bff1185fc222`. Accepted semantic plan: `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`.

Read source baseline in `.worktrees/semantic-theme-2026-10-03`: helper `009af7c626199e6e94d56b2565d721f6c4823e12adb36581ab2f2e0ed2652b79`; dark spec `84fa37ba59d71c8a483295266e46547444fe294dc072c24ca90880b4fdd468be`. Current route loop lines 278–318 covers Home, Settings and Meeting Details with overflow checks; its 8px filter and per-region exact resolver explain why this amendment changes the oracle rather than product styling.

Reviewer checks: numbered source/plan reads and SHA-256 via local Python; a deterministic Python scalar-compositing diagnostic generated 100,000 nested normal-alpha group cases (depth <=3, up to four children, random canvas and colors), comparing exact source-over against sorted flattened bounds. No counterexample beyond floating-point noise (`-5.68e-14`) appeared. This diagnostic is not a formal proof, CSS renderer validation or browser run. No amendment implementation exists to approve. Current background failures are reported unresolved paint, not confirmed near-white pixels.

Review changed_paths: only this report. No source, test, plan, Git or task-status changes. Blocker: I1 inventory clarification. Next action: amend that selection rule and required fixtures, then re-review the exact plan before implementation. Full-theme, snapshot, native and release gates remain open.
