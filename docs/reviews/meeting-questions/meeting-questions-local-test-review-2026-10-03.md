# FEAT-102 opt-in local inference test — independent QUALITY preflight review

Source-authored by `/root/plans_quality`, 2026-10-03.

**Status: QUALITY PASS. Critical 0 / Important 0 / Minor 0.** No actionable findings remain in the amended test revision. The first executed revision failed before inference; that failure remains recorded below. This accepts the bounded test design before execution; it is not a passed inference result or approval of any particular model/helper artifact.

## Scope and observed snapshot

Candidate: `.worktrees/main-features-2026-10-02/frontend/src-tauri/tests/meeting_questions_local_model.rs`.
Reviewed SHA-256: `8558d7c44f800a86910077d463b7514573b143ae5b0100b71b87039da5aba384`, verified before/after. All nine FEAT-101 candidate hashes still match the independently reviewed round4 manifest (base `53f4415058fb01e38b8892a47ac036bfffa394ba`).

`changed_paths`: `docs/fork/meeting-questions-local-test-review-2026-10-03.md` only, explicitly authorized review output. No product/test edits, model execution, native GUI, user-profile access or Graphify use by this reviewer.

## Resolved finding and acceptance strength

The superseded test `62baf4362d71e2e23535d48a7e65f654e73f40eb53ae2df2a84a5a9ca96868f0` had an Important false-positive oracle: contradictory answers could contain “Friday” and “Mira” and pass. Current lines 111-130 compare the entire normalized claim text with five explicit affirmative factual formulations. Negations, unsupported additional claims and unexpected paraphrases do not match; the previous contradictory Friday/Mira counterexample is rejected. This is a deliberately narrow synthetic oracle, not a general semantic-entailment evaluator. Legitimate unexpected paraphrases fail closed and require diagnosis rather than silently expanding the whitelist after a run.

Production strict JSON, selected-ID/exact-quotation and server-link validation still runs before that fixture oracle. Known-answer citations must reference selected-plan. Missing budget and neighbor-only codeword questions require insufficient_evidence with zero claims. Prompt assertion excludes the neighbor's Tuesday/codeword text before inference. Empty context must not invoke any model. SQLite close/reopen checks preservation of the selected transcript. These assertions are appropriate for the stated small controlled experiment; they do not prove all meeting questions or arbitrary models are faithful.

## Fixture correction after first execution

The preserved first-run source (`.worktrees/integration-backups-2026-10-02/feat102-local-test-run1.rs`, SHA-256 `a1a67c16bc45e495ee45eb2ada0d49dc5f1c5883168701917f1d9014151362f7`) used a symlink. Production `frontend/src-tauri/src/model_download.rs:926-930` rejects symlink model files before inference. The previous preflight missed this fixture incompatibility. The observed failure supersedes any assumption that the earlier fixture could execute successfully.

The preserved log `.worktrees/integration-backups-2026-10-02/feat102-local-inference-run1.log` (SHA-256 `b82aa645e436808e3ae17e05d3af524a39108392fd16c371dbfd408dbf73f39e`) records an integrity error after 1 ms and 0 passed / 1 failed. The coordinator reports exit 101; this reviewer independently read the failure log, not the originating process exit status. This provides no model-answer accuracy evidence.

`diff -u` of the preserved source against the current candidate confirms that the only source change is symlink creation replaced by `std::fs::copy` and its explanatory comment. The destination is in a fresh temporary directory. Copying gives production the regular file its integrity policy requires without bypassing the manifest/checksum validation or writing the source weights. Copy errors fail the fixture immediately. The factual oracle, production call path, cancellation handling and SQLite checks are unchanged. No open QUALITY finding remains for this bounded correction; execution of the amended revision still needs separate evidence.

## Scope and timeout control

The test is ignored by default and requires explicit local model/catalog/helper environment values. It creates a fresh temporary database, runs project migrations and inserts only synthetic fixtures. Lines 23-25 copy the supplied weights into a fresh temporary app-model directory as an independent regular file, leaving the source weights read-only, then load them through the existing production isolated client and its trusted catalog/manifest/integrity checks. No downloader, HTTP provider, real user-database path, installation or storage-format change is introduced.

Each generation has a 120-second deadline. Timeout cancels the same token and awaits the isolated generation future for up to eight seconds. Successful and ordinary-error completion traverse IsolatedSidecarGuard's awaited bounded shutdown. A timeout always returns an error and fails the test; it cannot produce an inference PASS. If the eight-second wait expires, dropping the future relies on the already reviewed guard/kill-on-drop fallback. The diagnostic cleanup_completed boolean does not establish final process reaping, and this review does not claim PID cleanup after timeout. Separate process observation would be needed for that assertion. Synthetic raw-output logging is scoped to these fixtures, not permission to log real user questions/transcripts.

