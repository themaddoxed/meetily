> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Rendered contrast oracle: independent SPEC review

2026-10-03. **FAIL — Critical 0 / Important 3 / Minor 0.** Review the oracle before using its results to select production contrast fixes. The prepaint adapter is outside this correction and its prior bounded acceptance is unchanged.

## Findings

1. **Important — overlap detection is not fail-closed.** `frontend/tests/helpers/rendered-contrast.ts:49–55` checks only hit-tested elements at the bounding-box center. Painted layers with `pointer-events:none` do not participate in that hit test; an overlay covering only part of the text or outline also need not intersect the center. Such layers are omitted while the helper returns an exact ancestor-only ratio. Requirement: Task 3 must fail closed for “overlapping/translucent positioned content” and unsupported painted layers. Classification: missing verification/false PASS risk. Reject intersecting painted layers across the measured text/indicator geometry independently of pointer hit-testing, or restrict the supported geometry explicitly and fail on ambiguity. Add negative fixtures for pointer-events:none overlays and partial/outline-edge overlap. Also include relative/sticky/translated overlaps rather than only absolute/fixed candidates.

2. **Important — translucent focus indicator is composited against two hypothetical backdrops.** `rendered-contrast.ts:90–96` computes `on(background)` for inner contrast and `on(outer)` for outer contrast. For a supported positive-offset outline the indicator lies outside the button and has one actual backdrop; the rendered outline RGB cannot be recomputed as if painted over the button for the first comparison. This can overstate or understate the inner ratio. Requirement: resolve the actual indicator geometry/color, then compare it with both required adjacent rendered surfaces. Classification: incorrect oracle. Establish its actual backing surface, composite once, then compare that rendered RGB to both surfaces; otherwise fail closed. Add a translucent outline fixture with different button/container colors and nonzero offset. Validate/reject intersecting ancestor shadows and border geometry that make the chosen backdrop ambiguous.

3. **Important — the focus test does not establish both effective modes.** `theme-foundation.spec.ts:575–578` registers a light storage init script and later a dark storage init script on the same page. Both persist across navigation; ordering is not a supported contract (installed Playwright 1.61.0 types document it at `playwright-core/types/types.d.ts:308–311`). The test records the requested mode without asserting root effective class/color-scheme, so an execution can measure the same mode twice. Requirement: keyboard-focus indicator measurement in both effective light and dark modes. Classification: partial verification. Use separate parameterized tests/pages or one controlled storage writer, and assert effective root class plus color-scheme before measuring each mode.

## Snapshot and checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`, accepted helper-path plan `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`. Read the complete helper, dark-surface diff, added focus test and installed Playwright API comments. No reviewer-run browser or synthetic reproduction is claimed. Findings follow directly from the implemented geometry/compositing/control flow.

Verified source hashes:

- `frontend/tests/helpers/rendered-contrast.ts`: `e8596c5bd6d05d6dd8cbfb77c8a4321064b87a7b4524c81f1bf825620569762f`
- `frontend/tests/theme-dark-surfaces.spec.ts`: `c8288756d960d664f6a3c14f1ae2747b66ace32a05dd9702ec4ad1d0fa01ad5b`
- `frontend/tests/theme-foundation.spec.ts`: `f26f9e0098b609087c6f5357b4cbb628145b468a9ded2101530c3fbfea2416e3`

The solid ancestor compositing loop keeps floating point precision and applies each ancestor's opacity after its local group; the supplied alpha fixture has independently consistent expected RGB values. Both suites import the same test-only helper. Unsupported effects on the ancestor chain and absent real locators fail instead of being silently skipped. Those properties do not cover the gaps above.

Read the live `/private/tmp/meetily-theme-rendered-contrast-red.log`: the light Settings System sample reports ratio 4.34923168556426; dark real-surface coverage fails because the `00:00` locator is absent. These are observed RED results, not a complete run verdict or accepted production diagnosis. Restore a real visible muted/status fixture locator without weakening the requirement. Preserve those failures; do not classify every failed assertion as a palette defect. Typecheck/ESLint success is implementer-reported, not independently rerun here.

Next action: repair the three oracle issues and add bounded geometry/focus fixtures, complete the real-element run, then independently review exact corrected bytes before changing PreferenceSettings/tokens. Native, full theme and release acceptance remain NOT VERIFIED. `changed_paths`: only this report; no test/source/status/Git edits or Graphify.
