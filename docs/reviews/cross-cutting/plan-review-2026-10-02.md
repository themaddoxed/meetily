> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# CI and theme independent review — 2026-10-02

These are plan acceptance reports, not implementation or release evidence.
Storage is reviewed separately.

## SPEC

# Independent SPEC review — round 2

status: CI SPEC PASS; theme SPEC PASS. Each: Critical 0 / Important 0 / Minor 0.
reviewer: independent plans_spec agent, source-authored result.
scope: updated CI and semantic-theme plans plus census. Storage not reassessed; previous Important remains open.
observed_snapshot: main HEAD 53f4415058fb01e38b8892a47ac036bfffa394ba, existing dirty/untracked plans/scanner overlay. Targets are untracked planning artifacts, not implemented product changes. Unrelated .beads.gate.lock and claude-code-provider-decision documentation observed and excluded.
changed_paths: [/private/tmp/meetily-plans-spec-round2.md]; repository changed_paths: [].

## Findings

CI: No actionable SPEC findings. docs/superpowers/plans/2026-10-02-macos-ci-release.md:226 explicitly implements buildUniversalApp/createDmgFromApp in Task 2 before its Universal/config GREEN. Line 227 limits Task 2 GREEN to bundle/config/deployment; topology and transport remain RED until their owners. Line 237 assigns topology/policy GREEN to Task 3; line 248 requires all six combined commands only after Task 4 implements transport. Nonzero matching tests are required. Universal, macOS 14.2, immutable transport, secretless verification, separate signing/notary and final draft requirements remain preserved.

Theme: No actionable SPEC findings. docs/superpowers/plans/2026-10-02-semantic-theme.md:322 now directs closeout to canonical admitted $MEETILY_REPO_ROOT/docs/prd, resolving the prior Minor. Initial/reload prepaint, hydration diagnostics, focus >=3:1, real-surface contrast and separate native acceptance remain required. Recorded 30 passed/3 failed browser baseline remains RED, never acceptance. No snapshot update is admitted.

## Checks and evidence

Environment: macOS checkout /Users/developer/Desktop/meeting-minutes, zsh. Read-only source commands; sole authorized output is this report.
- git rev-parse HEAD and git status --short: supplied base confirmed and dirty state observed.
- shasum -a 256 plus final Python hashlib guard: all final targets present and stable. An intermediate CI c5357dbf snapshot changed during review, was rejected by the guard before any report was written, and has no PASS from this report. Final CI 4f40b439 bytes were re-read and rechecked.
- nl/sed: inspected updated CI Task 2/3/4 and theme Task 7 against reviewed contract.
- Python JSON/hashlib with git show BASE:path: 26 CI and 18 theme census source entries; zero hash/base mismatches. Theme census plan_sha256 matches reviewed theme.
- Specification context from round 1: docs/prd/macos-fork.md FORK-035/061/143/144 and FORK-039/062/150/151, docs/testing.md release boundaries, preserved original plans. No new acceptance waiver introduced.

Evidence SHA-256 (final before/after):
- docs/superpowers/plans/2026-10-02-macos-ci-release.md: 4f40b439579dc453743e3fd15c8d12ee74cce53ae5481364ece5b5b2255ce373
- docs/superpowers/plans/2026-10-02-macos-ci-census.json: 465b29488c1bfb3097cab8cd4ddf3203ab679f6c6f8f3d6049abce7a69018ba8
- docs/superpowers/plans/2026-10-02-semantic-theme.md: ee091492f7fef678e8cf2c07f62c2e6615fe150c747a49d321273720f5f41d17
- docs/superpowers/plans/2026-10-02-theme-census.json: aab13fd017dbdf0026d2980c214afe18695d976074653a76c5dec9818f551d68

## Limits and next action

NOT VERIFIED: implementation/runtime, hosted runners, remote action SHA admission, native-input acquisition, Universal artifacts, signing/notarization/Gatekeeper, native appearance or fresh browser prepaint/focus/contrast. Review executes no release or native action. Existing preserved browser evidence was independently inspected in round 1, not rerun here.

blockers: none to bounded CI/theme SPEC PASS. External CI prerequisites remain implementation admission stops; theme RED baseline must be resolved through planned evidence. Storage remains open and excluded.
next_action: combine with independent QUALITY report; preserve exact hashes and reassess any changed target. Plan PASS does not establish product or release readiness.


## QUALITY

# Independent QUALITY re-review, round 2

