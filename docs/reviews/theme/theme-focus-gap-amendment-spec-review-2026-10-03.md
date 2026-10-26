# Appearance focus gap: independent bounded SPEC amendment review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding in the small gap amendment. Scope is only the added Task 3 permission to separate the three Appearance buttons; the geometry engine and current production diff are not independently reaccepted by this report.

## Evidence and rationale

Reviewed Task 3 clarification in `docs/superpowers/plans/2026-10-02-semantic-theme.md`, exact hash `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`. It permits only a constant 4px gap (`gap-1`), explicitly accounting for 8px additional selector width and preserving button padding, labels/icons, pressed state, tokens and surrounding section layout.

Read `/private/tmp/meetily-theme-focus-final.log`: the actual keyboard test fails closed on intersecting neighboring button paint. Independently viewed its failure screenshot: focused System's blue outer border is interrupted along the edge adjacent to selected Light. This supports the local separation correction; screenshot appearance alone is not a numeric contrast measurement. No gap implementation is claimed here.

The amendment corrects the physical overlap instead of excluding it from measurement. It retains actual rounded geometry, foreign-paint/clipping rejection, all six real keyboard mode/button measurements and both 3:1 thresholds. The constant gap remains inside already allowlisted PreferenceSettings; no new source path, palette change, padding adjustment or section redesign is admitted. The 8px selector-width increase is a real local layout change and must be checked for overflow/responsive behavior after implementation, under the existing browser requirements.

The previously reported ten helper fixtures are separate evidence and do not establish this actual focus case or a gap-adjusted runtime PASS. This report does not re-review their implementation.

## Exact read artifacts

- `docs/superpowers/plans/2026-10-02-semantic-theme.md`: `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`
- `/private/tmp/meetily-theme-focus-final.log`: `22e6d0f19072c48f3a08f43df6b04c56725c39a4811340d1cfd24d8b05e8f3d0`
- `/private/tmp/meetily-theme-focus-final-results/theme-foundation-appearanc-9e859-meets-3-1-in-light-and-dark-chromium/test-failed-1.png`: `1a9ea1c1ef1a09fd571913b104b7b3932e8ae61c9ed65963b534acd948a8c813`

Next action: after independent QUALITY amendment acceptance, apply only the constant selector gap, rerun the real Tab/ratio case in both modes, inspect the focused result and required overflow/regression checks, then review exact product bytes. Preserve the current failure. No snapshot update, commit/integration, full theme, native or release acceptance follows.

`changed_paths`: only `docs/fork/theme-focus-gap-amendment-spec-review-2026-10-03.md`. No source/plan/status/Git changes or Graphify.
