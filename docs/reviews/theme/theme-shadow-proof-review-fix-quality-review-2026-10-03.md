# Foreign-shadow proof correction — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL: Critical 0 / Important 1 / Minor 0.** The two earlier intervention findings are fixed, but the final mathematical acceptance does not atomically revalidate the scene inventory used by its pixel proof.

## Remaining finding

**Important — bind final proof consumption to the same scene inventory in the mathematical evaluation.** `frontend/tests/helpers/rendered-contrast.ts:303–308` returns the raw resolver result with the accumulated proofs. The last strict inventory check occurs in a previous browser evaluation at line 429. Between those browser callbacks, another DOM/paint update can occur. The next raw resolver only checks index/style/region fingerprints, not the actual candidate/sample/other-node identities, complete scene inventory or font inventory. Thus the strict stale-scene guarantee added for screenshots is absent at the point that consumes the certificate and returns acceptance.

Perform the final inventory validation and mathematical resolution synchronously in the same browser evaluation, using the captured inventory and sample/candidate handles. Apply this to any retry that consumes existing proofs, not just the last screenshot phase. Add a controlled mutation after the restored-scene check but before the final resolver call; an identical-node replacement or another inventory-only mutation must fail rather than return a ratio. This is a source-confirmed validation gap; the reviewer did not execute an adversarial timing reproduction. An extra separate awaited validation call alone retains the same inter-callback gap.

## Snapshot and resolved issues

Reviewed plan `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`. Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

- `frontend/tests/helpers/rendered-contrast.ts`: `432f308480192f96ae4ef59e7ac88e3a5ce27fdd3b84fab15fcf09938bad3b92`
- `frontend/tests/theme-dark-surfaces.spec.ts`: `8caf61251d0185caab80c9580b6d89543b679e20164e35cb1393d20eb642b9f1`

The new inventory captures actual node/child identity, text, attributes, canonical inline declarations and priorities, computed CSS, rectangles, relevant pseudo styles, font faces and viewport state. During disabled captures, only controlled candidates' box-shadow is normalized; current computed-none checks remain separate and mandatory. Phase validation now surrounds both disabled captures and checks the restored scene. The new temporal-style, foreign-paint and clone/moved-original fixtures exercise the earlier missing boundary and verify cleanup.

All five screenshots now use `caret: 'initial'` and allow animations. Joint removal, bounded stabilization, candidate cap and finally/allSettled restoration remain intact. The strict timestamp span locator matches `TranscriptView.tsx:287`; it measures the actual text leaf rather than relaxing mixed-color rejection. Attaching the partial ledger in finally preserves evidence even if a later sample fails.

## Observed checks

Reviewer performed read-only source inspection, exact hashes and parsed retained logs/attachments; no browser/test process was launched by this reviewer.

- `/private/tmp/meetily-theme-shadow-proof-fix-fixtures.log`, SHA-256 `3a7732ee1eb79df66773edc720bfcccb0e55ddad520401cacd65ab90c9cb3c45`: eight fixtures passed (10.6s), preceding the supplied type-only/locator/ledger delta.
- `/private/tmp/meetily-theme-real-text-shadow-fixed.log`, SHA-256 `61a18b1c5555f257835a6f2b068050944d71ae638e5a15387f3386f458c3b45d`: completed JSON reports expected 2, unexpected/skipped/flaky 0, duration 43.64s. Each mode contains nine named real-text measurements across all three routes, including the timestamp. Minimum recorded ratios are 4.542288769612194 light and 5.999400854306462 dark. Two shadow proof sets are retained per mode.

These successful static-scene cases do not test the final proof-consumption race. Lint/typecheck were not independently inspected in this review. The result is bounded to this oracle correction; full theme/background-scan/native/release acceptance remains open.

`changed_paths`: this report only. `blockers`: atomic stale-proof validation at final consumption. `next_action`: implement the narrow synchronous validation boundary and its regression, then repin and re-review. Earlier reports remain preserved.
