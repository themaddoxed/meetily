# FORK-123 schema/ownership candidate — bounded QUALITY review

**Status: bounded design is coherent; C0 / I0 / M0 actionable findings. Implementation remains NOT_ADMITTED.** No full-plan, executable schema, crash-recovery or migration acceptance is claimed.

## Findings and reasoning

No confirmed contradiction or data-loss requirement was found in this candidate. The first-intent rule has a finite base case (`2026-10-04-webview-schema-ownership.md:10–24`): validated native request and held gates authorize isolated preparatory writes; the intent becomes the first durable authority. A pre-intent interruption is explicitly `failed: unbound_setup`, preserves bytes and blocks normal startup (`:26–34`). This is a conservative operator-recovery boundary, not a claim of automatic recovery. Generation metadata also has an explicit nonrecursive publication exception (`:36–40`).

The DTOs distinguish retained package raw-file identity from its existing browser payload checksum and bind native manifests without mutable journal phase (`:60–75`). Pending preference replay remains absent-or-equal and cannot expand the existing allowlist. Receipt intent hashes bind the fuller roots/choice/identity authority transitively; native construction follows bound verification, not a client readiness flag. Terminal DB hashes are explicitly historical initialization evidence rather than a reason to overwrite later user edits.

Generated transcript-key handling resolves a real mismatch with the current importer. `frontend/src/services/indexedDBService.ts:702–782` adds transcript rows to an auto-increment inline-key store but does not record returned keys. The proposed record mapping collects `add` results and writes the complete marker in that same transaction (`:90–95`). This can be implemented in request callbacks without Web Crypto awaits: prepare content hashes before the transaction, queue the marker from the final successful add callback, and resolve only from transaction completion. Preserve the explicit empty-snapshot path so it queues its marker immediately. An add or marker failure must abort all writes.

For ID-less records, hashing the original content while separately binding the generated key avoids treating two identical transcript payloads as the same row. Explicit IDs remain part of the content. Reading marker plus mapped records in one readonly transaction and hashing after completion is compatible with the current transaction-based source snapshot. Validation must enforce the document's complete mapping: exactly one entry per package store/index, valid key types, unique destination keys within each store, and no omitted or extra mappings. These are consequences of the stated complete mapping and retained-package match, not new scope.

The new canonical format is typed and length-delimited, sorts valid Unicode keys by exact UTF-8 bytes, normalizes negative zero, rejects invalid numeric values/surrogates/duplicate keys and distinguishes omitted from null (`:132–155`). That avoids dependence on object insertion order or replacement UTF-8 encoding. It deliberately does not replace `webMigration.ts:184–203` sorting or its `stableJson` v1 payload hash. Cross-language golden fixtures are required; no Rust/TS encoder interoperability is claimed here.

## Conditions retained before implementation

Freeze concrete root-binding variants, all DTO field types and limits, and ownership-generation encoding on the admitted source. Missing-target parent/basename to created-root identity transition must itself be an immutable owner-bound publication, rather than mutation of the initial intent. A first-intent hard-link temporary and interrupted generation-metadata temporary need explicit classification under their documented base-case exceptions; do not mistake legitimate linked metadata for unrelated files or infer authority from a filename. Unknown/corrupt data remains fail-closed as specified.

Keep original browser v1 verification before the stricter combined projection. Current normalizers project known transcript fields; they cannot implement the new unknown-field rejection by silently dropping input fields first. Read raw records, enforce the combined schema, then apply the defined projection. Ordinary legacy import behavior remains separate.

No new production path is admitted here. Clean source, actual producer/identity pair, plugin/CSP/startup transport, real browser IDB behavior and temporary FS/SQLite crash proof remain open. The explicit unbound-setup operator boundary must remain visible in user-flow acceptance rather than being counted as successful resume.

## Snapshot and checks

Reviewed input SHA-256 before/after: `da7c4686f4159c453eac1b9a1d1fcd3dd823a4226dd7bb9a187c960c225f0908`.

Read the full candidate against the retained v3/amendment-v2 context and the actual adapter package validation/sorting/hash construction, IDB schemas and import transaction, plus the previously inspected migration authority/publication contract. Independently hashed current sources:

- `frontend/src/services/webMigration.ts`: `974740764867a367d695ab1166e52f0e2a47d5bb8bb3761ca0f90482adf04ebf`.
- `frontend/src/services/indexedDBService.ts`: `fea62c36fe851b7c29cbe8948fb545f838048d5f5ccdefb07af90371c273a6aa`.
- `frontend/src-tauri/src/database/migration.rs`: `f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c`.

Checks were read-only file/symbol inspection and SHA-256 calculation. No product edits, Git operations, app/profile access or product/runtime acceptance tests occurred. `changed_paths`: only this report. Existing reports and other edits are preserved. No documented-standard violation or speculative code smell was established.

Next action: freeze exact executable DTO/ownership rules and fixtures with the remaining source/startup gates, then seek implementation admission. Do not infer FORK-123 completion, native migration or release readiness from this design verdict.

## Supplemental encoding fixture observation

After the initial review, inspected the supplied standalone Node encoder and ran `node /private/tmp/meetily-canonical-fixtures-2026-10-04.mjs` read-only: exit 0, all 10 retained vectors matched in bytes and SHA-256, and five invalid inputs were rejected. Evidence JSON SHA-256: `61661aa4daf89a64a409541ca9f5cd2be3e3c4a5533a006ca21c800764f9f214`; Node script: `827ce05b9393dda839fcbb1618d76aa1cc99bfb3c632d24f10863a92c18b9d3a`; retained Python generator: `7057ae1b661db6bf5b725fe2b8073d9fbff294aea28a102e7db6a455481220ad`. The Python generator was not rerun because it writes the evidence file. This supports proposed-format vector consistency only: the Node fixture uses JSON.parse and is not a duplicate-key parser, schema validator, product TS/Rust encoder, or browser/native test. Those gaps remain explicitly unverified.
