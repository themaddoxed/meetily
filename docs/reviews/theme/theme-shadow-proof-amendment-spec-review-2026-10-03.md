# Disjoint foreign-shadow proof: independent SPEC amendment review

2026-10-03. **FAIL — Critical 0 / Important 2 / Minor 0.** The proposed bounded driver preserves useful constraints, but two causal-proof gaps must be specified before implementation.

## Findings

1. **Important — setting inline none does not prove the shadow was actually removed for the after images.** Plan lines 301–308 requires stable PNG buffers after an override but does not explicitly require computed box-shadow:none at each after capture. A delayed shadow transition can leave the old rendered/computed shadow present for both captures. Equal before/after/restored PNGs then “prove” non-interference even for an influencing shadow because the intervention never took effect. Classification: missing proof precondition. Require the intended computed none state on both after captures, original computed state after restoration, and unchanged relevant state across the captures. Wait for the real transition to settle or fail closed; do not disable it. Add a delayed-transition influencing-shadow negative fixture that cannot pass merely because its after images were premature. A generic stale-style fingerprint must explicitly distinguish the expected intervention from unexpected style drift.

2. **Important — independent shadow-removal proofs do not compose across candidates.** Lines 309–311 permits retrying with proofs for up to 16 shadows. Two overlapping opaque/saturated shadows can cover the same sample: removing either alone leaves the other, so each individual PNG comparison passes, yet the computed resolver can eventually exclude both and evaluate a background never shown by the browser. Classification: incorrect acceptance composition. Either reject multiple candidate shadows for one region or require a joint intervention proof for the complete exclusion set (before/all-disabled/restored), in addition to identity/region freshness. Revalidate the accepted set when discovering another candidate. Add a mutually masking two-shadow fixture; a cap on count does not fix causal masking.

## Scope and evidence

Reviewed only the new Task 3 paragraph at lines 293–320 in `docs/superpowers/plans/2026-10-02-semantic-theme.md`, SHA-256 `465682e54f4bfb1db97a842d1ac2a59505a83709cc2adf0385c2698de91dbcc4`, against its unchanged exact RGBA/opacity and fail-closed acceptance contract. This is source-based plan reasoning, not a reviewer-run browser reproduction. No new helper implementation was reviewed.

The proposed separation is otherwise within scope: existing helper path, Node/Locator driver for text only, no new dependencies or baselines, outward-rounded full in-viewport clip, exact PNG equality without masks/tolerances, candidate/ancestor/region identity, bounded discovery and finally restoration. Focus/background remain strict; a restored computed resolver still supplies the actual 4.5:1 decision rather than a screenshot-derived substitute. Those boundaries should remain while resolving the two findings.

## Next action and limits

Amend the before/after state contract and multi-candidate composition rule, add their negative fixtures to the required restoration/stale/positive/influencing cases, then obtain fresh SPEC/QUALITY before helper edits. Existing Sidebar-shadow failure stays unresolved; no source hash, named real text, contrast, full theme, native or release acceptance follows this plan review.

`changed_paths`: only `docs/fork/theme-shadow-proof-amendment-spec-review-2026-10-03.md`; no helper/source/plan/status/Git edits or Graphify.
