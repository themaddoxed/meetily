# FORK-123 schema/ownership v2 — bounded I1 re-review

Status: PASS for closure of prior SPEC I1 only; C0 / I0 / M0. No actionable finding remains in this correction. Product implementation remains NOT_ADMITTED; this is not a full-plan or FORK-123 acceptance.

The exact diff adds explicit-ID-first, ID-less-second insertion scheduling at `docs/superpowers/plans/2026-10-04-webview-schema-ownership-v2.md:94–102`, retaining original packageIndex and unchanged v1 package array/checksum. This prevents an automatically generated key from consuming a later explicit package key. It requires generated-key policy checks, one mapping per original store/index, key-type validation, unique keys within each store, and complete transaction rollback on record/marker failure. The complete marker is queued synchronously from the final successful add, with an explicit empty-snapshot case. Required regressions at lines 161–163 cover mixed IDs, readback, bounds, empty marker and genuine collision rollback. These statements close the design omission identified in the preserved v1 review.

## Evidence and scope

I inspected the source-bound probe script, its JSON and log, and matched recorded source hashes to current root sources. The script transpiles those actual service files without rewriting their implementation, uses separate ephemeral Chromium contexts at a loopback origin, and reads current IDB through the service API. Original package/import flow reports ConstraintError with empty meetings/transcripts and no operation marker. Reordered input through the existing commit primitive reports explicit key 1 and generated key 2, two stored transcripts and a marker. A genuine pre-existing key collision aborts the import while retaining the neighbor and leaving the new operation marker absent. The recorded Chromium version is 149.0.7827.55; the success JSON and log agree with the script's assertions and recorded exit 0.

These are inspected owner-run artifacts, not a reviewer-run browser test. The reordered case deliberately bypasses the public importer and uses its current IDB commit primitive: it verifies the scheduling remedy, not a fixed public importer. That importer remains unchanged and still exhibits the original failure. The probe does not implement or verify the proposed versioned persistent key mapping, full content/readback protocol, generated-key bounds, empty marker path or crash recovery. It is sufficient supporting evidence for this narrow design correction, not product acceptance.

Reviewer checks: `diff -u` v1/v2, `sed`/`nl` inspection of added contract and complete probe script/output, `shasum -a 256`, and independent Python SHA-256 final recheck. All seven inspected input hashes remained stable. Environment: local canonical repository and existing temporary artifacts on macOS; no application/test execution or user-profile access by this reviewer.

## Exact inspected inputs

| Path | SHA-256 |
| --- | --- |
| docs/superpowers/plans/2026-10-04-webview-schema-ownership-v2.md | 86fa8fe7c9932d2b7794ae7330bfaea6ff3a90a177b850242bc9833e8643b53a |
| docs/superpowers/plans/2026-10-04-webview-schema-ownership.md | da7c4686f4159c453eac1b9a1d1fcd3dd823a4226dd7bb9a187c960c225f0908 |
| frontend/src/services/webMigration.ts | 974740764867a367d695ab1166e52f0e2a47d5bb8bb3761ca0f90482adf04ebf |
| frontend/src/services/indexedDBService.ts | fea62c36fe851b7c29cbe8948fb545f838048d5f5ccdefb07af90371c273a6aa |
| /private/tmp/meetily-webview-mixed-id-probe-2026-10-04.cjs | ff5a6926be581cda37c4878b43d67ed66721cb8911e74df6ab79e0a6478f6751 |
| /private/tmp/meetily-webview-mixed-id-probe-2026-10-04.json | a03c69af98b8cf70706d24ce54893ecb712280af0413120afc685d3de19f8322 |
| /private/tmp/meetily-webview-mixed-id-probe-2026-10-04.log | 731d2816b132fa1cff7f296d38b22c1e5323cc2bef00cea7473ecfb82ce180fd |

changed_paths: only `docs/fork/webview-schema-ownership-v2-spec-review-2026-10-04.md`. Original candidate/review and all product, test, plan, Git and profile paths preserved.

Blockers outside this closure: clean-source admission, actual identity-pair/producer and plugin/CSP/startup gates remain open, as do executable schema validation, persistent mapping and native/browser crash/user-flow verification. Next action: retain I1 as closed at design level and carry the prescribed regressions into the separately admitted implementation. No full-plan, native, upgrade or release PASS is implied.
