# FORK-123 mixed transcript-ID scheduling — bounded QUALITY review

**Status: mixed-ID design correction accepted; C0 / I0 / M0 new actionable findings. Implementation remains NOT_ADMITTED.** This is a review of the scheduling/mapping contract and retained diagnostic evidence, not a product fix or full-plan admission.

## Assessment

The added rule in `2026-10-04-webview-schema-ownership-v2.md:94–105` queues all explicit transcript IDs before generated-ID records while retaining original packageIndex and v1 package ordering/checksum. This prevents an auto-generated key from consuming a later explicit ID in the same package. It does not overwrite genuine destination collisions or renumber an explicit source key. The current `indexedDBService.ts:765–772` simply queues `add` in input order; therefore the original empty-target failure is consistent with the inspected source.

The proposed complete mapping is created in the same records/marker transaction, uses returned IDB keys, and is queued synchronously from the final successful add callback. The empty-snapshot rule avoids waiting for a nonexistent callback. Proper key types, generated-key bounds, unique per-store target keys and one mapping for every original package store/index are now explicit. Hash work remains outside the active transaction. Failed key validation must abort the transaction just like a failed add or marker request, including generator state; this is covered by the required real-IDB regression boundary, not demonstrated by the current scheduling probe.

The fix preserves content/readback semantics from the previous candidate: identical ID-less transcripts remain distinct entries identified by packageIndex, generated IDs are excluded from original content projection but checked through their mapping, and explicit source IDs remain included. No change to legacy browser v1 checksum encoding or its sorting is authorized.

## Retained Chromium diagnostic

Independently read the complete probe script, JSON and log, and recomputed the current two source hashes against the recorded source binding. The script transpiles actual current public adapter/IDB sources, runs three separate ephemeral Chromium contexts on a loopback origin and blocks other origins. The retained log/JSON report Chromium `149.0.7827.55`, exit 0:

- Original package order: ConstraintError, no meetings/transcripts or operation marker committed.
- Explicit-first snapshot passed through the existing commit primitive: explicit ID 1 and generated ID 2, marker committed; package array remains generated-then-explicit.
- Genuine explicit-ID collision: attempted import rolled back, neighbor record retained, new marker absent.

These observations support the proposed scheduling decision. I did **not** rerun the browser probe or overwrite its artifacts. Its explicit-first branch bypasses the public package importer and passes a reordered snapshot directly to the current IDB commit primitive. It therefore does not prove a product importer fix, the new versioned persistent mapping, packageIndex-to-key readback, empty snapshots, generated-key bounds, quota/denial, or crash/reopen behavior. LocalStorage input is empty, so this probe is not combined-preference recovery evidence. The document correctly requires further real-IDB regressions rather than claiming those scenarios already passed.

## Exact snapshot and limits

Input SHA-256 before/after: `86fa8fe7c9932d2b7794ae7330bfaea6ff3a90a177b850242bc9833e8643b53a`.

Retained evidence:

- `/private/tmp/meetily-webview-mixed-id-probe-2026-10-04.cjs`: `ff5a6926be581cda37c4878b43d67ed66721cb8911e74df6ab79e0a6478f6751`.
- `/private/tmp/meetily-webview-mixed-id-probe-2026-10-04.json`: `a03c69af98b8cf70706d24ce54893ecb712280af0413120afc685d3de19f8322`.
- `/private/tmp/meetily-webview-mixed-id-probe-2026-10-04.log`: `731d2816b132fa1cff7f296d38b22c1e5323cc2bef00cea7473ecfb82ce180fd`.
- Current `frontend/src/services/indexedDBService.ts`: `fea62c36fe851b7c29cbe8948fb545f838048d5f5ccdefb07af90371c273a6aa`.
- Current `frontend/src/services/webMigration.ts`: `974740764867a367d695ab1166e52f0e2a47d5bb8bb3761ca0f90482adf04ebf`.

Checks: exact `diff -u` against preserved original, full diagnostic script/output inspection, and Python/hashlib source binding validation. No product, Git, profile, native application or test-artifact mutation occurred. `changed_paths`: only this report. No new documented-standard violation or speculative maintainability finding was established.

Next action: implement and verify the exact mapping/transaction contract only after remaining clean-source, schema/ownership and plugin/CSP admission gates are settled. Actual producer/pair, native store continuity, complete combined migration and release acceptance remain separate and open.
