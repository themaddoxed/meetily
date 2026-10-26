> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme prepaint implementation: independent SPEC review

2026-10-03. **FAIL — Critical 0 / Important 1 / Minor 0.** Scope: only the three-file prepaint seam against base `53f4415058fb01e38b8892a47ac036bfffa394ba` and accepted plan amendment `96094ccecd7e317004f555176d7ec013fb518d40a8083f9938075a8f61b03715`; not full theme acceptance.

## Important: head ownership causes a real hydration mismatch

`frontend/src/app/layout.tsx:243` assigns the entire head's dangerouslySetInnerHTML to the one script. Next also inserts metadata, CSS and scripts into that same head. On hydration, React compares the server head containing all those resources with the client inner-HTML value containing only the prepaint script and emits `Prop ... did not match` for dangerouslySetInnerHTML. Requirement: plan success criterion 1 / Task 2 requires no hydration warning/error. Classification: incorrect implementation of the zero-hydration contract. The behavior is observed, not hypothetical: all six matrix navigation cases fail the diagnostics assertion at `frontend/tests/theme-foundation.spec.ts:175`; server/client head contents in the raw log demonstrate the mismatch.

Fix the head ownership mismatch while retaining the single trusted resolver, before-CSS execution, stable script identity and unchanged strict diagnostics. Do not suppress the warning or weaken the collector to manufacture GREEN. Investigate a server/client-compatible seam; if the solution requires another path or revising the approved whole-head technique, amend and review before extending production scope. This review does not establish that a compliant correction is impossible.

## Checks and snapshot

Reviewer read complete production diff, current test oracle/owner identity changes and source caller search; `git diff --stat` reports exactly the three allowed files. Static script constant has no user-data interpolation and retains the existing resolver. The sole import/mount remains in layout. Stable ID markup matching addresses the prior Minor about incidental IIFE spelling. No additional source defect found within this seam.

Task root: `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`. Before/after hashes:

- `frontend/src/app/layout.tsx`: `dac7ec01049f481d79ae32127c80578f4438defc3f18ecf7ad5b83001bcc7723`
- `frontend/src/components/ThemeScript.tsx`: `6cce905b099bfb26e40ce65564b79b042454ee464b6197dbce83f1611aa10601`
- `frontend/tests/theme-foundation.spec.ts`: `468ddcdf90136041271e0ba0c94739d5b0d68a458ab4382a6d556e71dc140781`

Read completed `/private/tmp/meetily-theme-prepaint-green.log`, SHA-256 `abeec56e432adb6e70db0f72c76cab360e07b2766bbdfbdeaede8778a43c21d4`: **3 passed / 6 failed, 43.3 seconds**. The matrix reaches and passes preceding navigation pre-CSS, first-frame and emitted-order assertions, then fails diagnostics. Reload is not reached after each navigation failure. Passing cases cover ownership, intermediate-mutation observer regression and the original early-dark flag. Reviewer did not run this browser process or capture its shell exit; terminal log results were independently read. Reported typecheck/ESLint and eight startup Node passes were supplied by the implementer, not independently rerun/log-reviewed here.

Native/release and complete theme regression/contrast/accessibility checks are NOT VERIFIED. No full 12-transition PASS is possible on this snapshot. Next action: preserve failing hydration evidence and correct or amend the seam, then rerun the exact matrix and obtain fresh implementation review.

`changed_paths`: only `docs/fork/theme-prepaint-implementation-spec-review-2026-10-03.md`. No product, status, Git or Graphify changes.
