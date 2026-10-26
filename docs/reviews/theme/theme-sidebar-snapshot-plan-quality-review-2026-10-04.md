# Sidebar snapshot reconciliation plan — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded planning QUALITY PASS: Critical 0 / Important 0 / Minor 0.** The visible delta has a concrete integrated-source explanation. The proposal authorizes only reconciliation of the two stale baselines with semantic assertions and subsequent ordinary comparisons; it does not accept replacement images before their review.

## Snapshot and provenance

Reviewed `docs/superpowers/plans/2026-10-04-theme-sidebar-snapshots.md`, SHA-256 `a751c5d4938cbc9e7469976bda669e1d2942fb27ffd59524c8973c2d08946f03`. Task checkout `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

Read-only Git history confirms the snapshot directory last changed before this base in `8ffc5e6de85d494ecf25fd34afb40b7a67c432cf`. Commit `fb48ee963b9e6d9ba269138c6bd7f1715ff2904a` is an ancestor of the base and added live lifecycle metadata to the fixture and Sidebar. Current `SidebarProvider.tsx:200–226` groups meetings into localized date folders; `Sidebar/index.tsx:503–508` renders status and formatted duration. `src/testing/tauri-browser-mocks.ts:11–17` supplies ready status, duration 1800 and the fixed June 20 timestamp. These explain the added folder, Ready and 30:00 without a new product change.

## Independent visual inspection

Viewed both existing expected PNGs and both actual PNGs from the failed full run. In each mode, the prior direct meeting row becomes an indented child of the June 20 folder, with lifecycle/duration beneath its title. Logo/search/navigation and bottom action locations remain aligned. The reviewed change is coherent with the source and the already admitted light muted-token correction; no unrelated visible shell redesign was identified in these images.

Evidence SHA-256:

- Existing `home-light-chromium-darwin.png`: `aa3981eb38e051e6c64bdd17939d6b6f16c26ccfd6156b59f3662bf9f41fd44a`.
- Existing `home-dark-chromium-darwin.png`: `90d179e0141ec046eea4255e03ade4dcb1159f7f7dada411f14e440e1a3842e7`.
- Failed-run light actual PNG: `e694118eca915f75fe9219b06a19d2e12773340c5662dc0fd2d2045cbd679fa1`.
- Failed-run dark actual PNG: `ec27aedc113d3248eb801eed8ab4072f3128bad45edd159e17acd0326bebfeee`.

Actual images are under task `frontend/test-results/theme-shell-home-shell-ren-f0ac3-ntic-surfaces-in-light-mode-chromium/home-light-actual.png` and `theme-shell-home-shell-ren-55f90-antic-surfaces-in-dark-mode-chromium/home-dark-actual.png`; baselines are under `frontend/tests/theme-shell.spec.ts-snapshots/`.

## Plan adequacy and limits

The two additional PNG paths and already admitted shell spec are sufficiently narrow. Explicit visible date/status/duration assertions prevent a snapshot-only update from hiding missing lifecycle content. Retaining existing navigation, semantic colors, borders and overflow checks, recording old/new hashes, inspecting the new images and requiring a second non-update comparison provide reviewable acceptance. No masks, tolerance changes, stale fixture rollback or product styling workaround is authorized.

The date label depends on locale/timezone; record the environment with the image evidence and preserve the intended fixture interpretation. That is an evidence boundary, not permission to edit runner configuration. An unexplained additional visual delta must stop reconciliation as the plan says.

Parsed `/private/tmp/meetily-theme-full-current-2026-10-04.json`, SHA-256 `79b91ec70342882206fcbab043a63ecf8f39b71bfbb79237e3cbe6a62254a55d`: 128 expected, 2 unexpected, zero skipped/flaky, with only the two Sidebar screenshot failures (862 and 849 pixels). Reviewer did not run this test process or update images. A passing snapshot-update command alone will not satisfy the required ordinary comparison.

`changed_paths`: this report only. `findings: []`. `blockers: []` for this planning gate. `next_action`: back up the two baselines, add the semantic assertions, perform the narrow update and normal comparison rerun, then independently review exact final hashes/images. Full frontend/project/native/accessibility/release gates remain separate and open.
