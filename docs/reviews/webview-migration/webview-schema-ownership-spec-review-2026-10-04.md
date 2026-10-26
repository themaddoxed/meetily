# FORK-123 schema/ownership — independent bounded SPEC review

Status: CHANGES_REQUIRED for the proposed schema/ownership freeze; C0 / I1 / M0. Product implementation remains NOT_ADMITTED. This reviews the new document against contract v3, persistence amendment v2 and current source, not a completed implementation.

## Important finding

**I1 — Specify collision-free insertion scheduling for mixed explicit/generated transcript keys.** `docs/superpowers/plans/2026-10-04-webview-schema-ownership.md:83–95` admits both explicit source IDs and ID-less records and requires collecting each `add` key, but never specifies how to avoid allocating a key reserved by another record in the same package. Classification: missing contract detail with a concrete accepted-input failure.

Current `frontend/src/services/indexedDBService.ts:192–193` defines inline `id` with `autoIncrement: true`; its import loop at lines 774–777 queues transcripts in package order. `frontend/src/services/webMigration.ts:190–200` sorts first by meeting and sequence, not explicit-ID presence. Thus a valid package can contain an ID-less transcript at sequence 0 followed by explicit `id: 1` at sequence 1. On a new store the first add receives 1; the second add fails and aborts the whole transaction although neither package keys nor pre-existing destination records conflict. Retrying unchanged input repeats the failure. This is a design omission, not a claim that the proposed implementation already exists.

Resolution: freeze explicit-ID inserts before generated-ID inserts in the same transaction, preserving original packageIndex in every binding and leaving package v1 ordering/checksum unchanged. Queue the marker synchronously from request callbacks after all successful adds, including the empty-record case. Require a mixed-package regression whose ID-less entry sorts before explicit `id: 1`, verifying both records, distinct returned keys, complete mapping, readback and atomic rollback for a genuine destination collision. Validate generated keys under the new numeric-key policy before committing the marker.

The generator starts at 1, explicit numeric keys advance it, and abort restores it; this is confirmed by the [IndexedDB key-generator specification](https://w3c.github.io/IndexedDB/#key-generator-construct). No browser reproduction was run in this review.

## Other reviewed decisions

No other actionable finding in the bounded target. Lines 10–40 terminate ownership recursion: first intent is authority, pre-intent leftovers are preserved `unbound_setup` failures, and response-loss final intents require binding validation and directory resync. This does not promise automatic recovery before admission. The later immutable generation authority remains separate from its metadata temporary.

DTOs distinguish retained package bytes from legacy payload checksum, bind native roots/files and preserve the legacy different-identity rule. Fresh requests retain the real target identity. Records-plus-marker atomicity and actual returned-key mapping fit the existing three-store transaction; content projection preserves known optional fields and excludes only newly generated IDs. Actual record reads and separate preference reads are correctly distinguished from echoed expected hashes and native IDB inspection. New typed encoding is domain-separated and does not replace existing v1 checksum rules. Rust/TS implementations and strict DTO/duplicate-key validators remain future work.

## Snapshot and checks

Observed canonical-root bytes, unchanged on final hash read:

| Input | SHA-256 |
| --- | --- |
| docs/superpowers/plans/2026-10-04-webview-schema-ownership.md | da7c4686f4159c453eac1b9a1d1fcd3dd823a4226dd7bb9a187c960c225f0908 |
| docs/superpowers/plans/2026-10-04-webview-recovery-contract-v3.md | 39b7bc3de8bb6f7e9a93207335fc8dcd33c67358be9a8ad9550fd1142a6b36f3 |
| docs/superpowers/plans/2026-10-04-webview-fresh-persistence-amendment-v2.md | 37332f101672e60a26d443b0c39d75a1c516be8c5406cd4c85bfaf4970f9783b |
| frontend/src/services/webMigration.ts | 974740764867a367d695ab1166e52f0e2a47d5bb8bb3761ca0f90482adf04ebf |
| frontend/src/services/indexedDBService.ts | fea62c36fe851b7c29cbe8948fb545f838048d5f5ccdefb07af90371c273a6aa |
| frontend/src-tauri/src/database/migration.rs | f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c |
| docs/fork/evidence/webview-canonical-golden-vectors-2026-10-04.json | 61661aa4daf89a64a409541ca9f5cd2be3e3c4a5533a006ca21c800764f9f214 |

Reviewer checks: local `nl`, `sed`, `rg` and SHA-256 reads; source comparison with current key schema/import/sorting and native validation/publication. A read-only inline Python3 independent encoder recomputed all 10 supplied valid vectors, matching complete canonical bytes and SHA-256 (exit 0). This confirms those vectors only: it does not exercise duplicate/unknown-field rejection, Rust/TS product interoperability, transaction timing, filesystem crash recovery or native/browser execution. Initial lookup used the wrong lib directory for webMigration; corrected to the actual services path before assessment.

changed_paths: only this review report. Product/source/test/plan/Git/user-profile paths were not modified. Environment: local canonical root on macOS; documentation/source inspection and in-memory Python calculations only.

Blockers and next action: resolve I1 in the design, then freeze complete validators/fixtures on the admitted clean checkout. Actual identity-pair/producer, clean-source, plugin/CSP/startup, FS/SQLite/IDB crash and installed upgrade gates remain open. No full-plan, FORK-123, native or release PASS is given.
