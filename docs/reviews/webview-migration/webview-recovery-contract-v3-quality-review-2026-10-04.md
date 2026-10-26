# FORK-123 recovery contract v3 — bounded QUALITY review

**Status: the reviewed v2 corrections are acceptable at design level; C0 / I0 / M0 new actionable findings. Implementation remains NOT_ADMITTED.** This is not full-plan acceptance or a claim that the proposed transitions have been implemented or tested.

## Resolved findings

The skip/restart loop is resolved by `FirstLaunchAdmissionV1`, persisted before DB mutation, a pending classification that outranks DB existence, and a completed transition only after validation, checkpoint/close and durable storage (`2026-10-04-webview-recovery-contract-v3.md:17–20`, `:48–63`). This supplies the durable fact missing in v2. An interrupted owned DB remains isolated rather than becoming normal because its file exists. A changed choice or source conflicts; ordinary services remain withheld. The document explicitly requires interruption/reopen tests, not an event-based readiness shortcut.

The first-launch SQLite path is now reachable by explicit discovery/selection/import wrappers, bound native tokens and the retained browse/detect/import/error UI (`:38–45`, `:65–74`). The proposal retains Homebrew and app-data discovery while replacing current background automatic import with an explicit choice. Pending legacy admission can only resume its bound operation. This is compatible with the stated source-preservation boundary and avoids reopening arbitrary path commands to a recovery sender.

The allowlist now includes `database/commands.rs`, `OnboardingContext.tsx` and `LegacyDatabaseImport.tsx`; these correspond to actual initialization and import callers. Current `commands.rs:165–197` and `:201–272` manage AppState and emit database-ready; v3 explicitly requires factoring below those effects. Current OnboardingContext starts background discovery/initialization; v3 explicitly removes that behavior before admission. A completed first-launch admission remains a SQLite fact, not a combined WebView receipt.

## Implementation gates and source-specific cautions

These points must be resolved in the remaining source/schema admission; they do not invalidate the bounded state-transition correction:

- **Choose the exact fresh-creation seam before implementation.** `database/manager.rs:264–287` is not a fresh-only primitive: its generic constructor automatically imports an app-data legacy DB when the target is absent. Its actual fresh initializer is private (`create_fresh`), while existing-target opening requires a recognized native schema. Therefore simply extracting the current commands.rs call would not enforce the v3 fresh choice or resume a partially initialized owned DB. Freeze an explicit fresh-only/owned-resume API and its tests. If reusing the private initializer requires changing manager.rs, admit that path through the plan's additional-file review gate before editing; it is not currently in the allowlist. Do not silently reuse the legacy auto-import branch or duplicate the migration engine.
- **Freeze publication and ownership encoding.** New admission pending/completed persistence needs complete-write/sync plus atomic no-clobber publication, owned temporary recovery, and deterministic precedence over all unfinished/corrupt combined operations. A torn terminal pathname must not be confused with completed admission. Apply the same still-open publication decision to combined receipts; direct create-new writing is not itself an atomic-publication proof. The prior receipt caution remains open, not newly demonstrated as an implementation defect.
- **Defaults and retries must be strict.** Existing fresh commands log and continue when saving defaults fails. V3 requires complete validation and pending isolation on error, so extracted primitives must return failures. Repeating defaults is limited to the still-pending fresh owner, not imported preferences or later normal user settings. Terminal admission validation must tolerate legitimate later user edits rather than reset historical defaults.
- **Pending legacy resume must reuse existing ownership/journal recovery.** Close its pool before restart and retain source/WAL preservation. Do not retry a different source merely because the desired target exists. Current manager/import helpers already enforce app-data gates, path shapes and live-pool restrictions; retain those checks.

The plugin/CSP transport configuration, exact canonical schemas, clean source/QA-overlay admission and actual identity-pair gate remain explicitly open. Same-origin native store continuity and actual crash recovery are future verification gates. The narrowed macOS single-instance statement correctly describes socket coordination as a side effect, rather than claiming an effect-free plugin bootstrap.

## Observed snapshot and checks

- v3 before/after SHA-256: `39b7bc3de8bb6f7e9a93207335fc8dcd33c67358be9a8ad9550fd1142a6b36f3`.
- Unchanged v2 SHA-256: `2d68a99daa247f18d3442b805df9278765f4733e8521bb54818d96518dca4660`.
- Read the exact `diff -u` between v2 and v3 and the prior v2 QUALITY finding. Inspected current native fresh/import entrypoints and manager helpers, Onboarding/LegacyDatabaseImport callers, and local macOS single-instance source. Python hashlib confirmed all eleven current source hashes still match the retained rebind census. No external dependency API assumption was needed to establish the transition findings.
- Additional current sources: commands.rs `4cd381cbb06b09d57dba0115b38aff10ff6d4ea362c9441f8f516fd752a78f6a`; manager.rs `10aaabeada6df54e414ad5c0489c615a9bc2f0819ce0b3829801bd915e804880`.

No production, native, browser, real-profile or crash-injection tests were run; none are claimed as passed. No Git operation or product mutation occurred. `changed_paths`: only this report. Prior reports and other edits are preserved.

Next action: settle and review the exact fresh-only seam, durable admission/receipt schemas and remaining plugin/CSP/source gates before implementing the bounded contract. Do not close FORK-123 or infer native, migration or release readiness from this design review.
