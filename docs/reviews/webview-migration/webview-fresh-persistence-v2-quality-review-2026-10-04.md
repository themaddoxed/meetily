# FORK-123 fresh/persistence amendment v2 — bounded QUALITY review

**Status: I1 resolved at design level; C0 / I0 / M0 new actionable findings. Implementation remains NOT_ADMITTED.** The bounded correction is acceptable; this is not full-plan admission or runtime verification.

## Correction assessed

The v2 amendment replaces direct fresh reuse of the cross-identity manifest API with an internal `publish_owned_files` seam behind two independently validated authority types (`:45–56`). The existing legacy stage/publish/resume entrypoints retain unchanged `validate_manifest`, including its different-identity requirement. `OwnedFreshPublication` carries the real target identity and first-launch ownership rather than a fictional source identity. This resolves the previous incompatible domain contract while retaining one copy/hash/no-clobber engine.

Native-derived candidate and staging locations (`:58–66`) are siblings under a separate owner root outside target app-data. This is compatible with the existing root disjointness checks in `frontend/src-tauri/src/database/migration.rs:211–232`; candidate and staging are siblings to each other, while their ancestor is a sibling of the target. The explicit canonical containment/symlink/alias checks, durable owner binding, and no caller-provided paths prevent the textual layout from acting as authority by itself. Same-filesystem placement must be validated, not inferred merely from path strings.

The required tests pair a fresh same-identity positive with a legacy equal-identity negative and add overlapping/aliased-root cases (`:114–116`). These exercise the corrected boundary rather than only a shared helper's happy path. No test implementation or success is claimed by the document.

## Conditions retained for implementation review

Current `publish_manifest` checks all destinations before copying and verifies the staged source while publishing (`migration.rs:820–853`); `resume_published_manifest` validates existing destination bytes and copies missing files without clobbering (`:860–916`). Factoring must retain those checks, stage/root validation and sync/error behavior. Existing matching bytes alone are not a fresh owner proof: the fresh authority layer must enforce the stated operation/generation binding before shared mechanics may accept a published target.

The fresh-only pool/defaults design, no normal lifecycle side effects, checkpoint/close requirement, immutable intent/receipt protocol and post-completion preservation of user edits remain unchanged from the prior amendment. The exact ownership encoding, first durable generation-binding bootstrap, canonical DTOs and partial SQLx-prefix classifier are still explicit gates. In particular, do not treat the shared publisher's future implementation as resolving the encoding bootstrap by itself.

Clean source/QA preservation, actual identity-pair admission, plugin/CSP/IPC configuration and native store continuity remain open. No relaxation of legacy journal encoding, identity validation or source-preservation checks is authorized.

## Snapshot and checks

- Reviewed v2 SHA-256 before/after: `37332f101672e60a26d443b0c39d75a1c516be8c5406cd4c85bfaf4970f9783b`.
- Preserved original amendment SHA-256: `72d4435d7ae3333667962f2767c5561c8b5ef4c8696801cd90cf8098627ee6e8`.
- Current `frontend/src-tauri/src/database/migration.rs` SHA-256: `f249093eaa83e3116aece3f754135afd92e531e97c5a438affac7acba6d9349c`.

Read the exact `diff -u` between amendments and current manifest publication/resume source; independently rehashed the above files. This review used read-only source inspection, without Git actions, source changes, app/profile access, or tests. `changed_paths`: only this report. Previous failure report remains preserved. No additional documented-standard violation or speculative maintainability finding was established.

Next action: settle the remaining explicit encoding/schema/source/plugin gates and review their concrete result before implementation. Do not close FORK-123 or claim migration/native/release readiness from this bounded correction.
