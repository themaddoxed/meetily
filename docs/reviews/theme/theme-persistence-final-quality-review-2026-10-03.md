# Theme persistence and final text-test addendum — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the bounded test additions: Critical 0 / Important 0 / Minor 0.** No actionable finding remains in these persistence/system-pressure checks or the explicit three-route test budget. This does not turn the still-failing full theme gate green.

## Exact snapshot

Task root `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

- `frontend/tests/theme-foundation.spec.ts`: SHA-256 `31bfa7047f18d75b451af725b6b0ba9b078896677f16e6fdf0831becfca16fc5`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: SHA-256 `8c0848bd4b55506fcb0dafbe12a9e79fd5ac956e13d0c233b236d02170f17759`.
- Previously accepted helper remains `c818561cc7b95d6c80034170f28d3a5d0d5bdc91a2961ce5b7417740c380a761`; globals.css remains `a1f8eb1684c7a6e09e9476587e23f715c8689d233b7dbbf27826e8d512e96e99`.

Source pins checked before/after. Reviewed current delta and relevant surrounding harness in local Darwin; no source edits by reviewer.

## Behavior and observed evidence

Foundation lines 306-315 now verify system dark-to-light-to-dark transitions with matching root class and colorScheme. Lines 320-330 select each Light/Dark/System mode, check persisted storage, reload, verify selected aria state and effective theme, and recheck storage. System's effective dark result is explicit because OS is emulated dark. The loop adds no competing init scripts.

Lines 333-348 test both explicit modes against opposite initial system preference and live light/dark pressure. They verify actual matchMedia state, await two animation frames to allow media-change delivery, then require unchanged explicit effective theme and exactly one initial mocked native payload. This exercises the synchronous runtime subscription/call path; it does not claim arbitrary indefinitely delayed effects or installed native behavior. The assertions do not pass merely because a before-change DOM was sampled.

Read completed `/private/tmp/meetily-theme-persistence-pressure-final.log`, SHA-256 `d8cb07e9a2e4158dbed4d3b7911c5a5d2aca106bafc533cd1a08988332a8f821`: **4 passed in 29.6 seconds**. Exit 0 is coordinator-observed; reviewer inspected the log rather than running tests.

Dark-surface lines 65-66 set a 90-second aggregate budget for the existing three-route case. It is not a responsiveness threshold; no ratio, required locator, unsupported-paint rejection or per-expectation success criterion is weakened. Final execution reaches concrete paint errors instead of timing out.

Read `/private/tmp/meetily-theme-final-text.log`, SHA-256 `ab1ee1640848f5ce5cf41cf7386d5cf8fd8a1908a74da418f6ef4cdc297ccf31`, and parsed its separate JSON report `/private/tmp/meetily-theme-final-text.json`, SHA-256 `349269b0fa2719b1da443e91daf163b43b2f24d305d72ee0047c55c7b259f557`: **1 passed / 2 failed**, zero skipped/flaky, 42.5 seconds. Both failures are unresolved sidebar paint in the broader light/dark route cases, not measured low contrast or a timeout.

Decoded the successful Appearance attachment (decoded-body SHA-256 `73fe86d1307ec6546dc0391f46e59d07de0d724a919f36d8c88e6ca68b8f3054`). All eight description/label measurements exceed 4.5: light System/Dark minimum **4.542288769612194**; dark minimum **5.999400854306462**. This supplies concrete bounded GREEN for the preceding semantic light-token correction, without replacing the failed broader real-surface tests.

## Limits and next action

The prior five-suite result 48 passed / 8 failed predates these latest test bytes; it is historical, not a final full-suite run. Background scanning, Meeting Details/foreign-paint cases and focus auto-outline measurements remain unresolved and must not be labelled poor contrast or PASS. No native appearance, production bundle, full repository/release gate or screenshot approval is established. No reviewer browser/native invocation occurred.

`changed_paths`: `docs/fork/theme-persistence-final-quality-review-2026-10-03.md` only. `blockers: []` for this bounded test addendum; broader theme acceptance remains blocked by required unresolved measurements/failures. `next_action`: retain these exact logs/attachment and source pins durably, resolve the remaining gates without reducing coverage, and keep the task's partial status explicit.
