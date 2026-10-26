> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Appearance focus implementation: independent bounded SPEC review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding in the four-file focus correction. The actual keyboard focus gate passes in the reviewed development-browser evidence; this is not full theme/native/release acceptance.

## Snapshot

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`; accepted plan `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`. Independently checked supplied hashes:

- `frontend/tests/helpers/rendered-contrast.ts`: `4ecbf66441738267048a871af3070d3b9edb021e93ef4ec11576bac5dc012e25`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: `a761285bfa020419ff35dac56be42e43eb7738de650672ba9cdc230b2cd52e2e`.
- `frontend/tests/theme-foundation.spec.ts`: `18528fc93dfe8bfbe635c9a845748e5b83a38654a3870219b15719b5547b58e7`.
- `frontend/src/components/PreferenceSettings.tsx`: `7078398c24cf894a1ba676ecf16c57a5ac5aecb1cc83be82e5d26281d5163972`.

Reviewer read the analytical containment implementation, added fixtures, production diff and complete focus test. Only the already approved focus paths/behavior are accepted here; other accumulated changes keep their existing review boundaries.

## Contract assessment

PreferenceSettings adds a 4px selector gap, explicit 2px zero-offset semantic info-text focus-visible outline, and scoped shadow removal while the selector contains focus. Button padding, labels/icons, selected state and tokens are preserved. Direct box-shadow:none avoids treating transparent shadow layers as a uniquely measurable indicator.

The helper requires the expanded footprint inside the parent's uniform border-adjusted axis bounds, parses only supported circular pixel radii, and rejects conservative overlapping-radius cases. Corner escape is resolved only for a solid zero-offset outline without a competing shadow. It evaluates quarter-circle endpoints and the radial outward extremum when it lies in that quarter; no sparse samples replace the arc bound. The guard remains conservative outside supported geometry. Existing clipping, foreign paint, opacity, parent-background and unsupported-effect rejection remain active. Positive 8px radius, escaping 1px/0px corners, elliptical/nonuniform/oversized-radius rejection fixtures match the admitted cases.

The real test creates isolated light/dark pages, asserts effective root theme, reaches all three buttons by Tab, waits for scoped shadow transitions to settle, measures the same engine against both required surfaces, checks horizontal overflow and captures each mode. It does not replace keyboard navigation with programmatic focus.

## Evidence read and observed

`/private/tmp/meetily-theme-focus-gap.log` is terminal Playwright JSON: one expected test, zero unexpected/skipped/flaky, duration 20.663 seconds; SHA-256 `fd5725c369827fa771b39322e5641f9081bc3e7d3fd8b33f85e5d34d12f4219f`. Decoded its attachment: all six real button/mode measurements exist. Minimum inner/outer ratio is **7.014688543268176 in light** and **4.974482213656899 in dark**, above 3:1. The passing test includes the overflow assertions. Independently viewed both attached screenshots: visible uninterrupted blue outline and separated controls in the captured final focused state. Screenshots are context, not the source of numeric ratios.

`/private/tmp/meetily-theme-focus-owned.log`, SHA-256 `2cb6101cbaf8e7a002f0acb4bc09412f12870a6445dc5c1be1ec69e4ee849c2f`, records **10 fixture passes and 1 actual-focus failure** before the gap correction. Preserve that failure; it is not an overall passing run. Final focused JSON supplies the subsequent actual-focus PASS. Reviewer read artifacts rather than running the browser or capturing shell exit. No all-width responsive guarantee follows from one configured viewport.

## Limits and next action

This closes only the reviewed Appearance focus implementation/measurement gap. Required real Home/Meeting Details measurements, conservative dark-surface failures, snapshot drift and wider exact-snapshot gates remain separate. Native accessibility, installed application and release are NOT VERIFIED. Combine with independent QUALITY and run the remaining required regression checks before any full theme closeout or integration.

`changed_paths`: only `docs/fork/theme-focus-implementation-spec-review-2026-10-03.md`; no source/test/status/Git edits or Graphify.
