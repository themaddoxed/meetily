# FORK-123 recovery contract v2 — independent SPEC design review

**Design revision required; implementation remains NOT_ADMITTED. C0 / I2 / M0.** The two findings concern missing/contradictory first-launch transitions in the proposed contract, not alleged production defects. No native or production success is required or inferred from this documentation review.

## Important findings

1. **Define the durable effect of skip before promising restart completion.** Candidate lines 18 and 25–27 classify missing DB as first_launch, open the isolated choice before DB creation, and define skip only as a no-pending check followed by restart. Under that state table, a restart with the same missing DB returns to first_launch. The normal provider that currently initializes the DB is explicitly withheld by lines 104–108. Specify a durable successful skip outcome: for example, create and validate a fresh DB using the existing lower already-gated primitives while withholding normal AppState/services, then restart; alternatively define a durable native admission disposition and the corresponding next-startup table row. Preserve no-clobber, the pending-operation recheck and failure recovery. The existing `initialize_fresh_database` command (`database/commands.rs:201` onward) both initializes and manages AppState/emits readiness, so invoking it unchanged inside isolated recovery is not automatically consistent with the new isolation contract. Add skip → terminate/reopen → normal, plus failure-before-durable-skip cases. Classification: partial transition contract with a concrete restart-loop consequence.

2. **Preserve a reachable SQLite-only first-launch path, not just its source file.** Candidate lines 147–148 require preserving LegacyDatabaseImport's SQLite-only flow, but lines 104–108 withhold normal providers, and lines 117–128 reject ordinary commands from recovery while admitting only the new combined-package/skip/export subset. Current `LegacyDatabaseImport.tsx:26,36,62,87` needs select_legacy_database_path, detect_legacy_database, import_and_initialize_database and initialize_fresh_database. The active `OnboardingContext.tsx:156–219` also owns first-launch legacy discovery/import/fresh initialization; withholding that provider removes its existing route. None of the proposed recovery commands defines a compatible SQLite-only choice. Define an explicit first-launch-only SQLite path with sender/state-gated native selection/validation/import wrappers delegating to existing preservation logic, no arbitrary path authority, no combined receipt claim, and no normal services before a validated DB/restart. Decide how the existing discovery behavior is retained or deliberately replaced; do not silently discard it. Reconcile the exact mutation list if another existing caller/command seam is needed. Classification: contradictory preservation and command-isolation requirements / behavior regression in the proposed design.

## Resolved v1 questions and soundness limits

The v2 compatibility table correctly leaves valid legacy Completed journals on their historical normal/first-launch semantics and refuses to manufacture combined completion from them. Pending new intent blocks startup despite file Completed; a terminal receipt does not remain pending merely because intent files are retained. Legacy marker conflicts are explicit rather than silently replaying historical preferences.

Absent-or-equal preferences, unchanged omitted keys, exact prior/intended replay values and collision refusal avoid silently overwriting target preferences. Records plus pending preference intent are committed in one IDB transaction; preference application/readback and marker advancement remain separate explicit crash steps. The durable native receipt binds a trusted adapter readback, not an independently native-verified browser fact. Post-receipt edits remain legitimate. This resolves the earlier marker-only ambiguity without claiming cross-store atomicity.

Preparing ownership entries precede temporary creation; verified staged bytes are reused, owned partial generations are retained and budgeted, unknown entries fail closed, and legacy Preparing is outside automatic recovery. This is a workable design direction. Exact canonical encodings, durable record publication details and the ownership/link/retirement crash tests remain admission work; they are not reported as implemented.

The actual-pair requirement and unavailable-control behavior are honest: fixture pairs do not enable production transfer and disabled controls do not complete FORK-123. The selected export slot and clarified path list follow source. Same-origin persistent store continuity is correctly assigned native proof rather than assumed from Chromium. Plugin/CSP/transport lifecycle and clean-source gates remain explicitly open; no additional speculative scope is requested here.

## Snapshot, checks and limits

Reviewed 2026-10-04. Candidate SHA-256 before/after: `2d68a99daa247f18d3442b805df9278765f4733e8521bb54818d96518dca4660`. Prior SPEC report read: `d293444fb79b516f466b7552db92db58de5f3c2d7d14ec78a4ee25269d3a0436`.

Reviewer actions: read candidate and prior observations, inspect current source callers and native first-launch commands, hash relevant sources. No application/browser launch, user-profile access, Git action, production mutation or cross-chat message. Review changed_paths: only this report. Source bindings:

- `frontend/src/contexts/OnboardingContext.tsx`: `a33f90576559b7f9044cecdccb81c60e49dbe6c1a706e12801788b5c7b7b4e69`.
- `frontend/src/components/DatabaseImport/LegacyDatabaseImport.tsx`: `221f2739329fa218cf027dc21f99f90fa3f132d6de067191b5d4f8c538fb72b5`.
- `frontend/src-tauri/src/database/commands.rs`: `4cd381cbb06b09d57dba0115b38aff10ff6d4ea362c9441f8f516fd752a78f6a`.
- `frontend/src-tauri/src/database/manager.rs`: `10aaabeada6df54e414ad5c0489c615a9bc2f0819ce0b3829801bd915e804880`.
- `frontend/src-tauri/src/database/migration.rs`: `f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c`.
- `frontend/src-tauri/src/database/migration_commands.rs`: `e50615fbdf21e5de3385436c951c83260a375eca75621847b72692bc65710fad`.
- `frontend/src/services/webMigration.ts`: `974740764867a367d695ab1166e52f0e2a47d5bb8bb3761ca0f90482adf04ebf`.
- `frontend/src/services/indexedDBService.ts`: `fea62c36fe851b7c29cbe8948fb545f838048d5f5ccdefb07af90371c273a6aa`.

No tests or crash/restart experiments were run in this design review. Next action: resolve I1/I2 in the state table and exact first-launch command/UI protocol, then review the revised contract. Retain NOT_ADMITTED until the separately identified clean checkout, identity-pair and plugin/CSP/source-inspection gates are satisfied. No task, migration, native or release closeout is granted.
