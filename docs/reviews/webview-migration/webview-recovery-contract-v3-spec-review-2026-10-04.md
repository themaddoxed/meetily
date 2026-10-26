# FORK-123 recovery v3 — bounded independent SPEC review

**The two v2 design findings are resolved. C0 / I0 / M0 in the reviewed delta. Implementation remains NOT_ADMITTED.** This is acceptance of the first-launch contract corrections, not a full execution-plan, code or native acceptance verdict.

## Snapshot and scope

Reviewed 2026-10-04. Candidate v3 SHA-256 before/after: `39b7bc3de8bb6f7e9a93207335fc8dcd33c67358be9a8ad9550fd1142a6b36f3`. Preserved v2 hash: `2d68a99daa247f18d3442b805df9278765f4733e8521bb54818d96518dca4660`. Prior finding report: `0e62c06e44845bb9fcf5491da25ea0af6dfdee2d9a2fdb97bc5010a6491bcf02`.

Reviewed the full v2→v3 textual delta and relevant existing first-launch initialization/import seams. Reviewer changed_paths: only this report. No other file, product, Git or profile change; no application launch.

## Closure of the two Important findings

**I1, skip/restart loop — closed at design level.** The new table gives pending FirstLaunchAdmission precedence over DB existence and admits normal only after completed admission with a validated DB. The new section persists the bound choice before mutation, creates owned fresh data through lower initialization/default-setting primitives, validates/checkpoints/closes/syncs it, then durably completes admission before restart. This supplies the missing durable state change. Pending restart resumes the same choice; changed choices, foreign DBs and unowned sidecars fail safely. It neither relies on frontend events nor manages normal AppState to break the loop.

Existing `database/commands.rs:201–269` confirms why extraction is appropriate: the current public command couples lower DB/default initialization to AppState registration and a database-initialized event. V3 explicitly separates those normal-startup effects. Its requirement to validate defaults before completion also matters because current default-setting failures are merely logged; the new completion seam must treat failed/default-mismatching readback as pending failure, as the candidate requires.

**I2, unreachable preserved SQLite path — closed at design level.** The candidate now gives first-launch its own discovery, selection and import wrappers, opaque checked source tokens and same-gate revalidation. It preserves file/folder detection and Homebrew/default candidates as visible choices, deliberately replacing background automatic import with user choice. Pending legacy admission allows only bound resume. LegacyDatabaseImport retains SQLite-only UI through those wrappers, while Onboarding starts after validated admission/restart instead of silently initializing/importing in the background. The explicit allowlist now includes database/commands.rs, OnboardingContext and LegacyDatabaseImport. This reconciles isolation with the existing user path rather than leaving an unchanged but unreachable component.

Existing `manager.rs:322–341` supplies an already-gated import path that invokes the existing recovery/preservation engine and opens the result. The new coordinator must close that pool before restart as specified; it need not duplicate copy/WAL logic. Completed first-launch admission is correctly a SQLite fact, not combined WebView completion.

## Other changed claim and remaining limits

Read the installed locked single-instance 2.3.7 macOS source: setup computes an identifier-based /tmp socket, can notify/exit a second instance, and creates/removes the socket. V3's bounded coordination caveat is accurate; it does not assert side-effect-free plugin startup. This source read does not admit the final plugin/CSP configuration.

The retained staging ownership, preference conflict, trusted browser readback, native receipt and identity-pair decisions were not broadened by this delta. Exact schemas, durable file publication implementation, gate/lock ordering, every admitted path on a clean checkout, plugin/CSP/transport configuration and the enabled production identity pair remain open as documented. Implementation must preserve the pending-state precedence throughout interrupted first-launch work, including native defaults and pool close/sync errors.

## Checks and next action

Reviewer checks: read-only diff and current source reads; SHA-256 binding before/after; inspect installed single-instance source. No tests, native restart, user-data migration or crash experiment ran. Current relevant source hashes:

- `frontend/src-tauri/src/database/commands.rs`: `4cd381cbb06b09d57dba0115b38aff10ff6d4ea362c9441f8f516fd752a78f6a`.
- `frontend/src-tauri/src/database/manager.rs`: `10aaabeada6df54e414ad5c0489c615a9bc2f0819ce0b3829801bd915e804880`.
- `frontend/src/contexts/OnboardingContext.tsx`: `a33f90576559b7f9044cecdccb81c60e49dbe6c1a706e12801788b5c7b7b4e69`.
- `frontend/src/components/DatabaseImport/LegacyDatabaseImport.tsx`: `221f2739329fa218cf027dc21f99f90fa3f132d6de067191b5d4f8c538fb72b5`.

No blocker from the two reviewed corrections. Continue the remaining explicit design/source admission gates and obtain the full exact execution-plan review before implementation. This report does not close FORK-123 or grant migration/native/release readiness.
