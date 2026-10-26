# Legacy mixed-ID correction — independent SPEC/code review

Status: bounded PASS; C0 / I0 / M0. No actionable finding in the exact two-file candidate. This is the existing legacy import correction, not admission or implementation of the new combined migration protocol.

## Observed snapshot and behavior

Candidate `/private/tmp/meetily-webview-mixed-id-fix-2026-10-04` has HEAD `53f4415058fb01e38b8892a47ac036bfffa394ba`. Read-only Git status shows exactly the modified service and new browser test. The service's base blob SHA-256 is `fea62c36fe851b7c29cbe8948fb545f838048d5f5ccdefb07af90371c273a6aa`.

`frontend/src/services/indexedDBService.ts:770–780` constructs a separate array with explicit IDs first and ID-less records second; both partitions retain their relative order. It neither sorts nor mutates the caller's snapshot/records. Explicit keys therefore advance/reserve the native IDB allocator before generated keys are requested. All operations remain `add` requests in the original three-store readwrite transaction. Existing duplicate-key error handling, marker identity/retry behavior and transaction-completion resolution remain intact. No new async gap, schema, coordinator, overwrite fallback or changed package checksum is introduced. Empty or homogeneous transcript lists retain their previous behavior.

`frontend/tests/indexeddb-migration-keys.spec.ts:75` exercises the actual transpiled service against real browser IDB, verifying the formerly failing mixed order, generated key 2 versus explicit key 1, unchanged input ID order, idempotent retry, reload/readback and marker persistence. The case at line 107 verifies a genuine existing-key collision rolls back all new records and marker while retaining exact neighboring records. Isolated page contexts and a fulfilled fixture page avoid application writers. These are meaningful regressions for this correction; the test is discoverable by the existing Playwright `**/*.spec.ts` configuration. It intentionally tests the service boundary, not a complete migration UI/native flow.

## Exact input hashes

| Input | SHA-256 |
| --- | --- |
| Candidate frontend/src/services/indexedDBService.ts | 0e89234b20a9163ea348498daa3afaeaac4f2bbe09188f00bc43b74490b5a77e |
| Candidate frontend/tests/indexeddb-migration-keys.spec.ts | 110d2b9cfdb71f406e165f86dd674f1a9e7890b5eb9ae53c741055885dd4dcb6 |
| Root docs/fork/evidence/webview-mixed-id-fix-2026-10-04.json | 0a5a6562fa55a75fbeb95a5c8fccd3bbe78e75ea33cd2d388eb4ce3f899cba5f |

All three remained unchanged at final recheck. Source-before hash independently matches the Git base blob.

## Verification reviewed

Reviewer ran read-only `git rev-parse`, `git status --short`, exact service diff, source/test/config reads and independent Python SHA-256 checks. Environment: local macOS checkout. No tests were rerun by this reviewer. Existing owner-run evidence was inspected, including source-bound RED failure and GREEN output:

| Evidence under /private/tmp/ | Observed outcome | SHA-256 |
| --- | --- | --- |
| meetily-mixed-id-regression-red-2026-10-04.log | mixed-key ConstraintError: 1 failed, collision regression 1 passed | 3f405bc7c1430790ebddc074166e3695d3004896addbee7517598065d94d52e1 |
| meetily-mixed-id-regression-green-2026-10-04.log | Chromium 2 passed | 9fdae7d608b0f4b5933713d9d477e45a7bda664245b022d090e50226ca6fc072 |
| meetily-mixed-id-frontend-2026-10-04.log | Node 424 passed, Vitest 61 passed | b657ecbb91e6a9d2fc960a0b25fad12b31ace655fdf78e32c3c9e8d85ebb0f56 |
| meetily-mixed-id-types-2026-10-04.log | tsc invocation without diagnostics; evidence records exit 0 | 056c34d0e1c885a6957c09198ddfc28dc35d7a9a60e6a54550056dd65b471b58 |
| meetily-mixed-id-lint-2026-10-04.log | eslint invocation without diagnostics; evidence records exit 0 | f554b0edfdc1ec70e6005cf5fc6ad5d720033eba53a2ceeb7f5c5afc84fbb393 |
| meetily-mixed-id-critical-2026-10-04.log | final observed summary: 29 passed (1.5m) | a496b9350472f053850a77aa87b9d94a35d68e154ac890ef55cff20dd9a24208 |

The first five log hashes match the evidence manifest. The critical run completed its 29-test summary during review, while the inspected manifest still says IN_PROGRESS; its process exit code was not independently observed. No new full-gate or release conclusion is inferred from that summary.

changed_paths: only this report. Candidate/root source, tests, configuration, Git refs and user data were not changed by the reviewer.

NOT VERIFIED: combined protocol/schema/key mapping, native WKWebView persistence, migration UI, actual profile identity pair, fault injection for the new coordinator, installed upgrade and release gates. They are outside this correction. Next action: record terminal critical-process status and complete separately authorized integration checks; no SPEC correction is required for this exact candidate.
