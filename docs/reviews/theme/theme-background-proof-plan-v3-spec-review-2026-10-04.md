# Background absence plan v3 — independent SPEC review

**PASS for the bounded plan amendment. C0 / I0 / M0.** No actionable findings remain. This is plan acceptance, not implementation or full-theme acceptance.

## Snapshot and scope

Reviewed 2026-10-04. Exact plan `docs/superpowers/plans/2026-10-04-theme-background-proof-v3.md` SHA-256 before/after: `3da22e4b231f5cfabe5b37e09ed56c3ad684971a88d3ccd9e67a2de5ef91fe05`. Compared the full textual delta against preserved v2 `1a8cae8e3f81786a5100ec87ca809e7c25036e4d691223c722db89fdba9e2476`, whose inventory correction and restricted shadow rule were reviewed separately. Canonical semantic plan reference remains `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`.

Scope remains the two existing helper/dark-surface test paths. No production, palette, snapshot, dependency or text/focus acceptance change is proposed. Review changed_paths: only this report.

## Specification and soundness

Lines 9 and 33 correctly preserve the original predicate: a pixel is near-white only when every channel is at least 245. For a cell with valid component bounds `actualR <= uR`, `actualG <= uG`, `actualB <= uB`, any one `u < 245` proves that no pixel in that cell can satisfy the near-white predicate. Requiring this for every covered cell and boundary proves absence across the scope. `max(min(uR,uG,uB)) < 245` expresses that same sufficient condition; it must not replace the per-cell inventory/coverage checks.

Lines 21 and 31 retain conservative channel-wise gradient extrema and apply ascending source-component order independently to each channel. Different maximizing orders do not undermine the proof: these are three simultaneous inequalities, not a proposed exact RGB composite. The shared-opacity Bernoulli/association argument applies separately to each scalar channel. The v2 requirements for supported normal source-over paint, exact independently calculated group/sibling fixtures, bounded floating-point error and fail-closed unsupported effects remain intact.

The change properly avoids v2's unnecessarily strong rejection of opaque yellow/red solely because one channel exceeds 245. Line 37 adds those positive fixtures while preserving white, accumulated translucent white and mixed-color near-white negatives. It adds no palette exception. Zero-area/off-main source coverage, checked-switch-only background exception with non-exempt shadows, offscreen coverage and complete main remain unchanged. No hidden threshold or coverage relaxation was found.

## Checks and limits

Reviewer actions: read the v2-to-v3 diff in full, check the predicate and channel-wise proof algebra, rehash the exact v3 file. No implementation or browser run was performed. The reported real-page inventory motivating yellow/red handling was not independently audited in this bounded plan review; mathematical acceptance does not depend on those particular observed colors.

Blockers for implementing this bounded proposal: none. Next action: implement the two test paths, prove the required positive/negative fixtures and actual three-route checks with ledgers, then review exact implementation hashes. Full-theme, snapshot, native/macOS and release gates remain open.
