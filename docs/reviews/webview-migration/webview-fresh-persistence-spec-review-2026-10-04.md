# Fresh-only and immutable publication amendment — independent SPEC review

**Revision required: C0 / I1 / M0. Implementation remains NOT_ADMITTED.** The finding is a source-interface mismatch in the proposed design, not a claim that new production code failed.

## Important finding

**I1 — Existing migration stage/publish entry points cannot directly represent fresh creation.** Amendment lines 45–46 require publishing the new candidate through the “existing owned manifest/stage/publish/resume primitives.” Current `migration.rs:750–755`, `803–808` and `863–868` all call `validate_manifest`. That validation at `:280–285` requires nonempty **different** source and target application identities. A fresh candidate is created by the current application under its own identity; it has no distinct exporting producer. An unchanged migration journal cannot represent that authority without inventing a false identity or weakening the legacy migration guard. The proposal defines a separate first-launch intent but does not define the bridge to these entry points.

Resolve this explicitly: factor the existing copy/hash/no-clobber/owned-readback mechanics into a bounded internal owned-file publication seam, retaining separate typed authority validation for cross-identity migration versus first-launch fresh ownership. Keep the legacy journal schema and different-identity validation unchanged; do not populate fictional source identities. Bind the fresh path to its real first-launch intent, candidate generation and target, and specify a candidate/staging layout compatible with the existing disjoint-root constraints (`migration.rs:210–231`) or a separately reviewed ownership-specific relationship rule. Reuse mechanics rather than a parallel copy/WAL engine. Add a regression proving genuine same-identity fresh publication works while equal-identity migration journals still fail. Classification: incomplete/unreachable proposed interface under current source contracts.

## Source gates that are correctly addressed

The fresh-only initializer avoids confirmed wrong entry points: generic `new_already_gated` can import a legacy DB; normal `open_existing_native` performs lifecycle/summary/export/search recovery; `create_fresh` refuses existing paths and returns a manager with live-target registration. A private owned-candidate-to-SqlitePool seam is a viable way to isolate migrations/defaults without these lifecycle effects. The amendment correctly keeps the authentic empty/SQLx-prefix classifier and exact ownership encoding as future admission gates rather than asserting they already exist.

Freezing defaults before writes, propagating errors, applying them transactionally only to a pending fresh candidate and verifying results closes the current command's log-and-ignore failure behavior. Checkpoint busy/incomplete detection, pool close, file/directory sync and no-clobber publication are necessary before terminal admission. The new read-only bootstrap caveat is accurate: `is_first_launch_already_gated` invokes legacy import recovery (`manager.rs:353–365`).

The immutable intent/receipt publication direction is consistent with `persist_journal`'s complete temporary write, sync and no-clobber hard link (`migration.rs:531–595`). The proposed retry explicitly re-syncs the final directory and preserves/reconciles owned temporaries, so it must not merely copy the old early-existing-return or cleanup behavior verbatim. No torn final pathname or in-place pending/completed overwrite is authorized. Exact generation ownership still needs a finite bootstrap encoding for the first authority document; this is part of the explicitly retained encoding gate, not an additional unrequested subsystem.

## Snapshot and checks

Reviewed 2026-10-04. Amendment SHA-256 before/after: `72d4435d7ae3333667962f2767c5561c8b5ef4c8696801cd90cf8098627ee6e8`. V3 reference: `39b7bc3de8bb6f7e9a93207335fc8dcd33c67358be9a8ad9550fd1142a6b36f3`.

Current source bindings:

- `frontend/src-tauri/src/database/manager.rs`: `10aaabeada6df54e414ad5c0489c615a9bc2f0819ce0b3829801bd915e804880`.
- `frontend/src-tauri/src/database/commands.rs`: `4cd381cbb06b09d57dba0115b38aff10ff6d4ea362c9441f8f516fd752a78f6a`.
- `frontend/src-tauri/src/database/migration.rs`: `f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c`.

Reviewer checks were source reads and local SHA-256 checks only. No app, user profile, Git operation, SQLite mutation or runtime test. Review changed_paths: only this report. No product or other-agent files changed.

## Next action and boundaries

Resolve I1 in the bounded seam/API and authority description, then re-review those exact bytes. Full canonical schemas, ownership encoding, clean source, actual pair and plugin/CSP gates remain open as stated. No fresh-resume, migration, native or release PASS is established by this design review.
