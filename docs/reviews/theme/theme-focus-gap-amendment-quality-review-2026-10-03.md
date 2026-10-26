# Theme focus gap amendment — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the bounded planning amendment. Critical 0 / Important 0 / Minor 0.** No actionable finding in the added gap authorization. This is not acceptance of the pending production correction or the whole theme task.

## Observed snapshot and scope

Reviewed `docs/superpowers/plans/2026-10-02-semantic-theme.md`, SHA-256 `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`, specifically Task 3 lines 299–305 and its unchanged acceptance requirements. Source checkout: `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Current `frontend/src/components/PreferenceSettings.tsx` SHA-256 `a0bc1c8c9cfead9c3800eacc8048f2cb669f0fd17443ff07065bc76fd567bbac` has the earlier focus outline classes but no gap on the Appearance selector (line 132). Reviewed only the new planning authorization; the analytical engine and earlier implementation are outside this addendum.

## Assessment

The observed failure is consistent with the source: a zero-gap row puts the neighboring button inside the focused button's outside 2px outline region. The actual failure log reports intersecting button paint rather than a measured low contrast ratio. The screenshot shows System focused beside selected Light, with its right blue edge obscured. This supports correcting physical separation while keeping the intersection detector active.

A 4px inter-button gap leaves the 2px outline on the container background with clearance from the next button. Two gaps add nominally 8px to the intrinsic three-button selector width; they do not require changing button padding, labels, tokens, indicator offset or surrounding section classes. Tailwind `gap-1` is a rem-based spacing utility, so the stated pixel size describes the default root-font environment; implementation verification should inspect actual rendered separation rather than assume a universal fixed pixel size. No new configurable spacing abstraction is needed.

The amendment expressly retains rejection of intersecting paint and both adjacent-surface 3:1 checks. It does not authorize an allowlist, inset geometry, relaxed containment or a token-only contrast substitute. Its limited width increase remains subject to existing actual layout/overflow checks: unchanged section classes alone cannot prove unchanged wrapping. Verify the six real Tab control/mode measurements, visible indicator continuity and the wider retained checks after applying the gap.

## Evidence and boundaries

Reviewer actions: read-only plan/source inspection, SHA-256 checks, reading the existing test log, and viewing the existing failure screenshot. No browser/test process was launched by this reviewer. `/private/tmp/meetily-theme-focus-final.log`, SHA-256 `22e6d0f19072c48f3a08f43df6b04c56725c39a4811340d1cfd24d8b05e8f3d0`, contains one failed focused test with `intersecting paint=static; pointer-events:auto`. Viewed `/private/tmp/meetily-theme-focus-final-results/theme-foundation-appearanc-9e859-meets-3-1-in-light-and-dark-chromium/test-failed-1.png`. This inspection is not independent reproduction or an OS process-exit observation. The reported ten passing geometry fixtures are not freshly reviewed here.

`changed_paths`: this review report only. `findings: []`. `blockers: []` for the bounded planning gate. `next_action`: apply only the authorized selector gap, then independently verify the resulting source and actual focus/layout behavior. Full theme, native and release acceptance remain NOT VERIFIED.
