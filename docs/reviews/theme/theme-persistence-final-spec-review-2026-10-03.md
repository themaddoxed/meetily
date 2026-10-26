> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme persistence/OS pressure and test budget: independent SPEC addendum

2026-10-03. **Bounded SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding in the added Task 2 coverage or targeted timeout correction. Full theme acceptance remains incomplete.

## Source review and exact snapshot

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

- `frontend/tests/theme-foundation.spec.ts`: `31bfa7047f18d75b451af725b6b0ba9b078896677f16e6fdf0831becfca16fc5`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: `8c0848bd4b55506fcb0dafbe12a9e79fd5ac956e13d0c233b236d02170f17759`.
- Helper unchanged: `c818561cc7b95d6c80034170f28d3a5d0d5bdc91a2961ce5b7417740c380a761`.
- Globals unchanged: `a1f8eb1684c7a6e09e9476587e23f715c8689d233b7dbbf27826e8d512e96e99`.

Reviewer read the relevant complete cases and verified hashes. Removing exactly the new timeout line and its explanatory comment from dark-surfaces reconstructs prior accepted hash `3465fdba856027cbd00fd561f136e1991ec72165f553e913286f3fd09f4c9df8`; no other change is hidden in that file.

Foundation lines 306–317 verify System responds in both OS directions with matching root class and color-scheme. Lines 320–331 select Light, Dark and System, inspect stored values before/after real reload and assert the selected control and effective root state. No initialization script overwrites the newly persisted values on reload. Lines 334–352 check each explicit mode against actual emulated matchMedia state, allow rendering callbacks through two requestAnimationFrame turns, then require explicit root state and exactly one initial native-intent call. Native evidence is mocked frontend IPC intent, not a native window test. These cover the requested bounded behavior without resetting assertions to expected implementation details.

Dark-surface lines 65–66 increase only the total three-route case budget to 90 seconds for cold compilation/navigation. Assertion thresholds, missing-locator handling and fail-closed geometry remain unchanged; it is not a latency acceptance test. The new budget allows the real unresolved-paint failure to surface instead of a 30-second outer test timeout.

## Evidence independently read

- `/private/tmp/meetily-theme-persistence-pressure-final.log`: **4 passed, 29.6 seconds**, SHA-256 `d8cb07e9a2e4158dbed4d3b7911c5a5d2aca106bafc533cd1a08988332a8f821`.
- `/private/tmp/meetily-theme-final-text.log`: **1 passed / 2 failed, 42.5 seconds**, SHA-256 `ab1ee1640848f5ce5cf41cf7386d5cf8fd8a1908a74da418f6ef4cdc297ccf31`.
- Parsed `/private/tmp/meetily-theme-final-text.json`, SHA-256 `349269b0fa2719b1da443e91daf163b43b2f24d305d72ee0047c55c7b259f557`: one expected/two unexpected/zero skipped; both failures are unresolved foreign sidebar paint, not timeouts. Decoded Appearance attachment contains eight real light/dark description/selector pairs, minimum ratio **4.542288769612194**.

Reviewer did not execute these browser processes or independently capture shell exit codes; source/log/JSON checks were read-only. Appearance has bounded rendered evidence, while the complete Home/Settings/Meeting Details cases remain FAIL because required paint cannot be resolved. This is not proof those surfaces have inadequate contrast. Unknown focus geometry and other dark-scan/snapshot failures remain separate and open.

The prior full-five-suite 48 PASS / 8 FAIL snapshot predates these additions/budget change; it is historical partial regression evidence, not a full run of the current bytes. No full theme, native accessibility, production artifact or release PASS is claimed.

Next action: retain the open required gates and failure classifications; continue only scoped work to resolve them and obtain exact-snapshot QUALITY/verification before closeout. `changed_paths`: only `docs/fork/theme-persistence-final-spec-review-2026-10-03.md`; no product/test/status/Git changes or Graphify.
