# Full frontend gate: two obsolete locators — 4 October 2026

Status: pending bounded independent admission review. Supplements semantic-theme plan and accepted snapshot amendment; existing admitted 21 paths remain unchanged, adding only the two test files below (23 total). No product behavior change.

Evidence: mandatory full e2e returned exit1, 210PASS/6FAIL. Unmodified six-case single-worker rerun after Rust compilation returned exit1, 4PASS/2FAIL. The remaining failures are deterministic locators, not missing UI. See docs/fork/evidence/theme-project-full-2026-10-04.json. Prior failures remain retained; four isolated passes do not erase the earlier failure observations.

1. frontend/tests/interface-polish-acceptance.spec.ts:584 — reduced-motion listener cleanup test uses substring Settings, which also matches existing Open Audio Capture Settings on Home. Scope navigation to app-sidebar and exact Settings (and exact Home for return); retain hydration, probe removal, listener counts, media state and URL assertions.
2. frontend/tests/summary-template-selection.spec.ts:108 — existing dialog renders aria-label Section title 1 (TemplateEditorModal.tsx:241), while test expects Section 1 title. Scope to dialog Create summary template and exact textbox Section title 1. Retain sequential typing Decisions and value assertion; additionally require focus after typing. Do not alter product labels, fixtures, handlers, timeout or force clicks.

No snapshots, tolerances, runner config, manifests, dependency updates, native actions or commits admitted. No broader test refactor. Original source hashes:
- frontend/tests/interface-polish-acceptance.spec.ts: 8776a106a48b7896cf2a7e836727b359e6d734b6f40712e3a0f3c6ed55df10b0
- frontend/tests/summary-template-selection.spec.ts: 66487c4a0b6a89f822ca1b18b8a8bc0866b797a1f966ad2bd3bfb5bc3170c7de

After admission, change only these locators/assertion scopes. Run affected two cases unchanged expectations/timeout, then relevant entire two spec files; inspect exact diff and independent bounded implementation reviews. Preserve RED evidence. Rerun mandatory full frontend e2e because it failed, then canonical check.sh --full with already verified local ONNX Runtime and current-source ignored Intel helper; explicitly report ignored tests and native/exact-artifact gate NOT_RUN. No task/feature/release completion or integration implied.