Reviewer: /root/plans_quality; source-authored result, 2026-10-02.
status: CI QUALITY PASS; Theme QUALITY PASS. Both Critical 0 / Important 0 / Minor 0.
observed_snapshot: root HEAD/base 53f4415058fb01e38b8892a47ac036bfffa394ba; existing dirty planning/scanner overlay. Final exact target hashes below. An intermediate CI revision c5357dbfa3c0beda76920795746bdf4f3399a292b2d25e4e27c7066a0d31a9c0 had a residual sequencing issue; it is superseded and is not the accepted revision.
changed_paths: [/private/tmp/meetily-plans-quality-round2.md, /private/tmp/meetily-context-review.md] as explicitly authorized review outputs. No product/docs edits by reviewer.

## CI — QUALITY PASS

No actionable findings remain on the final exact revision. Prior Important sequencing defect is resolved at macos-ci-release.md:226-227,237,248: Task 2 now explicitly implements buildUniversalApp/createDmgFromApp before the full Universal fixture gate, alongside bundle/config/locked-runner behavior; Task 3 owns workflow topology checks; transport remains RED until Task 4; combined six-command GREEN runs only after transport implementation. This closes both the original prematurely required full GREEN and the intermediate residual DMG sentinel issue. No weakened assertions or waiver is prescribed.

NOT VERIFIED: proposed historical tool versions/full action SHA pins, authenticated immutable native-input acquisition, hosted runner capacity, hosted workflows/protected environments, actual Universal packaging, signing/notarization/Gatekeeper/draft. Task 0 still requires their applicable admission and frozen amendment review. Plan acceptance is not implementation admission or release readiness.

## Theme — QUALITY PASS

No actionable findings remain. Prior Minor incorrect foundation closeout target is resolved at semantic-theme.md:322: canonical admitted $MEETILY_REPO_ROOT/docs/prd is explicit. Existing three baseline failures, snapshot drift and strict prepaint/stylesheet ordering remain accurately exposed as implementation gates; this review does not convert them into runtime PASS. No native or browser execution occurred in this re-review.

NOT VERIFIED: twelve-transition held-stylesheet/pre-body matrix, hydration diagnostics, real focus/contrast, native appearance/accessibility, and whether conditional fixes can remain inside the admitted allowlist. The plan requires stop/amend/review if required correction exceeds scope. No completed-theme or release claim.

## Checks / evidence

Environment: root checkout, Darwin x86_64; static read-only review plus two authorized temporary reports.
- Read final Task 1-4 CI task dependencies and corrected theme closeout; compare with source-authored first review findings.
- shasum -a 256 before final review; Python hashlib final recheck: four exact target hashes stable and match the below pins.
- Recursive census JSON hash verification: 36 CI and 33 theme source/evidence records matched with zero mismatches (including existing theme confirmation artifacts).
- HEAD readback: 53f4415058fb01e38b8892a47ac036bfffa394ba.
- Prior baseline source inspection retained: docs/testing.md, CONTRIBUTING.md, PRD FORK-144/151, preserved plans and corresponding current source. No new execution evidence inferred from implementer narrative.

findings: [] for final CI and theme revisions.
blockers: [] for QUALITY acceptance of these two plans; external implementation prerequisites and NOT VERIFIED boundaries remain.
Storage: explicitly excluded from round 2; its prior Important findings remain open, no reassessment or waiver.
next_action: combine this QUALITY result with independent SPEC review for exact hashes; preserve Task 0 admission gates before implementation.

## Exact reviewed SHA-256
- `docs/superpowers/plans/2026-10-02-macos-ci-release.md`: `4f40b439579dc453743e3fd15c8d12ee74cce53ae5481364ece5b5b2255ce373`
- `docs/superpowers/plans/2026-10-02-semantic-theme.md`: `ee091492f7fef678e8cf2c07f62c2e6615fe150c747a49d321273720f5f41d17`
- `docs/superpowers/plans/2026-10-02-macos-ci-census.json`: `465b29488c1bfb3097cab8cd4ddf3203ab679f6c6f8f3d6049abce7a69018ba8`
- `docs/superpowers/plans/2026-10-02-theme-census.json`: `aab13fd017dbdf0026d2980c214afe18695d976074653a76c5dec9818f551d68`


## Storage SPEC

# Independent storage SPEC review — round 2

status: SPEC PASS. Critical 0 / Important 0 / Minor 0 on final exact target bytes.
reviewer: independent plans_spec agent; source-authored result.
observed_snapshot: HEAD/base 53f4415058fb01e38b8892a47ac036bfffa394ba plus explicitly censused dirty FEAT-100 overlay. This is a planning snapshot, not a clean implementation/release checkout.
changed_paths: [/private/tmp/meetily-storage-spec-round2.md]; repository changed_paths: [].

