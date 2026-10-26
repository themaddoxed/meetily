# Two e2e locator corrections — independent QUALITY plan review

Source-authored by `/root/plans_quality`, 2026-10-04.

**Bounded planning QUALITY PASS: Critical 0 / Important 0 / Minor 0.** Both proposed locator corrections match the existing UI and preserve the original behavioral checks. No actionable finding in the admitted two-file scope.

## Exact snapshot

Plan `docs/superpowers/plans/2026-10-04-theme-e2e-locators.md`: SHA-256 `432617704a37aec8e2139b7828bdf4c461a7ec2afcdda7068076822fac1fed36`.

Task checkout `.worktrees/semantic-theme-2026-10-03`, supplied base `53f4415058fb01e38b8892a47ac036bfffa394ba`; current unmodified target files:

- `frontend/tests/interface-polish-acceptance.spec.ts`: `8776a106a48b7896cf2a7e836727b359e6d734b6f40712e3a0f3c6ed55df10b0`.
- `frontend/tests/summary-template-selection.spec.ts`: `66487c4a0b6a89f822ca1b18b8a8bc0866b797a1f966ad2bd3bfb5bc3170c7de`.

Evidence manifest `docs/fork/evidence/theme-project-full-2026-10-04.json`: `18a5abcdb4a962025b145d21afceb74cf295bd91c812d1ed105aeca61466cdd3` when read. Its historical native-stage failures/retries are context, not acceptance granted by this review.

## Locator assessment

`interface-polish-acceptance.spec.ts:584` currently requests substring Settings globally. The isolated failure explicitly identifies the intended Sidebar button and the distinct Open Audio Capture Settings button. Scoping to app-sidebar plus exact Settings, and using the same scope/exact Home for return at line 603, selects the intended navigation path. It does not bypass hydration/listener cleanup: the server HTML/probe, listener counts, URL changes, unmount/remount and final hydration-error assertions remain required at lines 554–626.

`summary-template-selection.spec.ts:108` requests Section 1 title, but `src/components/MeetingDetails/TemplateEditorModal.tsx:241` constructs the accessible name from Section title followed by the index. The retained failure DOM lists dialog Create summary template and textbox Section title 1. Exact matching inside that dialog corrects the stale name without changing accessibility labels or product behavior. Keeping pressSequentially('Decisions') and the exact value assertion preserves typing coverage; the added focus assertion directly strengthens the named focus-retention purpose. No force-click, fill replacement or timeout increase is admitted.

The extra two test paths are explicitly added to the plan scope. Snapshot/config/fixture/product changes remain excluded. A targeted two-case run followed by both full spec files and the required full e2e/canonical gate is proportionate to this change and the known failed broader run.

## Observed evidence and limits

Reviewer actions: read-only source/plan/manifest inspection, SHA-256 checks, and reading retained full/isolated failure logs and the actual dialog error-context snapshot. No test process, native action or product edit was performed by this reviewer.

- `/private/tmp/meetily-theme-all-e2e-2026-10-04.log`, SHA `9d93fcc700ccbea7ec472e34cedebfab219d2505ff580caf4e72a0b7e9ceb0f1`: 210 passes, six failures, including the two locators and four other failures.
- `/private/tmp/meetily-theme-e2e-six-isolated-2026-10-04.log`, SHA `e4a65dc3c03a189d4cedee2a2c27c98228032502d363a64bfc4bcdff9c51c9c2`: four passes and two remaining locator failures; the exact strict-mode ambiguity and absent old textbox name are present.

The isolated successes do not establish why the other four originally failed, nor erase those observations. The plan appropriately preserves the RED evidence and requires a new mandatory full e2e run. No prior snapshots, increased timing allowances or assertion relaxation are used to turn those failures green. The canonical gate and native/exact-artifact readiness remain separately unresolved until their required evidence exists.

`changed_paths`: this report only. `findings: []`. `blockers: []` for this planning gate. `next_action`: make only the admitted locator/focus-assertion edits, obtain exact-diff implementation review and complete the specified targeted/full verification. No task close, integration or release acceptance is implied.
