# FORK-123: recovery and combined completion contract — review candidate

Status: design input, implementation NOT_ADMITTED. This refines the existing
2026-10-02 integration audit; it does not close FORK-123 or permit access to real
profiles. Current source observations are frozen in
[the rebind evidence](../../fork/evidence/webview-migration-rebind-2026-10-04.json).
The task remains the user-selected WebView export/import connected to existing
Rust prepare/publish/status, including restart recovery and source preservation.

## Source and authority

Base remains `53f4415058fb01e38b8892a47ac036bfffa394ba`. The canonical root has
preserved QA changes in lib.rs; the other ten entries in the eleven-path census equal base.
No clean product source has been admitted. Freeze the actual implementation
checkout and compare its source before editing; preserve QA registrations when
integrating later. Do not commit, launch an app, select final branding, read a
real profile, or publish under this design review.

Locked Tauri is 2.11.1. Its verified crate dispatches plugin commands separately
from application invoke_handler. A custom-command guard alone cannot isolate a
recovery window. Current inline main capability grants broad fs/store access.

The actual adapter diagnostic returned already_imported with zero preference
writes and a preference different from the verified package. This is a bounded
Node VM probe with fake Storage/IDB, not a browser or native acceptance result.
It establishes that the marker alone cannot prove an unfinished combined
operation completed its preferences step. It does not require restoring old
preferences after a successfully completed migration and subsequent user edits.

## Candidate decisions for independent review

1. Introduce a typed bootstrap snapshot: `normal`, `first_launch`,
   `recovery_required`, or `failed`, with a bounded pending-operation identity
   only for recovery. Inspection is read-only and holds the existing app-data
   gate; malformed, symlinked, unrelated corrupt, mismatched-root or conflicting
   operations never become normal startup. Do not parse error strings to decide
   readiness. Preserve existing legacy file-only journals and their semantics.
   New combined operations have a separately versioned intent before prepare.
   Presence of a pending combined intent blocks normal startup even when its
   legacy file journal reports completed.
2. The recovery window has a distinct label and explicit minimal capability.
   Normal main window/providers, tray, model initialization, recording, normal
   DB management and permission requests are withheld. Gate every application
   command by authoritative bootstrap state and sender window; plugin access
   uses the separate recovery capability, with no fs/store/notification or
   model/network permission inheritance. Selected package IO goes through
   scoped Rust commands; no arbitrary target paths or cleanup IPC. On complete
   readback, request a normal app restart rather than initializing services
   piecemeal. Unknown IPC, forged ready events and frontend route changes cannot
   grant readiness. Inspect plugin initialization side effects before selecting
   the final configuration and builder changes.
3. A native combined-operation intent owns operationId, source/actual-target
   identities, checked root bindings, canonical file-manifest hash, web-package
   hash and retained verified package. Persist and sync it before any prepare or
   browser write; bind retries to it, not to newly supplied roots/preferences.
   Changed inputs conflict without side effects. Keep the existing migration
   lock, budget, stage/publish/checksum and WAL logic as authority; add only the
   required typed coordinator seams. Preparing-phase recovery needs an explicit
   resumable staging contract: the present prepare retry refuses Preparing.
   Reconcile exact valid staged bytes under the same owner or stop safely; do
   not delete partial work or call a failed prepare complete.
4. File completed remains a file fact. For a pending combined operation, import
   records and a versioned marker containing a preference intent in the existing
   single IDB transaction. Add records exclusively as today. Then apply the
   allowlisted preference intent, verify each intended value, and mark its step
   applied in IDB. A crash after records commits leaves a replayable pending
   preference step. A crash during preferences repeats the same bounded intent
   under the migration-exclusive UI, not normal product writers. A matching
   records marker cannot short-circuit that step. No claim of atomicity across
   localStorage, IDB and Rust FS is made. Package validation and collision checks
   precede writes. Define how pre-existing target preferences and absent package
   keys are treated before implementation; do not silently enlarge the key set.
5. Require explicit browser readback of marker identity, records import and
   applied preference intent while the combined operation is pending. Rust
   checks the matching native intent and fresh file verification, then writes
   a durable combined receipt under the same owner. The protocol must state
   that browser facts are reported by the trusted adapter; Rust does not read
   IndexedDB itself. Do not accept a client boolean, marker alone or file phase
   alone as whole-profile completion. On response loss before receipt, resume
   the pending protocol. On receipt durability and restart, allow normal startup
   without reapplying old preference values to later user changes. Review the
   IDB marker/receipt handshake and crash boundary before admitting this seam.
6. Read the actual configured target identifier; no final product name or bundle
   ID is invented. User-selected export contains only existing allowlisted
   browser data, never credentials/sessionStorage or internal WebKit folders.
   The producer/consumer identity-pair admission remains required. Fixture pairs
   are test inputs only; installed old-to-new identity acceptance remains
   FORK-124/125/160 and the parked owner identity decision.

## Proposed files, still subject to admission

Modify: src-tauri/src/lib.rs, database/setup.rs, database/migration_commands.rs,
database/migration.rs, database/mod.rs, tauri.conf.json;
src/services/webMigration.ts, indexedDBService.ts, src/app/layout.tsx;
tests/lib/web-migration.test.ts. All are relative to frontend/.

Create bounded coordinator/bootstrap seams in
src-tauri/src/database/profile_migration.rs; src/services/profileMigration.ts;
src/components/DatabaseImport/ProfileMigrationGate.tsx and
ProfileMigrationRecovery.tsx; Rust test tests/profile_migration.rs and browser
test tests/profile-migration.spec.ts. A normal user export entry point requires
an explicit Settings path added after source review; its selection is a design
stop, not permission to edit any Settings component. LegacyDatabaseImport
retains its SQLite-only behavior unless a reviewed, separate profile control is
explicitly introduced. Keep unrelated identity/audio/models/storage changes out.

## Required proof and remaining design stops

- Freeze exact paths/API/value schemas; settle target preference collisions,
  existing file-only journal compatibility, Preparing resume, producer identity
  admission, export entry point, recovery capability/plugin initialization and
  marker/readback/receipt protocol with fresh independent SPEC/QUALITY.
- RED/GREEN tests must cover valid pending/file-completed-without-receipt startup,
  unrelated corrupt/unsafe/multi-owner journals, no service/provider/permission
  effects, direct custom IPC and plugin denial, wrong sender window, fake ready
  event, and exactly-once normal restart transition.
- Use real temporary SQLite/FS plus real browser IDB/localStorage: close/reopen
  at intent persistence, each staging/publication phase, IDB records commit,
  each preference write, preference readback/applied marker, receipt write/sync,
  response loss and restart. Assert source bytes unchanged, no duplicate rows,
  no normal UI during partial work, exact ownership, changed package/root/ID,
  collisions, quota/full-disk, denied storage, cancelled selection and concurrent
  requests. Simulated adapter probes cannot substitute for these tests.
- Browser acceptance includes normal export and target import/recovery user
  paths; old SQLite import remains covered. Run applicable data-integrity and
  migration Rust tests, targeted frontend/browser checks, then the mandatory
  project gate on the exact admitted diff. Review that exact diff independently.
- No data migration, clean install, native/installed upgrade, signing,
  notarization or release PASS is inferred from plan acceptance or tests.
