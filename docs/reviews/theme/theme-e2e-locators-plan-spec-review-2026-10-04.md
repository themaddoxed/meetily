# Two e2e locator corrections — independent SPEC plan review

**PASS for bounded admission. C0 / I0 / M0.** No actionable findings. Admission is for two test-only corrections; it does not convert the failed full gate into PASS.

## Snapshot and scope

Reviewed 2026-10-04. Plan SHA-256 before/after: `432617704a37aec8e2139b7828bdf4c461a7ec2afcdda7068076822fac1fed36`.

Observed feature baseline hashes match the proposal: `interface-polish-acceptance.spec.ts` = `8776a106a48b7896cf2a7e836727b359e6d734b6f40712e3a0f3c6ed55df10b0`; `summary-template-selection.spec.ts` = `66487c4a0b6a89f822ca1b18b8a8bc0866b797a1f966ad2bd3bfb5bc3170c7de`. Feature root `.worktrees/semantic-theme-2026-10-03`; supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Review changed_paths: only this report. No source, test, plan or Git edits.

The amendment adds exactly these two files to the prior 21-path scope (23 total). It prohibits product/fixture/timeout/configuration/snapshot changes and requires affected cases, both full spec files, exact-diff review, full frontend e2e and the canonical full gate. Scope and follow-up checks are proportionate to the actual failures.

## Cause and preserved behavior

The isolated log explicitly reports a strict-mode violation at `interface-polish-acceptance.spec.ts:584`: substring Settings resolves both the sidebar Settings control and Open Audio Capture Settings. Sidebar-scoped exact navigation selects the intended existing control; exact Home returns through the same route. The surrounding server-HTML/hydration probe, URL, probe removal, listener subscription/removal and media-state assertions remain required. This repairs interaction targeting without weakening cleanup behavior.

At `summary-template-selection.spec.ts:108`, the missing textbox locator asks for Section 1 title. Actual `TemplateEditorModal.tsx:241` builds its accessible name from sectionTitle followed by the index. The preserved failure DOM shows dialog Create summary template and textbox Section title 1. Dialog-scoped exact matching therefore targets the existing field. Sequential Decisions input plus exact final value and retained focus remain the behavioral oracle; the new focus assertion strengthens it. No product-label change or forced interaction is authorized.

## Evidence and checks

Reviewer actions: read source and plan, hash inputs, inspect exact isolated errors and preserved dialog DOM; no reviewer-run test.

- `docs/fork/evidence/theme-project-full-2026-10-04.json`: observed SHA-256 `18a5abcdb4a962025b145d21afceb74cf295bd91c812d1ed105aeca61466cdd3`. Records canonical full failure and separate prepared-stage retries without claiming native/release success.
- `/private/tmp/meetily-theme-all-e2e-2026-10-04.log`: SHA-256 `9d93fcc700ccbea7ec472e34cedebfab219d2505ff580caf4e72a0b7e9ceb0f1`; evidence reports 210 PASS / 6 FAIL.
- `/private/tmp/meetily-theme-e2e-six-isolated-2026-10-04.log`: read terminal **4 PASS / 2 FAIL**, SHA-256 `e4a65dc3c03a189d4cedee2a2c27c98228032502d363a64bfc4bcdff9c51c9c2`. Both retained failures match the source-backed locator causes.
- Preserved `summary-template-selection-c4419-focus-during-keyboard-input-chromium/error-context.md` under `/private/tmp/meetily-theme-all-e2e-failures-2026-10-04/` confirms the dialog and field names. TemplateEditorModal source SHA-256 `48dc6180daa9296ac4662f3ee563e826929456a3bb6c01d721842e99dd3678a5`.

The four isolated passes do not erase prior failures or establish their cause. Broader Rust, ignored-test, native and artifact gate results were not independently re-audited in this narrow plan review.

## Next action

No blocker to implementing the specified locator changes and focus assertion. Preserve the RED logs, then review exact new hashes and the required terminal runs. Full-theme/task/integration/release completion remains unapproved.
