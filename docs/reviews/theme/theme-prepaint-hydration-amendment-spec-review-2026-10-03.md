# Theme hydration seam follow-up: independent SPEC plan review

2026-10-03. **SPEC PASS for a bounded experiment; Critical 0 / Important 0 / Minor 0.** No actionable specification defect in the follow-up. This does not accept the failed product or predict runtime success.

## Scope and snapshot

Reviewed `docs/superpowers/plans/2026-10-02-semantic-theme.md`, section “Hydration-compatible seam follow-up”, lines 59–83, SHA-256 `cb02d9b92fb27b7e0ed6a066ba8fa7d966644b4f5208e8afb7dc2203f413bd5f`. Read `docs/fork/evidence/theme-prepaint-raw-head-failure-2026-10-03.json`, SHA-256 `92a1f747dd41297ae198806841528d8ab7ffc333ec4c211f505defb1ec035b52`; it explicitly rejects the previous experiment and records six hydration failures with no reload reached.

Task source remains the previously rejected snapshot on base `53f4415058fb01e38b8892a47ac036bfffa394ba`: layout `dac7ec01049f481d79ae32127c80578f4438defc3f18ecf7ad5b83001bcc7723`, ThemeScript `6cce905b099bfb26e40ce65564b79b042454ee464b6197dbce83f1611aa10601`, foundation test `468ddcdf90136041271e0ba0c94739d5b0d68a458ab4382a6d556e71dc140781`. Reviewer verified those bytes, read the proposed section and inspected the installed renderer. No new implementation/browser result is claimed.

## Contract assessment

The proposal narrows the correction to the same layout and ThemeScript paths: server-only trusted raw markup and client normal script tree share one static IIFE, ID and text. There is no user-data interpolation, new preference resolver, effect, resource deletion, dependency patch or scope expansion. Server HTML order remains subject to the same strict before-CSS proof. Client singleton-head reconciliation is a hypothesis that must retain framework resources and existing script identity.

Installed Next-bundled React `react-dom.development.js:36307` resolves head to the existing document.head; `:35931` contains inline-script hydration matching. This supports investigating the adapter but does not guarantee the correct script will be matched: the renderer's heuristic considers src/type/crossorigin and does not initially identify inline scripts by their text. Therefore the plan's exact ID/text, single live script, no re-execution and no mismatch criteria are necessary, not optional.

The amendment explicitly retains the previous FAIL and rejects suppression, diagnostic filtering, duplicate execution and replacement of head content. It keeps the full twelve navigation/reload transitions, first-frame correctness and zero hydration diagnostics. No acceptance criterion is waived for the new serialization choice.

## Required experiment evidence

After separate QUALITY plan acceptance, implement only the admitted adapter and matching source ownership assertions. Instrument script execution distinctly from ConfigContext's legitimate same-state runtime theme application; root toggle count alone cannot distinguish them. Check emitted and live script count/identity on both navigation and reload, hydration diagnostics, and retained Next metadata/styles/resources. A duplicate/replaced script, resource deletion, re-execution or mismatch rejects this hypothesis. Preserve failures rather than suppressing them. Rebind evidence to the new plan and exact product hashes.

`changed_paths`: only `docs/fork/theme-prepaint-hydration-amendment-spec-review-2026-10-03.md`. No product/status/Git edits, browser execution or Graphify. Blockers to product acceptance remain the existing hydration failure and unexecuted proposed correction; native/release/full theme acceptance are NOT VERIFIED. Next action: complete independent QUALITY amendment review, then run the bounded experiment and seek exact-diff review.
