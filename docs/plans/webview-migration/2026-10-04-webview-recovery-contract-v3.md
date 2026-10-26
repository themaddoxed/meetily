# FORK-123 combined profile recovery contract v3

Status: concrete design candidate; NOT_ADMITTED for implementation. Supersedes
[v2](2026-10-04-webview-recovery-contract-v2.md) after its SPEC findings, retaining
its historical evidence. No product change or real-profile action is authorized
by this document. Source is base53 plus the exact QA overlay; clean dev admission
awaits the separately requested QA commit/transfer. Preserve QA registrations.

## Compatibility and startup classification

Inspection holds the existing app-data gate and uses existing migration lock
order; classification must not initialize the DB or normal services. It returns
one tagged value, never an error-string match:

| On-disk state | Bootstrap result |
| --- | --- |
| No operation and existing valid DB | normal |
| Pending first-launch admission, even with a newly created DB | isolated first_launch recovery |
| Completed first-launch admission with validated DB | normal |
| No operation and missing DB | first_launch |
| Valid legacy Completed, no combined intent | existing normal/first-launch semantics |
| Legacy unfinished, no combined intent | failed: legacy_recovery_requires_operator |
| Valid combined intent, no matching durable receipt, including no journal or file Completed | recovery_required |
| Matching terminal receipt and valid intent | normal/first-launch without replaying preferences |
| Corrupt, unsafe, foreign, conflicting or multiple unresolved owners | failed |

`first_launch` opens the isolated import/skip choice before DB creation. Choosing
skip is an explicit native transition requiring the same no-pending-operation
check, durable owned fresh-DB admission described below, then restart; no event
grants readiness. Pending combined recovery
cannot be skipped. A failed state exposes diagnostics without ordinary services.
Retained terminal intents are not pending merely because their files exist.
Legacy markers remain legacy; encountering one for a new pending combined
operation returns `legacy_marker_conflict` without rewriting records/preferences.

## First-launch fresh and SQLite admission

Preserve the existing SQLite-only choice with first-launch-only commands
`discover_legacy_profile_sources`, `select_legacy_profile_source`,
`import_legacy_profile_source`. Discovery retains the existing Homebrew
`/usr/local/var/meetily/meeting_minutes.db` and app-data legacy location checks,
returns validated opaque tokens plus display metadata, and offers import rather
than starting it before the user chooses. Manual selection retains supported
file/folder detection. Tokens bind actual checked native paths/identity and are
revalidated under the app-data gate; arbitrary payload paths grant no access.
No selected source is opened by the agent during design work.

Fresh/legacy choice persists and directory-syncs `FirstLaunchAdmissionV1`
(version, operationId, kind fresh/legacy, canonical target, optional bound source,
pending/completed state) before DB mutation. Only one admission/combined owner
may exist. Pending admission outranks DB-exists classification on restart; it
resumes its bound choice, never treats a partial DB as normal. A changed choice
or input conflicts. Legacy import delegates to the existing already-gated
preservation/import journal and verification; no second copy/WAL engine. Fresh
creation uses the existing lower initialization/default-setting primitives
extracted from commands without managing AppState or emitting database-ready.
Create-new target ownership must precede opening it; an unowned existing DB is
never initialized or overwritten. Validate the complete DB/schema/defaults,
checkpoint and close its pool, sync the owned DB and directory, then durably mark
admission completed before restart. A partial owned fresh DB can rerun only
idempotent initialization/defaults under the same pending owner; unowned sidecars
or invalid content fail safely. All errors preserve bytes and isolated state.
No normal AppState/provider/model/notification service is managed in this path.

Keep LegacyDatabaseImport SQLite-only by routing its commands through these
scoped wrappers in first-launch mode, preserving browse/detect/import/error UI.
Allow scoped legacy selection and discovery only in first_launch without pending
combined recovery; a pending legacy admission allows bound resume only. Normal
legacy behavior remains separately guarded. Onboarding no longer imports or
creates a DB in the background; it starts only after validated admission/restart.
Homebrew/default discovery remains visible in the isolated choice. Tests cover
fresh skip and legacy import, interruption at every admission/DB/durable-finish
boundary, retry and terminate/reopen to normal without looping or duplicate data.
A completed first-launch admission is a SQLite fact, not WebView completion.

## Values and trusted boundaries

The following are proposed new schemas, not claims about existing APIs. UUIDs
use the existing operation validation; hashes are validated lowercase SHA-256.
Persisted documents reject unsupported versions and unknown authority fields.

