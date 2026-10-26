> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Focus transform guard: independent bounded SPEC review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** The reported transform-coordinate Important is closed for the measured element/ancestor chain. No additional actionable finding in this bounded guard/fixture correction. Full theme acceptance remains open.

## Source and behavior

Reviewed `measureRenderedContrast` validation and added transformed/asymmetric fixtures. `validate` rejects nonidentity 2D linear transforms, every matrix3d transform, nonunit individual scale/zoom, nonzero individual rotate and perspective. Only finite 2D matrices with identity linear terms and arbitrary translation survive. Translation changes viewport origin without changing CSS-pixel lengths, so existing widths/radii and translated rectangles retain compatible units. The validator runs on the measured node and ancestor chain before the numerical result; it is also used on relevant descendant text chains. No approximate inverse transform or unsupported geometry is accepted.

Six negative fixtures cover matrix scale/rotation/skew, individual scale, zoom and perspective. A positive translation fixture protects the supported case. Four asymmetric bottom-corner fixtures test 8px containment versus 1px escape on left/right corners, reducing the risk of a corner-index assumption. Existing rounded-parent, overlap, clipping and real keyboard assertions are retained. No production focus/token change is included in this follow-up.

## Exact snapshot and evidence

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`; plan `d429a5c310fb835c018de90238667d1f36c80f0a1f93c10fec83163f344876df`. Independently read and hashed:

- Helper: `5d868ad12a691ab7d9e95b0cf0dfe8f719201123929cf4bb5f7532cefe422c5e`.
- Dark-surface tests: `0e6e370016712e829fb8113d9981b53e2ea80c6f2c5eb792b4937395fc09c841`.
- Foundation unchanged: `18528fc93dfe8bfbe635c9a845748e5b83a38654a3870219b15719b5547b58e7`.
- PreferenceSettings unchanged: `7078398c24cf894a1ba676ecf16c57a5ac5aecb1cc83be82e5d26281d5163972`.

Read terminal `/private/tmp/meetily-theme-focus-review-fix.log` JSON, SHA-256 `bf191650f824526bf1395771618660e2659316ed95f4b27a7801468864164adf`: **22 expected / 0 unexpected / 0 skipped / 0 flaky**, 21.662 seconds. Its decoded focus ledger contains all six real button/mode records with minimum inner/outer ratio **4.974482213656899**. This is actual completed focused-run evidence, not a projected outcome.

Preserved `/private/tmp/meetily-theme-focus-transform-red.log`, SHA-256 `b8441237d55bc5906d0f7f5a1cb55b355ec4eb66628f5990278d805adca69027`, records **4 passed / 1 failed** before the fix: the scaled geometry unexpectedly resolved. That failing run remains failed and provides regression provenance.

Reviewer read source/log/JSON rather than executing the browser or capturing shell exits. Latest lint/typecheck/audit were pending in the capsule and are not asserted PASS here. This focused result does not rerun all theme suites or establish production/native WebView geometry support.

Next action: combine with independent QUALITY, finish pending static checks and remaining full-theme gates without promoting unresolved measurements to contrast failures. Native accessibility, release and complete theme acceptance remain NOT VERIFIED. `changed_paths`: only `docs/fork/theme-focus-review-fix-spec-review-2026-10-03.md`; no source/test/status/Git edits or Graphify.
