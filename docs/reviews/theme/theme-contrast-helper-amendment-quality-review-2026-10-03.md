# Shared rendered-contrast helper amendment — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the path/module amendment: Critical 0 / Important 0 / Minor 0.** No actionable finding blocks creation of the narrowly specified test helper after both independent plan gates. This is not acceptance of an implementation or contrast result.

Observed plan `docs/superpowers/plans/2026-10-02-semantic-theme.md`, SHA-256 `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`, verified before/after. Task root `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`. The proposed `frontend/tests/helpers/rendered-contrast.ts` is absent. Existing task diff still lists only the three previously reviewed prepaint files; no helper implementation was reviewed or created.

The amendment at plan lines 85-100 adds exactly one test-only module shared by the foundation keyboard-focus case and dark-surface suite. It requires no registered tests and no production callers. This avoids spec-module imports registering another suite's tests and avoids duplicating a substantial color/geometry oracle. The allowed path list explicitly includes path 19. The original 18 hashes remain historical baseline pins; the helper must be recorded as a new file, not falsely labelled unchanged.

Task 3 retains full-precision RGB/alpha compositing, an opaque resolved canvas, named real-element assertions and explicit failure for unsupported rendering effects. It still rejects skipping translucent candidates. Helper math fixtures alone cannot replace actual Home/Settings/Meeting Details and keyboard-focus measurements; indicator contrast must cover both adjacent surfaces. These requirements are not weakened by moving the engine into a shared test module. The current dark-surface alpha-skip implementation remains something Task 3 must replace, not accepted coverage.

Implementation review should check that fixture and real-element paths actually call the same engine, that unsupported geometry produces an actionable failure rather than dropping samples, and that pure color math remains distinguishable from browser DOM/geometry acquisition. No speculative public production API, new dependency, palette change or additional path is justified by this amendment. Fixture tests belong in the already admitted suites.

Reviewer checks: read amendment/allowlist/Task 3 criteria, inspected current dark-surface alpha handling, verified helper absence and scoped Git status in local Darwin. No tests, product edits or Git mutation. The ongoing broader theme run supplies no result to this planning review.

`changed_paths`: `docs/fork/theme-contrast-helper-amendment-quality-review-2026-10-03.md` only. `blockers: []` for this bounded QUALITY plan gate. `next_action`: after SPEC acceptance, implement the shared test engine and meaningful positive/negative fixtures, then independently review exact code and real-element results. Full theme, native and release acceptance remain open.
