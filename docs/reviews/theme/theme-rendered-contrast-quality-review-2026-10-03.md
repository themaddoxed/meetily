> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Rendered contrast oracle — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL: Critical 0 / Important 3 / Minor 0.** Repair the oracle before using its results as authorization for production contrast/focus changes.

## Findings

1. **Important — the two-mode focus test can measure the same mode twice.** `frontend/tests/theme-foundation.spec.ts:575-577` adds one init script per loop iteration to the same Page; the first script remains installed when the second navigation runs. The harness relies on relative execution order of both writers to themeMode and never asserts actual effective mode before measurement. A result labelled dark can therefore still describe light. Use isolated pages/contexts per mode or one deterministic mode-setting mechanism, and assert root class plus colorScheme before collecting each mode's measurements. This is an oracle reliability defect, not a demonstrated production focus defect.

2. **Important — painted layers can be accepted without being resolved or rejected.** `frontend/tests/helpers/rendered-contrast.ts:49-55` inspects only elementsFromPoint at the target center and ignores descendants. `resolve:57-67` then composites only the ancestor background/opacity chain. Hit testing is not a complete painting inventory: pointer-events:none overlays can paint without appearing in that list; a painted child under the sampled text is explicitly excluded; another layer can cover part of the text/indicator away from its center. Ordinary outer box shadows on ancestors are also not rejected by validate (line 37 rejects only inset shadows), even though they can affect an adjacent focus surface. The helper can report a compliant uniform-color ratio for an unresolved rendered sample, contrary to the fail-closed contract. Establish actual sampling geometry and reject or resolve intersecting descendant/sibling/pseudo/shadow paint, including non-hit-testable paint. Add negative fixtures for those supported rejection boundaries before treating a numerical ratio as rendered evidence.

3. **Important — focus measurement substitutes an outline-only policy for the required indicator measurement.** Helper lines 75-79 require a solid nonnegative-offset outline and reject every boxShadow, including a sharp uniquely identifiable focus ring. They never parse shadow layers as required by the accepted Task 3 contract. Lines 89-96 also assume the parent background describes the entire outer adjacency without proving that geometry. A valid existing ring or an ordinary selected-button shadow becomes an oracle error, which must not be interpreted as permission to change production styling. Implement the promised visible-indicator geometry and layer selection (or obtain an explicit narrower plan amendment), handle unambiguous sharp rings and reject genuinely unresolved/blurred/clipped cases. Verify positive ring and outline fixtures plus negative ambiguity fixtures. Current PreferenceSettings includes selected shadow-sm, so this is directly relevant to the tested controls.

## Snapshot and reviewed checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`; plan `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`.

| Reviewed test path | SHA-256 |
| --- | --- |
| `frontend/tests/helpers/rendered-contrast.ts` | `e8596c5bd6d05d6dd8cbfb77c8a4321064b87a7b4524c81f1bf825620569762f` |
| `frontend/tests/theme-dark-surfaces.spec.ts` | `c8288756d960d664f6a3c14f1ae2747b66ace32a05dd9702ec4ad1d0fa01ad5b` |
| `frontend/tests/theme-foundation.spec.ts` | `f26f9e0098b609087c6f5357b4cbb628145b468a9ded2101530c3fbfea2416e3` |

Read full helper, added real-text/focus cases, dark-surface diff and current selector styling. The straightforward ancestor-group alpha math preserves floating-point channel values and the fixture checks its multilayer calculation. A shared engine is actually imported by both suites. Unsupported gradients/filter/text-shadow/blend modes throw rather than silently pass. The prior blanket translucent-candidate skip is removed. These facts do not resolve the paint/geometry gaps above.

The available coordinator log `/private/tmp/meetily-theme-rendered-contrast-red.log` was still progressing when inspected. It reports light Settings System text ratio 4.34923168556426 and a separate dark Meeting Details missing locator `00:00`. Treat the first as a measured candidate issue requiring stable sampled-state/paint confirmation; the second is missing fixture/locator evidence, not a product contrast failure. The ledger is attached only after loops complete, so a failure currently loses the intended full sampled-color ledger; preserve measurements on failure during diagnosis. No terminal full-suite result is claimed here. Coordinator typecheck/ESLint exit 0 was not independently rerun.

Reviewer performed read-only local source/diff/log/hash inspection; no browser/model/native execution, product fix or Graphify use. `changed_paths`: this report only. `blockers`: the three oracle defects, missing real-element evidence and incomplete run results. `next_action`: fix and regression-test the oracle, resolve the actual fixture locator without reducing required coverage, rerun controlled measurements and only then classify genuine production RED. This review does not accept full contrast/theme/native/release readiness.
