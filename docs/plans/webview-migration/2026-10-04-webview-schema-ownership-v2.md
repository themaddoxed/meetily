# FORK-123: proposed schema and first-ownership freeze v2

Status: bounded review candidate extending contract v3 and fresh/persistence
amendment v2. NOT_ADMITTED for product implementation. This freezes proposed
values; it does not modify existing file-journal or browser-package v1 encoding.
Clean source, producer/pair, startup/CSP and actual crash/user-flow gates remain. Preserve the original candidate and
its SPEC mixed-ID finding as historical evidence.

## First authority document: finite bootstrap

The first immutable authority document IS the intent; it does not require a
preceding generation-ownership document. Native holds the existing app-data gate
and migration lock, validates the actual request/pair/roots, creates a unique
same-filesystem operation directory with create-new semantics and retains the
verified input before writing intent. These preparatory metadata/package writes
are permitted in the isolated setup namespace, never to source/live DB/browser
records/preferences. No prepare/default/file publication/browser mutation runs
before intent and its directory have been synced successfully.

Publish the first intent using complete temporary write/sync, no-clobber final
link and directory sync. Its expected contents come from the validated native
request, not a filename. Later documents/generations use the durable intent as
their owner. This explicit first-document rule terminates the ownership recursion.
Existing intent is accepted only after full schema/binding/root verification
and directory resync; another value conflicts.

An interruption before a valid final intent leaves unstarted setup metadata,
NOT an admitted resumable migration. Bootstrap reports `failed: unbound_setup`
and preserves all bytes; it must not infer ownership/adopt an orphan from its
UUID or filename. No normal services start in this diagnostic state. The UI
explains that the operation did not reach admission and needs an explicit
operator resolution; do not claim successful recovery. This conservative
pre-intent boundary does not weaken the promised resume of valid post-intent
operations. A well-formed final intent after response loss can be validated and
directory-resynced before bound resume. Unknown/corrupt entries fail closed.

Later publication temps are registered in immutable generation entries bound
to this intent before candidate/temp data writes. A generation entry uses the
same owner-authenticated atomic publisher; it does not require another
generation entry for its own metadata temporary. Interrupted generation metadata
is preserved and diagnosed; it never authorizes an unrecorded data temporary.
Bound incomplete candidate/copy generations remain budgeted; no cleanup IPC.

## Common types and document shapes

New native JSON uses camelCase, version 1, UTF-8 and strict duplicate/unknown-key
rejection. Unknown schema versions fail before mutation. UUIDs are canonical
lowercase strings; SHA-256 values are 64 lowercase hexadecimal characters.
Filesystem sizes/device/inode/generation counters are decimal strings, not JS
numbers; reject noncanonical digits/signs and checked arithmetic overflow.
Paths are native-derived canonical paths; no arbitrary path IPC is added.

`RootBinding`: `{path, device, inode}` for an existing validated root; missing
target root is bound to its validated existing parent plus the exact basename
until native create-new establishes and persists the concrete root identity.
`FileBinding`: `{path, device, inode, byteLength, sha256}`. Directory identity
is not a file-content proof; every source/published file is rechecked separately.
Root changes/symlinks/aliasing conflict; a compatible path string alone is not
authority. Exact OS identity availability must be admitted for each platform.

`CombinedIntentV1`: `{version, kind:"combined", operationId, sourceIdentity,
targetIdentity, rootBindings, manifestSha256, packageFileBinding,
packagePayloadSha256, preferenceIntent, recordIntent}`. `rootBindings` names the
four existing source namespaces plus target and disjoint staging roots.
`manifestSha256` authenticates the immutable validated native manifest, excluding
mutable journal phase; its authority still requires the admitted different pair.
`packageFileBinding` pins retained raw bytes; `packagePayloadSha256` remains the
existing verified browser v1 payload checksum. They are different checksums.

`FirstLaunchIntentV1`: `{version, kind:"first_launch", operationId, targetIdentity,
targetBinding, choice, candidateRoot, stagingRoot, frozenDefaults, sourceBinding}`.
`choice` is `fresh` or `legacy`; fresh requires `sourceBinding:null` and frozen
summary/transcription config, legacy requires the bound selected source and
`frozenDefaults:null`. Never populate a synthetic source application identity.
Candidate/staging remain native-derived disjoint siblings outside target app-data.
The intent is immutable; completed state is derived from a matching receipt.

`PreferenceIntentEntry`: `{key, intendedValue, intendedSha256, expectedBefore}`.
`expectedBefore` is null or that same string value (absent-or-equal policy).
Reuse exactly the existing localStorage key/prefix allowlist; duplicate keys,
unallowlisted keys, a different pre-existing value or changed replay inputs
conflict before records commit. Keys absent from the package remain untouched.

`RecordIntentEntry`: `{store, packageIndex, sourceKey, contentSha256}`. `store`
is meetings/transcripts; packageIndex is a nonnegative safe integer within that
verified package array. Meeting sourceKey is its string meetingId. Transcript
sourceKey is its explicit safe integer ID, or null when IDB must assign it.
Repeated meeting/explicit transcript keys conflict; distinct transcripts without
IDs may share sequence/text and must remain distinct package entries.