`CombinedIntentV1` contains `version=1`, `operationId`, `sourceIdentity`,
`targetIdentity`, `rootBindings`, `manifestSha256`, `packageSha256`,
`preferenceIntent`, and `stagingOwnership`. Root bindings are canonical validated
native roots; clients never replace them on retry. Preference intent contains
only package allowlisted keys with exact value/hash and observed target value
(absent or equal). Neither secrets nor sessionStorage are added to the package.
The retained verified package and file manifest are synced before the intent;
their directory entries are synced before any prepare or browser mutation.
Orphan retained files without a valid intent are preserved, diagnosed and never
adopted from a new request. Intent/receipt mismatches fail closed.

`CombinedMarkerV1` contains the same operation/source/target/package binding,
the exact preference intent, record identities plus canonical content hashes,
and state `records_committed_preferences_pending` or `preferences_applied`.
Records and the pending marker commit in one existing IDB transaction using
`add`; pre-existing destination records are conflicts, never overwritten.
Canonical record encoding must be shared with package checksum construction,
versioned and fixture-tested before implementation; counts alone are inadequate.

`BrowserReadbackV1` contains `version=1`, all operation bindings, observed marker
state, recomputed record identities/content hashes and observed intended
preference values. The adapter enumerates and hashes actual stored records,
validates the full marker and rereads every intended preference. It never
substitutes echoed package hashes for readback. Rust validates sender and active
intent, compares this report and independently re-verifies every published file.
Browser facts remain a trusted-adapter report; Rust does not inspect IDB.

`CombinedReceiptV1` binds version, operation, roots, identities, file manifest,
package and accepted browser report hash. Write create-new under the same owner,
sync the receipt and its directory before replying complete. An identical retry
returns the terminal receipt; a different report/binding conflicts. Receipt
durability with response loss is terminal on restart. Post-receipt user edits
are legitimate; normal startup never restores historical preferences.

## Preference policy and resumable stages

Admission is absent-or-equal only. Before committing any records, read every
allowlisted intended preference and reject a different existing value. Package
keys omitted from the intent stay untouched. Replay while pending accepts only
the recorded prior value or the intended value; a newly differing value is a
conflict. Apply intended values, read them back, then advance the marker in IDB.
An interrupted write repeats only that same intent. Storage denial/quota/conflict
keeps recovery pending and preserves records, preferences and source bytes.

Reuse existing manifest/root/copy/hash/budget/publication helpers. Add an owned
Preparing resume seam, rather than retrying the current empty-staging API.
Before each copy, persist and directory-sync an ownership entry binding the
operation, file and generated temporary path. Verified staged final paths are
reused after complete hash/length validation. Missing entries are copied with
create-new semantics. A partial temporary file with a durable matching owner is
retained outside the publication manifest; persist a new bounded generation
before retry. Budget includes all retained generations plus the remaining copy
and publication requirement. Insufficient disk space is a pending blocked
diagnostic, not a successful resume. Unknown entries, symlinks, conflicting final
hashes and unowned temporary files stop without deletion/adoption. No cleanup IPC.

The new copy seam must cover the gap between ownership persistence and temporary
creation and between hard-link publication and temporary retirement. Retained
complete temporaries may be reused only after hash verification and an owned
create-new final link. Never infer ownership from a filename. Legacy interrupted
staging remains outside this automatic resume path. Existing Publishing resume,
WAL checks, verification and lock semantics remain authoritative.

## Window, services and command isolation

Remove configured automatic main-window creation. Build exactly one window from
typed bootstrap: normal `main`, or `profile-recovery` for first_launch/recovery/
failed. Main cannot exist until normal admission. Recovery serves a dedicated
static route without normal providers, recording, model, notifications or tray
initialization. The normal layout/provider branch must be selected from queried
native state before mounting. The single-instance callback focuses the current
admitted window; it never creates or focuses an absent main window.

Recovery and normal windows use the same target identity, persistent WebView
store and origin, with no incognito or per-label data-directory override. A real
native restart/readback must prove store continuity; Chromium proves adapter
semantics only. No synthetic event or frontend route can change native state.

