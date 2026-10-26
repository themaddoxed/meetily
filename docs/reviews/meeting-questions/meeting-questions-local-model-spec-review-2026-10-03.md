# FEAT-102 local model — independent SPEC acceptance

status: SPEC PASS for the exact controlled local CPU/Gemma run described below. Critical 0 / Important 0 / Minor 0; no actionable findings in that bounded acceptance scope.
reviewer: independent plans_spec agent. Read and assessed raw machine-produced replies and test source; did not run the model/test.
observed_snapshot: source base 53f4415058fb01e38b8892a47ac036bfffa394ba plus reviewed FEAT-100/101 changes and isolated opt-in test. Client correction 0a512f4c… is present in both root and candidate. Runtime evidence is run4 log ed8657a9…, test19d78d66…, model gemma4:e2b, exact helper hash below.
changed_paths: [docs/fork/meeting-questions-local-model-spec-review-2026-10-03.md]; product changed_paths: [].

## Semantic findings and acceptance

The source-authored run4 log terminates with 1 passed / 0 failed / 0 ignored / 0 filtered, total 130.62 seconds. This is one opt-in integration test with three real model requests plus empty-context and SQLite-reopen checks, not three independently repeated trials.

The selected stored transcript is “Orion ships on Friday. Mira owns the launch. The budget is undecided.” The neighboring transcript instead states Tuesday and contains private codeword ZEBRA-917.

- Asked shipping day and owner: raw model JSON returned exactly two affirmative claims, “Orion ships on Friday.” and “Mira owns the launch.” Each carries its matching literal quote and selected-plan transcript ID. Independently reading these claims against the selected fixture confirms their meaning, not merely keyword/JSON validity. Production validation constructs links with id=selected and transcriptId=selected-plan. Elapsed request path: 95,808 ms.
- Asked the exact approved budget: raw reply is insufficient_evidence=true, claims=[]; this is correct because no amount is stated and the budget is undecided. Elapsed: 17,921 ms.
- Asked the private codeword: raw reply is insufficient_evidence=true, claims=[]; the selected meeting contains no codeword. Elapsed: 16,623 ms. The generator closure also asserts before inference that neither ZEBRA-917 nor Tuesday occurs in the prompt.

Test source calls production answer_with_generator and generate_with_builtin_isolated with SYSTEM_PROMPT and catalog-derived budget. Raw replies pass unmodified to strict production parsing/quote validation; there is no permissive extraction or repaired JSON. Its affirmative meaning oracle rejects unrecognized/additional/negated claims rather than accepting keyword presence. An empty meeting uses a generator that panics if called, and returns insufficient successfully. The temporary migrated SQLite is closed/reopened and its selected transcript is asserted byte-for-byte unchanged.

These observations satisfy FEAT-102's controlled-meeting local requirements: supported answer with verifiable sources, explicit absence-of-evidence answers, and neighboring content isolation. They do not demonstrate general model reliability or guarantee every future answer.

## Checks, provenance and environment

- Read complete durable run4 log and test source; independently recomputed both SHA-256 values before/after review.
- Read trusted production model manifest and catalog: Gemma4 E2B Q4_K_M, context32768, manifest revision90f9618340396838ee7ff5b0ba2da27da62953d3, expected size3,106,736,256 and SHA9378bc…. The test makes an independent regular temporary model copy because production rejects symlinks; the full production integrity gate remains active before helper startup.
- Independently rehashed current helper binary: matches 2fadd975…. Log identifies that exact helper path and gemma4:e2b. CPU/no-default-features build provenance comes from the recorded build, not from inference-log text alone. This reviewer did not rehash the 3.1GB cached original again; successful fresh temporary-copy verification through the production gate is the run's model-integrity evidence.
- Independently checked root/candidate client correction hash; no drift. Read prior integrity-cache code showing successful caching only after full bytes and identity verification.
- Execution owner records Cargo1.98.1 and profile.dev.package.sha2.opt-level=3 for this run; log shows sha2 recompiled. This is an optimized cryptographic calculation, not evidence of skipped checksum validation. Latencies include validation/setup/generation and are not pure token-generation benchmarks.
- Read-only reviewer tools: cat/sed/rg and shasum/hashlib in local macOS workspace. No Graphify, credentials, inference, network, application installation or user-database mutation by reviewer.

