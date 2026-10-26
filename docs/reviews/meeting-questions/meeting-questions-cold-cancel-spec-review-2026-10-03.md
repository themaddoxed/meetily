# FEAT-101 cold-verification cancellation — independent SPEC review

status: SPEC PASS for the bounded client correction. Critical 0 / Important 0 / Minor 0; no actionable finding.
reviewer: independent plans_spec agent; source-authored review, not a reviewer-run test claim.
observed_snapshot: base HEAD 53f4415058fb01e38b8892a47ac036bfffa394ba. Candidate client at .worktrees/main-features-2026-10-02/frontend/src-tauri/src/summary/summary_engine/client.rs SHA-256 0a512f4cbd1bd56f26461bd30a6b2d64534f620f69980b1927164ba1726a23bb. Root client remains prior accepted SHA-256 06e29d2d21a681c69024fda753e1b8dbdd657921f8d9b51698d2e5853411c50b. Other eight FEAT-101 candidate paths match their frozen manifest values. Local-model experiment/test is outside this correction review.
changed_paths: [docs/fork/meeting-questions-cold-cancel-spec-review-2026-10-03.md]; product changed_paths: [].

## Findings and basis

No actionable SPEC finding remains. client.rs:130 introduces await_isolated_verification around the existing verification future. The biased cancellation branch wins when already cancelled; a post-result cancellation check and a second check immediately before ensure_running prevent an isolated cancelled preflight from reaching helper startup. The default summary path still awaits the original verification with its original error context and shared manager behavior.

Full model integrity is retained. Inspection of model_download.rs:213–249 confirms IntegrityCache inserts a successful entry only after complete verify_file and a second matching identity stamp. The wrapper cancels by dropping an incomplete verification future, not by replacing success or allowing startup on unverified bytes. No hash, manifest or catalog constraint is weakened.

The new pending-verification test synchronizes entry with Notify before cancelling and asserts that the continuation representing helper startup is not reached. The companion test retains verification errors and tests already-ready cancellation plus no-token success. Existing real temporary-file same-size corruption rejection remains in the selected test target. These tests establish wrapper behavior and integrity regression coverage, not the duration/outcome of a full cold Gemma run.

## Checks and evidence

Environment: local macOS root and candidate; zsh; no Graphify, model inference, credentials, network or native application action by reviewer.
- shasum/Python hashlib before and immediately before report: exact candidate SHA stable, prior/root SHA verified; no absent or changed reviewed target (NOT REVIEWED: none).
- diff -u saved prior client against candidate: only isolated verification wrapper/wiring, pre-start cancellation check and two regressions added.
- Source reads: model integrity cache and full-file verifier; confirm cache publication happens after verified bytes/stamp, summary branch unchanged.
- Read existing durable machine log .worktrees/integration-backups-2026-10-02/feat101-cold-cancel-unit.log: compilation from candidate frontend/src-tauri, 6 passed / 0 failed / 0 ignored, including both new regressions and same-size corruption check. Binary uses the established shared target/check-full directory under feat071-audio-controls. This is inspected execution evidence, not reviewer execution. Exit0 was reported by the execution owner; the log independently shows successful test result.

Evidence SHA-256:
- Candidate client: 0a512f4cbd1bd56f26461bd30a6b2d64534f620f69980b1927164ba1726a23bb.
- .worktrees/integration-backups-2026-10-02/feat101-client-before-cold-cancel.rs: 06e29d2d21a681c69024fda753e1b8dbdd657921f8d9b51698d2e5853411c50b.
- .worktrees/integration-backups-2026-10-02/feat101-cold-cancel-unit.log: 9dfe916deca8f2f0fd406de70911187f74f25e73f53b63be6858be1db3572763.

blockers: none to this exact bounded correction's source SPEC PASS.
NOT VERIFIED: successful real cold-preflight cancellation timing/cleanup, real model response or semantic faithfulness, installed native IPC, release readiness. Earlier FEAT-102 timeout failures remain failures; this review does not turn them into successful model evidence.
next_action: combine with independent QUALITY and relevant required checks; preserve this exact candidate hash before authorized transfer. Re-run the bounded real experiment only as new evidence for the fixed cancellation path; do not close FEAT-102 from these unit results.
