# Sidebar snapshot amendment — independent SPEC review

**PASS for this bounded plan amendment. C0 / I0 / M0.** No actionable findings. New PNG baselines still require exact-image review and ordinary comparison PASS after generation.

## Snapshot and scope

Plan SHA-256 before/after: `a751c5d4938cbc9e7469976bda669e1d2942fb27ffd59524c8973c2d08946f03`. Reviewed 2026-10-04 against base `53f4415058fb01e38b8892a47ac036bfffa394ba`, canonical semantic plan `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`, and accepted background amendment v4. Review changed_paths: only this report. No source, baseline image, configuration or Git mutation.

The amendment names exactly the two light/dark Darwin Chromium PNGs and adds them to the existing 19-path scope. It retains the existing shell test and adds visible date/status/duration assertions before screenshot comparison. Backup, old/new hashes, rendered inspection, narrow update, ordinary non-update comparison and independent final review are explicit. No masks, tolerance increases or fixture rollback are permitted. This satisfies the canonical plan line 202's route for a legitimate inspected screenshot correction and explicitly supersedes the conflicting Task 4 no-update instruction within these two paths.

## Provenance and visual assessment

Independently viewed all four expected/actual PNGs with the image tool. Both actual images add the `Jun 20, 2026` folder, nest the meeting beneath it, and show `Ready` / `30:00`. No unexplained visible navigation, branding, bottom-action, border or overall shell-layout change was observed. Visual inspection is bounded to these 256×1000 sidebar images, not the full application.

Read base source: `SidebarProvider.tsx:200–226` constructs date folders; `Sidebar/index.tsx:503–508` renders lifecycle and duration. Current files and `src/testing/tauri-browser-mocks.ts` match the exact base bytes. The fixture supplies the fixed timestamp and 1800-second duration. `git merge-base --is-ancestor fb48ee9 53f4415…` returned 0. Read-only Git history confirms the PNGs last changed at `8ffc5e6de85d494ecf25fd34afb40b7a67c432cf`. Thus the depicted lifecycle/date delta originates in already integrated behavior, not a new product change introduced to justify updating snapshots.

## Evidence

SHA-256 of inspected images, identified by attachment name in the full-run JSON:

- `home-light-expected.png`: `aa3981eb38e051e6c64bdd17939d6b6f16c26ccfd6156b59f3662bf9f41fd44a`.
- `home-light-actual.png`: `e694118eca915f75fe9219b06a19d2e12773340c5662dc0fd2d2045cbd679fa1`.
- `home-dark-expected.png`: `90d179e0141ec046eea4255e03ade4dcb1159f7f7dada411f14e440e1a3842e7`.
- `home-dark-actual.png`: `ec27aedc113d3248eb801eed8ab4072f3128bad45edd159e17acd0326bebfeee`.

Source SHA-256: SidebarProvider `735122dfc5ff33a72071282f59dd7fd8d8d8302bf9e9b3ef48fca2e78be653ad`; Sidebar/index `a8be0aa2f5c613fc6b7d3b2f290dd7383dac13f7c01c0bd933954a588ab63926`; mock fixture `006985f3fd81881e7e228bd3f7c25a6aca0e17ee71d25ef1dddc33f3a72927eb`.

Independently parsed `/private/tmp/meetily-theme-full-current-2026-10-04.json`: **128 expected / 2 unexpected / 0 skipped / 0 flaky**, duration 102.781s; failures are the two shell snapshot cases. JSON SHA-256 `79b91ec70342882206fcbab043a63ecf8f39b71bfbb79237e3cbe6a62254a55d`. This is a failed full run and remains preserved; amendment acceptance does not relabel it as successful. No reviewer-run suite is claimed.

## Next action and limits

No blocker to the specified narrow update after the DOM assertions and backups. Review the actual new image/source hashes and ordinary two-case comparison results before accepting the replacement baselines. Full frontend/project checks and native/accessibility/release gates remain separate; browser snapshot reconciliation establishes none of those by itself.
