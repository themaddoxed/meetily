# FORK-123 fresh/persistence amendment — independent QUALITY review

**Status: revision required for the bounded source gate; C0 / I1 / M0. Implementation remains NOT_ADMITTED.** No product implementation or runtime test result is claimed.

## Important finding

**I1 — direct reuse of the identity-migration manifest API does not admit a fresh database for the same application.** Amendment `:43–50` requires publishing the fresh candidate through the existing manifest/stage/publish/resume primitives. Those public primitives call `validate_manifest` (`frontend/src-tauri/src/database/migration.rs:755`, `:808`), which rejects equal source/target identities at `:280–285`. A fresh candidate has no genuine old application identity. Passing the configured target identity as both sides therefore fails; inventing an old identity or relaxing the legacy check would violate the retained identity contract.

Define the precise seam before admitting this source gate: a typed owned fresh-publication request should reuse the lower verified copy/no-clobber/publication operations without pretending to be an old-to-new identity migration. Alternatively explicitly factor the domain-neutral operations below the existing manifest validator, keeping the legacy public API and identity checks intact. Add a fresh same-identity positive test and retain an equal-identity legacy-migration negative test.

Freeze candidate/staging locations at the same time: current `MigrationRoots::validate` (`migration.rs:211–232`) requires source, staging and target roots to be disjoint. A candidate placed inside the app-data operation directory cannot be passed as a source root to those helpers. The amendment has not fixed that path, so this is a placement constraint to resolve with the seam, not evidence that the document already mandates an overlapping path.

## Accepted corrections and remaining boundaries

The proposed fresh-only pool API correctly excludes current generic constructor behavior (`manager.rs:264–287`, including automatic legacy import) and normal reopen lifecycle effects (`manager.rs:199–228`). Adding manager.rs to the reviewed mutation list is appropriate. Returning SqlitePool rather than DatabaseManager avoids `from_pool` live-target registration; the coordinator must still guarantee exclusive ownership and close all its pool handles before publication. The amendment explicitly requires that behavior and does not claim it already exists.

Frozen defaults, propagated errors and a transaction are materially different from current commands.rs:230–253, which logs default-setting errors and continues. Limiting retries to a pending owned candidate avoids overwriting imported data or later user preferences. The exact empty/authentic-prefix classifier and ownership encoding remain explicit gates rather than unproved reuse of normal reopen.

The publication protocol now specifies complete file write/sync before no-clobber final linking, parent-directory synchronization before terminal reply, and final validation plus directory resync after response loss. Immutable intent plus separate completion receipt removes the in-place pending/completed mutation ambiguity. It also states that corrupt/unresolved combined work outranks terminal fresh admission and that legitimate later data/default changes do not invalidate initial completion.

The amendment borrows the discipline of `migration.rs:527–590`, not the entire existing implementation: that function's existing-final early return does not resync the directory, and its temporary cleanup differs from the new preservation contract. The new path must implement its stated differences. Full ownership encoding still needs a finite bootstrap rule: the first ownership-generation binding cannot recursively require another unpublished ownership document under the same rule. Resolve this explicitly in the remaining encoding gate, including interruption before the first durable binding. No claim that this encoding already exists is made here.

Legacy import continues through its existing preservation engine. Read-only bootstrap now correctly avoids calling `is_first_launch_already_gated` as pure inspection, because it invokes legacy recovery. Native plugin/CSP configuration, clean source, actual identity-pair admission, full canonical schemas and real crash/reopen verification remain separate gates.

## Snapshot and checks

Reviewed amendment SHA-256 before/after: `72d4435d7ae3333667962f2767c5561c8b5ef4c8696801cd90cf8098627ee6e8`.

Read the amendment against v3 (`39b7bc3de8bb6f7e9a93207335fc8dcd33c67358be9a8ad9550fd1142a6b36f3`) and current source using bounded file reads and symbol searches. Independently hashed:

- `frontend/src-tauri/src/database/manager.rs`: `10aaabeada6df54e414ad5c0489c615a9bc2f0819ce0b3829801bd915e804880`.
- `frontend/src-tauri/src/database/commands.rs`: `4cd381cbb06b09d57dba0115b38aff10ff6d4ea362c9441f8f516fd752a78f6a`.
- `frontend/src-tauri/src/database/migration.rs`: `f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c`.

No Git action, application/profile access, source mutation or runtime test occurred. `changed_paths`: only this report. No additional documented-standard violation or speculative code smell is asserted.

Next action: specify and review the typed fresh-publication seam and compatible candidate/root layout without weakening the legacy identity/root checks; then resolve the remaining explicit gates before product edits. This report does not admit the full plan, close FORK-123, or certify migration/native/release readiness.
