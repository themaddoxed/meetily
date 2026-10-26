# FORK-123 recovery contract v2 — independent QUALITY review

**Status: revision required; implementation NOT_ADMITTED. C0 / I1 / M0.** This verdict concerns the proposed design, not production/native test readiness. No product defect introduced by an implementation is claimed.

## Important finding

**I1 — skip has no defined durable startup transition.** `docs/superpowers/plans/2026-10-04-webview-recovery-contract-v2.md:18`, `:25–28` and `:104–106`: missing DB selects first_launch; skip checks for no pending operation and restarts; first_launch again creates the recovery window. The contract never specifies what durable state changes that classification after skip. Its intent/receipt schemas describe migration, not a skipped first launch. Following the specified steps literally loops back to the import/skip choice. This also leaves the authorized route to the existing fresh-database/onboarding flow undefined.

Current source confirms there is no existing skip marker to supply the missing transition: `frontend/src-tauri/src/database/manager.rs:353–364` ultimately detects absence of `meeting_minutes.sqlite`; `database/commands.rs:201–220` creates a database through the existing fresh initialization path, which the proposed normal-only command gate would withhold in recovery.

Resolve explicitly before implementation: either persist a versioned, owner/root-bound skip admission and add its startup-table row, allowing the existing normal onboarding/fresh-DB flow after restart, or define a narrowly authorized fresh-DB initialization transition before restart, with its side effects and crash semantics. The first option avoids silently granting the existing broad initialization command to recovery. Define precedence so any later pending/corrupt migration still blocks the skip admission. Add tests for restart after skip, response loss, a crash before/after its persistence, and later pending-operation discovery. This is a concrete state-machine gap, not a demand to execute a native app during design review.

## Other conclusions and bounded questions

Compatibility is explicit: legacy Completed keeps its old semantics; unfinished legacy operations remain failed/operator-owned; legacy browser markers conflict rather than masquerading as combined completion. Native intent binds both roots and package, and records use collision-preserving IDB `add`. The v2 preference policy now makes absent/equal admission, untouched omitted keys, and pending replay conflicts explicit. It does not promise a cross-store transaction or destructive rollback of successfully written keys.

The owned Preparing generations address partial staging without deleting unknown data and include retained bytes in budget. Existing `migration_commands.rs:126–143` refuses Preparing; an owned seam is necessary. The proposed bounded, hash-checked reuse is compatible with source preservation; the actual generation limit and serialization remain schema admission work.

Receipt readback now requires actual record identities/content and preferences rather than marker/count/boolean success, and expressly acknowledges the trusted-adapter boundary. Fresh file verification avoids relying on `migration_commands.rs:197`, whose Completed retry returns its existing journal. Post-receipt preference edits are preserved.

One publication detail must be frozen with the remaining schema/implementation admission: v2 lines 66–70 say create-new receipt plus file/directory sync, but should name atomic final publication and owned incomplete temporary handling. Directly creating/writing the terminal pathname could leave a truncated receipt on interruption. Existing `migration.rs:527–590` provides a complete-write/sync then no-clobber hard-link pattern; use the same discipline with the new owned-generation recovery rules. This is recorded as an unresolved implementation detail, not a confirmed requirement to write a partial terminal receipt. Do not blindly reuse its temporary-file cleanup semantics where v2 requires preservation.

Startup isolation now addresses automatic main-window creation, provider mounting, single-instance focus, sender authority and same-origin/store continuity. Exact plugin lifecycle and recovery CSP/IPC transport remain explicitly open at lines 130–140 and 173–176. These are legitimate pre-implementation inspection gates; this review does not certify a viable final configuration without that inspection. Also avoid treating `is_first_launch_already_gated` as a pure inspection helper: it calls `recover_import_already_gated` at manager.rs:358. The new classifier needs an explicit treatment of that existing legacy recovery behavior.

PreferenceSettings has the proposed Data Storage Locations section and recordings action (`frontend/src/components/PreferenceSettings.tsx:187–240`). A separate browser export control fits that location without conflating LegacyDatabaseImport's SQLite behavior. Identity/producer gating and unavailable controls are explicitly not task completion. No new identity or wider export allowlist is implied.

No additional documented-standard violation or maintainability smell was substantiated. Real preservation/reopen checks required by `docs/testing.md` remain future implementation verification, not prerequisites for this documentation review.

## Snapshot, checks, and limits

Reviewed v2 SHA-256 **`2d68a99daa247f18d3442b805df9278765f4733e8521bb54818d96518dca4660`** before and after review. Read the retained v1 QUALITY findings and relevant current bootstrap, manager/fresh-DB initialization, staging/journal, adapter/IDB, and export-entry sources. A Python hashlib check confirmed all eleven current source hashes still match the retained rebind census. No Git command, application, real profile access, browser/native test, or product mutation was performed in this review.

Additional inspected source SHA-256 values:

- `frontend/src-tauri/src/database/commands.rs`: `4cd381cbb06b09d57dba0115b38aff10ff6d4ea362c9441f8f516fd752a78f6a`.
- `frontend/src-tauri/src/database/manager.rs`: `10aaabeada6df54e414ad5c0489c615a9bc2f0819ce0b3829801bd915e804880`.
- `frontend/src/components/PreferenceSettings.tsx`: `b5193068ab468c076833fee8268f45b5d2cffda2c6e93a9169cbd31d769e0eaa`.

`changed_paths`: only `docs/fork/webview-recovery-contract-v2-quality-review-2026-10-04.md`. Earlier evidence/reports and other edits are preserved.

Next action: define the durable skip transition and its recovery tests; then settle exact schemas, atomic receipt publication, clean-source admission, and plugin/CSP transport gates before implementation. The actual identity pair and native store-continuity proof remain separately named gates. No full-plan, FORK-123 completion, migration, native or release PASS follows from this review.
