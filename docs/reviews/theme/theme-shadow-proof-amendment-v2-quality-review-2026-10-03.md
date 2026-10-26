# Foreign-shadow proof amendment v2 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** No actionable planning defect remains. The prior effective-intervention Important is resolved by explicit phase checks; implementation and real-text acceptance are still pending.

## Snapshot and resolution

Reviewed `docs/superpowers/plans/2026-10-02-semantic-theme.md`, SHA-256 `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`, particularly Task 3 lines 293–328. No implementation of the new driver is accepted here.

Lines 305–308 now require computed `box-shadow: none` for every candidate before and after each disabled-shadow capture, bounded stabilization, and original computed-shadow/fingerprint restoration around the final capture. This rejects an unchanged screenshot caused by an ineffective override or a delayed transition that never reaches the intended state. The delayed/blocked-override regression is explicitly required at lines 323–324. Restoring original inline values and priorities in finally remains mandatory even when comparison or state validation fails.

Lines 302–310 require a new joint removal proof for the entire accumulated candidate set on every discovery step. This addresses overlapping shadows that individually conceal each other's removal. The overlapping-shadow negative must genuinely exhibit that distinction: single removal leaves the region unchanged while removing both changes it. A generic visible-shadow fixture would not verify the compound failure mode.

The remaining limits keep this mechanism bounded: full outward-rounded viewport region, exact stable PNG equality across baseline/intervention/restoration, fresh identity/style/ancestor/geometry binding, no caching across samples/runs, hard failure beyond 16 candidates, and text-only use inside the existing helper path. The computed RGBA resolver still determines the 4.5:1 result; the image protocol only proves non-interference by the jointly tested foreign shadows.

## Implementation review obligations

These are interpretations of the existing contract, not additional production scope. The fingerprint must identify the actual DOM nodes and current text region, retain unrelated computed styles throughout the intervention, and permit only the intended temporary shadow change. All accepted candidates must be restored before the final mathematical resolver retry. A stale proof or failure to restore any candidate must throw, not return a usable measurement. Cap overflow and timeout must preserve the same cleanup behavior.

Screenshot calls must obey the no-masking/no-other-paint-change clause: do not silently hide relevant paint through screenshot options, inject screenshot CSS or disable animations. Check the computed phase and fresh geometry on both sides of each capture, including baseline/restoration consistency; identical bytes alone cannot replace those checks. The outward clip must cover the full sample at the chosen viewport/device scale without out-of-viewport clipping or inadvertent scrolling. These details need source and fixture evidence in the implementation review.

The exclusion must affect only the proved foreign shadow expansion, leaving actual border-box intersections, descendants/pseudo-element effects, unsupported non-shadow paint and all focus/background rules active. No product change, dependency, screenshot baseline or additional path is authorized.

## Checks and limits

Reviewer actions: read-only exact-plan inspection and SHA-256 checks; compared the explicit requirements with the prior review finding. No browser/test/DOM experiment was performed. A viable plan is not evidence that the real Sidebar shadow is non-influencing. Positive, influencing-shadow, delayed-override, joint-removal, restoration and stale-proof fixtures must run before the driver is used for real text.

`changed_paths`: this report only. `findings: []`. `blockers: []` for the bounded planning gate. `next_action`: implement the bounded protocol and its fixtures, then obtain an exact-source implementation review before accepting named real-text measurements. Full theme/native/release acceptance remains NOT VERIFIED; the prior FAIL report remains preserved.
