# Background proof implementation — independent SPEC review

**FAIL. Critical 0 / Important 3 / Minor 0.** Three acceptance-oracle defects require correction; the completed real-route run also remains red.

## Findings

1. **Important — checked-switch exception expands to descendants.** Feature `frontend/tests/helpers/rendered-contrast.ts:114–116` uses `closest` and skips every descendant's background. Accepted plan v3 line 9 says “Retain the existing deliberate checked-switch background exception ... do not add another exception.” The original filter used `matches`, excluding only the switch itself. A white descendant, including a positioned panel outside the switch, can therefore be omitted and falsely pass. Use the own-node predicate; add a white-descendant negative. The new “checked switch fill” fixture at `theme-dark-surfaces.spec.ts:245` includes a white span but expects PASS, encoding the unintended expansion. Classification: incorrect implementation / scope creep.

2. **Important — background blending is not rejected.** Helper `:23–39` validates `mix-blend-mode` but omits `background-blend-mode`; `:56–70` treats fill and gradient as normal source-over layers. Plan v3 line 25 requires “Reject non-normal blending.” A solid RGB(240,240,240) fill plus the same constant gradient under `background-blend-mode:screen` produces approximately RGB(254,254,254), while this bound stays approximately 240 and certifies absence. Reject non-normal background blending and add a negative fixture. Classification: missing guard / incorrect bound model.

3. **Important — painted text pseudo backgrounds are omitted.** Helper `:101–110` enumerates only `::before` and `::after`. Plan v3 line 23 requires “Validate pseudo-element background/image/shadow paint too.” A `p::first-line { background:white }` or `::first-letter` white background is real background paint but enters neither the DOM layer list nor the two-pseudo loop. Add fail-closed detection or conservative inventory for these supported painted pseudos and negative fixtures. These backgrounds are distinct from excluded text foreground. CSS explicitly applies background properties to these pseudos: [CSS Backgrounds and Borders §1.1](https://www.w3.org/TR/css-backgrounds-3/#placement). Classification: partial coverage / false-negative path.

## Snapshot

Feature root: `.worktrees/semantic-theme-2026-10-03`; supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Exact SHA-256 before and after source review:

- `frontend/tests/helpers/rendered-contrast.ts`: `3cef9cbcc8831a39505348a760273125369404ab268e36a6bcf03f1c82b0a5fc`.
- `frontend/tests/theme-dark-surfaces.spec.ts`: `38c65863b06d20a9e70c650ed7d4c8e57c4a7aed45bf70c50ba10830234953b7`.
- Root accepted v3 plan: `3da22e4b231f5cfabe5b37e09ed56c3ad684971a88d3ccd9e67a2de5ef91fe05`.

Per-channel sorted bounds, whole-main cell coverage, closed boundary inclusion, zero-size/off-main shadow inventory, upward arithmetic margin and explicit node/layer/cell/operation caps follow the intended model. Those properties cannot compensate for omitted or misclassified paint. Existing checked-control descendants are not an authorized semantic exemption merely because a fixture passes.

## Checks and evidence

Reviewer actions: read numbered source and accepted plan, inspect test cases and rational-check script, parse completed route JSON, hash inputs/evidence. Evidence read from implementer-owned runs, not claimed as reviewer executions:

- `/private/tmp/meetily-bg-diag/fixtures.log`: terminal 29 PASS, 7.6s; SHA-256 `b64fde9a9b1a7c86d7048a1f81f9714663f393b60f67502bf6cab701e7e27347`. It lacks the three rejection cases above.
- `/private/tmp/meetily-bg-diag/routes.log`: completed JSON **0 expected / 3 unexpected / 0 skipped / 0 flaky**, duration 39.112s. Every failure is `Unresolved background bound: opaque HTML canvas` at the resolver call. SHA-256 `5a122a7c2440687812b9035a2126cd4cab99755313d01c2723c7fa69ad1648dd`. No route established background absence, and the errors do not establish actual near-white paint.
- `/private/tmp/meetily-bg-diag/check-opacity-bound.py`: SHA-256 `1857bff900438246298306989d549dc7db3be73f0e90a95cef1898b1ffd3f599`; associated result `opacity-bound-result.json`: `a86ec978f9a68427d7787f7e78f32b60c7851d1eda7d31a9d676b0b15810336e`. Reported 166,698 exact rational cases, zero violations; independent nested alpha/group/sibling formula is useful supplemental evidence, not production inventory coverage or browser acceptance.

A reviewer attempted a read-only synthetic Chromium reproduction using the installed Playwright/TypeScript packages. Browser launch failed before scenarios ran because default `chromium_headless_shell-1228` cache was absent; no browser was downloaded. Findings are source-confirmed defects with concrete counterexamples, not claimed reviewer-observed rendered pixels. Typecheck/ESLint success was reported by the implementer; no corresponding terminal logs were independently read here.

## Scope, blockers and next action

Review changed_paths: only this report. No product/test/plan/Git changes. Correct I1–I3 with meaningful RED/green fixtures; resolve the real HTML-canvas admission failure without silently changing the plan's canvas contract; then re-review new exact hashes and terminal route evidence. Full-theme, snapshots, native and release gates remain open.
