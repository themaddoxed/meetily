# Public snapshot privacy audit — 2026-10-27

Scope: a history-free source snapshot of the previously published `cf092f3`.
This report contains no actual credentials, private email values or personal
home paths. The old remote has not been replaced by preparing this local copy.

## Results

- The candidate has one root commit with neutral `Meetily Contributors` /
  `noreply@example.invalid` author and committer identities, no old parents,
  branches or tags.
- Gitleaks 8.30.1 returned exit 0 on the clean source directory and root history.
- Exact original local username/home path and publishing-author name/email
  checks returned no remaining text matches, including decoded gzip logs.
- 42 remaining PNG/JPEG graphics were checked locally with Apple Vision CPU OCR:
  236 recognized text lines, no email/home-path/original-author indicators.
  All recognition requests succeeded; the initial sandbox attempt failed to
  create pixel buffers and its empty results were rejected as invalid evidence.
- Twelve documentation UI screenshots with transcript/local-machine context and
  three animated demos (2,648 frames) were omitted. Their originals stay local.
- No application database, real audio recording, private-key file, model weight,
  environment-secret file, build output or installed dependency is included.
- Product sources, migrations and the helper remain byte-identical to `cf092f3`.
- The project index, scoped PRD validation, documentation-guide test and production
  policy passed. The edited shell checker passes syntax validation.
- Transformation-only whitespace comparison passed. Checking the entire initial
  root as one new patch exposes pre-existing whitespace in inherited files;
  those were not silently reformatted or described as passing.

The earlier remote audit covered 98 refs (84 branches, 13 tags, one pull ref),
1,029 commits and 4,774 blobs. Gitleaks reported 43 patch-history candidates and
84 candidates when scanning all distinct text versions/decoded gzip blobs.
Every candidate was classified as public PostHog project tokens, browser-storage
key identifiers or redaction fixtures. Text extracted read-only from four old
SQLite blobs passed Gitleaks (exit 0); their provenance/content is not asserted
safe, and those databases are absent from the new snapshot.

PostHog distinguishes public `phc_` project tokens from private API credentials:
[official documentation](https://posthog.com/docs/privacy#is-it-ok-for-my-api-key-to-be-exposed-and-public).
No credential was tested against an external service.

## Evidence boundaries

These checks found no confirmed private credential in the candidate. Pattern
matching, OCR and visual inspection cannot prove absence of every possible
secret or personal detail. Public handles, synthetic test identities, package
identifiers and required license attribution remain.

Historical evidence was anonymized and labeled as edited copies; original
checksums identify private originals, not their edited public derivatives.
The previous complete application gate remains historical. No full rebuild,
app/DMG, installed/native smoke, signing, notarization or release qualification
was performed on this source snapshot.

The retained historical architecture diagrams show FastAPI/vector storage and
Electron, while the current application is Tauri/Rust. They are explicitly
labeled historical in `docs/assets/README.md`. Old Git workflow/provenance scripts
also reference refs intentionally absent here; bootstrap/archival limits are
spelled out in [publication notes](../../development/public-source-snapshot.md).

Replacing visible refs in a public fork cannot erase its old pull ref, upstream
network objects, GitHub caches or other people's copies. A genuinely separate
publication requires a standalone repository. The full prior remote is retained
in a verified local Git bundle; no destructive remote operation is performed by
this audit.