## Findings and resolved defects

No actionable SPEC findings remain in the final snapshot.

The former Important missing derived-output contract is resolved by docs/superpowers/plans/2026-10-02-recording-storage.md:147 onward. It specifies immutable export versions and durable payloads, accepted/published heads, owner/revision CAS, acceptance plus search plus pending-intent transaction, new NULL-folder exports, distinct version paths, no replacement of capture or legacy/user files, stale-finisher isolation, per-file crash reconciliation and pointer commit, explicit new-version behavior and external-change conflicts. Task 7 requires real temporary-storage/SQLite close-reopen tests for these paths. These provisions preserve accepted summary state and automatic export-pending semantics without weakening immutable capture publication.

The descriptor-only contract at lines 138–145 and Task 3 explicitly rejects unsupported descriptor codecs before any pathname writer; victim bytes/inode must remain unchanged during a symlink swap test. The formerly contradictory risk text at line 444 now forbids pathname fallback as well. A pre/post inode check is no longer treated as prevention of a child write to a substituted victim.

The obsolete census design_stops entry is removed. Canonical documentation closeout is retained. New FEAT-100 reader and tests are explicitly included in the 62-path allowlist (lines 338–339); Task 7 at line 419 requires rejecting hidden selected-meeting IDs, visible transition, rollback/reopen and preserving existing four isolation cases. I read the current reader SELECT and API registration diff; the newly discovered caller is therefore covered rather than bypassing planned visibility.

## Checks, commands and outcomes

Environment: local macOS checkout /Users/developer/Desktop/meeting-minutes, zsh. Read-only source commands, no native/browser/remote execution; this report is the sole write.
- git rev-parse HEAD: supplied base unchanged.
- shasum -a 256 initially and final Python hashlib assertion: final plan/census present and stable. Intermediate 079a60d8/7c67dcef bytes changed before report creation and are superseded, not approved by this result.
- nl/sed/rg: read full preceding revision and changed final sections; checked global invariants, schema, allowlist, tasks, safety controls and concrete summary/source seams.
- Python JSON/hashlib recursive census validation: 116 distinct hashed source entries checked, zero current hash mismatches. The two base differences are explicitly declared FEAT-100 overlays rather than falsely marked base-equal.
- git show BASE:path plus SHA-256: api/api.rs base 81f4ead67a83e18bb21592337800a5b24a2dcf80cd80ba43883789ee7e0ffb89 and lib.rs base 085076e349686a5d43d4d385a0f9ba4d4f0c5a8797cbea44f7d8a02b78fc90ae independently match census. Current values are 21b3bd44bf41ce1d12a0d29c22c18a671e98d718a1738f36a6011f3e28d309e7 and ab3a0a10c42853ce7968d0787d62f99d50022b2c2c25e2d4acdc1815f6300dea.
- Read summary commands/service/metadata/persistence and repository ownership/search seams; read current MeetingContextRepository and inspected its four test cases. No implementation test was run or claimed.
- SPEC authority carried forward: current PRD FORK-043/063/126/127 and original preserved plan 2bda589971d6e6cf6bfbbc4e170325afc1b34db2b64a058263be880886620689, reviewed in round 1; docs/testing.md data-safety/release boundaries retained.

Final evidence SHA-256, before/after:
- docs/superpowers/plans/2026-10-02-recording-storage.md: 814a48a60b0891fd24bfb03f52e9e65e0ccdf727b55b8061029e3418a9d1361f
- docs/superpowers/plans/2026-10-02-recording-storage-census.json: 267e105bc868342bcb36a688ec5f5071783c56ccbdbeb11de9fb91a5ce9eea33

## Boundaries and next action

blockers: none to this bounded planning SPEC PASS. Clean product admission remains required by Task 0; current dirty source is not implementation authorization.
NOT VERIFIED: actual migrations, SQL transaction/crash/reopen behavior, no-clobber/fsync races, FFmpeg codec compatibility and descriptor inheritance, runtime UI/native capture, physical power-loss durability or release readiness. FEAT-100 full implementation acceptance is outside this storage SPEC review. No NOT REVIEWED final plan/census target: both final target bytes were reviewed; intermediate changed snapshots are superseded.
next_action: combine with independent QUALITY review; preserve these hashes and repeat admission/census/review for any source or plan drift before implementation.


