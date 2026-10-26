# Theme diagnostic v2 — independent QUALITY supplement

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for this diagnostic refinement: Critical 0 / Important 0 / Minor 0.** The two specific v1 limitations are addressed within this bounded observation. This is not prepaint acceptance or implementation admission; the clean-source/route gate remains blocked.

## Snapshot and checks

Base: `main@53f4415058fb01e38b8892a47ac036bfffa394ba`, with preserved dirty overlays. Read the complete v2 script/result in the local Darwin workspace, hashed both before/after, and independently compared all 18 admitted theme hashes against root and the diagnostic fixture checkout: all match. No reviewer browser execution occurred; diagnostic exit 0 is coordinator-observed.

- `/private/tmp/meetily-theme-order-diagnostic-v2.cjs`: SHA-256 `4394d1c212506c19df392278c7e746139ca199ed52c58ca538fae5e9e631daef`.
- `.worktrees/integration-backups-2026-10-02/theme-order-diagnostic-v2-2026-10-03.json`: SHA-256 `2f694133d22d63de46c9d59f88b8547972d5ea2ce511337a22bd30051b3c156f`.
- v1 script/result, admission JSON and previous QUALITY review retain their prior hashes (`cbb52eb4…`, `47b7ef36…`, `450123be…`, `d2e80fb7…` respectively).

## Assessment of the two changes

At script line 32, v2 waits up to 3000 ms for both dark root class and dark colorScheme before releasing the held stylesheet. The retained result says that state was not observed during the bounded wait; the subsequent snapshot still has body absent, dark=false, empty colorScheme and no toggle events. This addresses the immediate-snapshot race in v1. It does not establish impossibility at every later time or a precise parser-blocking root cause.

At lines 34-38, v2 obtains HTML from the Response returned by the actual `page.goto`, rather than relying solely on a separate request. Its navigation-specific fields record HTTP 200, stylesheet index 129 and theme-script index 736. Thus actual navigation markup corroborates the stylesheet-before-script ordering. The original separate-request fields remain in the report but are no longer the sole basis for that claim.

After CSS release, the diagnostic records correct dark state, first dark toggle with body absent and a later toggle with body present. This remains consistent with the old test overwriting its first-event flag; the later identical call is not proof of a corrective visual change.

## Residual limits and next action

The broad rejection handler on `waitForFunction` maps every error to false, not just timeout. Consequently the strict evidence statement is “the awaited state was not observed”; future assertion code must distinguish TimeoutError from instrumentation/context failures and preserve the reason. The successful subsequent evaluation/navigation and empty pageErrors provide context but do not independently prove why that promise rejected.

The script remains a single dark-stored/light-system diagnostic with no assertion suite. It does not fail on a false theme result, missing/count-invalid script markers, hydration console diagnostics, or an incorrect first CSS-renderable frame. It observes toggle calls, not every class/style mutation; only the first matching CSS request is held. It does not cover reload or the twelve-transition matrix, production bundle, visible flash, contrast or native appearance. Exit 0 denotes successful collection only.

`changed_paths`: this supplement only. `blockers`: clean source/destination and agreed route are still absent. `next_action`: preserve this stronger bounded observation, then implement the planned fail-closed matrix only after admission. Keep route cleanup, exact resource/order assertions and differentiated instrumentation errors in that corrective test. Previous review and all product sources remain unchanged.