`RecordBindingEntry`: `{store, packageIndex, targetKey, contentSha256}`. In the
same records-plus-marker IDB transaction, collect each `add` request's returned
key; persist the complete mapping only after all add successes, before transaction
completion. Queue explicit transcript IDs first, then ID-less transcripts,
retaining original packageIndex for every result; do not change package v1
array ordering or checksum. Otherwise a generated key can consume a later
explicit ID even in an empty target. Reject unsafe/out-of-policy generated keys
within the transaction. Queue the complete marker synchronously from the final
add success callback; an empty snapshot queues its marker immediately. Require
exactly one mapping per package store/index, proper key types and unique target
keys per store, with no missing or extra mapping. If any record or marker write
fails, the whole transaction aborts.
Do not start Web Crypto awaits inside an active transaction: compute intended
hashes before opening it. No transcript-index/sequence fallback is a primary key.

`CombinedMarkerV1`: `{version, operationId, sourceIdentity, targetIdentity,
packagePayloadSha256, preferenceIntent, recordBindings, state}`. States are
`records_committed_preferences_pending` and `preferences_applied`. Existing
unversioned markers remain legacy conflicts for this combined protocol. The
existing operation object store can hold discriminated versioned entries without
rewriting old entries or silently treating their presence as applied preferences.

`BrowserReadbackV1`: `{version, operationId, sourceIdentity, targetIdentity,
packagePayloadSha256, markerState, observedRecords, observedPreferences}`.
Read every mapped target key in one readonly IDB transaction along with the
marker; after transaction completion validate/project/hash all records. Each
observed record includes store/packageIndex/actualKey/contentSha256. Actual
key/content must match the retained package and marker mapping; counts or echoed
expected hashes do not suffice. Read every intended localStorage value separately;
no cross-store atomicity is claimed. Recovery is exclusive to ordinary writers.

For originally ID-less transcripts, exclude the destination-generated ID from
content projection but require its exact mapped targetKey. For explicit source
IDs, include/check the original ID. Preserve every existing defined transcript
field, including optional timing/partial fields, and every meeting field. Reject
unknown fields in the new combined import before mutation rather than silently
discarding them. Original legacy import behavior remains separate.

`CombinedReceiptV1`: `{version, kind:"combined", operationId, intentSha256,
manifestSha256, packagePayloadSha256, browserReadbackSha256}`.
`FirstLaunchReceiptV1`: `{version, kind:"first_launch", operationId,
intentSha256, choice, targetIdentity, initializedDatabaseSha256}`.
Native constructs receipts after bound verification, pool close/checkpoint/sync
where applicable, and atomic publication/directory sync. No client boolean
authorizes completion. The fresh DB checksum is initialization evidence, not a
requirement to rewind legitimate later user edits. Browser facts remain trusted
adapter reports; a hash is not independent native inspection of IDB.

## New canonical content encoding

Do not change the existing browser v1 payload checksum or localeCompare ordering.
New document/record/readback digests use a separate domain
`MEETILY-CANON-V1` followed by one zero byte and a typed recursive byte encoding:

- null: tag 0; false/true: tags 1/2;
- string: tag 3, unsigned 64-bit big-endian UTF-8 byte length, exact UTF-8 bytes;
- number: tag 4, finite IEEE-754 binary64 big-endian, normalize negative zero to
  positive zero; no NaN/infinity or unsafe numeric identity integers;
- array: tag 5, unsigned 64-bit big-endian count, each encoded element in order;
- object: tag 6, unsigned 64-bit big-endian field count, keys sorted by unsigned
  UTF-8 bytes, each encoded string key then value.

Reject unpaired UTF-16 surrogates in keys/strings before UTF-8 encoding; replacement
encoding would otherwise create collisions. Reject duplicate JSON object keys
before materializing parsed DTOs; ordinary JSON.parse alone is insufficient.
Omitted optional fields stay omitted; undefined values/array holes are invalid
canonical input. No Unicode/date/value normalization is introduced. Schemas fix
array order: preference entries by UTF-8 key; record intent/bindings/readback by
store then packageIndex; native manifest by namespace then target relative path.
Do not normalize existing package v1 ordering/checksum under this new algorithm.
Real IDB regressions must cover ID-less sequence 0 followed in the package by
explicit id 1, distinct returned keys, readback, generated-key bounds, empty
snapshot marker, and full atomic rollback on a genuine destination collision.
Cross-language golden fixtures must cover Unicode ordering, binary64 numbers,
negative zero, empty/null/omitted fields, generated transcript keys, duplicate
keys and unsupported fields. Rust and TS implementations must produce identical
bytes/digests; green tests for only one encoder do not establish interoperability.

## Admission and verification boundary

This proposes DTOs rather than implemented validation. Independently review exact
first-intent/crash precedence, record-key mapping and canonical encoding against
current adapter/IDB/native sources. Then freeze schemas/fixtures and owned paths
on the admitted clean implementation checkout. Preserve existing native manifest
validation and browser-package v1 verification tests. Real FS/SQLite/IDB storage
denial, quotas, close/reopen and crash steps remain required before task acceptance.
No native/user-profile migration, old-to-new upgrade or release PASS follows.
