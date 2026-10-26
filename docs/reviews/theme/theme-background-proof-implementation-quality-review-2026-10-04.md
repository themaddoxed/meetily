# Background absence proof implementation — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**QUALITY FAIL: Critical 0 / Important 3 / Minor 0.** The numerical bound follows the reviewed model, but omitted blending/pseudo paint and an expanded exception can admit unsupported white backgrounds.

## Findings

1. **Important — reject non-normal background blending.** `frontend/tests/helpers/rendered-contrast.ts:24–31` checks mix-blend-mode but not background-blend-mode. A solid `rgb(200,200,200)` background with a supported constant `linear-gradient(to right,rgb(100,100,100),rgb(100,100,100))` and `background-blend-mode:color-dodge` can render white. The current normal-source-over bound remains approximately 200 and can pass. Add a non-normal background-blend-mode guard for principal and pseudo styles plus a color-dodge regression. This violates the admitted rejection of non-normal blending, not merely an optional heuristic.

2. **Important — restrict the checked-switch exception to its admitted node.** Helper line 114 uses `closest('[role="switch"][data-state="checked"]')`, exempting all descendant fills/images as well as the switch itself. The prior checker used matches on the control; the plan permits no new exception. An arbitrarily large white child under a checked switch is now invisible to the bound. The new `checked switch fill` fixture at `theme-dark-surfaces.spec.ts:245` even expects a white descendant to pass, so it freezes the broadened exception. Use the self-only predicate and require a white-child negative. Keep shadows inventoried independently, as they already are.

3. **Important — fail closed on active unsupported pseudo backgrounds.** Helper lines 101–110 inventory only ::before/::after; no check rejects top-layer ::backdrop paint. A shown modal dialog with a dark principal background and `dialog::backdrop { background:white }` can cover main with white while its only inventoried box remains dark. The backdrop is not a DOM element, so the all-element traversal cannot repair the omission. Reject active unsupported modal/popover backdrop paint or explicitly account for it, with a white-backdrop negative. The plan requires unsupported pseudo paint to fail. The reviewer identified this source-level counterexample without executing a new browser experiment; do not interpret it as measured product-route behavior.

## Snapshot and numerical assessment

Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Reviewed plan v3 `3da22e4b231f5cfabe5b37e09ed56c3ad684971a88d3ccd9e67a2de5ef91fe05`.

- `frontend/tests/helpers/rendered-contrast.ts`: `3cef9cbcc8831a39505348a760273125369404ab268e36a6bcf03f1c82b0a5fc`
- `frontend/tests/theme-dark-surfaces.spec.ts`: `38c65863b06d20a9e70c650ed7d4c8e57c4a7aed45bf70c50ba10830234953b7`

Removing only the newly prepended function in memory reproduces accepted helper SHA `009af7c626199e6e94d56b2565d721f6c4823e12adb36581ab2f2e0ed2652b79`; the text/focus implementation is byte-preserved.

The three component bounds use independently ascending components and the accepted opacity-product overestimate. Closed rectangle intersection includes boundary-only coverage, whole-scope shadows avoid a guessed blur cutoff, zero-area/off-scope owners are visited before principal-fill filtering, and opacity-hidden ancestry/visibility-visible descendants are distinguished. Bounds have finite node/layer/cell/update caps and a 1e-9 positive update margin. With at most 5000 ancestor factors, the usual double-precision relative product error times the 255 channel range is below that margin for a contributing layer; the additive margin also covers each short update. The exact-rational experiment supports the model but does not validate CSS paint inventory or browser parsing.

## Observed evidence and limits

Reviewer actions: read-only source and fixture inspection, in-memory byte comparison, SHA-256 checks, and parsing retained logs/math-script contents. No browser, test process or enumeration was run by this reviewer.

- `/private/tmp/meetily-bg-diag/fixtures.log`, SHA `b64fde9a9b1a7c86d7048a1f81f9714663f393b60f67502bf6cab701e7e27347`: 29 passes (7.6s), with no regressions for the three gaps above.
- `/private/tmp/meetily-bg-diag/routes.log`, SHA `5a122a7c2440687812b9035a2126cd4cab99755313d01c2723c7fa69ad1648dd`: complete JSON reports 0 expected / 3 unexpected (39.11s). All three routes fail `opaque HTML canvas`. This is an unresolved prerequisite, not a detected white island or route acceptance.
- `/private/tmp/meetily-bg-diag/check-opacity-bound.py`, SHA `1857bff900438246298306989d549dc7db3be73f0e90a95cef1898b1ffd3f599`, and `opacity-bound-result.json`, SHA `a86ec978f9a68427d7787f7e78f32b60c7851d1eda7d31a9d676b0b15810336e`: inspected Fraction enumeration of parent/child/nested group and external sibling in both stacking positions, reporting 166698 cases / 0 violations. This is supplemental finite-model evidence, not implementation or browser proof.

`changed_paths`: this report only. `blockers`: the three Important findings and unfulfilled real-route canvas prerequisite. `next_action`: preserve this snapshot, add targeted regressions, repair the guards/exception, and resolve the canvas contract through the authorized planning path before another source review. Full theme/native/release acceptance remains open.