New scoped application commands: `get_profile_bootstrap`,
`select_profile_package`, `begin_combined_profile_migration`,
`resume_combined_profile_migration`, `finish_combined_profile_migration`,
`skip_profile_import`, `restart_after_profile_admission`, and
`save_web_profile_export`. Native file selection/save owns paths and returns an
opaque retained-package token; no caller-supplied target paths. Begin validates
the admitted identity pair and binds roots/intent. Resume accepts operationId
only. Finish accepts the versioned browser report, never a readiness boolean.
First-launch discovery/selection/import wrappers are also admitted only in the
state above. All other ordinary custom commands reject a recovery sender/state
before any effect;
the existing low-level migration commands become normal-only or internal to the
coordinator. Recovery allows only its exact command subset; save export is
normal-only. Sender checks use the actual invoking window, never payload labels.

Recovery capability grants no fs/store/notification/model or general window/app
permissions. New commands are native-gated. Restart is the scoped native command,
not generic frontend process permission. Plugin dispatch bypasses the application
handler in locked Tauri 2.11.1; inspect plugin initialize/setup hooks before
admitting builder changes. Register side-effectful notification/store plugins
only in the normal branch; bounded native dialog selection must not grant
frontend dialog/file capabilities. Single-instance may remain only after its
startup effects are inspected. Locked macOS single-instance setup creates/removes
an identifier-bound /tmp socket and may exit a second instance; this is bounded
coordination, not a no-side-effect bootstrap promise. A dedicated recovery CSP must suppress ordinary
network/asset egress while retaining the actual required Tauri IPC transport.
Exact transport/CSP/plugin lifecycle configuration remains a source-inspection
gate: this document does not pretend capability names alone prove isolation.

## Export, identity and implementation scope

Add the separate browser export control to PreferenceSettings Data Storage
Locations, retaining the recordings folder action. Settings already mounts this
component. Explain that the package is the allowlisted browser portion; native
files retain their existing manifest path. Preserve LegacyDatabaseImport's
SQLite-only flow. Selection cancellation is effect-free. Export reads source
data; the save operation creates the selected package without changing it.

Current target configuration is `com.meetily.ai`. Source identity must come from
the actual exporting producer; target from actual config. An explicitly admitted
pair, compatible producer and checked package protocol are required to enable
production begin/export. Until then controls show unavailable; no fabricated
importable package or default fixture identity is emitted. Generic protocol
tests can use explicit fixtures; FORK-123 is not complete with disabled controls.
Naming remains the parked owner decision. Real installed old-to-new admission
and acceptance remain FORK-124/125/160.

Mutation allowlist, relative to frontend: existing src-tauri/src/lib.rs,
src-tauri/src/database/{setup,commands,migration_commands,migration,mod}.rs,
src-tauri/tauri.conf.json, src/services/{webMigration,indexedDBService}.ts,
src/app/layout.tsx, src/contexts/OnboardingContext.tsx,
src/components/DatabaseImport/LegacyDatabaseImport.tsx,
src/components/PreferenceSettings.tsx, src/lib/i18n/catalog.ts,
tests/lib/web-migration.test.ts. Create src-tauri/src/database/profile_migration.rs,
src/services/profileMigration.ts, src/components/DatabaseImport/
{ProfileMigrationGate,ProfileMigrationRecovery}.tsx, src/app/profile-recovery/
page.tsx, src-tauri/tests/profile_migration.rs, tests/profile-migration.spec.ts.
Normalize this list into individual source hashes on the admitted clean checkout
before editing; any additional lifecycle/config/route file requires review.

## Execution admission and proof

1. Independently review v3 against current sources; settle exact canonical
   schemas, plugin/CSP transport and startup construction from locked dependency
   source. Admit clean dev plus preserved QA; freeze every owned path and base.
   Actual identity pair/producer remains an enabled-production gate.
2. RED/GREEN temporary FS/SQLite tests cover the startup table, lock ordering,
   changed inputs/roots, source preservation, orphan/conflict/unsafe files,
   Preparing ownership generations and existing Publishing resume. Inject close/
   reopen before and after every package/intent/ownership/copy/journal/fsync step.
3. Real browser IDB/localStorage tests cover every records/preference/marker/
   readback step, collisions, quota/denial, response loss and retry without
   duplicates. Export/import/recovery are visible user paths; retain SQLite import.
4. Assert no normal providers/services/permissions before admission; forged IPC,
   plugins, wrong sender, ready events and network/asset access fail. Check exact
   native store continuity and exactly-once restart on the authorized test app.
5. Run applicable project checks and independently review the exact diff. Native,
   installed upgrade, migration, signing, notarization and release status remain
   separate measured gates. Do not close FORK-123 from plan acceptance or fixtures.
