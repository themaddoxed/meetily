# Foreign-shadow atomic freshness correction — independent QUALITY review

Source-authored by `/root/plans_quality`, completed 2026-10-04 (requested report filename retained).

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** No actionable finding remains in the reviewed atomic-freshness correction. The preceding Important is resolved; wider theme acceptance remains separate.

## Exact snapshot

Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Independently calculated source SHA-256:

- `frontend/tests/helpers/rendered-contrast.ts`: `009af7c626199e6e94d56b2565d721f6c4823e12adb36581ab2f2e0ed2652b79`
- `frontend/tests/theme-dark-surfaces.spec.ts`: `84fa37ba59d71c8a483295266e46547444fe294dc072c24ca90880b4fdd468be`
- Root `docs/superpowers/plans/2026-10-02-semantic-theme.md`: `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`

## Source assessment

`measureRenderedContrast` now validates the original opaque inventory synchronously at helper lines 17–50 before consuming any supplied shadow certificates. The check covers node identity, child/text identity, viewport/scroll/device scale, attributes, canonical inline declarations, computed styles, geometry, pseudo styles and font inventory. It uses the restored phase, so it does not normalize away the box-shadow value. The controlled-node raw style attribute exception remains paired with exact canonical inline value/priority comparison; it does not ignore other inline declarations.

Both the driver's main measurement/retry (line 342) and restored freshness measurements (415) pass the retained inventory and controlled indices into this callback. The final mathematical resolution and scene validation therefore run in one synchronous browser task. A DOM replacement scheduled between the preceding screenshot-phase check and this callback is caught before a ratio can be returned. Earlier disabled-phase identity/style checks, computed-none waits, joint shadow removal, caret preservation and finally cleanup remain in place.

The deterministic regression at `theme-dark-surfaces.spec.ts:193–226` counts the five actual screenshots, waits for the final restored phase callback, then replaces the candidate with an identical clone before the next measurement. It requires the replacement to have occurred, exactly five frames, rejection for changed node identity and restored shadow paint. Overridden public test-page methods are restored in finally. This targets the identified race rather than merely repeating the earlier intervention-mutation tests.

This review is limited to the correction and its integration with the previously inspected driver. It does not add a requirement for unrelated refactoring or broaden paint support.

## Observed verification

Reviewer actions: read-only source/plan inspection, independent SHA-256 checks, and parsing existing completed test logs and attachments. No browser or test process was launched by this reviewer; retained outputs are not independent OS exit-status observations.

- `/private/tmp/meetily-theme-shadow-atomic-fixtures.log`, SHA-256 `785458104da23827b921fbf687422d8b90066b2b3f9e4f430960f46c88ce58ed`: nine fixture passes, including the exact final-callback replacement regression, duration 11.0s.
- `/private/tmp/meetily-theme-real-text-atomic.log`, SHA-256 `4f914d59ecf898d3fed5190286c0d88d1d2231439a1b37c58f5ef270758a2f74`: complete JSON reports expected 2, unexpected/skipped/flaky 0, duration 43.35s. Each mode records nine actual text samples across Home, Settings and Meeting Details, including the timestamp, and two shadow proof sets. Minimum recorded ratios are 4.542288769612194 light and 5.999400854306462 dark.

Typecheck success was reported by the owner but not independently verified in this review. The exact sampled text evidence is not full theme/background-scan, native WebView, packaged app or release proof. Prior FAIL reports remain preserved.

`changed_paths`: this report only. `findings: []`. `blockers: []` for this bounded correction. `next_action`: carry these exact-source results into the wider theme checks without closing unrelated outstanding gates.