## Storage QUALITY

# Storage independent QUALITY review, round 2 — final revision

Reviewer: /root/plans_quality. Source-authored review, 2026-10-02.
status: QUALITY PASS. Critical 0 / Important 0 / Minor 0.
observed_snapshot: root HEAD/base 53f4415058fb01e38b8892a47ac036bfffa394ba plus explicit reviewed uncommitted FEAT-100 source overlay and planning/scanner overlay. The exact current plan/census are pinned below. This is not an admitted clean implementation checkout.
changed_paths: [/private/tmp/meetily-storage-quality-round2.md] only, explicitly authorized report; no product/docs edits by reviewer.

## Findings and resolved issues

No actionable findings remain on this final revision.

The previously missing derived-output contract is concrete at recording-storage.md:147-216: immutable complete payload/version/intents, accepted and published SQLite heads, owner/revision CAS, automatic export for NULL-folder rows, accepted summary/search preservation, language-only/manual edits, crash reconciliation and user-edited/deleted-file preservation. Published-head advancement requires both files and current accepted-version ownership; old finishers cannot clear newer pending state. Separate immutable directories preserve legacy/capture files. Missing legacy storage leaves accepted SQLite usable and export pending. Tests require duplicate retry, each file/pointer crash boundary, external change/deletion, old pointer correctness and preserved accepted/search content.

The FFmpeg pathname fallback is now forbidden consistently in the domain interface, Task 3 and principal risk control (:138-145, :379, :444). Unsupported descriptor codecs return a typed failure without authorizing pathname writing. Required symlink-swap testing asserts victim bytes/inode unchanged, not only an error after corruption. This resolves the earlier before/after inode-check safety flaw and its intermediate stale fallback clause.

The latest revision also reconciles the new current FEAT-100 consumer rather than pretending the old census still describes dirty main. Scope is now 62 distinct paths, including meeting_context.rs and its tests (:338-339). Task 7 (:419) requires hidden-ID rejection, successful visibility transition, rollback/reopen and preservation of existing four FEAT-100 scenarios. The source snapshot explicitly distinguishes immutable base from the reviewed dirty overlay. Source hash checks match current files.

## Checks and evidence

Environment: root checkout on Darwin x86_64; read-only inspection with the single authorized report write.
- Read complete changed domain/schema/contracts, task sequence, 62-path allowlist, and principal risks; targeted fallback/derived/victim/overlay census.
- Reviewed current summary/service.rs, commands.rs, persistence.rs, metadata.rs and database/repositories/summary.rs seams, including accepted-summary export-pending and NULL-folder behavior; these existing seams are within the scope. New tables fit the admitted migration; storage/repository paths are admitted. No required extra product path was confirmed.
- Reviewed current FEAT-100 command registration and selected-meeting reader against future hidden-row rules; added context source/test paths and explicit Task 7 cover the observed consumer.
- Python JSON/hashlib: 62 unique allowlist paths; all 74 hashed current source records match; zero mismatches. Census plan_sha256 matches the plan.
- Prior comparison of the old 72-record census against git show base:path matched all records; the observed API/lib drift was reviewed and is now explicit in refreshed census. Earlier snapshots are superseded, not accepted by this result.
- shasum/Python final readback: final plan/census hashes stable and equal the below pins.

## NOT VERIFIED boundaries

This is plan acceptance, not implemented storage proof. Real FFmpeg descriptor/codec compatibility and seek behavior, macOS no-clobber race, unsafe FFI, migration/reopen/crash tests, concurrent publication/CAS, frontend export outcomes, native packaging and physical power-loss durability have not been executed by this reviewer. All remain required future implementation gates. Proposed version-retry, explicit-new-export revision allocation, FK/nullability and error-mapping details must be verified in the concrete migration and implementation review against this accepted contract; no exception to no-clobber or accepted-summary ownership is granted. No release readiness is claimed.

blockers: [] for QUALITY acceptance of these exact plan bytes. Task 0 clean-base/source admission and fresh implementation evidence remain required.
next_action: combine with independent SPEC review on the same hashes; implementation may begin only after its existing admission and authorization gates.

## Exact reviewed SHA-256
- `docs/superpowers/plans/2026-10-02-recording-storage.md`: `814a48a60b0891fd24bfb03f52e9e65e0ccdf727b55b8061029e3418a9d1361f`
- `docs/superpowers/plans/2026-10-02-recording-storage-census.json`: `267e105bc868342bcb36a688ec5f5071783c56ccbdbeb11de9fb91a5ce9eea33`
