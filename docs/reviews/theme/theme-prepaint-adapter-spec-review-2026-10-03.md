> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Theme prepaint hydration adapter: independent SPEC review

2026-10-03. **SPEC PASS for the bounded three-file seam — Critical 0 / Important 0 / Minor 0.** No actionable finding remains within this correction. The previous whole-head hydration mismatch is resolved in the reviewed development-browser matrix. This is not full theme, native, production-build or release acceptance.

## Observed snapshot and checks

Task root `/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`; diff against `53f4415058fb01e38b8892a47ac036bfffa394ba`. Reviewer inspected complete production diff, complete added test harness/matrix and scoped status; only these three files are modified. Their verified hashes match the supplied exact snapshot:

- `frontend/src/app/layout.tsx`: `656fbe7d72285e8e644455c3ed29ef7c8b5e0ca65758d11572d938c4cd5ca0f0`.
- `frontend/src/components/ThemeScript.tsx`: `995e8340bfdc49d53cd71ee459f8d9281a6760e3f1bcdf8a9580f6debe9a0ced`.
- `frontend/tests/theme-foundation.spec.ts`: `113232a1ffbf0da13a7b667f81201323f8463271bad6bea81193f9849ab046a3`.

Reviewed against accepted plan amendment `cb02d9b92fb27b7e0ed6a066ba8fa7d966644b4f5208e8afb7dc2203f413bd5f`. Read completed `/private/tmp/meetily-theme-prepaint-adapter.log`, SHA-256 `bd36d88af90b2610dec39ba1ad975cf46fe2b36e392443a2968df7324700c900`: **9 passed, 54.4 seconds**. The six parameterized matrix tests each execute both navigation and reload, so the completed run covers all twelve planned transitions. Other passes cover source ownership, intermediate-mutation regression and original early-dark flag.

These are independently read source/log results, not reviewer-run tests. Reviewer did not own the process or capture its shell exit. Typecheck/ESLint and eight startup Node passes were reported by the implementer and are not independently rerun/log-attested here. Browser configuration invokes loopback Next development server (`playwright.config.ts:25–26`), not production export or native WebView.

## Behavior and prior finding closure

`layout.tsx:243–247` selects trusted head markup only for server rendering and the normal ThemeScript element only for client rendering. `ThemeScript.tsx:27–31` exports the same static IIFE to both representations with identical ID/text. No preference input is interpolated into markup; the existing parser/resolver and ConfigContext ownership are unchanged. There is no new suppression, theme effect, dependency/config patch or resource deletion.

The matrix verifies script-before-stylesheet response order; expected root state before held CSS release with an early prebody event; first post-stylesheet frame; no later wrong theme event; zero collected hydration diagnostics. It also checks one live script and retention of the original parser-created node, one execution distinguished through document.currentScript, viewport metadata, loaded held stylesheet and retained Next script resources. This execution counter excludes legitimate ConfigContext root toggles. Because all checks precede each loop's completion, the terminal nine-pass result supports closure of the previous hydration Important on this exact tested path. No cosmetic filtering of warnings was introduced.

## Remaining boundaries

Single Settings route, development Chromium matrix only. Broader theme persistence/failure/live-system/native-intent coverage, rendered surface/focus contrast, full frontend/browser/build/canonical checks, actual native appearance/accessibility and release gates remain separate. Resource assertions sample the required head categories; they are not a universal resource inventory. No future renderer-version compatibility guarantee follows; preserve these regressions on dependency changes.

Next action: combine with independent QUALITY implementation review and continue the remaining accepted plan gates. No task closeout, commit or integration permission follows this scoped PASS. `changed_paths`: only `docs/fork/theme-prepaint-adapter-spec-review-2026-10-03.md`; no product/status/Git edits or Graphify.
