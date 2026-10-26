# Rendered contrast round3 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL: Critical 0 / Important 1 / Minor 0.** Prior mode isolation and several paint/indicator gaps are addressed; actual outside-focus geometry still permits a false numerical PASS.

## Important remaining finding

`frontend/tests/helpers/rendered-contrast.ts:65-66` tests foreign paint against the button's border-box rectangle only. The accepted indicator's outside reach is computed later, and lines 123-129 assume the parent background is its backing. An overlay lying wholly outside the button but over its outline/sharp ring does not intersect the tested rectangle and is silently ignored. For example, a narrow opaque strip beside the right edge can obscure the ring without touching the button. The function can report its unoccluded high contrast while the actual focus indicator is obscured or blended against another color.

Additionally, line 52 skips all ancestors in the foreign-paint loop; validate rejects inset shadows but does not reject ancestor outer shadows. Such paint can affect the outside indicator/adjacent surface and is not included by resolve(parent), which models backgrounds only. This is the unclosed part of the prior geometry finding, not a request for a general browser renderer.

Repair: determine accepted indicator reach before validating its surroundings; resolve or reject paint intersecting the actual outside indicator and adjacent sample region, including ancestor effects not represented in the background model. Add a negative fixture whose occluder intersects only the ring, plus the relevant ancestor-shadow case. Preserve fail-closed classification where geometry cannot be resolved. Do not change production focus styling to accommodate an oracle blind spot.

## Snapshot and resolved issues

Task root `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`; plan `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`.

| Test path | SHA-256 before/after |
| --- | --- |
| `frontend/tests/helpers/rendered-contrast.ts` | `0ebc127c8f46324bf3699ffa058ca834178a6477d5345a1f265056fea32536e0` |
| `frontend/tests/theme-dark-surfaces.spec.ts` | `60f4f4986ba98621cf0ef81db371e5ebbc07ab34198e5ce4548968e09bfdeed3` |
| `frontend/tests/theme-foundation.spec.ts` | `3a65c9afb76235d5e9873b64c6537d48de60d2a779a8cc968a5e351ad37452ea` |

The focus test now creates an independent Page for each mode and asserts effective root class/colorScheme, removing the accumulated-init-script ambiguity. The rectangle scan covers noninteractive and partial-edge overlays over the measured element and rejects painted descendants; its new regression is relevant. A unique centered sharp positive-spread ring is accepted, while unresolved auto/blurred/multiple indicators remain errors. The indicator is now composited once over the outer backing and the resulting color compared against both surfaces, fixing the earlier two-different-colors assumption. The actual timestamp locator now uses the existing `[00:00]` button.

## Observed execution and limits

Read completed coordinator log `/private/tmp/meetily-theme-rendered-contrast-round3.log`, SHA-256 `8a168efde611d5e7e26797e0b88eac45748d2877be214c67f5d1ebbc1745b936`: **7 passed / 3 failed in 37.3 seconds**. The fixture math/negative paint and ring cases pass. Real light Home text reports 3.4642012861127154; dark real-element and focus cases stop on unresolved intersecting paint. These errors are not proof of a light island or inadequate focus contrast. In particular the current focus run does not establish a measured auto-outline failure; it stops earlier on the intersection guard. Dark Meeting Details coverage and both focus modes are not complete.

Before interpreting the numerical light ratio as a stable production defect, retain sampled foreground/background/ancestor values and check transitions/opacity have settled. The ledger is still attached only after all loops succeed, so failing runs lack the intended completed measurement attachment. Preserve failure measurements during diagnosis; do not weaken thresholds or skip unresolved real elements.

Reviewer actions: read exact helper and changed callers/regressions, inspected actual completed log and verified hashes in local Darwin. No reviewer browser/test run, production token/Preference change, native action or Graphify. `changed_paths`: this report only. `blockers`: remaining focus-geometry false-pass case and incomplete real-element/focus evidence. `next_action`: repair and regression-test the outside indicator region, then rerun controlled real-element measurements and classify measured failures separately from NOT_MEASURABLE geometry. Full theme/native/release readiness remains unverified.
