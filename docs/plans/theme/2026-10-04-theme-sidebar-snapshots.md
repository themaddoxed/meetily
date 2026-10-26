# Reviewed Sidebar snapshot reconciliation proposal — 2026-10-04

Status: pending independent SPEC/QUALITY review; do not write a baseline until accepted. Supplements main semantic-theme plan SHA83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41 and background amendment v4 SHAc61c921567f0d6e12f7760c6e0585bd7c529f51cb312585a7d00f08bf512a997. This is a bounded correction to outdated fixture screenshots after inspecting the rendered delta, not permission to accept arbitrary visual changes.

## Current evidence

Feature base is main/dev `53f4415058fb01e38b8892a47ac036bfffa394ba`. Current five theme suites exit1: 128 PASS / 2 FAIL / 0 skipped, `/private/tmp/meetily-theme-full-current-2026-10-04.json`. Only failures are light/dark Sidebar screenshot comparisons (862/849 differing pixels); background, text, focus and other named browser scenarios pass. Owner visually inspected both expected/actual image pairs: new Sidebar contains the date folder `Jun 20, 2026`, lifecycle `Ready` and duration `30:00`; the meeting moves into that folder. Logo, navigation, action buttons and overall shell remain aligned. These additions are already integrated product behavior, not new theme implementation.

The existing PNG baseline last changed in `8ffc5e6de85d494ecf25fd34afb40b7a67c432cf` (`test(ui): update v0.4.1 sidebar snapshots`). Later integrated `fb48ee9` (`feat(meetings): add live lifecycle statuses and recovery`) updated SidebarProvider, Sidebar/index and the fixture metadata. SidebarProvider lines200–226 build date folders from startedAtMs/createdAt. Sidebar/index lines503–508 show status and duration. Current fixture `theme-test-meeting` has fixed `2026-06-20T09:00:00Z` timestamp and duration_seconds1800, so the observed date/time labels have a source explanation. Existing light muted-token correction remains admitted by the earlier measured contrast RED/GREEN; do not apply an unrelated palette/UI change during reconciliation.

## Admitted delta

Keep the existing 19-path theme implementation scope and conditionally add only:

20. `frontend/tests/theme-shell.spec.ts-snapshots/home-light-chromium-darwin.png`
21. `frontend/tests/theme-shell.spec.ts-snapshots/home-dark-chromium-darwin.png`

`frontend/tests/theme-shell.spec.ts` is already admitted. Add explicit visible fixture assertions for date folder, `meeting-status-theme-test-meeting` = Ready and `meeting-duration-theme-test-meeting` =30:00 before screenshot comparison. Keep all existing navigation, recording, search, background, border and overflow checks. Do not hide/mask/normalize the new rows, pin an old fixture shape, increase screenshot tolerance or modify actual product behavior to resemble the old image. No other snapshots, manifests, UI/Rust files or runner config change.

Back up the two old baselines to a dated private temporary evidence directory and record SHA-256. Run only the two named `home shell renders semantic surfaces` cases with snapshot updating against this exact current fixture/source/environment; this narrowly supersedes the main plan's Task4 prohibition after rendered inspection and independent amendment acceptance. Record new image hashes and source hashes. Inspect expected/actual/new PNGs and ensure every accepted visible delta is explained by integrated lifecycle/date behavior and the already reviewed theme correction. If another unexplained change appears, stop and investigate before accepting it.

Rerun exactly the two named snapshot cases without snapshot updating. Require ordinary comparisons PASS. Independent reviewers inspect the actual baseline delta and exact implementation hashes before acceptance. The previous failed full-five run remains preserved; repeat affected screenshot checks after the baseline change, without repeating already successful non-snapshot cases unless new source changes or a concrete concern require it. Then continue full frontend/project checks and the separate native/accessibility/exact-artifact gates. Browser screenshot reconciliation cannot establish release readiness or native correctness.
