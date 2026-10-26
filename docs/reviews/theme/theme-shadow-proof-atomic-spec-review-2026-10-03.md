# Shadow-proof atomic consumption — independent SPEC review

Status: **PASS for this bounded correction**. Critical: 0; Important: 0; Minor: 0. No actionable findings remain in the reviewed delta. Full semantic-theme acceptance remains open.

## Snapshot and scope

Reviewed 2026-10-04, feature worktree `.worktrees/semantic-theme-2026-10-03`, against the accepted Task 3 shadow-proof contract in the canonical plan. This is a dirty-source hash-bound review, not a commit or release approval. Supplied base: `53f4415058fb01e38b8892a47ac036bfffa394ba`.

| File | SHA-256 before and after review |
| --- | --- |
| `frontend/tests/helpers/rendered-contrast.ts` in feature worktree | `009af7c626199e6e94d56b2565d721f6c4823e12adb36581ab2f2e0ed2652b79` |
| `frontend/tests/theme-dark-surfaces.spec.ts` in feature worktree | `84fa37ba59d71c8a483295266e46547444fe294dc072c24ca90880b4fdd468be` |
| Root `docs/superpowers/plans/2026-10-02-semantic-theme.md` | `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41` |

Review changed_paths: only `docs/fork/theme-shadow-proof-atomic-spec-review-2026-10-03.md`. Product, tests, plan and Git state were not edited by this reviewer.

## Contract and correction

The accepted contract requires fresh identity/state when consuming a text-only shadow exclusion, not merely successful PNG comparisons earlier in the asynchronous driver. `rendered-contrast.ts:17–49` now validates the opaque original DOM/CSS/rectangle/font/viewport inventory within the same synchronous browser callback that measures and consumes the proofs. It checks node and child identity as well as values; an identical replacement cannot inherit an earlier exclusion. The restored-phase check permits no controlled shadow-value difference. Both restored measurements at lines 342 and 415 pass the inventory and candidate indices. There is no intervening await between inventory validation and proof consumption inside the measurement function.

The regression at `theme-dark-surfaces.spec.ts:193–225` swaps an identical candidate clone after the fifth PNG and final restored-phase verification, then requires the public driver to reject with a node-identity error. It asserts that the injection actually occurred at five captures and restores the temporary Playwright API wrappers. This targets the previously uncovered final-consumption interval, rather than only repeating an earlier capture-phase mutation test.

The correction does not change contrast thresholds, computed-color math, exact PNG equality, joint candidate removal, or the focus/background paths. It remains inside the existing two test paths.

## Checks and evidence

Reviewer actions: read exact source with numbered lines; SHA-256 source/plan before and after; parse completed Playwright JSON and decoded contrast attachments; read terminal fixture log. No reviewer-run browser, typecheck, native or model test is claimed.

- `/private/tmp/meetily-theme-shadow-atomic-fixtures.log`: observed terminal **9 passed (11.0s)** in Chromium, including the final-consumption replacement regression. SHA-256 `785458104da23827b921fbf687422d8b90066b2b3f9e4f430960f46c88ce58ed`.
- `/private/tmp/meetily-theme-real-text-atomic.log`: completed JSON, **2 expected, 0 unexpected, 0 skipped, 0 flaky**, no top-level errors; duration 43.351 s. SHA-256 `4f914d59ecf898d3fed5190286c0d88d1d2231439a1b37c58f5ef270758a2f74`. Each mode records 9 named text samples across Home, Settings and Meeting Details, with 2 shadow proofs. Minimum recorded ratio: light **4.542288769612194**, dark **5.999400854306462**. These exceed the accepted 4.5:1 threshold for those samples.

Environment boundary: supplied feature Playwright Chromium browser runs with synthetic meeting fixtures. Logs were independently read, not rerun. Typecheck exit 0 was reported by the implementer but not independently established from a supplied typecheck log in this review.

## Blockers and next action

No blocker for this bounded correction. It closes the reviewed final-consumption race under the accepted supported-paint model. Continue the outstanding full-theme gates; these two text scenarios do not establish all surfaces, snapshots, native WebView behavior, installed-app behavior or release readiness.
