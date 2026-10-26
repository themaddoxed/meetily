# FEAT-102 local real-model verification — 2026-10-03

Status: controlled real local model acceptance complete. Independent final SPEC and QUALITY passed with C0/I0/M0; FEAT-101/102 and source/integration milestone FEAT-078 are closed. Earlier failures are retained below.
Installed native UI, external providers and release readiness remain separate.

The existing Gemma 4 E2B GGUF is 3,106,736,256 bytes and SHA-256
9378bc471710229ef165709b62e34bfb62231420ddaf6d729e727305b5b8672d,
matching the trusted model_download manifest at revision 90f9618340396838ee7ff5b0ba2da27da62953d3.
No model was downloaded and no user settings were read or changed.

An ignored candidate test invokes the production isolated generation path on
synthetic selected/neighbor/empty meetings in a temporary migrated SQLite DB.
It copies the verified existing model into an independent regular temporary file. A faithful affirmative
answer must match complete normalized known claims about Friday and Mira;
keyword presence alone is insufficient. Unknown budget and neighboring secret
questions require explicit insufficient evidence. Every accepted citation must
belong to selected-plan and its application-generated selected-meeting link.
Unknown paraphrases fail closed rather than silently expanding the oracle.
The test also checks empty-context generator avoidance and temporary DB reopen.

The independent SPEC plan review passed. QUALITY found the keyword-only
oracle gap, now corrected; final test review and execution are pending.
Each generation has a 120-second deadline, cancellation, and an 8-second awaited
cleanup bound. A timeout always fails; PID cleanup after that bound is not proven
by the test and must not be claimed. This is neither installed-app/native UI
acceptance nor coverage of every local/network model.

Current CPU-only helper build and test compilation handles/logs are in the
[evidence census](evidence/meeting-questions-local-model-2026-10-03.json).
Re-poll those handles before starting another build; the test compile is waiting
on the helper's target lock. Preserve raw synthetic replies and final process
exits in durable logs. No network inference, credentials, model installation,
user DB mutation, commit, push, signing or release action is included.

Run 1 exited 101 before inference: the original temporary symlink was correctly
rejected by production model identity checks. The fixture now uses a regular-file
copy without changing or bypassing integrity validation. The first log and test
source are preserved under the ignored integration backups.

Run 2 exited 101: no answer before 120 seconds, followed by an eight-second
cleanup wait that did not complete (128,033 ms total). No faithfulness, unknown-
budget, neighbor-codeword or reopen acceptance is claimed. The failing durable
log is retained. A phase-only logger was added to the synthetic test to distinguish
cold model hash verification from helper startup/generation; it filters INFO
messages to summary_engine and changes no product behavior. The cold debug hash
hypothesis is not yet verified.

Run 3 also exited 101 (128,057 ms) with cancellation cleanup outside its
eight-second bound. Phase logging did not reach helper startup; a scoped process
read showed the test at 98.5% CPU with about 19 MB RSS and no helper row. Source
review confirmed that integrity-await is not cancellable and can be followed by
helper startup after cancellation. This confirms the cancellation defect; no
standalone hash benchmark or real generation timing is claimed.

A correction is frozen only in the candidate client: isolated Q&A selects
cancellation against integrity verification and checks again before helper
startup; the global summary path and full SHA verification remain unchanged.
Two unit regressions and retained real corruption/cache coverage are running.
Fresh independent SPEC/QUALITY and checks are required before root transfer.

The cold-cancellation correction passed six focused client tests, Clippy with
warnings denied (exit 0), formatting (exit 0), and fresh independent SPEC/QUALITY
C0/I0/M0. Only the reviewed client was copied into main after guarding its old
hash, with exact-byte readback. FEAT-101 remains open pending the ongoing model
verification and final closeout; no commit or publication was made.

## Real local run 4 — PASS

Cargo 1.98.1 compiled only sha2 with a CLI package optimization override
(`profile.dev.package.sha2.opt-level=3`), supported by the official
[Cargo configuration reference](https://doc.rust-lang.org/cargo/reference/config.html).
The complete SHA check, source, weights, prompt and 120-second deadline remained
unchanged; no manifest/lockfile/config file was edited.

The opt-in test exited 0. The model returned two affirmative claims, “Orion ships
on Friday.” and “Mira owns the launch.”, each citing the same exact selected
transcript excerpt. Unknown approved budget and neighbor-only private codeword
questions both returned `insufficient_evidence=true` with no claims. The full
strict parser and selected-source link checks accepted the responses. Empty
context did not call a generator, and the real temporary SQLite database reopened
with its original transcript preserved. The three requests took 95,808, 17,921
and 16,623 ms; the test completed in 130.62 seconds. This is one passing local
model fixture run, not evidence that every question/model is faithful.

The complete raw synthetic replies and phase log are retained at
`.worktrees/integration-backups-2026-10-02/feat102-local-inference-run4-optimized-sha.log`,
SHA-256 ed8657a9e19b4db31a949e510be3d520eb978616a1c71745d0d9b86adeb57049.
The successful test executable is pinned in the census. A scoped process snapshot
after its terminal exit found no test/helper process; this does not retrospectively
prove PID cleanup in the earlier timeout runs.

Network providers, installed native command/UI, arbitrary meeting questions,
other model/hardware combinations, packaged behavior and release gates remain
NOT_RUN. Real artifact smoke is a separate release requirement.

## Final bounded acceptance

The permanent [SPEC report and parent audit](../reviews/meeting-questions/meeting-questions-local-model-spec-review-2026-10-03.md)
and [QUALITY report](../reviews/meeting-questions/meeting-questions-local-model-quality-review-2026-10-03.md)
independently read the exact raw model replies and source. Both accept the
controlled local experiment with C0/I0/M0; neither claims its own inference run.
The exact opt-in test was copied to main after an absence guard and byte readback.
No new commit or publication was made. Source and test hashes, every failed run,
profile conditions and acceptance limits remain in the census.
