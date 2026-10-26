# Disjoint shadow proof v2: independent SPEC amendment review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** Both v1 Important findings are closed at the plan level. No helper implementation or actual named-text acceptance is claimed.

## Snapshot and scope

Read Task 3 “Disjoint foreign-shadow proof clarification” in `docs/superpowers/plans/2026-10-02-semantic-theme.md`; exact SHA-256 `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`. Scope is the revision of the two causal-proof conditions and associated negative fixtures. No product/test source changes were reviewed or made.

## Closure of prior findings

- **Actual intervention:** before and after each disabled-shadow capture, every candidate must have computed box-shadow exactly none, with a bounded five-second stabilization wait. After restoring original inline values/priorities, each original computed shadow and fingerprint must hold before and after the final image. Delayed, ineffective and failed-restoration cases reject the proof. Two stable pictures of a shadow that was never removed no longer satisfy the contract.
- **Set composition:** every expanded candidate set requires a new joint removal proof covering all previously accepted candidates plus the new one. Independent per-shadow certificates cannot be added. A mutually masking opaque-shadow negative fixture is explicitly required, so removing each shadow separately cannot justify excluding their combined paint.

Both fixes preserve the exact full-region PNG comparisons, outward-rounded viewport clip, before/after stability, identity/style/ancestor/rectangle/region freshness and 16-candidate cap. Unsupported non-shadow effects, actual border-box overlap, stale state or any changed pixel remain fail-closed. Computed RGBA/ancestor-opacity math still owns the 4.5:1 decision; screenshots only certify exclusion of this precisely bounded foreign-shadow contribution.

The driver remains text-only within the existing helper path and 19-path allowance. Focus/background requirements, dependencies, product source and screenshot baselines are unchanged. Positive, genuinely influencing, delayed/blocked, mutually masking, restoration and stale-proof fixtures are required before real Home/Settings/Meeting Details adoption.

## Next action and limits

After independent QUALITY amendment acceptance, implement the bounded driver and its proof-state/cleanup tests, freeze exact hashes and obtain implementation review. Final code must invalidate all certificates when candidate set, measured region or fingerprint changes, and must restore every touched inline property even after failure. These are implementation obligations, not results established by this plan PASS.

No browser, contrast, full-theme, native or release PASS. `changed_paths`: only `docs/fork/theme-shadow-proof-amendment-v2-spec-review-2026-10-03.md`; no source/plan/status/Git edits or Graphify.
