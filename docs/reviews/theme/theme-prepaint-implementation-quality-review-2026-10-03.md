> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme prepaint implementation — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL: Critical 0 / Important 1 / Minor 0.** The ordering correction triggers a real hydration mismatch and fails the accepted zero-diagnostic contract.

## Important finding

**Root-head raw HTML disagrees with the head hydrated by Next.** In `frontend/src/app/layout.tsx:243`, the new `dangerouslySetInnerHTML={{ __html: THEME_SCRIPT_MARKUP }}` expects the complete head contents to equal one script. The actual server head includes that script followed by Next-generated metadata, stylesheet/preload links and runtime scripts; the observed browser head also includes library style content. React reports `Prop ... did not match ... dangerouslySetInnerHTML` on head. All six mode/OS matrix tests fail at their hydration-diagnostics assertion, after the earlier ordering/state assertions.

Evidence: completed coordinator log `/private/tmp/meetily-theme-prepaint-green.log`, SHA-256 `abeec56e432adb6e70db0f72c76cab360e07b2766bbdfbdeaede8778a43c21d4`, ends with 3 passed / 6 failed in 43.3 seconds and contains the concrete server/client head mismatch for each matrix row. The filename does not indicate a green result. The raw-head seam was a supported ordering hypothesis; this runtime result demonstrates that its current implementation is not hydration-safe.

Action: reconcile server and client ownership of head contents while retaining one early trusted script and all automatically generated resources. Do not add suppressHydrationWarning, filter this diagnostic, skip a row or relax ordering to obtain green. Any necessary architecture/interface/allowlist change must receive explicit amendment review before expanding the correction. Require the full matrix to pass without warnings on the revised exact bytes. This review does not assert that no permitted solution exists.

## Exact reviewed snapshot

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Git diff/status shows exactly these three dirty paths:

| Path | SHA-256 |
| --- | --- |
| `frontend/src/app/layout.tsx` | `dac7ec01049f481d79ae32127c80578f4438defc3f18ecf7ad5b83001bcc7723` |
| `frontend/src/components/ThemeScript.tsx` | `6cce905b099bfb26e40ce65564b79b042454ee464b6197dbce83f1611aa10601` |
| `frontend/tests/theme-foundation.spec.ts` | `468ddcdf90136041271e0ba0c94739d5b0d68a458ab4382a6d556e71dc140781` |

Plan authority remains SHA-256 `96094ccecd7e317004f555176d7ec013fb518d40a8083f9938075a8f61b03715`. Pins verified before/after; no source changes by reviewer.

## Other reviewed behavior and evidence limits

The script body and preference-resolution behavior are unchanged. Exported markup contains only the source-owned IIFE and stable script ID; its sole interpolated storage key is the existing source constant. No stored preference, transcript or caller-supplied data is interpolated into raw markup. No concrete injection defect was found. The old JSX child is removed, there is one head mount, and no simultaneous children/raw-HTML props are introduced. The new source ownership test and actual-response exact-ID check fit the reviewed amendment.

The repaired mutation reconstruction and recorded-early-event oracle remain present. Actual test execution now passes the ownership regression, intermediate-mutation regression and old first-dark-event test. Each matrix row reaches the final diagnostics assertion on navigation: early resolution under held CSS, the rAF checkpoint, later-state checks and script-before-stylesheet assertion have therefore passed for that navigation. Reload is not reached after its navigation fails. This is six partial navigation successes with six hydration failures, not twelve complete transitions or visible/compositor-level no-flash proof.

Typecheck, ESLint and eight startup Node tests are coordinator-reported successful; the reviewer did not execute them or independently read their full logs in this review. Their success does not negate the observed browser failure. Reviewer checks were exact diff/source/status/hash inspection and actual completed browser-log reading in local Darwin. No product edits, browser invocation, native action, Git mutation, Graphify or snapshot update.

`changed_paths`: `docs/fork/theme-prepaint-implementation-quality-review-2026-10-03.md` only. `blockers`: confirmed hydration mismatch; complete matrix and broader theme/native/release acceptance remain unresolved. `next_action`: preserve this failed experiment and logs, review a bounded hydration-safe adjustment before any scope expansion, then test the revised exact source without weakening the accepted contract.
