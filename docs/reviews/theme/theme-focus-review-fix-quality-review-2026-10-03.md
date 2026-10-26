# Theme focus oracle correction — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** No actionable finding remains in the reviewed focus correction. The previous coordinate-space Important is resolved; this does not close the full theme task.

## Snapshot

Task source `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`; focus plan amendment `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`. Exact paths relative to task `frontend/`:

- `tests/helpers/rendered-contrast.ts`: `5d868ad12a691ab7d9e95b0cf0dfe8f719201123929cf4bb5f7532cefe422c5e`
- `tests/theme-dark-surfaces.spec.ts`: `0e6e370016712e829fb8113d9981b53e2ea80c6f2c5eb792b4937395fc09c841`
- `tests/theme-foundation.spec.ts`: `18528fc93dfe8bfbe635c9a845748e5b83a38654a3870219b15719b5547b58e7`
- `src/components/PreferenceSettings.tsx`: `7078398c24cf894a1ba676ecf16c57a5ac5aecb1cc83be82e5d26281d5163972`

## Correction assessment

The new guard in `rendered-contrast.ts:28–45` admits only a finite six-component 2D transform matrix with identity linear part. Scale/rotation/skew, 3D matrices, nontrivial individual scale/rotate, perspective and zoom fail closed. A 2D translation preserves pixel units, axis orientation, distances and radii, so it is compatible with the existing viewport-rectangle proof. The resolver validates the measured element and its complete ancestor chain before returning a ratio; a transformed ancestor cannot silently reach successful focus acceptance. Conservative rejection of equivalent unsupported CSS spellings is an unresolved result, not a false contrast certification.

An in-memory removal of only the added guard reproduces the previously reviewed helper hash `4ecbf66441738267048a871af3070d3b9edb021e93ef4ec11576bac5dc012e25`. Thus the correction leaves the reviewed endpoint/interior-extremum quarter-arc proof, strict pixel radii, uniform-border checks, foreign-paint rejection and full-precision composition intact. Its corner order is TL/TR/BL/BR, consistent with the loop. Full-circle containment used for an entering quarter arc may conservatively reject supported-looking layouts; it cannot certify a point outside that circle. No sparse point sampling replaces the proof.

`theme-dark-surfaces.spec.ts:52–75` adds independent bottom-left/bottom-right positive and escaping-arc cases, six unsupported-transform negatives and a translated positive. These cover the previously missed scaling scenario and protect both lower-corner index mappings. The preserved 8px/1px/0px cases distinguish successful curved containment from escaping and unresolved corners. The production selector and real Tab test retain their previous reviewed hashes: semantic 2px zero-offset indicator, scoped shadow suppression, admitted inter-button gap, all six controls/modes, settled shadows, both 3:1 comparisons and viewport overflow assertion.

## Observed verification

Reviewer checks: read-only source analysis, SHA-256 pinning, in-memory helper-delta comparison, and parsing retained test outputs on the local macOS checkout. No browser/test process was launched by this reviewer; log inspection is not an independent OS exit-status observation.

- `/private/tmp/meetily-theme-focus-transform-red.log`, SHA-256 `b8441237d55bc5906d0f7f5a1cb55b355ec4eb66628f5990278d805adca69027`: previous unguarded scaling regression failed while four asymmetric fixtures passed. This preserves the regression's RED evidence.
- `/private/tmp/meetily-theme-focus-review-fix.log`, SHA-256 `bf191650f824526bf1395771618660e2659316ed95f4b27a7801468864164adf`: complete JSON reports 22 expected, zero unexpected/skipped/flaky, duration 21.66s. Its real-focus attachment contains all six control/mode entries with minimum inner/outer ratio 4.974482213656899. This supports the bounded fixture and actual-focus result on this snapshot.

Lint/typecheck/audit outcomes were not independently inspected here. Wider theme text/background paint gaps and the full suite remain separate; no claim is made for arbitrary CSS geometry, native WebView, packaged app, signing or release readiness. The earlier FAIL report remains preserved.

`changed_paths`: this report only. `findings: []`. `blockers: []` for this bounded review. `next_action`: use this exact snapshot's focus result in the wider theme verification while retaining all unresolved wider acceptance gates.
