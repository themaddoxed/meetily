# FEAT-101 independent plan review

## SPEC

# Independent SPEC review — FEAT-101 plan round 2

status: SPEC PASS; Critical 0 / Important 0 / Minor 0. No actionable SPEC findings on final reviewed bytes.
reviewer: independent plans_spec agent; source-authored review.
observed_snapshot: HEAD 53f4415058fb01e38b8892a47ac036bfffa394ba plus current FEAT-100 source overlay. Plan SHA-256 0d9c0bc76a313b4cb45599c75e83f6f28a0cdff61a0201e5cf1dfb95455a0e01, stable before/after. This supersedes previous 9303e4f5 plan review.
changed_paths: [/private/tmp/meetily-questions-plan-spec.md]; repository changed_paths: [].

## Findings and evidence

No actionable SPEC finding remains. Plan lines 18–24 now bind consent to a native opaque fingerprint over actual provider/model, normalized endpoint and generation parameters. One settings/custom-JSON snapshot and predispatch revalidation cover a same-provider/model endpoint change without exposing credentials to IPC.

Lines 26–41 explicitly own isolated-only sidecar cleanup, kill-on-drop, stderr suppression, bounded kill/reap before blocking stdin operations, async success/error cleanup, dropped-future fallback and no startup after cancellation. Real helper-process tests must assert termination/reaping and a separate summary fixture staying alive. This directly covers the current source hazards: client.rs uses the global summary manager; sidecar.rs shutdown can await stdin and Drop only sets a flag. Existing summary behavior remains outside the isolated mode.

Selected-meeting SQLite context, full corpus budget/overflow rejection, selected model/provider visibility, insufficient evidence, validated IDs/exact quotes, native source links, stale/deleted meeting rejection and browser navigation beyond page one remain explicit. Quote validation is not claimed to establish entailment. FEAT-102 actual inference/faithfulness and parent completion remain separate.

## Checks and limits

Environment: local macOS workspace, zsh. Commands: shasum -a 256, nl/cat/rg/sed source and PRD reads, git rev-parse HEAD, final Python hashlib guard. Reviewed full amended plan and live settings/sidecar cleanup seams. Relevant source/PRD hashes unchanged from preceding review. No product edits, network inference, credentials access, install, native application or browser execution.

SPEC authority: docs/prd/fork-features.md FEAT-078:168–172, FEAT-101:294–296 and FEAT-102:298–300.
NOT VERIFIED: actual process lifecycle/isolation, config-consent race execution, runtime model accuracy, semantic faithfulness, actual browser navigation and release readiness. Required real helper-process tests must exercise production isolated wiring; fake model replies alone do not satisfy them. No NOT REVIEWED final target: exact plan bytes present and stable.
blockers: none to bounded planning SPEC PASS.
next_action: implement and validate exact production wiring, then obtain independent implementation SPEC/QUALITY. FEAT-102 remains open.

## Evidence SHA-256
- docs/prd/fork-features.md: 7c93d3859299719e957c30a2ee5f4972b6e445d30ce77b2744140a5bd34aed8c
- frontend/src-tauri/src/database/repositories/meeting_context.rs: 64c28e74042a206c8ed4c1bc847165aa1741ee72f498921d62227c2ed3275558
- frontend/src-tauri/src/database/repositories/setting.rs: a0a4be3df825b04da24cf64c7faffd82156b483a599feca6038c0833ce27a063
- frontend/src-tauri/src/summary/summary_engine/client.rs: d944994753b0ef103e183ccf311c46045d33e436642926183164c4f653b61e49
- frontend/src-tauri/src/summary/summary_engine/sidecar.rs: b3b7a0b3023df314518b56ec6f6591e7ce5ce9a0614ca2472a33f07ea50976fc
- frontend/src-tauri/src/summary/llm_client.rs: 2bfc35f8f97616ecd69d613f5f56f452807be28390062db0dacb9505a7b2be5f
- frontend/src/app/meeting-details/page.tsx: cbed853e1a85ee40583b81a875bcad78cfe02544f67343eb373ef3096bb24b12


## QUALITY

# FEAT-101 meeting questions plan — independent QUALITY round 2

