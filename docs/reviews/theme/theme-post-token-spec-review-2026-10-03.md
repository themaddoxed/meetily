> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme oracle guards and muted token: independent SPEC review

2026-10-03. **Bounded source SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding remains in the two targeted guards and minimal light-muted-token correction. Browser acceptance is pending/incomplete; this is not full theme acceptance.

## Prior findings and correction

The text-range path now validates each intermediate descendant ancestor, rejects nonunit opacity, rejects painted descendant backgrounds and delegates unsupported filters/effects to the existing fail-closed validator. This closes the nested-opacity/filter counterexamples without pretending to support their compositing. The added regression exercises both rejected cases.

The focus path now requires the full expanded indicator footprint inside the immediate parent's uniform inner rectangle, excluding border widths and a conservative maximum corner radius. This closes the no-shadow parent-edge/border escape that invalidated the assumed backdrop. A new negative fixture covers escape beyond the parent's surface; the positive sharp-ring fixture now gives that surface actual padding. Earlier ring-only overlap rejection, single indicator compositing and independent effective-mode pages remain intact.

`frontend/src/app/globals.css:92` changes only the light `--muted-foreground` from 45.1% to 43.9% lightness. This is the owning semantic token permitted after the preserved focused RED (System label ratio 4.34923168556426). Dark tokens, status meanings and other palette values are unchanged. Independent scalar arithmetic for neutral 43.9% over muted 96.1% gives approximately 4.548:1 instead of 4.351:1 for 45.1%; this supports the direction and size of the correction but is not rendered-surface acceptance.

The new separate Appearance test measures real description/selector spans with the same helper, isolates modes in separate pages and asserts effective root state. It supplements the still-present Home/Settings/Meeting Details and keyboard-focus contracts; it does not replace or waive their failures. Unresolved foreign paint and unsupported auto-outline remain unmeasurable, not measured low contrast or light islands.

## Snapshot and actual checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`, helper-path plan `e993b579f5b5adce9c418a76d30c479d21db893fc4a9c77e2c6c640bf925b005`. Read targeted helper/fixture changes and exact CSS diff; scoped status includes the new untracked helper directory, not merely Git diff's tracked-file statistics. Verified hashes:

- Helper: `c818561cc7b95d6c80034170f28d3a5d0d5bdc91a2961ce5b7417740c380a761`.
- Dark surfaces: `3465fdba856027cbd00fd561f136e1991ec72165f553e913286f3fd09f4c9df8`.
- Foundation: `3a65c9afb76235d5e9873b64c6537d48de60d2a779a8cc968a5e351ad37452ea`.
- Globals CSS: `a1f8eb1684c7a6e09e9476587e23f715c8689d233b7dbbf27826e8d512e96e99`.

Reviewer did not run browser tests. `/private/tmp/meetily-theme-post-token.log` was still running when reviewed; there were no terminal totals. It had advanced through the new helper fixtures into the actual light-surface case. This does not establish their final terminal PASS or the suite's exit. Typecheck/ESLint/audit exit 0 are implementer-reported, not independently rerun/log-reviewed here. The previous prepaint adapter PASS remains scoped to its reviewed snapshot.

## Remaining gates

Collect the completed exact-snapshot browser log, actual Appearance rendered values and guard regression results. Keep all Home/Meeting Details/focus/dark-scan failures open and classified accurately; further geometry support or production corrections need their own scoped proof/review. Require full remaining plan gates and independent QUALITY before closeout or integration. Native/accessibility/release remain NOT VERIFIED.

`changed_paths`: only `docs/fork/theme-post-token-spec-review-2026-10-03.md`. No source/test/status/Git edits or Graphify. Next action: finish and inspect the in-progress run; source findings are closed, runtime acceptance remains outstanding.
