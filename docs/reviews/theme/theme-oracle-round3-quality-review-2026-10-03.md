> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme oracle round3 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the revised bounded test oracle: Critical 0 / Important 0 / Minor 0.** The previous intermediate-mutation false negative and live-body polling false rejection are addressed. The theme contract remains RED; this is not product acceptance or a completed twelve-transition matrix.

## Snapshot and checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Only `frontend/tests/theme-foundation.spec.ts` is dirty. Reviewed test SHA-256 before/after: `c58fead9f4727a15ef36cf595933653c4ae9e0472f36ee0b425a699fb0b9330d`.

Read the full revised harness/matrix/regression and scoped Git diff/status in the local Darwin workspace. Independently read the completed coordinator-run log `/private/tmp/meetily-theme-oracle-round3.log`, SHA-256 `a27ddfc7c69078bc422445d62aa7191c32ab11e1e2cfc44b5a52d281a9c10924`: 2 passed, 6 failed in 59.6 seconds. Passing cases are the new observer regression and first-dark-event characterization; all six matrix navigation cases fail at the held-CSS predicate, line 138. The reviewer did not run the browser or own its process exit status. Artifacts are under `/private/tmp/meetily-theme-oracle-round3` and are not a durable repository evidence bundle by themselves.

## Resolved findings

At test lines 39-61, the observer initializes both tracked attributes from their first old values, walks records in order, derives each new value from the next same-attribute record (or final DOM value) and updates the combined class/style state. This preserves wrong-then-right states within one observer batch, instead of labelling every record with the final DOM. Exact no-op attribute writes are skipped. The actual browser regression at lines 76-91 uses direct setAttribute calls, bypassing the toggle hook, and verifies false/true dark transitions plus light/dark colorScheme transitions after body exists. The later all-events assertion therefore rejects the previously missed concrete counterexample. No checksum-like or semantic weakening of expected theme state is introduced.

At lines 131-138, the held-CSS predicate requires current effective state plus a retained matching early event; it no longer requires document.body to remain absent when Playwright samples it. This fixes rejection of a valid early resolution merely because parser progress creates body before the test's next poll. Events are reset by the initialization script on each navigation/reload and the first CSS remains held until after this assertion. Missing early evidence or incorrect current state still times out as a test failure. Body presence on observer records remains observed at callback delivery, while the toggle hook is synchronous; this is sufficient for the currently reviewed parser-blocking script shape, not a universal timestamp for arbitrary scripts that manipulate body synchronously. No broader event-time guarantee is claimed.

## Remaining verification boundaries

The two passing harness/characterization tests do not turn six genuine contract failures into PASS. With unchanged production source, every mode/OS navigation still fails the three-second held-CSS assertion. No reload or subsequent first-frame, response-order or hydration assertion is reached. This supports proceeding to the explicitly reviewed ordering correction; it does not demonstrate a visible flash or establish the final correction.

The stylesheet-load rAF remains a browser checkpoint rather than compositor/pixel evidence. The full matrix must later reach both transitions for every row, verify actual navigation markup/order, check no subsequent incorrect effective states through readiness, and retain hydration diagnostics. The positive corrected prepaint path and its readiness behavior are not exercised by this RED run. A dedicated synthetic positive timing fixture would give additional regression coverage, but the fixed predicate no longer has the identified live-body dependency.

Final typecheck/lint and full focused checks for these bytes are not supplied by this log and remain required. Existing limits concerning exact server root, no stale-server reuse, snapshots, contrast and native/release evidence remain. Production correction via root-head serialization still needs the explicit plan amendment/independent review described in the previous report; this test review does not authorize an unreviewed ownership-contract change.

`changed_paths`: this report only. `blockers: []` for acceptance of this oracle correction; product ordering and remaining matrix gates are still unresolved. `next_action`: retain this log/source binding durably, complete the scoped static checks, review the explicit source-seam amendment, then make the bounded production correction and rerun the complete matrix. No product files, task status or Git refs were changed by this reviewer.