Reviewer: /root/plans_quality. Source-authored result, 2026-10-02.
status: QUALITY PASS. Critical 0 / Important 0 / Minor 0.
observed_snapshot: root base 53f4415058fb01e38b8892a47ac036bfffa394ba plus reviewed FEAT-100 dirty overlay. Exact final plan and relevant existing adapter hashes below verified before/after.
changed_paths: [/private/tmp/meetily-questions-plan-quality.md] only, authorized report overwrite. No product/docs changes.

## Findings

No actionable findings remain on the final exact plan revision. This supersedes round 1 FAIL for plan 9303e4f5ee55e284f81f2f3ddb8bae1693435aa7a9cfa115a272965e7c142255; the two Important issues are resolved in the amended contract, not claimed implemented.

## Resolved process cleanup issue

meeting-questions.md:31-41 now admits a narrow isolated-only sidecar.rs lifecycle path. It requires kill_on_drop, stderr null, no blocking stdin shutdown before termination, start_kill plus bounded wait/reap, handle release, stop-before-new-start invariant, and an owned async guard covering success/all errors with Drop fallback. Real temporary-process tests must verify PID termination/reaping across startup failure, normal success, errors, cancellation, deadline and dropped future while a separate summary fixture remains alive. This gives the dedicated manager a concrete executable lifecycle contract instead of relying on the existing SidecarManager Drop flag or unbounded shutdown write. Existing summary manager methods remain outside that behavior change.

## Resolved consent/configuration issue

:18-24 replaces expected-provider/model-only checking with an opaque native configuration fingerprint over provider, model, normalized endpoint and generation parameters. The sanitized config command supplies the fingerprint/actual provider/model/transport; consent binds to it and native code revalidates the current Settings snapshot before dispatch. Same-provider/model endpoint changes are explicitly rejected. No credential material is returned or included in the fingerprint.

## Other reviewed requirements

The dedicated generation entry point shares current catalog prompt formatting, trusted summary_manifest and MODEL_INTEGRITY_CACHE checks; no second downloader or weaker checksum is permitted (:26-30). Fresh selected-meeting SQLite context and post-generation deterministic reread reject deletion/edit drift. Full-corpus budgeting rejects overflow without silent truncation. Strict output/known-ID/exact-quote validation and server-created links bind attribution while explicitly not claiming semantic entailment. UI request IDs and isolated cancellation prevent stale result replacement or summary lifecycle mutation. FEAT-102/model-faithfulness acceptance remains separate.

## Checks and boundaries

Environment: root checkout on Darwin, static read-only review plus requested report.
- Read the complete amended 98-line plan and compare with both source-authored round1 findings.
- Retained current source inspection of built-in client integrity/generation, sidecar spawn/send/shutdown/Drop, HTTP adapter response/body handling, and SettingsRepository configuration loading.
- shasum/Python hashlib final check: four exact pins stable; no source edits were performed by this reviewer.
- No tests, helper/model/native GUI, external inference, credential inspection or user-profile access executed during review.

NOT VERIFIED: implementation of async guard/drop during runtime teardown, exact child reaping on failure, shared-summary isolation, trusted-checksum regression tests, token budget math, UI IPC integration, real deep transcript link navigation, hostile-transcript/model adherence and native release readiness. These require the planned exact-diff implementation review and executable tests. During implementation, check that whole-request cancellation/deadline also covers HTTP body consumption (existing adapter only selects cancellation around header acquisition) and that local/network classification remains valid under the actual HTTP client's redirect policy. Those current adapter behaviors have not been experimentally verified here and are not claimed safe merely from plan acceptance.

blockers: [] for QUALITY plan acceptance.
next_action: combine with SPEC acceptance on this exact hash, implement within the admitted scope, then obtain fresh exact-diff reviews and required checks. This PASS is not implementation, native, provider accuracy or release proof.

## Exact reviewed SHA-256
- `docs/superpowers/plans/2026-10-02-meeting-questions.md`: `0d9c0bc76a313b4cb45599c75e83f6f28a0cdff61a0201e5cf1dfb95455a0e01`
- `frontend/src-tauri/src/summary/summary_engine/client.rs`: `d944994753b0ef103e183ccf311c46045d33e436642926183164c4f653b61e49`
- `frontend/src-tauri/src/summary/summary_engine/sidecar.rs`: `b3b7a0b3023df314518b56ec6f6591e7ce5ce9a0614ca2472a33f07ea50976fc`
- `frontend/src-tauri/src/summary/llm_client.rs`: `2bfc35f8f97616ecd69d613f5f56f452807be28390062db0dacb9505a7b2be5f`
