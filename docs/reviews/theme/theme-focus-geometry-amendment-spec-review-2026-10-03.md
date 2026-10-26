# Theme focus geometry clarification: independent SPEC amendment review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding in the bounded clarification. This admits a measured focus correction and a narrowly supported geometry proof; it does not establish their implementation or runtime success.

## Scope and snapshot

Reviewed `docs/superpowers/plans/2026-10-02-semantic-theme.md`, Task 3 “Focus geometry clarification”, lines 293–309, alongside unchanged rendered contrast/focus criteria at lines 313–318. Exact SHA-256: `03fdc9232e35f4f7e174841b0776ac1e87c4505e55351fb3442f077289133ab1`. Read current Appearance classes and semantic info-text definitions for context. No new implementation diff is submitted in this review.

The proposed production change is confined to already allowlisted PreferenceSettings: a solid 2px zero-offset semantic info-text outline for focus-visible Appearance buttons, with competing button shadows suppressed only during selector focus. Labels, icons, pressed state, padding/layout and tokens remain unchanged. The existing helper/test paths already cover the geometry fixtures; no twentieth path or shared primitive is introduced.

## Contract fit

The plan explicitly distinguishes unsupported UA auto-outline measurement from demonstrated low contrast. It authorizes a deterministic explicit indicator without claiming the original indicator failed the 3:1 ratio. Choosing info-text is a proposal subject to actual measurement against both inner and outer surfaces in both modes, not a token-name guarantee.

Replacing a conservative bounding-rectangle rejection with a complete analytic containment proof does not inherently weaken fail-closed behavior. The amendment requires finite circular pixel radii, uniform borders, nonoverlapping corner radii and every relevant arc/straight edge accounted for through analytical extrema. Unsupported elliptical/nonpixel radii, clipping, foreign paint, competing indicators and ambiguous backgrounds remain rejected. The implemented coordinates must correspond to the actual outer outline and inner parent background, with border/radius adjustment and full precision; a sparse sampled approximation cannot satisfy this plan.

Positive rounded-parent and negative escaping-arc fixtures are required before production classes, followed by independent resulting-code review. Existing real Tab-driven six button/mode measurements and both 3:1 thresholds remain mandatory. Helper-only fixtures cannot close focus acceptance. The actual full geometry engine must still preserve prior opacity, foreign-paint and parent-background guards; this clarification is not permission to allowlist a failing element.

## Next action and limits

After matching independent QUALITY plan acceptance, implement and validate the bounded geometry fixtures first, then the minimal focus classes and real keyboard measurements. Reject the candidate if any supported geometry cannot be proven or either actual ratio fails. Preserve failed/unsupported evidence and rerun the relevant broader gates after the visual correction; snapshots remain unchanged unless separately amended/reviewed.

No browser, production focus, native, full-theme or release PASS is asserted. `changed_paths`: only `docs/fork/theme-focus-geometry-amendment-spec-review-2026-10-03.md`. No source/plan/status/Git edits or Graphify.
