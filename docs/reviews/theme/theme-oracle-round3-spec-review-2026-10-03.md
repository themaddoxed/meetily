> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme oracle round 3: independent SPEC review

2026-10-03. **SPEC PASS for the bounded oracle correction, Critical 0 / Important 0 / Minor 1.** The six matrix cases remain RED; this does not accept theme implementation or the full prepaint contract.

## Finding

**Minor, unchanged:** `frontend/tests/theme-foundation.spec.ts:158–160` identifies the inline owner through an IIFE prefix and `let storedMode = null;`. A valid correction to ThemeScript can fail this assertion solely because its spelling changes. Preserve single-owner/order checks but bind identity to a stable source-owned identifier or explicit script identity when implementing a seam correction. This is implementation coupling, not evidence that the currently observed RED is false.

## Prior Important findings

- **Closed:** lines 39–61 reconstruct each class/style transition using the next same-attribute record's oldValue, falling back to the final DOM only for the last record. Independent class/style cursors retain cross-attribute sequence. Same-value records are skipped. The prior wrong→correct sequence no longer collapses to a correct final snapshot. The direct-setAttribute regression at lines 76–91 asserts both intermediate class values and both color-scheme values.
- **Closed for the admitted parser-script owner:** lines 132–139 poll the preserved early complete-theme observation plus current correct root state, rather than demanding live body absence at asynchronous poll time. The unchanged ThemeScript runs synchronously as a parser script. MutationObserver bodyAbsent remains callback-time metadata; this review does not generalize it to scripts that themselves create/remove body within the same task. If a proposed correction changes that timing model, require a focused event-time regression before accepting it. No such production change is present here.

## Snapshot and checks

Root `/Users/developer/Desktop/meeting-minutes`; task `.worktrees/semantic-theme-2026-10-03`, admitted base `53f4415058fb01e38b8892a47ac036bfffa394ba`. `git status --short` reports only `frontend/tests/theme-foundation.spec.ts` modified. Read the exact source and prior finding sites, checked final SHA-256 unchanged across review: `c58fead9f4727a15ef36cf595933653c4ae9e0472f36ee0b425a699fb0b9330d`. No production or snapshot changes reviewed.

Read the completed Playwright log `/private/tmp/meetily-theme-oracle-round3.log`, SHA-256 `a27ddfc7c69078bc422445d62aa7191c32ab11e1e2cfc44b5a52d281a9c10924`: **2 passed / 6 failed, 59.6s**. Passing cases are the intermediate-mutation regression and stored-dark-before-DOMContentLoaded case. All six matrix failures remain at the held-CSS early-theme predicate; reload and subsequent first-frame/markup/hydration checks are not reached. The log provides terminal test totals; reviewer did not own the process or independently capture its shell exit code. No reviewer-run browser test, typecheck or other gate is claimed.

## Boundaries and next action

The stronger recorded-event oracle preserves the strict pre-CSS requirement; no production relaxation occurred. Proceed with bounded investigation of the existing allowlisted ThemeScript/layout seam, preserving these RED artifacts. Do not infer a visible wrong-theme frame from the pre-CSS failure. Any required new source path, dependency/config change or contract revision needs plan amendment and fresh review. A correction must eventually pass all six navigation/reload rows, first-renderable-frame, no corrective mutation and hydration checks, plus remaining planned gates.

`changed_paths`: only `docs/fork/theme-oracle-round3-spec-review-2026-10-03.md`. No source/status/Git changes or Graphify. Native/release NOT VERIFIED. No Important blocker remains in this bounded test correction; matrix implementation remains RED.

## Separate verdict: prepaint seam plan amendment

**SPEC PASS for the bounded seam amendment, Critical 0 / Important 0.** Reviewed `docs/superpowers/plans/2026-10-02-semantic-theme.md` SHA-256 `96094ccecd7e317004f555176d7ec013fb518d40a8083f9938075a8f61b03715`, section “Admitted execution and prepaint seam amendment”, lines 20–57. This is permission within the plan to test the proposed seam, not evidence it works.

The change retains the existing IIFE and sole ThemeScript source owner, serializes only trusted static markup with stable `meetily-theme-prepaint` identity, mounts it once through the root head's inner HTML and expressly forbids user-data interpolation, duplicate resolvers, dependency patches and hydration correction. It keeps the same 18 files and strict 12-transition/first-frame/hydration acceptance. Replacing the literal JSX ownership assertion with single source ownership and emitted script ID/count/order appropriately addresses the surviving Minor once implemented and tested.

Installed renderer inspection supports the proposed route: `pushStartHead` at line 4254 sends the root head into headChunks through the generic element writer; `pushInnerHTML` at line 2957 appends trusted inner HTML; `writePreamble` at lines 5503–5557 flushes headChunks before stylesheet resources. That gives a concrete source-supported hypothesis without modifying Next. Actual emitted HTML, execution timing and hydration still require the complete browser matrix; no successful runtime result is inferred from renderer source alone.

Before production edits, record matching QUALITY acceptance and bind the admission/evidence to this new plan hash. The original census plan_sha256 remains the previous plan's provenance; its source hashes remain useful but must not be represented as a new-plan hash match. Existing plan ownership prose is interpreted through this explicit narrow amendment. No change to commit/integration/native/release authority follows.
