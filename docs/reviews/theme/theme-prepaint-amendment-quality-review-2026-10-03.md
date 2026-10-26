> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme prepaint seam amendment — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the explicit planning amendment: Critical 0 / Important 0 / Minor 0.** No actionable planning defect blocks the bounded correction attempt. This is separate from the round3 oracle PASS and does not accept an unimplemented or untested production fix.

## Exact scope and available source

Plan `docs/superpowers/plans/2026-10-02-semantic-theme.md`, SHA-256 `96094ccecd7e317004f555176d7ec013fb518d40a8083f9938075a8f61b03715`, pinned before/after. Reviewed the 3 October amendment and corresponding ownership/matrix clauses. Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

Production remains unchanged: layout.tsx SHA-256 `8acfd90d71814a9653065cfbe97bf911106c09c34ef8b9ebcb9d3966d523801a`; ThemeScript.tsx SHA-256 `11e2dfca5441cb2417a9d4089f8c27a516578d2cbdd8bda5f93201199b01eab6`. Only the separately reviewed theme-foundation test is dirty. Review activity was read-only except this report.

## Available seam and safeguards

The installed dependency is Next 14.2.35 with its bundled React server renderer; this should not be described as a React version numbered 14.2.35. In `node_modules/next/dist/compiled/react-dom/cjs/react-dom-server.edge.development.js`, `pushStartHead:4254-4259` sends root-head props to `pushStartGenericElement` with headChunks as target. That function at line 4427 calls `pushInnerHTML`; at lines 2957-2977, trusted __html bytes are appended directly to that target. `writePreamble:5513-5516` emits headChunks before stylesheet resource flushing at line 5553. This source path supports the proposed ordering mechanism. It is not a successful browser, hydration or production-build experiment.

Plan lines 35-49 explicitly amend the mount interface: the existing trusted IIFE remains owned by ThemeScript.tsx and is serialized once into root head, with stable `meetily-theme-prepaint` identity. No user-controlled interpolation, duplicate resolver, runtime correction effect, dependency patch or Next configuration is authorized. Keeping the script literal trusted is necessary because raw markup has no React escaping protection. Runtime preference parsing/native synchronization and the 18-path allowlist remain intact.

The plan now replaces literal JSX-component counting with both source-bound single ownership and actual-response exact-ID/count/order assertions. Thus it does not silently discard the earlier single-owner requirement. Correct implementation must remove the old child mount, avoid simultaneous children and dangerouslySetInnerHTML (the inspected renderer explicitly rejects that combination), and preserve automatically emitted head metadata/resources. The unchanged twelve-transition and zero-hydration-diagnostic gates are the acceptance check for that behavior; adding suppression or tolerating a hydration warning would not satisfy the amendment.

## Existing mocked caller

`frontend/tests/lib/product-analytics-startup-behavior.test.mjs:451` stubs only the previous ThemeScript function. The harness compiles layout through CommonJS and its JSX factory stores props; `renderTree:144-154` traverses children and does not interpret raw head HTML. The proposed constant markup export therefore becomes undefined in this mock unless explicitly supported, but a passive imported value used as __html does not itself call an undefined function or break the startup flow. This is not evidence of a definite regression and does not require an automatic allowlist expansion.

Run that existing startup test against the actual production diff. Keep theme ownership/markup correctness covered by the scoped real-module/source/browser assertions, not this unrelated mock. If implementation instead calls a new exported function or otherwise requires changing the mock, stop for a caller/allowlist amendment; do not silently edit the out-of-scope file or retain unnecessary legacy API solely to conceal a regression.

## Evidence limits and next action

Checks: exact plan/product hashes, scoped Git status, amendment/ownership text and installed renderer/caller source inspection in local Darwin. No reviewer product edits, framework patch, browser experiment or test execution. The earlier round3 log established 2 passing harness/characterization tests and 6 failing navigation rows; no successful reload/first-frame/response-order/hydration matrix follows from it.

`changed_paths`: `docs/fork/theme-prepaint-amendment-quality-review-2026-10-03.md` only. `blockers: []` for QUALITY acceptance of this precise amendment. `next_action`: after independent SPEC acceptance, implement only the reviewed source seam and exact-ID assertion, run the existing mocked caller test, final static checks and complete matrix, and review the exact resulting diff. Preserve no-op-versus-correction semantics in the oracle and all existing snapshot/native/release gates. An unexpected hydration/resource consequence or extra required path triggers reassessment rather than contract relaxation.
