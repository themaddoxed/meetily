# Existing IDB mixed-ID fix v2 — independent QUALITY review

**QUALITY PASS for the exact two-file v2 diff; C0 / I0 / M0.** No actionable finding remains in the reviewed allocator or its focused tests. Broader v2 checks are not accepted by this report until their own terminal evidence is available. This is not combined-coordinator, native migration or release acceptance.

## Historical regression retained

The earlier explicit-first implementation was unsound for the existing legacy API's numeric-ID domain. Retained actual Chromium observations show the original service accepts a generated row followed by explicit `2 ** 53`, while the explicit-first candidate rejects it with `Maximum key generator value reached`. My earlier PASS missed this boundary and is superseded for current acceptance; the old report is preserved as history. The new code must retain that valid case without weakening genuine destination-collision rejection.

## Source review

`frontend/src/services/indexedDBService.ts:773–806` allocates all generated rows before explicit inserts. The reserved-key set comes from the package's explicit IDs. Only after a generated `add` succeeds does the handler consider deletion, and the deleted key is exactly that successful request's result (`:794–800`). Because this is `add`, not `put`, success means this transaction inserted the row; it cannot authorize deletion of a pre-existing neighbor. No untrusted explicit key is independently passed to delete.

On a reserved result, deletion completes before retrying the same generated row. Otherwise the next generated row is queued. Each next request is scheduled inside the current request callback; there is no timer, promise/Web Crypto await or transaction-completion gap. Existing transaction abort/error handling covers failed adds and the new failed delete. Explicit rows and the operation marker are queued only after generated allocation completes. The marker can be queued before explicit request results arrive, but the single readwrite transaction and its completion/abort handlers retain atomicity: an explicit collision cannot leave generated rows or a durable successful marker behind.

The allocator does not advance to a large package explicit ID until generated rows have been assigned. It skips reserved generated keys without changing their explicit owners. The small local state machine is bounded by the generated rows plus reserved keys encountered as the monotonic generator advances; callbacks do not recursively grow a synchronous call stack. Existing generator exhaustion or genuine invalid/colliding explicit keys still fail safely. No `Number.isSafeInteger` restriction is added to the legacy importer, so the valid explicit `2 ** 53` compatibility case remains admitted. Stricter new combined-protocol key policy is a separate contract.

Filtering creates new arrays, and each stored row remains a spread copy. Snapshot array order, contents and package checksum inputs are unchanged. Matching-marker retries, meeting insertion, operation identity and native/profile authority are unaffected. Empty/generated-free input reaches the existing explicit/marker path immediately. No additional documented-standard violation or speculative maintainability smell was established.

## Verification reviewed

Read the full four-test file and retained owner-run real-IDB log. The tests cover:

- Mixed generated/explicit ID 1, idempotent retry, unchanged input order, page reload and retained records/marker.
- Genuine explicit collision with exact neighboring snapshot preserved and new marker absent.
- Explicit `2 ** 53` with a generated row still receiving ID 1.
- Multiple reserved low keys plus `2 ** 53` and two generated rows, asserting exact complete stored records and unchanged input snapshot.

`/private/tmp/meetily-mixed-id-regression-v2-2026-10-04.log` ends with **4 passed (13.4s)**; parent supplied terminal exit 0. Its independently computed SHA-256 is `ca80e1190a963901218ead0e329d5a3fc47f842e6ce847092ee27fc6f93e6c91`. These are retained owner-run results, not my browser execution.

Not separately exercised by these four tests: forced delete-request failure, marker-write failure/quota, process/power-loss durability, or browser key-generator exhaustion already present before import. The new failure handler uses the same transaction-abort mechanism; no confirmed data-preservation defect was found. A combined reserved-skip-then-late-collision fixture would strengthen coverage, but the inspected source and existing atomic transaction boundary do not establish a blocker requiring it for this correction.

The tests invoke the current commit primitive with fixture identities. They do not prove public package export/import UI, preferences migration, versioned generated-key mapping/readback, SQLite/FS receipts or native WebView behavior. Earlier v1 broad-check counts are not carried forward as v2 success. Broader v2 lint/types/unit/critical-browser checks were reported running and are outside this report's verified terminal evidence.

## Exact snapshot and review actions

Candidate `/private/tmp/meetily-webview-mixed-id-fix-2026-10-04`, HEAD `53f4415058fb01e38b8892a47ac036bfffa394ba`. Read-only status showed exactly the modified service and untracked new test. Independently checked before/after:

- Service SHA-256: `7911d22335c1761e0af3d9da82e6d68ca1970cb0b0f67abbaec54dcab1d7c62c`.
- Test SHA-256: `def77eca2308a04eb2657fa4ac1daacb20c51c960ef911a6c8d1a6c3ea859f6f`.

Historical max-key evidence inspected: `/private/tmp/meetily-max-key-before.json` SHA `86d5643ddac5a117c7f40c358f129cfd9b67b65f31a16406bda7d676324d28be`, binding original service `fea62c36…`; `/private/tmp/meetily-max-key-after.json` SHA `a557862db8078cbd79532a2fc3361246b8c9214ba162daf94141ae65294bf5a4`, binding superseded explicit-first service `0e89234b…`. Neither is presented as a v2 runtime test.

Own checks were scoped read-only Git diff/status/HEAD inspection, source/test/log reads and SHA-256 calculations. No source, Git ref, profile, native app or existing evidence mutation occurred. `changed_paths`: only this new report; the previous report remains unchanged.

Next action: finish and bind broader v2 checks to these exact hashes, then use the separately authorized integration route. No full-project, native or release readiness follows from this bounded code-review PASS.
