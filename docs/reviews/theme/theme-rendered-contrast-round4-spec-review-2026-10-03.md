> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Rendered contrast round 4: independent SPEC review

2026-10-03. **FAIL — Critical 0 / Important 2 / Minor 0.** Ring-only overlap detection is now fixed and its regression passes. The general oracle still has two false-measurement paths; use bounded rejection guards rather than expanding into a general renderer.

## Findings

1. **Important — the assumed focus backdrop is not geometrically established.** `frontend/tests/helpers/rendered-contrast.ts:155–158` unconditionally resolves the immediate parent as the ring's actual outer backdrop. Containment checks at lines 98–103 apply only when an ancestor has a shadow, and lines 148–151 only when overflow clips. A button flush with a colored parent's edge, with visible overflow and no shadow, can paint its ring outside that parent over a different grandparent background; a ring crossing the parent's border similarly sees a different color. The helper returns a numeric ratio using the wrong backdrop. This is the remaining backdrop portion of the prior focus-geometry finding. Requirement: identify both actual adjacent surfaces and fail on ambiguous indicator geometry. Fix: require the entire supported ring footprint to lie within a proven uniform inner parent surface excluding borders/rounded corners, or resolve the actual separate backing regions. Add no-shadow parent-edge and border-crossing rejection fixtures.

2. **Important — new descendant-text traversal omits intermediate paint/opacity.** `rendered-contrast.ts:78–88` accepts descendant text when its computed color matches the selected element, but compositing at lines 142–143 starts from the selected element, not each text node's actual parent. Descendant opacity/filter is not validated by the foreign-paint loop either when it has no background/shadow. For example, a white selected div with black color containing `<span style="opacity:0.1">Text</span>` returns nominal black-on-white contrast even though the glyphs are pale gray. Requirement: complete actual text ancestor/opacity stack and fail-closed unsupported effects. Classification: new partial-verification gap introduced by Range traversal. Either reject descendant text whose intermediate ancestors alter opacity/effects/background/style, or calculate per text-parent chain and use the worst required ratio. Add a nested-opacity and nested-filter negative fixture; do not merely check inherited color.

## Exact snapshot and checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`; base `53f4415058fb01e38b8892a47ac036bfffa394ba`, accepted helper plan `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`. Read full helper, new fixtures and actual-surface flow; independently checked supplied SHA-256 values:

- Helper: `6ce584adf108a8a70899a6120f369980128d2bc3ea73870188fd04e8f1c0f6de`.
- Dark surfaces: `5c43dbdebd8c89acf15c77ddd388fff978466d151d129c71cb01ca5d538c1093`.
- Foundation: `3a65c9afb76235d5e9873b64c6537d48de60d2a779a8cc968a5e351ad37452ea`.

The above counterexamples are source-derived, not reviewer-executed browser reproductions. Independently read completed `/private/tmp/meetily-theme-rendered-contrast-round4.log`: **9 passed / 3 failed, 38.4 seconds**. Ring-only and ancestor-shadow rejection fixtures pass. The light Settings System sample still reports 4.34923168556426 after the Home animation readiness check; dark measurements stop at unresolved foreign sidebar shadow; focus stops at unsupported auto-outline geometry. These latter failures are NOT_MEASURABLE under this oracle, not measured bad contrast or confirmed light islands.

The Home opacity wait addresses a known transient measurement without modifying production. It does not eliminate the descendant-stack gap above. The specific System span sample has no demonstrated contamination from that gap; its numeric RED remains evidence to preserve, but this report does not approve the general oracle or a production token change yet. The accepted prepaint adapter and already-closed mode/compositing findings remain unchanged.

Next action: add bounded fail-closed guards and fixtures for the two identified paths, then obtain exact-snapshot oracle acceptance before the proposed token fix. No full theme/native/release PASS. `changed_paths`: only `docs/fork/theme-rendered-contrast-round4-spec-review-2026-10-03.md`; no product/status/Git/Graphify changes.

Log SHA-256: `ffcd9cfdc555058158dc1c04abe4551071934c223bd25898122893a4ac6dcdc6`.
