> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme matrix RED — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY FAIL for the complete future acceptance oracle: Critical 0 / Important 1 / Minor 0.** The observed held-CSS RED is supported independently of the finding below. Production ordering correction is justified by that strict-contract failure, but this test must not be accepted as a complete no-correction/prepaint proof until its mutation oracle is repaired.

## Important finding

**Queued mutations are all labelled with the final live state, masking intermediate theme corrections.** In candidate `frontend/tests/theme-foundation.spec.ts:48-55`, MutationObserver records `snapshot()` at callback delivery for every queued class/style mutation. At line 112, the acceptance assertion checks those snapshots and ignores the saved `oldValue`. A post-body sequence such as `root.style.colorScheme='light'; root.style.colorScheme='dark'` in one synchronous turn under expected dark yields two records both labelled dark. Direct className/setAttribute or classList.add/remove changes similarly bypass the synchronous toggle spy. The test can therefore pass despite a forbidden later incorrect state and correction, contrary to plan line 195.

Repair: preserve actual attribute transitions using each record's old value and the next same-target/same-attribute record's old value (final attribute for the last record), or instrument relevant writes synchronously. Track initial and subsequent effective state, distinguish no-op writes from real changes, and reject an incorrect/corrective transition through readiness. Add a controlled regression proving the same-turn wrong-then-right class/style sequence is detected while harmless repeated correct-state writes pass. MutationObserver bodyAbsent is also a callback-time observation, not automatically the exact mutation-time timestamp; preserve the synchronous pre-body evidence separately.

## Exact snapshot and observed RED

Task root: `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`. Git reports only `frontend/tests/theme-foundation.spec.ts` modified. No production or snapshot changes are in this diff.

- Test SHA-256: `39a305239efd6bed0b63f0216b449c32441c184fd7855bd5629a43149bff3ad1`.
- Evidence `docs/fork/evidence/theme-matrix-red-2026-10-03.json`: `02341ec784c5e482d24d2c8b564eb16f7d4c0ccc6cfa8a598ab64ad0c090cf66`.
- Round2 log `.worktrees/integration-backups-2026-10-02/theme-matrix-red-round2-2026-10-03.log`: `b319419f51b5a12e49fbfb770c92239140dba72ea91037396f780df184e89be3`.

Read complete test diff and round2 assertion results: all six mode/OS rows fail at line 97 after the bounded held-CSS poll, with body absent, empty colorScheme and dark=false. This fails even expected-light rows because colorScheme remains unset. The failure is before the flawed later-mutation acceptance assertion and is not explained away by that flaw. The round2 log has six failures; coordinator exit 1 is consistent with them. No reload case or downstream first-frame/markup/hydration assertion was reached. Round1 cleanup errors remain historical and are not counted as the root product failure. The line-123 load wait addresses the observed teardown race, though any future cleanup exception must still be reported separately from the primary assertion.

The old-test `??=` change at line 200 preserves the first toggle observation rather than overwriting it. The new route is installed before navigation, checks HTTP status, requires CSS interception and polls under the gate; actual navigation response is retained for later markup checks. These are appropriate fail-closed improvements. The diagnostic failure proves violation of the selected strict ordering criterion in this environment, not an observed visible wrong-theme frame.

## First-frame and matrix boundaries

The stylesheet load listener schedules an rAF snapshot for the exact held URL. This is a useful browser checkpoint, not a screenshot or compositor measurement. It assumes that held resource corresponds to the first app stylesheet and does not by itself characterize every outstanding stylesheet. The actual-response check requires one recognizable script before the first app stylesheet, but this path has not executed successfully yet. Six tests each contain navigation followed by reload; a failed navigation aborts its reload, so the available evidence is six navigation failures, never twelve completed transitions. Final typecheck after the cleanup edit remains required.

## Bounded correction investigation

Inspected installed Next 14.2.35 `get-layer-assets.js:59-68`: CSS links have React resource precedence. Bundled React `react-dom-server.edge.development.js:5489` writePreamble flushes styles before hoistable script chunks. Next `node-web-streams-helper.js:189-232` inserts useServerInsertedHTML content at the closing head. Reordering the existing JSX child or using that insertion hook is therefore not sufficient evidence of a fix.

There is a plausible allowed-file experiment: React `pushStartHead` at line 4254 sends root-head generic output to headChunks; `pushStartGenericElement` at line 4427 writes trusted innerHTML there; writePreamble emits headChunks before resource styles. Serializing the same trusted theme script into root-head innerHTML through existing layout.tsx/ThemeScript.tsx could therefore affect ordering without Next configuration or dependency edits. This is a source-derived candidate, not an executed or accepted fix. It needs proof of unique script ownership, hydration safety and metadata/resource preservation. Because the current plan explicitly describes a single ThemeScript component mount, any changed serialization interface/ownership assertion must be explicitly reconciled in the plan and independently reviewed; do not silently weaken that contract. No impossibility claim or production change was made here.

`checks`: local Darwin read-only diff/status, SHA checks, actual log inspection and installed framework source analysis; no reviewer browser/test execution. `changed_paths`: this report only. `blockers`: Important mutation-oracle false negative; unresolved production ordering correction and unexecuted downstream matrix gates. `next_action`: repair the oracle and its detection regression, then investigate the bounded source seam under the unchanged strict success criteria and applicable amendment review. Full theme, native and release acceptance remain NOT VERIFIED.
