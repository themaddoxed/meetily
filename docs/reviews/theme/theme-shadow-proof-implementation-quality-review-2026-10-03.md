# Foreign-shadow proof implementation — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL: Critical 0 / Important 2 / Minor 0.** The joint-removal and cleanup mechanics address the prior plan concerns, but the causal proof can still accept a changed intervention scene, and screenshot defaults alter unrelated paint.

## Findings

1. **Important — validate phase-specific scene and real node identity during intervention.** `frontend/tests/helpers/rendered-contrast.ts:344–349` checks only computed shadow values around the disabled screenshots. Full `verifyFresh` runs before intervention and after restoration (334–339, 362–366). A mutation that exists only while the shadow is disabled can change sample/ancestor/candidate paint or geometry, compensate for removal, then restore the original fingerprint. The five images and final fingerprint can match without proving shadow-only non-interference. The current stale fixture at `theme-dark-surfaces.spec.ts:140–150` leaves its mutation permanent, so it does not cover this case.

The proof also represents identity as DOM index plus tag/classes/computed CSS/rectangles (helper 125–140), while removal acts through a separately captured handle (308). It never confirms that the current indexed candidate is that same node. A same-looking replacement at the index, with the original handle still connected elsewhere, can cause the driver to remove the wrong shadow. Index/style equality is not node identity.

Add a phase-aware fingerprint with only the intended shadow override normalized; check sample region, all candidate/ancestor styles and geometry, and actual node identity around both disabled captures. Bind the candidate handles and original sample node to the nodes being validated, using strict identity or stable opaque node IDs. Revalidate before accepting the restored proof. Add temporary-mutation-and-reversion and candidate-replacement regressions. These are source-confirmed missing checks; the reviewer did not execute those counterexamples.

2. **Important — preserve caret paint in every screenshot.** Calls at helper 336–337, 346, 348 and 364 specify `animations: 'allow'` but omit `caret: 'initial'`. In the installed Playwright 1.61 implementation, `coreBundle.js:20861` passes `options.caret !== "initial"`; lines 20714–20729 temporarily set caret-color to transparent on inputs, textareas and contenteditable elements. Therefore these snapshots can hide unrelated paint despite the admitted no-mask/no-other-paint-change contract. Explicitly preserve the initial caret in all five captures and add a relevant capture-option or focused-editable regression. This finding is based on inspected local dependency code, not assumed API behavior.

## Snapshot

Reviewed accepted plan `83e82c5891c7d699485ab0910936a67bf4a8e6690e42eb2b6ec43df07767de41`. Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

- `frontend/tests/helpers/rendered-contrast.ts`: `c140f4945042f80ad4245159cce7e8587c5f5771185fba5de25ce4d0d2b2be77`
- `frontend/tests/theme-dark-surfaces.spec.ts`: `9a068bb682f350a469d1acfb9c52c9d5dc99423807b309bc2b68ea77f853aebb`

Joint removal really covers every accumulated candidate, restoration uses allSettled so one failure does not prevent attempts for the others, original values/priorities are restored, and effective none/original-shadow waits are bounded. The proof cap fails on overflow. Actual border intersections and focus/background paths retain their strict behavior. These properties do not resolve the findings above.

## Evidence and limits

Reviewer actions: read-only source/dependency inspection, SHA-256 checks and retained-log inspection. No browser/test process or DOM intervention was performed by the reviewer.

- `/private/tmp/meetily-theme-shadow-proof-fixtures.log`, SHA-256 `2578f4dbcb6565f26fe892132aee6fe656e506802366b3941a8576d52937008f`: five fixture passes. This is retained earlier fixture evidence, not a rerun of the exact subsequent evidence-recording delta.
- `/private/tmp/meetily-theme-real-text-shadow.log`, SHA-256 `0e9fffea068a7e9ef2d389b2141e00bbdd092ced0f35e22a6134720004de150f`: complete JSON reports expected 0, unexpected 2, duration 57.76s. Both modes fail closed on mixed text color at the Meeting Details timestamp sample. The real-text suite is not green, and the log is not evidence that all named text measurements are accepted.

The mixed-color failure is recorded separately from the two oracle defects: selecting a text leaf or otherwise supporting mixed text requires its own scoped validation; it must not be bypassed as shadow evidence. No claim is made for full theme, native WebView or release readiness.

`changed_paths`: this report only. `blockers`: both Important findings and unresolved real-text acceptance. `next_action`: repair phase binding and screenshot options, add the negative regressions, repin and re-review; preserve these exact-snapshot results.
