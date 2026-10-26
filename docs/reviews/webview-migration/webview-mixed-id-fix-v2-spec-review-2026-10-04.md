# Legacy mixed-ID v2 — independent SPEC/code review

Status: bounded PASS; C0 / I0 / M0. No actionable finding in the supplied two-file correction. The prior explicit-first candidate and its historical review are superseded for admission by the demonstrated maximum-generator regression; this review does not carry forward that prior conclusion.

Observed checkout: `/private/tmp/meetily-webview-mixed-id-fix-2026-10-04`, HEAD `53f4415058fb01e38b8892a47ac036bfffa394ba`. Read-only status shows exactly one modified service and one untracked regression spec. Both source hashes matched the capsule and remained unchanged on final read.

## Correctness and data safety

`frontend/src/services/indexedDBService.ts:770–809` partitions without modifying input records or array order, reserves all explicit package IDs in a Set, and allocates ID-less records before high explicit keys advance/exhaust the generator. A reserved-key deletion is reachable only from the corresponding successful `add` callback and uses that request's returned key. Because `add` cannot overwrite an existing row and the operation holds the same readwrite transaction, the deletion targets only the newly inserted row, not neighboring destination data. Deletion completion schedules the retry. Generator progression makes each distinct reserved key skippable at most once; this is bounded by actual package reservations, not the numeric distance to a large ID. Asynchronous request callbacks avoid recursive stack accumulation.

Non-reserved generated records remain; subsequent explicit records and the legacy operation marker are queued only once generation completes. Generated/explicit/meeting/marker failures retain the common abort path, so intermediate adds/deletes and allocator changes remain transaction-local. Empty and explicit-only snapshots immediately queue explicit records/marker. Retry detection remains ahead of writes. No promises, Web Crypto waits, overwrite fallback, new schema, coordinator or package checksum mutation were introduced. The maximum explicit-key behavior remains supported without prematurely exhausting allocation.

## Verification and coverage

The real-IDB source test at `frontend/tests/indexeddb-migration-keys.spec.ts:75` covers mixed import, unchanged input order, retry and reload/reopen. Line 107 covers genuine destination collision and exact neighbor preservation. Line 135 covers explicit `2 ** 53` after a generated row. Line 152 exercises multiple reserved-key skips, another generated row, maximum explicit key, exact input preservation and exact final record content. The last case directly traverses the new add/delete/retry path rather than merely rechecking the old happy path.

I inspected the complete changed implementation, complete new test, Git diff/status/HEAD, historical maximum-key JSONs and current logs. Historical JSON outcomes show base `fea62c…` importing generated key 1 plus explicit 2^53 and prior candidate `0e8923…` rejecting the same package with maximum-generator ConstraintError and full rollback. Their reused generic scope strings are not evidence of a v2 run; these are historical regression artifacts only.

The owner-run v2 regression log records four Chromium tests passed. The broader frontend log reaches Node 424 passed and Vitest 61 passed. Typecheck/lint logs contain their respective commands without diagnostics; their process exit codes were not independently observed by this reviewer. No tests or application were launched in this read-only review, and no old-candidate broad browser outcome is promoted to v2 evidence. Fault injection for delete failure and actual native WKWebView/packaged migration remain unverified, not claimed successes.

## Input/evidence hashes

| Path | SHA-256 |
| --- | --- |
| /private/tmp/meetily-webview-mixed-id-fix-2026-10-04/frontend/src/services/indexedDBService.ts | 7911d22335c1761e0af3d9da82e6d68ca1970cb0b0f67abbaec54dcab1d7c62c |
| /private/tmp/meetily-webview-mixed-id-fix-2026-10-04/frontend/tests/indexeddb-migration-keys.spec.ts | def77eca2308a04eb2657fa4ac1daacb20c51c960ef911a6c8d1a6c3ea859f6f |
| /private/tmp/meetily-max-key-before.json | 86d5643ddac5a117c7f40c358f129cfd9b67b65f31a16406bda7d676324d28be |
| /private/tmp/meetily-max-key-after.json | a557862db8078cbd79532a2fc3361246b8c9214ba162daf94141ae65294bf5a4 |
| /private/tmp/meetily-mixed-id-regression-v2-2026-10-04.log | ca80e1190a963901218ead0e329d5a3fc47f842e6ce847092ee27fc6f93e6c91 |
| /private/tmp/meetily-mixed-id-frontend-v2-2026-10-04.log | 355b64c248db73512ed1ec884fff4719ebb50cb0343c771d52f493049e6b790e |
| /private/tmp/meetily-mixed-id-types-v2-2026-10-04.log | 056c34d0e1c885a6957c09198ddfc28dc35d7a9a60e6a54550056dd65b471b58 |
| /private/tmp/meetily-mixed-id-lint-v2-2026-10-04.log | f554b0edfdc1ec70e6005cf5fc6ad5d720033eba53a2ceeb7f5c5afc84fbb393 |

Checks performed by reviewer: read-only `git diff`, `git status --short`, `git rev-parse HEAD`, `cat`/`tail` source/evidence reads, SHA-256 and independent final Python hash comparison on local macOS. No product, test, Git, profile or prior report mutation. changed_paths: only this report.

No bounded SPEC blocker remains for this correction. Next action: finish and bind v2 broader-check terminal results before separately authorized integration. This PASS does not admit the new combined schema/coordinator, close FORK-123, prove a native migration/user flow, or establish release readiness.
