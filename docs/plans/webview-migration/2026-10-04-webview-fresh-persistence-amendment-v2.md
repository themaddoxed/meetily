# FORK-123: fresh-only API and durable publication amendment v2

Status: review candidate for two remaining source gates of
[contract v3](2026-10-04-webview-recovery-contract-v3.md). Implementation remains
NOT_ADMITTED. No change to product/user data, branding or authority follows.
This amendment adds `frontend/src-tauri/src/database/manager.rs` to the proposed
mutation list, subject to independent review and clean-source admission.

## Exact fresh-only seam

Existing manager.rs generic `new_already_gated` selects legacy import when the
native target is absent. `create_fresh` refuses existing paths; `open_existing_native`
also recovers summary/meeting lifecycle, reconciles exports and rebuilds search.
None is the proposed isolated fresh-resume API. Do not call the generic constructor
or normal reopen from a pending fresh admission.

Propose `initialize_owned_fresh_already_gated(OwnedFreshCandidate) -> SqlitePool`
inside manager.rs. The ownership value is constructed only by the native
coordinator holding the existing app-data gate; private fields bind operation,
canonical operation directory, candidate path and durable generation identity.
The target is an isolated candidate, not the current app-data live DB. No legacy
path or caller-supplied fallback is accepted. It must not return a DatabaseManager,
register a live target, manage AppState, emit readiness, recover meeting lifecycle,
reconcile summaries, request permissions or start services.

Extract only SQLx connection/migration and search-schema initialization from
existing fresh logic. For a new generation, native ownership is durable before
candidate creation; validate path components, unique regular files and absence of
foreign sidecars. SQLx creates the owned candidate. On resume, authenticate its
persisted owner/file identity and accept only empty owned initialization state or
an authentic supported SQLx prefix. An unsupported ledger, foreign tables or
identity conflict is pending failure; do not use destructive database recovery.
The exact empty/prefix classifier and file-ownership encoding must be defined
and tested before this seam is admitted, not assumed from normal reopen.

Extract a `write_fresh_defaults(pool, frozenDefaults) -> Result` seam from
commands.rs. Freeze recommended defaults in the first-launch intent before the
first write, including the selected transcription and summary configuration.
Errors propagate instead of being logged and ignored. Retry writes/checks only
the pending fresh candidate; imported data and terminal/user-edited defaults
never enter this function. Apply defaults transactionally and verify exact values.

Coordinator finishes migrations, verifies the DB and settings, checks SQLite
checkpoint result including busy/incomplete status, closes the pool and syncs
the candidate. Factor the lower copy/hash/no-clobber/readback mechanics into
an internal `publish_owned_files` seam, with two separately validated typed
requests: existing cross-identity migration authority and
`OwnedFreshPublication` from the first-launch coordinator. Legacy public
stage/publish/resume still call unchanged `validate_manifest`, including the
nonempty different-identity rule. The fresh request has only the actual running
target identity, admission operation/generation, validated candidate manifest
and target; it never invents a source application identity. Both request types
validate their own authority before reaching shared mechanics. Reuse existing
copy/verification and resume logic, not a parallel file-copy/WAL engine.

Freeze native-derived roots as disjoint siblings on the target filesystem:
`<target-parent>/.profile-first-launch/<operation>/candidate`,
`<target-parent>/.profile-first-launch/<operation>/staging`, and the actual
`target_app_data`. Candidate/staging must not be inside target app-data or overlap
one another. The owner root is checked for containment/symlinks/aliasing and
bound durably before writes; clients never choose these paths. Maintain existing
app-data gate and migration lock order with the fresh owner mapping inspected
before implementation. First-launch identity semantics do not relax root or
no-clobber checks. Destination creation remains no-clobber. A target already published by
the same pending owner is accepted only after current content/ownership readback;
an unrelated target conflicts. Maintain existing unique-link/live-pool checks;
interrupted owned temporary links must be reconciled before normal DB opening.
Sync target and parent, persist the terminal first-launch receipt, then restart.

Legacy choice uses the existing gated import/preservation engine, never this
fresh seam. Its candidate/source/WAL ownership remains authoritative. The
existing legacy import pool must be closed before terminal restart. Bootstrap
classification must not invoke `is_first_launch_already_gated` as a pure read:
that function calls legacy recovery. A read-only classifier reports the existing
legacy recovery state; actual legacy recovery runs only in the explicitly
admitted, isolated bound-resume transition.

## Atomic immutable documents and precedence

Freeze first-launch admission as an immutable intent plus a separate immutable
completion receipt; do not mutate a `pending/completed` field in place. Bootstrap
derives pending when valid intent lacks its matching receipt. Any unresolved,
corrupt or conflicting combined operation takes precedence over terminal fresh
admission. Terminal receipt authenticates ownership/choice/root and successful
initialization; on later normal starts it does not restore defaults or require
historical content hashes to match legitimate user edits.

Use the existing complete-write/sync/no-clobber hard-link discipline from
`migration.rs::persist_journal`, factoring a shared bounded publisher only if
needed. New terminal pathname is never opened for partial direct writing.
For each intent/ownership/receipt publication:

1. Hold the existing owner/gate/lock and validate parent/root/symlink constraints.
2. Allocate a same-directory create-new temporary under the owner and persist its
   generation binding before document mutation. Serialize complete validated
   versioned bytes; flush and sync the file.
3. Publish with a no-clobber hard link and sync the directory. Existing final
   document is accepted only when structurally valid and exactly bound to the
   expected value. A different owner/value or malformed final document conflicts.
4. Reply terminal only after directory synchronization succeeds. Response loss
   resumes by validating the final document and retrying the directory sync.
5. Retain/reconcile owned incomplete temporary generations without treating them
   as final state. Reuse a complete temporary only after full validation. Unknown
   files are never adopted or deleted. Recheck the budget, including retained
   generations; disk-full remains pending failure, not completed admission.

Keep legacy journal encoding and its established phase updates unchanged. The
new immutable first-launch/combined documents do not reinterpret legacy journals.
Root/package/file/browser binding and trusted-adapter readback remain as in v3.
Full canonical DTO field/serialization freeze remains a separate admission gate.

## Required focused proof

A same-identity owned fresh-publication regression must pass while an
equal-identity legacy migration journal remains rejected. Test overlapping and
aliased candidate/staging/target roots and retain the legacy negative tests.
Fresh choice with both target absent and legacy DB present must create fresh
data while preserving legacy bytes; it must not silently import. Resume authentic
partial SQLx prefixes/default transactions without normal lifecycle effects.
Reject wrong owner, aliased/linked/symlinked candidate, foreign SQL, invalid
ledger, unknown sidecar, live target and changed defaults. Verify strict failures,
pool close, checkpoint busy/error, retained bytes and retry after interruption.

Crash/reopen at intent/temp creation, every migration/default write, candidate
checkpoint/close, stage/publish step, receipt file sync, hard-link publication,
directory sync and response loss. Prove pending precedence, source preservation,
no-clobber destination, no torn terminal document and no default replay after
completed receipt plus legitimate user edits. Use real temporary SQLite/FS;
source review is not runtime proof. Run applicable project checks after authorized
implementation; plan review does not close FORK-123/native/release gates.
