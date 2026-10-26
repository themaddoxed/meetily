> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Rendered contrast round 3: independent SPEC review

2026-10-03. **FAIL — Critical 0 / Important 1 / Minor 0.** Two prior Important findings are closed. The remaining finding is the focus-geometry portion of the previously requested fail-closed overlap protection.

## Important: focus overlap checks exclude the actual indicator region

`frontend/tests/helpers/rendered-contrast.ts:65–66` intersects foreign paint against the button's border-box rect only. The accepted outline/sharp shadow ring is outside that box; its `reach` is calculated later at lines 105–110 and used only for overflow clipping at lines 119–120. An opaque or translucent positioned overlay covering only the ring outside the button passes the overlap scan. The helper then returns nominal contrast from ancestor backgrounds, although the real indicator is occluded or painted over. Requirement: Task 3 must resolve actual indicator geometry and fail closed for “overlapping/translucent positioned content.” Classification: partial fix / false PASS risk.

Compute the supported focus footprint before overlap validation and validate all of its painted region plus required adjacent surfaces. A conservative expanded rectangle is acceptable if ambiguity fails; do not silently assume the immediate parent's background across a ring extending outside that parent or over its border/shadow. Add a negative fixture with pointer-events:none paint intersecting only the external ring, leaving the button rect untouched. The current edge-overlay fixture overlaps the text sample itself, so it does not cover this case.

## Closed findings and checks

- Mode isolation is corrected: foundation focus case creates a separate page for each mode and asserts actual root dark/color-scheme before Tab measurements. No conflicting page init scripts accumulate.
- Focus alpha math is corrected for the supported clear outer backdrop: one indicator color is composited over the outer surface and compared with both required backgrounds. The sharp single shadow-ring fixture discriminates this from the old two-color calculation.
- Text-box overlap checking now scans visible DOM rectangles, includes pointer-events:none and partial overlaps, and handles foreign shadow extents. This addresses the original center-only text gap. Conservative rejection of ambiguous paint is not a white-island or low-contrast finding.

Reviewed complete helper and both consumers, source line numbers and completed `/private/tmp/meetily-theme-rendered-contrast-round3.log`. No reviewer-run test. The terminal log records **7 passed / 3 failed, 37.3 seconds**: positive math/sharp-ring and negative effects/edge-overlay fixtures pass; actual light Home returns 3.4642012861127154; dark real-surface and focus cases fail on unresolved intersecting paint. These results do not establish a fully measured real-surface/focus ledger or justify treating every RED as a palette defect. Unknown auto-outline geometry remains unmeasurable, not proof of poor contrast. The broad dark-scan acceptance was not established by this focused run.

## Exact snapshot

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, admitted base `53f4415058fb01e38b8892a47ac036bfffa394ba`, helper-path plan `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`.

- `frontend/tests/helpers/rendered-contrast.ts`: `0ebc127c8f46324bf3699ffa058ca834178a6477d5345a1f265056fea32536e0`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: `60f4f4986ba98621cf0ef81db371e5ebbc07ab34198e5ce4548968e09bfdeed3`.
- `frontend/tests/theme-foundation.spec.ts`: `3a65c9afb76235d5e9873b64c6537d48de60d2a779a8cc968a5e351ad37452ea`.
- Completed log: `8a168efde611d5e7e26797e0b88eac45748d2877be214c67f5d1ebbc1745b936`.

Hashes match the supplied snapshot. Next action: repair the remaining external-focus footprint gap and prove its rejection fixture, then complete bounded real measurements before any production contrast correction. The prepaint adapter's prior scoped PASS is unaffected. Full theme/native/release acceptance remains NOT VERIFIED.

`changed_paths`: only `docs/fork/theme-rendered-contrast-round3-spec-review-2026-10-03.md`. No test/source/status/Git changes or Graphify.
