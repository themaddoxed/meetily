# Theme post-token oracle and light contrast — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the bounded source correction: Critical 0 / Important 0 / Minor 0.** The last reported descendant-opacity false PASS is closed, and the minimal light-token change follows the independently reviewed System-text RED. This is source acceptance with runtime verification still incomplete, not full contrast/theme acceptance.

## Exact snapshot

Task root `.worktrees/semantic-theme-2026-10-03`, base `53f4415058fb01e38b8892a47ac036bfffa394ba`.

| Reviewed path | SHA-256 before/after |
| --- | --- |
| `frontend/tests/helpers/rendered-contrast.ts` | `c818561cc7b95d6c80034170f28d3a5d0d5bdc91a2961ce5b7417740c380a761` |
| `frontend/tests/theme-dark-surfaces.spec.ts` | `3465fdba856027cbd00fd561f136e1991ec72165f553e913286f3fd09f4c9df8` |
| `frontend/tests/theme-foundation.spec.ts` | `3a65c9afb76235d5e9873b64c6537d48de60d2a779a8cc968a5e351ad37452ea` |
| `frontend/src/app/globals.css` | `a1f8eb1684c7a6e09e9476587e23f715c8689d233b7dbbf27826e8d512e96e99` |

Git status shows these changes alongside the previously reviewed layout/ThemeScript adapter. No PreferenceSettings focus-class change is present. Review considered the exact helper/caller source and one-line globals.css diff; no product changes by reviewer.

## Findings resolved and token scope

The text-range loop now validates each ancestor between a sampled text parent and the supplied element, rejects non-unit opacity, and rejects a nontransparent intervening background. The same validation checks unsupported filters/masks/blend/text-shadow effects. The new regression explicitly supplies opacity-only and filter-only descendants; both must throw rather than return a misleading ratio. This resolves the concrete previous false-pass counterexample without pretending to composite unsupported descendant effects.

Focus backing is additionally constrained: the entire expanded indicator rectangle must lie inside the parent background beyond its border/corner bounds before parent color is used. A new outside-parent fixture checks rejection, while the sharp-ring positive fixture provides enough padding. The helper remains conservative; unresolved foreign paint, ambiguous UA auto outlines and unsupported effects are errors, never evidence of bad product contrast or a compliant sample.

`globals.css:92` changes only light `--muted-foreground` from 45.1% to 43.9% lightness. The light System label previously measured 4.34923168556426 against the muted container. Darkening the shared semantic foreground directly addresses that pair without scattered feature overrides, changes to the dark token or threshold weakening. All consumers of the light token are affected, so broader regression results still matter.

The new Appearance case uses an independent Page per mode, confirms actual root class/colorScheme, and measures the real description plus all three button label spans using the same helper. This isolates the intended corrected pair from unrelated unresolved Meeting Details geometry while preserving the original broader real-element tests. It is a scoped additional check, not a replacement for their acceptance.

## Observed checks and limits

Reviewer actions: read-only local Darwin source/diff/status inspection and SHA checks. Available `/private/tmp/meetily-theme-post-token.log` was a live 56-test run when inspected, with helper fixtures progressing into real-element tests. No terminal aggregate or final token GREEN is claimed by this report. Typecheck/ESLint/theme-audit exit 0 are coordinator-reported and were not rerun by reviewer. No reviewer browser/native execution or screenshot inspection occurred.

The prior outside-ring, ancestor-shadow, mode-isolation and descendant-effect regressions give specific oracle coverage; this remains a deliberately restricted computed-color evaluator, not arbitrary CSS/compositor proof. Current background scanning, unresolved foreign sidebar paint and UA-auto focus indicators still require separate resolution/evidence. A source PASS does not convert those unknowns into acceptable contrast, and does not justify a production focus redesign.

`changed_paths`: `docs/fork/theme-post-token-quality-review-2026-10-03.md` only. `blockers: []` for this bounded source QUALITY gate. Runtime acceptance remains pending the completed scoped Appearance result, affected theme regression results and unresolved required measurements. `next_action`: retain exact final logs/ratios and classify genuine measured failure versus NOT_MEASURABLE; keep full theme/native/release tasks open until their own gates are complete.
