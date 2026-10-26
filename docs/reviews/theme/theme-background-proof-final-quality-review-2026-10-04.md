# Background proof settled-route addendum — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded QUALITY PASS: Critical 0 / Important 0 / Minor 0.** No actionable finding in the added bounded wait. The focused Settings rerun now supplies the previously missing settled-route evidence; this is not full-suite or release acceptance.

## Exact snapshot and delta

Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`:

- `frontend/tests/helpers/rendered-contrast.ts`: `eb1e22b0f3669f100add85587ea3646b1e989755c43b75cbd8bf011cdbd8e52f` (unchanged).
- `frontend/tests/theme-dark-surfaces.spec.ts`: `31d8afbe359dfef17a4f4b02f7dbbc01e828478a26e9fff1c37e3265ee69757f`.

In-memory removal of only spec lines 402–413 reproduces previously reviewed spec SHA `529c2c43163a33887845742577c5513c059b2b295c031922019c48cc8425b98e`. No other source delta is included in this addendum.

## Assessment

The new expect.poll waits at most five seconds for positive-area visible DOM transforms to be none or finite 2D matrices with identity linear part. It reports any remaining unsupported transform rather than sleeping a fixed duration or altering CSS. Persistent visible scale/rotation/skew remains a failed wait. The exact same background function then runs and independently rejects unsupported geometry/effects, including anything the precondition does not inspect. A change between poll success and measurement therefore cannot bypass the engine guard.

The precondition intentionally characterizes a settled representative route, not the prepaint phase. It preserves the separate prepaint contract, dark-root assertion, overflow check, canvas/inventory requirements and <245 bound. There is no animation disabling, paint override, transform allowlist for a particular component or altered color threshold. A transient Settings mount transform is consequently handled as a state-readiness condition while its earlier failure stays recorded.

## Evidence and limits

Reviewer actions: read-only source inspection, exact SHA-256 checks, in-memory delta comparison and parsing the retained Settings JSON/ledger. No browser/test process was launched by this reviewer.

`/private/tmp/meetily-bg-diag/settings-settled.log`, SHA-256 `7d36acc7c083437427d68829dd873d950925736ae457dbe54087ab0430ff9630`, is complete: expected 1, unexpected/skipped/flaky 0, duration 13.85s. Settings reports BODY canvas `[10,10,10]`, 29 layers, 390 cells, 19059 operations, one recorded checked-control exception and maximum near-white upper bound 133.2131968042615. This closes the focused runtime gap for the settled Settings scene on the stated source.

Earlier Home and Meeting Details passes used the unchanged helper and prior spec before this readiness-only addition; they are not represented as fresh reruns of this final spec. The owner's full five-suite run is outside this addendum and is not assumed passing. Typecheck/targeted ESLint success was owner-reported, not independently rerun. Full theme, native/macOS and exact-artifact release acceptance remain separate.

`changed_paths`: this report only. `findings: []`. `blockers: []` for the bounded wait correction and focused Settings gate. `next_action`: assess the completed full-suite evidence on the final snapshot and continue only the already required wider gates. Prior reports/failures remain preserved.
