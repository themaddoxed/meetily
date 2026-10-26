# FORK-123 — WebView/profile migration integration review input

Source base: `main@53f4415058fb01e38b8892a47ac036bfffa394ba`.
This is an integration audit and proposed route, not implementation or
independent plan approval. FORK-123 remains open. [Source evidence](2026-10-02-webview-migration-census.json)
binds inspected paths to the audited base source hashes; no real user profile was read.
The later FEAT-100 command registration changes lib.rs in the working tree;
refresh this source census before implementation.

## Existing implementation retained

`1ae518a1c90e3175539ac03ab286b5519d7e28fe` is an ancestor of current main.
The registered commands are `prepare_profile_migration`,
`publish_profile_migration` and `get_profile_migration_status` in lib.rs.
The core already owns staging, file checksums, manifest validation, locks,
space budget, SQLite WAL preservation and the incomplete-journal startup guard.
Do not implement a second copy of those operations.

The WebView adapter creates/verifies deterministic packages and imports through
a real IndexedDB sink. Its marker binds operationId, sourceIdentity,
targetIdentity and payloadSha256. IndexedDB commits meetings/transcripts/marker
in one transaction; localStorage writes have best-effort rollback on a rejected
commit, not one cross-store atomic transaction. An operation marker alone does
not prove all browser preferences and file publication share one transaction.

## Confirmed source integration gaps

1. `database/setup.rs` refuses normal database initialization when migration is
   incomplete. `lib.rs` setup returns Err on that result before tray/services.
   Consequently the app does not finish setup to make recovery UI available.
   A UI button calling the registered publish command cannot solve that restart
   case by itself. Keep the refusal to open partial data; provide a distinct
   guarded recovery shell, not normal startup on an incomplete profile.
2. Rust journal phase `completed` follows file verification in
   migration_commands.rs. That path does not invoke or acknowledge the WebView
   sink. File completion must never be rendered as whole-profile completion.
3. The existing LegacyDatabaseImport dialog calls legacy SQLite import/fresh
   initialization, not the WebView adapter or the profile prepare/publish/status
   commands. The adapter export/import functions currently have no product UI
   callers. That dialog must retain its current SQLite-only behavior until the
   reviewed profile path is connected deliberately.
4. The manifest's source/target identity strings differ, but a producer still
   needs the admitted pair and the consumer must bind target to the actual app.
   Final naming is explicitly parked in FORK-028/155. Do not invent a final
   bundle ID, ask the already-deferred naming question again, or declare real
   old-to-new upgrade acceptance from two synthetic fixture identities.

## Proposed bounded route requiring independent review

- Export user-selected, allowlisted WebView data with the existing adapter and
  immutable package checksum/operation identity. Persist only an explicitly
  selected export artifact; never copy arbitrary internal WebKit directories.
  Do not export credential/sessionStorage data or silently include new keys.
- Before normal target initialization, admit a clean target and verify the
  package identity/checksum/schema against the actual configured target. Keep
  existing-database refusal, source-root relationship checks and no-clobber.
- Add an explicit recovery-only bootstrap mode for a valid pending operation;
  keep normal database/provider/model/audio/tray/permission work behind the
  existing readiness guard. Unrelated corruption remains a startup failure.
  The review must define its typed state, allowed commands and transition into
  normal startup so recovery cannot become a data-write bypass.
- Coordinate existing file prepare/publish/status and the existing WebView sink
  under the same exact identity. Store a durable combined completion receipt
  only after file readback and the matching WebView marker/preferences are
  verified. Review the receipt schema and crash order before editing the core;
  do not rename file-phase completed to mean browser import was performed.
- Resume from authoritative journal/receipt/marker after response loss or process
  restart. Reject changed payload/root/identity and IDB collisions without
  overwriting another operation. Preserve published data and source bytes.
- Keep user copy focused on exported data, validation, recovery and restart.
  Operation IDs, journal internals and implementation details are diagnostics,
  not a substitute for an understandable user flow.

## Scope to admit, not an authorization to mutate

Existing boundaries are lib.rs, database/setup.rs, migration_commands.rs,
webMigration.ts, indexedDBService.ts, LegacyDatabaseImport.tsx and app/layout.tsx.
A small dedicated migration coordinator/recovery boundary plus focused tests
will likely be required. Freeze its exact path/interface list after review;
do not silently admit a broad setup rewrite, replace existing migration core,
change capture/models, choose branding, or rewrite user data.

Before implementation, independent review must settle recovery bootstrap,
combined receipt/localStorage crash semantics, target identity admission and
exact scope. The latest operator instructions allow the available independent reviewer route;
no missing personal-orchestrator prerequisite remains.

## Required evidence for implementation acceptance

- Real temporary SQLite/FS and IndexedDB fixtures: file prepare/publish plus
  actual adapter/sink marker, then close/reopen/retry without duplicate data.
- Faults at prepare, each file publication phase, each browser preference write,
  IndexedDB transaction, receipt write, and before/after normal startup. Validate
  source preservation, collision refusal, changed identity/checksum, partial
  localStorage restoration, cancellation and two concurrent operations.
- Browser user flow with first-run/recovery/import/export states and exact IPC
  arguments; locked/corrupt/active target must not expose normal product work.
- Required checks and an independently reviewed exact diff. Final bundle-ID
  installed-app upgrade remains separate FORK-124/125 and FORK-028 evidence;
  browser fixtures cannot close native or release acceptance.

No implementation, app launch, install, real profile access, TCC change,
commit, push or publication is part of this audit. No new acceptance PASS is
claimed from the source census.
