> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# FORK-152 Task 0 — independent QUALITY review

Source-authored by `/root/plans_quality`, 2026-10-03.

**QUALITY PASS for the bounded source census and diagnostic characterization: Critical 0 / Important 0 / Minor 0.** No actionable defect blocks accepting these artifacts with their stated limits. **Implementation admission remains BLOCKED** on the agreed route and clean source/destination. Task 0, FORK-152 and theme acceptance are not complete. This is not acceptance of a corrected prepaint contract.

## Exact observed snapshot

- Root: `/Users/developer/Desktop/meeting-minutes`; main/base `53f4415058fb01e38b8892a47ac036bfffa394ba`, tree `bb8b8bdda6b53deaa926362f59b661410479ef78`. Dirty unrelated overlays remain present.
- dev: `6f2c4488edc6b9fffd45f8b2dd31bcd6079fe37e`; read-only `git rev-list --left-right --count dev...main` returns `0 82`.
- Plan `docs/superpowers/plans/2026-10-02-semantic-theme.md`: SHA-256 `ee091492f7fef678e8cf2c07f62c2e6615fe150c747a49d321273720f5f41d17`.
- Census `docs/superpowers/plans/2026-10-02-theme-census.json`: SHA-256 `aab13fd017dbdf0026d2980c214afe18695d976074653a76c5dec9818f551d68`.
- Admission `docs/fork/evidence/theme-admission-2026-10-03.json`: SHA-256 `450123beb695780338fe10ecb698d8f5c241686025f9a1a0a3271815b047d990`.
- Diagnostic script `/private/tmp/meetily-theme-order-diagnostic.cjs`: SHA-256 `cbb52eb4a8b0739b3202f976fe1c77914cb836f12a456cc075b88c88132e3d4c`.
- Diagnostic result `.worktrees/integration-backups-2026-10-02/theme-order-diagnostic-2026-10-03.json`: SHA-256 `47b7ef364edbcbf2f5c049aabb4c9bdde765a02cc7310c12eed607889dc9dce9`.

Python SHA comparisons and `git show <commit>:<path>` independently confirm all 18 allowlist entries match the census, admission, main bytes and diagnostic fixture checkout `.worktrees/main-features-2026-10-02`. All 18 census caller-path hashes also match current root. The five documented dev differences are ConfigContext, PreferenceSettings, tauri-browser-mocks, tauri.conf and layout. A source match for these theme paths does not make either dirty checkout clean or establish whole-build equivalence.

## Ownership and existing failure characterization

Targeted caller searches and surrounding source confirm one ThemeScript mount (`frontend/src/app/layout.tsx:244`), the existing `themeMode` key (`frontend/src/lib/theme.ts:11`), pre-hydration script ownership and one runtime resolver in `ConfigContext.tsx:455-471`. Explicit modes do not subscribe to later system changes; system mode does. Native synchronization remains separate best-effort serialization. PreferenceSettings selects the raw mode; both BlockNote consumers use `isDarkTheme` only. webMigration preserves the opaque preference value and adds no resolver. No new live resolver/key/caller was found.

`frontend/tests/theme-foundation.spec.ts:75` overwrites its single early-toggle flag on every matching call. The diagnostic records first toggle with body absent and a later identical dark toggle after body exists. This supports the documented instrumentation error. A repeated `toggle('dark', true)` is not by itself evidence of a corrective visual change; future tests must distinguish observed calls from actual changes to effective theme.

The observed dark-stored/light-system navigation reports root dark=false and empty colorScheme while CSS is held, then correct dark state after readiness. The separately fetched HTML reports stylesheet index 129 before script index 736. No visible flash, production bundle behavior or installed-native appearance is established.

## Diagnostic limitations and corrective-test requirements

These are limits of an explicitly non-asserting diagnostic, not accepted product behavior:

- At diagnostic script lines 30-32, the sample occurs immediately after a CSS request and root availability. It does not wait for a parser/script checkpoint. A pre-theme sample can race subsequent script execution even in a corrected implementation. Treat this as the observed instant, not conclusive proof that the theme could never become correct while CSS remained held. The planned corrective test must use a bounded wait for the required pre-body theme event/state before releasing CSS, fail if it cannot occur and release the route in cleanup.
- Lines 17-22 inspect a separate HTTP response rather than the navigation response. The substring search does not assert exactly one theme script or fail explicitly for a missing marker; stylesheet detection is simplistic. A future assertion suite must bind ordering to the actual navigation/reload response, require both resources, check count/order and match the held stylesheet to the intended app CSS.
- Only the first matching CSS request is held. The current probe collects toggle calls but does not instrument all root class/style mutations, console hydration diagnostics or the first CSS-renderable animation frame. It does not cover the twelve-transition matrix. Page errors are recorded, not asserted. An exit 0 is collection success only; `RED_PREPAINT_ORDER_CONTRACT` remains a diagnostic classification, not a failing assertion-suite result.
- Local Next 14.2.35 `dist/server/app-render/get-layer-assets.js:59-68` emits stylesheets with resource precedence. This is consistent with the observed ordering and explains why JSX head placement alone is not sufficient evidence of final HTML order. It does not prove the permitted source seam cannot solve the requirement. Any required path outside the 18-path allowlist, Next configuration change or contract relaxation must trigger amendment and fresh review, as the plan already requires.

Plan line 195 calls for the stronger held-stylesheet matrix, root mutation events, first-frame state and hydration diagnostics; line 196 binds single ownership and conditional correction. The probe is useful input to those tests, but must not be promoted unchanged into their acceptance oracle. Baseline 30 passed / 3 failed remains RED; no snapshot updates are justified by this review.

## Checks, boundaries and next action

Reviewer actions in the local Darwin workspace: read-only Git status/ref-content comparison, Python SHA validation, scoped caller searches, source/plan/census/PRD/diagnostic reading and local installed-Next source inspection. The coordinator's observed diagnostic exit 0 is not a reviewer browser run. No implementation, browser rerun, native action, release check or Graphify use occurred.

`changed_paths`: `docs/fork/theme-admission-quality-review-2026-10-03.md` only. Plan, census, admission, diagnostic and 18 theme paths rechecked unchanged after review.

`blockers`: clean/idle task and integration roots plus an agreed dev/feature route have not been admitted. `next_action`: resolve that route, record exact clean commit/tree and absolute roots, rebind on any drift, then implement the planned failing tests before conditional product corrections. Remaining acceptance gates include twelve prepaint transitions, hydration, focus/actual-surface contrast, explained sidebar snapshot drift, scoped/full checks, independent implementation review and separately authorized native accessibility/appearance verification. Native and release readiness remain NOT VERIFIED.
