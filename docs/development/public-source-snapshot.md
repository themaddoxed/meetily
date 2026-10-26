# Clean public source snapshot

This repository is a new source snapshot with one root commit and neutral commit
identity. It intentionally omits the previous Git graph, branches, tags and Git
author/committer identities. The original repository, full history and unmodified
evidence are retained locally, outside this publication. License notices and
upstream attribution remain intact.

## Privacy transformations

- Real developer home paths in documents, evidence and one historical planning
  checker were replaced by generic developer paths.
- Contact/Git-provenance fields in documentation were anonymized. Clearly
  synthetic test identities, dependency package identifiers, public repository
  handles and legally required attribution are retained.
- Compressed text logs were decoded, anonymized and recompressed. A banner marks
  them as edited privacy copies. JSON/Markdown evidence is marked similarly.
- No application databases, real audio recordings, environment secrets, build
  output, installed dependencies or model weights are included.

Historical SHA-256 values still identify the original private evidence. They do
not verify the edited public copies. Historical PASS results remain historical;
no release candidate is created or qualified by this publication.

## Development and checks

The initial `main` is a source bootstrap, not a release. Create clean `dev` from
this root commit before resuming development and use the normal feature workflow.
References to old commits, worktrees and legacy branches in older plans and
migration scripts are archival; they cannot be resolved in this history-free
repository. In particular, old source/patch-provenance migration checkers require
the private archive and are not fresh-clone acceptance checks for this snapshot.

Outstanding platform-surface and native/release checks remain documented in
[resume notes](../reviews/cross-cutting/development-resume-notes-2026-10-26.md).
The previous full automatic gate is historical and is not claimed for this new
root commit. No app/DMG, native smoke, signing or notarization was run here.

## Scope of the publication audit

The old remote was inspected locally: 98 refs, 1,029 commits and 4,774 unique
blobs, including decoded gzip text and text fields extracted read-only from four
historical SQLite blobs. Gitleaks 8.30.1 findings were triaged: public PostHog
project tokens, browser-storage identifiers and redaction fixtures; no confirmed
private credential was found by those checks. This is not a guarantee against
all possible secrets or sensitive content.

The clean snapshot is checked separately before publication. The audit does not
revoke credentials, erase copies made by third parties, or guarantee that old
commits disappear from GitHub's shared fork network or caches. Replacing the
repository only controls the new repository's visible refs and source snapshot.

Three optional animated documentation demos (2,648 frames) were omitted instead
of publishing media whose personal contents had not been fully reviewed. Their
originals remain in the private local archive. Static images are checked locally
with OCR; OCR cannot prove absence of every kind of personal content.

Twelve documentation UI screenshots were also omitted: they contain transcript
content or local-machine context whose private/synthetic origin is not proven.
Only branding and explicitly historical architecture diagrams remain in docs.
The legacy FastAPI/vector-database and Electron diagrams do not describe the
current Tauri/Rust implementation; this discrepancy is labeled in assets/README.
