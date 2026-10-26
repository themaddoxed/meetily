# FORK-152 Task 0 independent SPEC admission review

Date: 2026-10-03. Result: **PASS for exact source census and bounded diagnostic classification; implementation admission BLOCKED pending route and clean checkout.** Critical 0 / Important 0 / Minor 0 new review findings. This is not implementation acceptance or permission to begin product edits.

## Observed snapshot and checks

Reviewer read-only checks: `git rev-parse HEAD main dev`, `git rev-list --left-right --count dev...main`, scoped source searches/reads, Python SHA-256 and `git show <revision>:<path>` comparisons. Root HEAD/main is `53f4415058fb01e38b8892a47ac036bfffa394ba`; dev is `6f2c4488edc6b9fffd45f8b2dd31bcd6079fe37e`; distance is 0/82. Root has pre-existing dirty overlays. No refs changed.

Independently compared all 18 admission paths against their recorded hashes, census, exact main bytes and `.worktrees/main-features-2026-10-02` diagnostic checkout: all match. Five differ on dev: ConfigContext, PreferenceSettings, browser mocks, tauri.conf.json and app/layout. Therefore source admission cannot silently transfer to current dev. The remaining dirty paths were not admitted for theme implementation.

Hashes were checked before and after source inspection, unchanged:

| Artifact | SHA-256 |
| --- | --- |
| `docs/superpowers/plans/2026-10-02-semantic-theme.md` | `ee091492f7fef678e8cf2c07f62c2e6615fe150c747a49d321273720f5f41d17` |
| `docs/superpowers/plans/2026-10-02-theme-census.json` | `aab13fd017dbdf0026d2980c214afe18695d976074653a76c5dec9818f551d68` |
| `docs/fork/evidence/theme-admission-2026-10-03.json` | `450123beb695780338fe10ecb698d8f5c241686025f9a1a0a3271815b047d990` |
| `.worktrees/integration-backups-2026-10-02/theme-order-diagnostic-2026-10-03.json` | `47b7ef364edbcbf2f5c049aabb4c9bdde765a02cc7310c12eed607889dc9dce9` |
| `/private/tmp/meetily-theme-order-diagnostic.cjs` | `cbb52eb4a8b0739b3202f976fe1c77914cb836f12a456cc075b88c88132e3d4c` |

## Source and diagnostic assessment

`frontend/src/app/layout.tsx:244` mounts ThemeScript once in head. `ThemeScript.tsx:8` reads the established key; lines 20–21 set root class and color-scheme. `ConfigContext.tsx:458–471` owns effective runtime resolution/system subscription and lines 498–500 native synchronization. Both BlockNote consumers use only `isDarkTheme` (`Editor.tsx:19,54`; `BlockNoteSummaryView.tsx:91,280`). PreferenceSettings exposes raw preference selection. Scoped source search found no additional theme resolver/key/owner; webMigration only preserves the existing key.

The read diagnostic reports stylesheet markup offset 129 before script offset 736; while first CSS was held, body was absent, root dark was false, color-scheme empty, events empty. After app ready, dark/color-scheme were correct, with first toggle before body and a repeated dark=true toggle after body. This supports the recorded **RED ordering-contract observation**, not visible incorrect paint. A repeated toggle with the same force is not proof of a corrective state mutation.

Diagnostic script lines 17–22 inspect a separate HTTP response; lines 29–32 sample once after root exists. It does not establish a sustained pre-script state or capture the first CSS-renderable frame. It collects pageerrors, not console hydration warnings; records toggle calls, not all class/style changes. Its successful process exit means observation completed, not assertions passed. The supplied environment is loopback Next 14.2.35 development / Playwright 1.61.0 Chromium 1228, stored dark under light OS only. Reviewer read script/output and verified source identity; did not rerun browser or independently attest the supplied process exit/environment installation.

## Required remaining gates

- **Route/source:** plan lines 16 and 141–147 require an agreed route and clean/idle admitted source/destination, absolute task/integration roots, exact commit/tree and rechecked census/callers. Current null clean root and pending route are explicit blockers. Rebind and review if the chosen source differs; preserve existing overlays.
- **Correction scope:** Task 2 already conditionally admits the smallest prepaint/order correction in the named owners after a saved focused RED. The diagnostic justifies prioritizing that test. It does not waive Task 0 or replace the named failing assertion. Layout already places the component in head; generated CSS order must be tested, not inferred from JSX. If fixing generated order needs another path (for example Next configuration), amend the plan and independently review first.
- **Matrix:** plan success criterion 1 and Task 2 still require six stored-mode/OS combinations on navigation and reload (12 transitions), root class/style mutation tracking before CSS release with body absent, first renderable rAF, no later corrective mutation, and pageerror plus console hydration collection. Live system changes, persistence/failure/native-intent contracts remain required.
- **Snapshot/contrast:** historical 30-pass/3-fail baseline stays RED. No snapshot updates are admitted. Any new path/snapshot change requires amendment/review. Real rendered-surface contrast and keyboard focus, owner checks and full required browser gates remain outstanding.
- **Acceptance:** fresh implementation SPEC/QUALITY review and canonical closeout remain separate. Native appearance, installed application and release checks are NOT VERIFIED; no release-ready claim follows.

`changed_paths`: only this review report. No product/status/Git edits, browser run, native action, or Graphify use. Blocker: unresolved route/clean source admission. Next action: obtain the route decision, admit the exact clean source, then execute the plan's focused characterization gates before any conditional correction.