## Checks and evidence

Environment: local Darwin workspace, read-only inspection apart from this report.

- Read the complete initial test and final factual-oracle correction, including synthetic data setup, three inference questions, empty-context guard, timeout/drain and SQLite reopen path.
- Re-read production isolated guard/shutdown control flow; retained prior independent review of model-integrity and generation paths.
- `shasum -a 256` and Python `hashlib` before/after: exact final test pin matches.
- Python comparison of all nine FEAT-101 candidate paths with round4 manifest: unchanged.
- Read the preserved first-run failure log and compared its preserved test source against the amended candidate; the sole delta is the temporary model-file copy described above.
- No reviewer compilation, test execution or inference performed. Cached-weight SHA/size and helper artifact provenance reported by the coordinator are not independently verified by this review. No second-run result is claimed here.

**NOT VERIFIED:** actual inference/accuracy, exact executed model/helper hashes and CPU behavior, timeout PID reaping, performance, installed native command/UI integration, network-provider behavior, generalized semantic faithfulness and release readiness.

`blockers: []` for this exact test's QUALITY preflight. `next_action`: bind the amended test/model/helper artifacts and obtain the next authorized local synthetic opt-in run; retain raw output, actual result and timing, and report any oracle mismatch or timeout without weakening the checks. FEAT-102 acceptance must be based on that subsequent evidence within its stated limits.


## Follow-up: cold verification cancellation gap (2026-10-03)

**Current broader execution assessment: QUALITY FAIL, Critical 0 / Important 1 / Minor 0.** This follow-up supersedes the earlier unconditional no-findings assessment of the production path exercised by this fixture. The copy correction and strict synthetic oracle remain valid; actual inference acceptance is still unproven.

**Important — cancellation is not observed during cold model integrity verification, and helper startup precedes the next check.** Candidate `frontend/src-tauri/src/summary/summary_engine/client.rs:214` awaits `verify_model_path_integrity` without passing or selecting against the request token. That function at line 122 calls `IntegrityCache::verify_file_cached`; `frontend/src-tauri/src/model_download.rs:233` calls `verify_file`, which at line 950 invokes its cancellable implementation with `None`. The loop hashes the complete model, regardless of a newly cancelled token. On completion, `client.rs:234` calls `ensure_running` before the next token check at line 238. Cancellation during cold hashing therefore continues disk/CPU work and can initiate helper startup after cancellation. The Q&A outer deadline (`meeting_questions.rs:426-431`) bounds eventual failure but does not make explicit user cancellation responsive during this phase. This is a confirmed control-flow defect, independent of whether it caused the observed run timeout.

Action: make the full integrity await interruptible by the request token while preserving complete SHA-256 validation and successful-verification-only cache admission; check cancellation before helper startup. Add a deterministic cancellation regression with a pending integrity phase and a startup spy, proving prompt cancellation and no helper startup. Do not weaken checksums or substitute a warmed cache as proof of cold-path cancellation.

Observed evidence: preserved run2 log `.worktrees/integration-backups-2026-10-02/feat102-local-inference-run2.log`, SHA-256 `57d32f2db7c2fafff67a0a0b12b108303de844023d46357b079718bdc861ba78`, reports `cleanup_completed=false`, `elapsed_ms=128033`, generation_timeout, and 0 passed / 1 failed. Exit 101 is coordinator-reported; this reviewer read the log. This does not establish whether a helper existed or remained alive.

The available run3 phase-log snapshot `.worktrees/integration-backups-2026-10-02/feat102-local-inference-run3-phases.log`, SHA-256 `c213eb64940c4a408052c2dcf0353ee0a7f28a0e2e8314b8f7b8ab52ade4e1ca`, ends at `Model: gemma4:e2b` and contains no `Spawning llama-helper sidecar` entry (`sidecar.rs:305`). It is consistent with delay before startup, including integrity hashing, but is not sufficient to establish the exact blocking operation or root cause. This was a live, incomplete log snapshot, not a completed run result.

Observed candidate test with test-only PhaseLogger: SHA-256 `19d78d66e275d7b96fd30fcd6a5f0e1dfdbf58867c15c7629d171f4c6dc2f2dd`. All nine FEAT-101 source/test paths still match the round4 manifest. Additional inspected unchanged integrity source SHA-256: `0c36db0e0acb64dec301e22dc1768cd8317f394abd60058a4e6a35432d0949c9`. Checks: read current source control flow and available logs; Python SHA comparison against manifest, all nine matched. No reviewer tests, inference, helper process observation or compilation. `changed_paths`: this report only. `blockers`: cold-phase cancellation defect and absent successful inference evidence. `next_action`: resolve and regression-test cancellation; separately diagnose timing with phase evidence before attributing run2's timeout to hashing.
