# Rendered contrast round4 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL for the general helper: Critical 0 / Important 1 / Minor 0.** The previous outside-ring geometry finding is resolved. The observed leaf-text light-mode failure is separately credible and is not invalidated by the remaining descendant-text defect.

## Important finding

In `frontend/tests/helpers/rendered-contrast.ts:80-92`, text ranges are collected from every descendant text node, but only its parent color is compared with the supplied element. `resolve` later starts from the supplied element, omitting the descendants between it and the actual text. The foreign-paint scan permits a static descendant with transparent background and opacity below 1 because it has no recognized paint. Thus `<div style="color:black"><span style="opacity:0.1">Sample</span></div>` on white is accepted with black/white ratio 21, while the rendered glyphs are mostly white. Descendant filters or other effects can likewise escape the ancestor-chain validation.

Fix: either resolve each sampled text node from its actual parent, including its complete chain, or fail closed unless intervening descendant rendering is equivalent (opacity, filters, masks, text-shadow, etc.). Add the opacity-only descendant regression. This is a concrete false PASS in the new range-based API, not a requirement to implement a general renderer. Named leaf-element samples can be evaluated separately.

## Exact snapshot

Task root `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

- Helper: `6ce584adf108a8a70899a6120f369980128d2bc3ea73870188fd04e8f1c0f6de`.
- Dark-surfaces test: `5c43dbdebd8c89acf15c77ddd388fff978466d151d129c71cb01ca5d538c1093`.
- Foundation test: `3a65c9afb76235d5e9873b64c6537d48de60d2a779a8cc968a5e351ad37452ea`.

Hashes bound to `frontend/tests/helpers/rendered-contrast.ts`, `frontend/tests/theme-dark-surfaces.spec.ts`, and `frontend/tests/theme-foundation.spec.ts` respectively, checked before/after.

The indicator reach is now computed before foreign-paint validation; the expanded rectangle catches outside-ring-only occlusion. Ancestor-shadow checks reject cases where the expanded region is not safely inside the ancestor's corner bounds. Both targeted negative fixtures pass. Single sharp ring compositing and per-mode Page isolation remain intact. Home measurement now waits for its animated ancestor opacity to reach 1.

## Actual result and correction boundary

Read completed `/private/tmp/meetily-theme-rendered-contrast-round4.log`, SHA-256 `ffcd9cfdc555058158dc1c04abe4551071934c223bd25898122893a4ac6dcdc6`: **9 passed / 3 failed, 38.4 seconds**. Light Settings System leaf-span text again measures **4.34923168556426**, below 4.5. Its supplied locator is the span containing the label, so the descendant-opacity counterexample does not apply to this specific direct text sample. Current light tokens (`globals.css:91-92`) are muted 96.1% and muted-foreground 45.1%; PreferenceSettings assigns muted foreground to the unselected System button inside the muted container. This supports a narrowly scoped semantic light-token correction with a rerun of that exact assertion and affected pairs. It does not accept the entire helper or authorize unrelated styling changes.

Dark measurement stops on unresolved foreign sidebar paint, and focus stops on `auto 1 0; shadows:none`. These are NOT_MEASURABLE results, not proof of poor dark contrast or inadequate focus visibility. No automatic production focus correction follows. Full dark/focus and light-route coverage are incomplete; preserve per-sample failure measurements in the evidence.

Reviewer checks: read current helper/callers/token/selector source, exact hashes and completed coordinator log. No reviewer browser run, screenshot inspection, product edit or native action. `changed_paths`: this report only. `blockers`: descendant-text false PASS and unresolved real-element coverage. `next_action`: reject or correctly resolve descendant rendering with regression evidence; independently retain/rerun the narrow light System RED for any minimal token change. Full theme/native/release acceptance remains open.
