# Foreign-shadow proof amendment — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL for this exact planning snapshot: Critical 0 / Important 1 / Minor 0.** The bounded causal-image approach is viable, but the intervention must be proven effective before equal images can certify a shadow as irrelevant.

## Finding

**Important — verify effective shadow removal and restoration, not only the inline assignment.** `docs/superpowers/plans/2026-10-02-semantic-theme.md:301–304` requires stable images before/after overriding `box-shadow` and after restoring its inline value/priority. It does not explicitly require the computed shadow to become `none` before and throughout the after captures. With a delayed CSS transition, both after screenshots can retain the original influencing shadow; an override defeated by a stronger declaration has the same failure mode. All five PNGs can then be identical without any causal removal having occurred. Candidate/style freshness alone does not distinguish unchanged baseline paint from a successful intervention unless the expected phase-specific computed shadow is checked.

Require a bounded wait for computed `box-shadow: none`, verify it around both after screenshots, and fail if the state cannot be reached or stays unstable. Restore the original inline value/priority in finally, then verify the original computed shadow and the baseline fingerprint before the final screenshot and resolver retry. Do not disable animations to force success. Add a delayed or ineffective-override negative fixture: it must not mint a non-interference proof while the original shadow is still painted. This is needed even if the initial real Sidebar fixture currently has no such transition.

## Snapshot and assessment

Reviewed plan SHA-256 `465682e54f4bfb1db97a842d1ac2a59505a83709cc2adf0385c2698de91dbcc4`, Task 3 lines 293–320. Current local helper/source and retained `/private/tmp/meetily-theme-final-text.log` confirm a conservative foreign-sidebar-shadow rejection; that failure is unresolved paint, not proof of deficient contrast. There is no implementation of this new protocol reviewed here.

The other proposed boundaries are appropriate: a full outward-rounded viewport clip, exact bytes without tolerance or masks, repeated baseline/intervention captures, restored baseline capture, candidate/style/ancestor/rectangle/text-region identity, no cross-run caching, and a hard discovery cap. PNG equality is conservative when encoding changes and does not require adding a decoder. The intervention must remove only the candidate shadow; accepting its proof must not exclude the node's border, descendants, pseudo-element paint or unrelated effects. Changed pixels must remain a failure rather than be reclassified as a favorable contrast measurement.

Sequential candidate proofs need the same restored scene and fresh fingerprint before each attempt and the final computed resolver. The requirement to fail on stale state already supports that boundary. The 16-candidate cap must fail on overflow, never silently stop discovery. The text-only driver stays in the existing helper path; focus/background scans and the mathematical RGBA threshold remain unchanged. Synthetic positive, influencing-shadow negative, restoration and stale-state fixtures are necessary before named real text can use the mechanism.

## Checks and limits

Reviewer actions: read-only plan and source inspection, SHA-256 pin, and inspection of the retained unresolved-text failure. No browser, screenshots, mutations of product DOM or test executions were performed by the reviewer for this proposal. No claim is made that the Sidebar shadow is actually irrelevant until a correct causal intervention demonstrates it.

`changed_paths`: this report only. `findings`: one Important protocol gap. `blockers`: effective removal/restoration verification and its regression. `next_action`: amend those requirements, repin and re-review before implementing the driver. No product/dependency/path-count expansion or full theme/native/release acceptance is approved by this report.
