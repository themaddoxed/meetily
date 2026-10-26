> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# FORK-152 clean Task 0/1 admission: independent SPEC review

2026-10-03. **SPEC PASS — Critical 0 / Important 0 / Minor 0.** No actionable finding. The clean-route/source prerequisites for beginning planned characterization/RED tests are met in the observed snapshot. The aggregate Task 0 gate still requires the separately assigned independent QUALITY acceptance; this SPEC report does not substitute for it. No implemented behavior or theme acceptance is claimed.

## Scope and observed snapshot

- `$MEETILY_REPO_ROOT`: `/Users/developer/Desktop/meeting-minutes`; canonical documentation and unrelated existing overlays remain outside the product task.
- `$THEME_TASK_ROOT`: `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`, branch `feature/semantic-theme-2026-10-03`.
- `$ADMITTED_INTEGRATION_ROOT`: `/Users/developer/Desktop/meeting-minutes/.worktrees/dev-integration-2026-10-03`, branch `dev`.
- Both roots independently report clean `git status --porcelain=v1 --untracked-files=all`, HEAD `53f4415058fb01e38b8892a47ac036bfffa394ba`, tree `bb8b8bdda6b53deaa926362f59b661410479ef78`. Backup `themaddoxed/dev-before-main-2026-10-03` resolves to `6f2c4488edc6b9fffd45f8b2dd31bcd6079fe37e`.

The parent capsule records explicit user option 1 authorization for the dev fast-forward and feature-from-dev route. Reviewer verified resulting refs/worktrees; did not execute the transition or independently reconstruct its command history. Current clean state establishes Task 1's resulting isolated state; it does not prove historical branch/path absence before creation.

## Checks

Reviewer ran read-only Git status/revision/tree/branch checks and Python SHA-256 comparisons on both roots. All 18 exact census entries match in both roots before and after inspection. Scoped source search confirmed one ThemeScript mount (`frontend/src/app/layout.tsx:244`), existing `themeMode` key (`frontend/src/lib/theme.ts:11`), runtime owner in ConfigContext (`:460–471`), and effective-theme-only BlockNote projections (`Editor.tsx:19,54`; `BlockNoteSummaryView.tsx:91,280`). No new theme source/caller/path contract was observed; webMigration is preference preservation only.

Installed package JSON independently confirms Next 14.2.35, React 18.3.1, Playwright 1.61.0, TypeScript 5.9.3. Task `frontend/pnpm-lock.yaml` SHA-256 is `3a181a8c8a6cbbe169c9b0627e5c4c17123d818cd454c6de8ee6ce2cfd091418`, identical to exact base. This proves installed version metadata and unchanged lock, not a reviewer-run install. The evidence records two failed offline attempts and final exit 0 with 906 reused/2 downloaded/909 added and ignored unrs-resolver build scripts; those historical process results were not independently rerun or log-attested in this review. Nothing here converts those failures into successful attempts. Runtime dependency adequacy remains subject to the planned checks.

Frozen artifacts, unchanged at final recheck:

- `docs/fork/evidence/theme-clean-admission-2026-10-03.json`: `67d1a6b765a8fa1ce615034a2b526bb2fd6777f2a8ea3f2abc161faf64de2b93`
- `docs/superpowers/plans/2026-10-02-semantic-theme.md`: `ee091492f7fef678e8cf2c07f62c2e6615fe150c747a49d321273720f5f41d17`
- `docs/superpowers/plans/2026-10-02-theme-census.json`: `aab13fd017dbdf0026d2980c214afe18695d976074653a76c5dec9818f551d68`

## Admission boundaries and next action

Task 0/1 SPEC admission permits starting Task 2's planned tests in the exact task root once matching QUALITY acceptance is recorded. The unchanged accepted plan requires saving focused named RED evidence before any conditional production correction; prior bounded diagnostics and historical suite results are baseline observations, not first-paint, hydration, focus or contrast acceptance. Unique output directories and preserved failure attachments remain mandatory.

No snapshot update, new path, package/lockfile change, resolver or broader refactor is admitted. Source/caller/contract drift requires amendment and review. The full 12-transition prepaint matrix, hydration diagnostics, rendered contrast/focus, native-intent and subsequent regression gates remain open. Any conditional fix stays inside the exact 18-path allowlist.

Keep product edits and later selective staging in the task worktree; do not import or stage canonical-root overlays. Commits, destination integration and any push retain their explicit authorization gates after exact-diff tests and SPEC/QUALITY review; choosing the route grants none of those later actions. Recheck clean destination/base before integration. Native GUI, installed artifact, accessibility and release gates remain NOT RUN/NOT VERIFIED; no task closeout follows this admission.

`changed_paths`: only `docs/fork/theme-clean-admission-spec-review-2026-10-03.md`. No product, task status or Git changes. No Graphify. Next action: combine this exact-snapshot SPEC PASS with QUALITY, then begin the planned characterization tests; preserve every resulting RED until its reviewed correction.