## Evidence paths and SHA-256
- .worktrees/integration-backups-2026-10-02/feat102-local-inference-run4-optimized-sha.log: ed8657a9e19b4db31a949e510be3d520eb978616a1c71745d0d9b86adeb57049
- .worktrees/main-features-2026-10-02/frontend/src-tauri/tests/meeting_questions_local_model.rs: 19d78d66e275d7b96fd30fcd6a5f0e1dfdbf58867c15c7629d171f4c6dc2f2dd
- .worktrees/main-features-2026-10-02/frontend/src-tauri/src/summary/summary_engine/client.rs: 0a512f4cbd1bd56f26461bd30a6b2d64534f620f69980b1927164ba1726a23bb
- .worktrees/feat071-audio-controls/target/check-full/debug/llama-helper: 2fadd975d96596d46cf26f6a8917e8d10da44e1ec56207b30df68de728113720.
- Trusted Gemma weights SHA-256: 9378bc471710229ef165709b62e34bfb62231420ddaf6d729e727305b5b8672d (production manifest; full verification exercised by run).

## Residual limits and next action

Earlier run1 fixture failure and run2/run3 timeouts remain failures. Run4 differs by the independently reviewed cancellation correction and optimized full SHA computation; it does not prove the prior unoptimized profile meets the deadline. No cancellation timeout occurred in run4, so it is not a new real-timeout-cleanup demonstration. Reported post-run process absence establishes only observation after completion, not continuous PID/reap tracking; existing isolated-process regressions are separate evidence.

NOT VERIFIED: other models, other quantizations/hardware/profiles, general semantic faithfulness, external/network providers, installed/native GUI/IPC and release readiness. No fresh release gate or broad user-data preservation claim. The full application endpoint's settings/consent path is covered separately by FEAT-101; this test directly exercises the production context/generation/validation seams.

blockers: none for this narrowly identified local FEAT-102 acceptance. Final task documentation must retain model/profile/fixture limits and earlier failures.
next_action: combine with independent QUALITY, update stale experiment metadata to the terminal run4 result, and record FEAT-102 acceptance with these exact bounds. Do not generalize this result to all models, network/native acceptance or release readiness.

## Explicit FEAT-078 parent acceptance audit

Read current docs/prd/fork-features.md:172–176 and dependency statuses after the local-run review. FEAT-076 and FEAT-100 are closed; FEAT-101 is still administratively reopened with its accepted correction recorded, and FEAT-102 is awaiting closeout. Independently compared all nine root FEAT-101 files to current candidate: byte-identical, including the cold-cancel correction.

Original parent criterion mapping:
- Selected meeting only / neighboring meeting excluded: production SQLite reader and bounded prompt, deterministic isolation cases, plus this real local inference run's before-generation neighbor-text assertions and unknown-codeword response.
- Links open supporting transcript: actual browser source navigation beyond the first page was independently read as PASS in the preceding code review; model run returns server-validated selected-row source links. No installed-native claim follows.
- Selected local/network provider visible: implemented UI and actual browser provider/consent scenarios, independently inspected earlier. Remote inference itself is separate permission scope.
- Explicit insufficient evidence / no guess presented as a found fact: real controlled known-fact answer and two evidence-absent responses are semantically correct; strict attribution validation and the generated-answer/evidence label are present. This demonstrates the controlled acceptance cases, not a universal guarantee against hallucination.

Conclusion: the stated FEAT-078 source/integration milestone criteria are covered once the corrected FEAT-101 and bounded FEAT-102 acceptance are formally recorded. No additional concrete unmet original criterion was identified. Administrative child closeout and parent audit recording remain; this report does not change their statuses. The user-visible browser path plus actual production generation/validation seam evidence support this bounded milestone; they do not establish native installed end-to-end, all-model/all-provider acceptance or release readiness.
