> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# FORK-039 semantic theme migration verification plan

> Execute the canonical mdtask/SDD task graph. Checkbox items are future verification/implementation checks; independent SPEC and QUALITY gates remain mandatory. Historical skill names do not expand authorization.

**Goal:** prove and freeze the migrated semantic theme contract from PR 556 on the macOS fork: `system`, `light`, and `dark` remain persistent, first-paint-safe, live, readable, and synchronized with the native Tauri window without turning this migration task into the final UI redesign.

**Architecture:** keep one small frontend theme boundary. `ThemeScript` resolves the stored preference at the parser-blocking pre-paint seam and `app/layout.tsx` owns its single mount/order; `ConfigContext` owns the runtime preference, live system-media subscription, and the effective `isDarkTheme` projection; `lib/theme.ts` contains pure parsing/resolution plus serialized best-effort native synchronization. BlockNote consumers render that effective projection and never resolve storage or OS preference themselves. Semantic CSS variables feed Tailwind/shared components. Browser storage is preference storage only, not product lifecycle authority. Tauri receives `light`, `dark`, or `null` (`system`) through its existing app-theme capability. No backend domain or IPC contract changes.

**Observed dependency stack:** Next.js 14.2.35, React 18.3.1, TypeScript 5.9.3, Tailwind CSS 3.4.19 class mode, @tauri-apps/api 2.11.0, Playwright 1.61.0/Chromium 1228, Node 26.10.0, pnpm 10.17.1. Manifest ranges are not installed versions; this rebind updates no dependencies.

## Current planning authority and execution gates — 2 October 2026

- Current planning base: `main@53f4415058fb01e38b8892a47ac036bfffa394ba`, tree `bb8b8bdda6b53deaa926362f59b661410479ef78`. The source checkout has an uncommitted scanner/plans overlay, not a clean admitted theme implementation or release base.
- Original preserved plan SHA-256: `076e8cdb505d284b81d5382a1575faa1b3047799bacbad059f26360b307182a3`, at `.worktrees/macos-fork-foundation/.superpowers/sdd/macos-fork/fork-039-plan.md`. FORK-034/057 are integrated; their historical uncommitted-patch STOP is obsolete. Do not re-import an old branch.
- FORK-150 is this rebind and baseline classification. FORK-151 must independently review these exact bytes with SPEC/QUALITY Critical 0/Important 0. FORK-062/039 and implementation/native acceptance children remain open.
- Before implementation, admit a clean checkout/base and the agreed dev/feature route against current main. Record its commit/tree and absolute `$THEME_TASK_ROOT`; rebind/review after source, caller, allowlist or contract drift. Preserve other overlays.
- Scope retains the original 18 existing paths; only the shared test helper proposed below may become path 19 after fresh amendment review. Screenshot changes or a new required path require plan amendment and fresh independent review; never update snapshots merely for green tests.
- Commits, Git integration, push/PR, identity/signing, release and installed-app/native host appearance actions need their own applicable authorization. Future command examples grant none. Historical integration/foundation lines remain preservation sources, not current destinations.

## Admitted execution and prepaint seam amendment — 3 October 2026

The operator selected option 1: advance local `dev` to exact
`main@53f4415058fb01e38b8892a47ac036bfffa394ba` and start feature work from it.
The previous dev commit `6f2c4488edc6b9fffd45f8b2dd31bcd6079fe37e` is retained at
`themaddoxed/dev-before-main-2026-10-03`. The admitted task root is
`/Users/developer/Desktop/meeting-minutes/.worktrees/semantic-theme-2026-10-03`
on `feature/semantic-theme-2026-10-03`; the clean integration destination is
`/Users/developer/Desktop/meeting-minutes/.worktrees/dev-integration-2026-10-03`
on `dev`. Both began at the exact base/tree above; no existing dirty checkout
was switched or reset. Clean admission received independent SPEC and QUALITY
PASS; see `docs/fork/theme-clean-admission-*-review-2026-10-03.md` and its census.
The lockfile is unchanged after a frozen install. This does not authorize a new
implementation commit, push, PR, native installation, or release.

Amendment awaiting fresh independent review: the single prepaint resolver remains
owned by `ThemeScript.tsx`, but its mount may be trusted serialized script markup
rather than a React `<ThemeScript />` child. In the installed React server renderer,
head inner HTML is emitted in `headChunks` before hoisted stylesheet resources;
a normal script child is emitted after those resources. After the corrected
matrix proves RED, export one trusted markup value containing the existing IIFE
from `ThemeScript.tsx`, with a stable `meetily-theme-prepaint` script ID, and use
that value once as the root head's `dangerouslySetInnerHTML`. Do not interpolate
user data into markup, add another resolver, patch dependencies/Next config, or
introduce a hydration correction effect. Retain storage parsing and runtime
ownership unchanged. Replace the literal JSX mount assertion with source-bound
single markup ownership plus response HTML asserting exactly one script with
that ID before the first app stylesheet. This is the only amended production
seam; the 18-path allowlist and all twelve-transition/no-flash/hydration criteria
remain unchanged. Production edits await SPEC/QUALITY review of this amendment.

The mutation oracle must reconstruct ordered attribute states from each record's
`oldValue` and the next same-attribute record, rather than repeatedly snapshotting
the final DOM. A regression must detect synchronous wrong-then-correct class and
style writes. The held-CSS assertion uses the recorded before-body event together
with current class/color-scheme, without requiring body to remain absent at the
later polling instant. Browser test success alone does not establish native or
release readiness.

### Hydration-compatible seam follow-up — awaiting independent review

The first raw-head experiment passed the six first-navigation preCSS and markup
order assertions, but failed all six hydration diagnostics: Next's server head
also contains metadata/styles/resources, while the client __html prop contains
only the theme script. Reload was not reached. This experiment is rejected;
`suppressHydrationWarning`, diagnostic filtering and contract relaxation are
not corrective actions.

Within the same two production paths, the next bounded hypothesis is a
server/client serialization adapter: retain one source-owned IIFE, export both
its trusted static markup and a `ThemeScript` component rendering the identical
single script with identical stable ID/text. The root layout uses raw head HTML
only during server rendering (`typeof window === 'undefined'`), and a normal
`<head><ThemeScript /></head>` tree on the client. This is a rendering adapter
for one existing script, not a second resolver or a hydration-time theme effect.
React's singleton-head hydration must match the existing inline script while
retaining Next-owned resources. Source-bound checks must assert the mutually
exclusive server/client branches, one component occurrence, one source IIFE,
and identical exported markup/component script identity. The browser must prove
one emitted script on navigation and reload, no duplicate execution attributable
to hydration, preCSS/first-frame correctness and zero hydration diagnostics.
If the renderer replaces head content, duplicates the script, re-executes the
resolver on hydration, or reports a mismatch, reject this hypothesis and retain
the failed evidence. No dependency/config/path expansion is authorized here.

### Shared rendered-contrast helper amendment — awaiting independent review

The prepaint adapter now has bounded SPEC/QUALITY PASS on the exact three-file
diff and 9 browser tests, including all 12 transitions. This does not close
FORK-152/153. Task 3 needs a single rendered-color engine shared by the dark
surface suite and the keyboard-focus case in the foundation suite. Importing a
`.spec.ts` module as a helper would also register its tests in another suite;
placing this test engine in production theme code would mix test geometry with
runtime ownership. Amend the product/test allowlist from 18 to 19 paths by adding
only `frontend/tests/helpers/rendered-contrast.ts`. It is a test-only module with
no registered tests or production callers. Both suites import the same engine;
Task 3 fixtures test that engine's compositing and fail-closed behavior. No
other new path, shared UI primitive, snapshot, dependency or palette scope is
admitted. This path amendment requires fresh SPEC/QUALITY before helper edits.
The original 18 source hashes remain the admitted existing-file census; the new
helper is absent at base and must be recorded as newly created, not a hash match.

## Audited lineage and baseline

- Foundation task: `FORK-039` says to verify and freeze the semantic theme from PR 556, preserve `system/light/dark` and contrast, and not treat it as final design. `FORK-031` is its only declared dependency and is complete. FORK-039 blocks FORK-046 and FORK-053.
- PR 556 is the replacement contribution after the earlier PR 555 was closed when the source branch was renamed to `feature/meetily-0.4.0-dark-theme`; issue 554 is the related contribution request. The exact local contribution head is `9cd089afac3a9ced4adc7e3cb4a622cb8e0319e9`; its merge-base is `0bedd8b7d805fc924fb706380c829cb1b48d4caf`.
- The contribution range introduced the stored theme mode, pre-paint script, semantic tokens, global fixed-neutral audit, deterministic Tauri browser fixtures, representative Playwright coverage, and the full surface migration. The current integration is not a raw PR import: later commits added safe storage fallback, stronger status contrast, the updated neutral palette, Tauri native appearance synchronization, serialization of competing native calls, secondary-surface coverage, and refreshed shell snapshots.
- Current data flow is `localStorage['themeMode'] -> ThemeScript -> html.dark + color-scheme -> ConfigContext -> semantic tokens/components`, with `matchMedia('(prefers-color-scheme: dark)')` listened to only in `system` mode. Missing, invalid, or unreadable storage resolves to `system`; failed storage writes or native calls do not block the selected web theme.
- Ownership is explicit. `ThemeScript.tsx` owns only pre-hydration resolution; `app/layout.tsx` owns its exactly-once parser order; `ConfigContext` is the only runtime owner and publishes effective `isDarkTheme`; `PreferenceSettings` changes the raw preference only; `BlockNoteEditor/Editor.tsx` and `AISummary/BlockNoteSummaryView.tsx` consume `isDarkTheme` only. A consumer must not read `themeMode`, `localStorage`, `matchMedia`, or call native theme synchronization independently.
- Migration preserves the existing key `themeMode` and exact values `system|light|dark`. Missing, invalid, unreadable, and pre-theme installations resolve to `system`; no second key, dual-write, legacy boolean, storage version, or PR-branch cherry-pick is introduced. The rewritten integration tip, not PR 556's old branch, is implementation authority. If Task 0 finds another live key or resolver, stop and amend the migration decision before product edits.
- Native synchronization is intentionally best effort. Explicit `light`/`dark` calls Tauri `setTheme`; returning to `system` sends `null`. Initial `system` leaves native ownership with macOS. The Tauri capability includes `core:app:allow-set-app-theme`, and window configuration does not force a static theme.
- The static audit currently scans all `frontend/src` CSS/JS/TS/TSX and rejects fixed neutral surface utilities and listed hard-coded light neutrals with an empty exception allowlist.
- Historical focused baseline at the planning snapshot: `pnpm --dir .../frontend test:theme-audit` PASS and `pnpm --dir .../frontend typecheck` PASS. The focused browser command was attempted but the managed sandbox denied binding `127.0.0.1:3118` with `listen EPERM`; escalation was unavailable because the execution allowance was exhausted. This is `NOT_COMPLETED`, not a product failure and not browser PASS. Historical green runs are provenance only, not current acceptance.
- The architecture contract makes Rust authoritative for recording/job/persistence lifecycle. Theme preference is presentation state and may remain in browser storage; FORK-039 must not touch lifecycle ownership, domain types, capture, models, persistence, or job IPC.

## Current caller census and baseline classification

[The census JSON](2026-10-02-theme-census.json) records every matched source/test
path with a decision/hash and all 18 allowlist paths. Theme source matches current
main; only the documented scanner/package script and plans are dirty. Root deps
differ from the lockfile, so checks used the isolated checkout at the same commit
with exact pnpm dependencies. Mock/browser and native evidence remain separate.

- Owners retained: `lib/theme.ts` pure parsing/resolution plus serialized native
  sync; `ThemeScript.tsx` pre-hydration only; one trusted theme script mount in layout head
  before body; `ConfigContext.tsx` sole runtime preference/media/effective-theme
  owner. Disposed/delayed calls are invalidated; failed storage/Tauri writes do
  not block the selected web theme.
- Consumers characterized: PreferenceSettings changes raw mode and aria-pressed;
  both BlockNote components consume `isDarkTheme` only. webMigration retains the
  opaque themeMode preference; it is not another resolver. All fixture/mock and
  unrelated test matches have explicit decisions; no second live owner found.
- Static audit: exit 0. Configured typecheck passed for this unchanged theme source
  in the preceding source check. Native capability/window-config Node checks:
  2 passed, exit 0, from exact frontend cwd. This is configuration intent, not
  installed-app titlebar or host appearance evidence.
- Sandbox browser attempt: exit 1 and zero tests; a direct server probe confirmed
  `listen EPERM 127.0.0.1:3118`. Next returned exit 0 despite the failed-start log;
  its process exit alone must not count as server PASS.
- Authorized loopback browser run outside sandbox across all five theme suites:
  **30 passed, 3 failed, 0 skipped/flaky, exit 1**. Failures are the early-dark
  assertion and light/dark sidebar snapshots. This is current RED, not complete
  theme acceptance. Historical green checks cannot substitute for these results.
- A diagnostic fixture collected both actual dark toggles: first bodyAbsent=true
  at readyState=loading, then bodyAbsent=false at readyState=complete. The old test
  overwrites its early flag on each toggle and fails its final false assertion.
  Retain the first event or collect all events in characterization; do not weaken
  the contract. One passing diagnostic probe is not repair of the existing suite.
- Visually compared actual/expected snapshots: current sidebar adds the date
  group Jun 20, 2026 and Ready 30:00 row. Background/border assertions pass, but
  content drift is not automatic permission to update snapshots. Images are unchanged.
- Head observation places a stylesheet before ThemeScript. The stronger
  first-stylesheet-release criterion, full stored-mode/system matrix, hydration
  diagnostics, keyboard focus and real-surface contrast still need the planned
  tests. Existing synthetic contrast probes are supplemental, not full acceptance.

A focused confirmation reran only the three failing assertions with unique output:
0 passed, 3 failed, exit 1. Its screenshots, diffs and traces were preserved before
any later run; no screenshot baseline was updated.

Private logs/artifacts are preserved under
`.worktrees/integration-backups-2026-10-02/theme-baseline/`; tracked JSON records
public-safe assertions and source hashes. No theme source, snapshots, app
installation, TCC, host appearance, signing or release changed.

### Apple HIG interpretation for this migration

- macOS normally prefers following system Appearance. `system` therefore remains the default and must follow a live OS change. Existing explicit `light` and `dark` overrides are preserved because the accepted task explicitly requires them; do not expand this exception or redesign Settings.
- Normal/small text must reach at least 4.5:1 contrast; meaningful large graphics, focus indicators, and essential control/state boundaries require 3:1 where the indicator itself carries meaning. Do not require every decorative border to reach 3:1.
- Selection cannot rely on color alone: the existing labeled buttons, icons, and `aria-pressed` state are part of the contract. Keyboard focus must remain visible.
- Verify light/dark plus macOS Increase Contrast, Reduce Transparency, and their combination. Each macOS accessibility-mode row needs measured real-app contrast, not a visual impression: normal text at least 4.5:1 and meaningful focus/state boundaries at least 3:1; translucent surfaces must either become opaque or remain compliant after their actual backing layers are included. Browser automation cannot prove native title bar, traffic lights, inactive-window appearance, or live macOS accessibility settings.
- FORK-046 owns the final HIG UI specification and redesign. FORK-039 must not change navigation/layout, typography scale, materials, window structure, Settings architecture, branding, iconography, or introduce new palettes.

## Exact provisional product allowlist (19 paths after helper amendment)

The existing 18 paths are reconciled here but not admitted for implementation until FORK-151 review and Task 0 clean-base admission. A changed path outside this list is a hard stop and plan amendment, not an implicit extension.

**Expected test/policy changes:**

1. `frontend/tests/theme-foundation.spec.ts`
2. `frontend/tests/theme-primitives.spec.ts`
3. `frontend/tests/theme-dark-surfaces.spec.ts`
4. `frontend/tests/theme-secondary-surfaces.spec.ts`
5. `frontend/tests/theme-shell.spec.ts`
6. `frontend/tests/e2e-build-boundary.test.mjs`
7. `frontend/scripts/check-theme-tokens.mjs`

**Conditional corrective production changes, only after a focused RED proves a real contract failure:**

8. `frontend/src/lib/theme.ts`
9. `frontend/src/components/ThemeScript.tsx`
10. `frontend/src/contexts/ConfigContext.tsx`
11. `frontend/src/components/PreferenceSettings.tsx`
12. `frontend/src/app/globals.css`
13. `frontend/tailwind.config.js`
14. `frontend/src/testing/tauri-browser-mocks.ts`
15. `frontend/src-tauri/tauri.conf.json`
16. `frontend/src/app/layout.tsx`
17. `frontend/src/components/BlockNoteEditor/Editor.tsx`
18. `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`
19. `frontend/tests/helpers/rendered-contrast.ts` — test-only shared engine; amendment above.

Paths 16-18 are admitted because they are the pre-paint mount and the two live effective-theme consumers. They remain no-change characterization targets unless the named ownership/pre-paint/consumer test is RED; do not broaden this into feature styling. No snapshot image change is expected or admitted. If a legitimate contract correction changes a screenshot, stop, inspect the rendered delta, amend the allowlist and plan, and repeat independent plan reviews before accepting it. Do not update snapshots merely to make a test green. Do not edit package manifests/lockfiles, Playwright config, shared primitives, any other feature component, Rust source, Next configuration, architecture documents, or old contribution plans/specs.

## Success criteria

1. Missing/invalid/unreadable preference resolves to `system`; the full stored-mode × emulated-system matrix for `light`, `dark`, and `system` is correct on initial navigation and reload. The root class and `color-scheme` are set while `body` is absent and before the first app stylesheet is released, so the first CSS-renderable frame is correct; no hydration error/warning or post-hydration theme correction occurs.
2. `system` tracks a live system color-scheme change; explicit modes ignore later system changes until the user returns to `system`.
3. `html.dark` and root `color-scheme` always represent the same effective theme.
4. Initial explicit modes and later selections synchronize to the native window; `system` maps to `null`; the latest queued selection is final; a Tauri failure leaves the web theme usable.
5. Selector state is exposed with labels/icons and exactly one `aria-pressed=true`; keyboard focus is visible and controls remain operable. A real keyboard-focused Appearance button has a rendered indicator whose computed color reaches 3:1 against every adjacent rendered surface in both effective modes.
6. Representative home, Settings, and Meeting Details surfaces contain no light islands in dark mode and no horizontal overflow. Global static policy has no unexplained fixed neutral classes or hard-coded light palette values.
7. Computed primary/muted/status text meets 4.5:1 on named real Home, Settings, and Meeting Details surfaces; meaningful status icons/focus/state indicators tested by the suite meet 3:1. The rendered-color engine walks the actual element/ancestor background and opacity stack without intermediate rounding and fails closed when images, gradients, blending, filters, masks, text shadows, or ambiguous box-shadow geometry prevent an exact result. Synthetic token probes remain supplemental and cannot satisfy this criterion alone.
8. Browser and native evidence are reported separately. A browser mock proves frontend intent/IPC payload only; an actual Tauri app on macOS proves native window behavior.
9. Product diff stays inside the rewritten exact allowlist, passes required gates, receives SPEC and QUALITY implementation review with Critical 0/Important 0, becomes one local non-merge commit, and is fast-forward integrated only after approval.
10. No final-redesign claim, no release claim, and no PRD completion before separate closeout.

---

### Task 0: Admit exact source and review this rebind

**Files:** plan/census/review artifacts only; no product edits.

- Re-read FORK-039/062/150/151 and implementation children in the canonical PRD;
  validate dependencies. Baseline classification is not independent review.
- Admit clean/idle source and destination against current main/tree and the agreed
  dev route. Record absolute `$MEETILY_REPO_ROOT`, `$THEME_TASK_ROOT` and
  `$ADMITTED_INTEGRATION_ROOT`; quote them. Do not reset/stash another overlay.
- Verify source hashes in census, one ThemeScript mount/runtime owner, themeMode
  key and both effective BlockNote consumers. A new resolver/key/caller or required
  extra path stops implementation for amendment and fresh review.
- Bind current baseline to admitted source/dependencies; preserve named errors,
  exit and selected-test counts. Zero tests/server EPERM are NOT_COMPLETED. Red
  assertions stay red until reviewed correction; use unique Playwright output dirs
  and preserve attachments before another run.
- Freeze plan/census and require fresh SPEC/QUALITY PASS Critical 0/Important 0.
  Review first-event instrumentation, stylesheet ordering, hydration/focus/contrast
  coverage and snapshot drift. No product/snapshot edits before this gate.

---

### Task 1: Create the isolated task state

**Files:** no product edits.

- [ ] Prove the reviewed base is still exact/clean, branch ref is absent, and worktree path is absent. Create from the exact SHA recorded in Task 0; never use a moving branch name as the start point:

```bash
git -C $ADMITTED_INTEGRATION_ROOT status --short
git -C $ADMITTED_INTEGRATION_ROOT rev-parse HEAD
git -C $MEETILY_REPO_ROOT show-ref --verify --quiet refs/heads/$THEME_TASK_BRANCH
test ! -e $THEME_TASK_ROOT
git -C $MEETILY_REPO_ROOT worktree add $THEME_TASK_ROOT -b $THEME_TASK_BRANCH $THEME_BASE_SHA
```

Expected: clean new worktree at the reviewed SHA. Existing unknown ref/path means stop and inspect, never delete/reuse it automatically.

- [ ] Install only from the locked graph if dependencies are absent:

```bash
pnpm --dir $THEME_TASK_ROOT/frontend install --frozen-lockfile
git -C $THEME_TASK_ROOT status --short
```

Expected: lockfile unchanged and clean product tree.

---

### Task 2: Freeze theme resolution, persistence, and native intent

Current baseline proves an instrumentation defect: retain the first dark-toggle
observation and collect later events, then separately test first stylesheet release
and hydration. Do not accept the single diagnostic probe as the full matrix.
Any actual owner/order correction needs focused RED in the conditional allowlist.

**Files:** primarily `frontend/tests/theme-foundation.spec.ts`; conditional fixes only in allowlisted theme owners/mocks/config.

- [ ] Add characterization cases for all three values and opposite-system pressure: explicit `light` remains light while emulated OS becomes dark; explicit `dark` remains dark while OS becomes light; `system` follows both directions live; root `dark` and `style.colorScheme` agree before and after each transition; every button selection persists and reloads; missing/invalid/read-failure use `system`; write/native failure does not block rendering.
- [ ] Make first-paint proof a deterministic Playwright matrix, not a settled-DOM assertion. Cover all six stored-mode/effective-system rows (`light` under light/dark OS, `dark` under light/dark OS, and `system` under light/dark OS) on both a fresh navigation and a real `page.reload()`, for twelve transitions total. Before navigation, use `page.addInitScript` to instrument root `class`/`style` mutations and root `DOMTokenList.toggle`, recording whether `document.body` existed at each mutation; attach `page.on('pageerror')` and `page.on('console')` collectors; and hold the first `/_next/static/css/*.css` response with a Playwright route gate. For every transition, require the expected root `dark` state and `colorScheme` mutation while the stylesheet is still held and `body === null`, then release CSS and require the first CSS-renderable `requestAnimationFrame` to have that same state. Through the app-ready assertion, require no later corrective root theme mutation and no hydration warning/error (`hydration`, `did not match`, `server HTML`, `extra attributes`, or equivalent React hydration diagnostic). Also inspect response markup/order so the single inline `ThemeScript` precedes the first app stylesheet. A screenshot or an eventually-correct root class is not first-paint evidence.
- [ ] Characterize ownership in the same focused suite or a source-bound test: `app/layout.tsx` mounts the trusted `ThemeScript.tsx` markup exactly once; `ConfigContext` publishes the effective `isDarkTheme`; both BlockNote consumers take only that projection and do not access `localStorage`, `matchMedia`, `setTheme`, or `set_app_theme`. If the prepaint/order case is RED, the only permitted layout correction is to restore the single `ThemeScript` in `head` before app CSS; if a consumer case is RED, migrate it to `isDarkTheme` without changing editor content or styling. Do not create a second resolver or hydration-time correction effect.
- [ ] Pin native intent separately: initial untouched `system` emits no ownership call; initial explicit preference synchronizes once; user sequence `light -> dark -> system` produces `light`, `dark`, `null`; delayed/failing calls cannot leave an older preference as final. Keep the strict mock rejection of missing/`undefined`/`sepia` payloads.
- [ ] Run the focused test before production edits:

```bash
pnpm --dir $THEME_TASK_ROOT/frontend exec playwright test tests/theme-foundation.spec.ts --project=chromium --reporter=line
```

Characterization additions may already be GREEN because this task freezes migrated behavior. Record that honestly. If any required behavior is RED, save the exact named assertion/output, make the smallest change in the conditional allowlist, and rerun that exact test GREEN. Do not refactor theme ownership or add abstractions without a failing contract.

---

### Task 3: Freeze semantic policy and computed contrast

**Disjoint foreign-shadow proof clarification (2026-10-03):** Keep computed RGBA
and ancestor-opacity math as the text acceptance engine. Its conservative blur
bounding box currently rejects real text near the Sidebar expand button, even
when the foreign element's actual border box is disjoint from all measured text.
Within the existing shared helper path, add a test-only Node/Locator driver for
text measurements. When this one ambiguity occurs, the browser function reports
the candidate identity/computed-style and ancestor/rectangle fingerprint plus
its exact text measurement region. The driver may certify non-interference only
by exact equality of two stable screenshot PNG buffers with the shadow present,
two buffers after temporarily overriding box-shadow to none jointly for the
entire accepted candidate set plus the newly discovered candidate, and a final
buffer after restoring every original inline value/priority in finally. Require
computed box-shadow exactly none for every candidate before and after each of
the two disabled-shadow captures, with a bounded 5-second stabilization wait;
require each original computed shadow and fingerprint restored before and after
the final capture. A delayed/ineffective override or failed restoration rejects
the proof. Do not add separate per-shadow certificates together: every expanded
set needs its own joint removal proof, so overlapping shadows cannot mask each
other's removal. Cover the entire measurement region, rounding clip bounds outward;
require it inside the viewport. Do not disable animations or other paint, mask
pixels, use tolerances, cache across elements/runs, or compare only selected
points. Any changed/nonstable pixel, stale candidate/style/ancestor/region
fingerprint, actual border-box overlap, or unsupported non-shadow effect must
fail closed. Retry the computed resolver with a fresh proof only for this exact
foreign shadow and region; every other validation remains active. Cap discovery
at 16 candidates and fail when the cap is exceeded. This is causal pixel evidence
that excluded paint does not affect the full sample, not a shadow allowlist.

Keep the driver restricted to text; focus and background scans retain their
strict geometry and acceptance requirements. Add a disjoint-shadow positive and
a genuinely influencing-shadow negative fixture, plus a delayed/blocked override
negative, overlapping-shadow joint-removal negative, and restoration/stale-proof
checks, before using the driver for named real Home/Settings/Meeting Details
text. No product source, dependency, screenshot baseline, path count, or 4.5:1
threshold change is admitted by this clarification. Independent SPEC/QUALITY
review is required before helper edits and again for the implementation.

**Focus geometry clarification (2026-10-03):** The current real keyboard case fails
closed on the UA `outline: auto` because its paint geometry is not uniquely
parseable; this is not evidence that the UA indicator has low contrast. This
measurement failure permits the conditional `PreferenceSettings.tsx` correction:
use a solid 2px, zero-offset `info-text` outline on each focus-visible Appearance
button and suppress only Appearance button box shadows while the selector has
keyboard focus. Keep labels, icons, pressed state, button padding, and tokens. A real Tab run
showed that the next selected button covers the outside outline because the
selector currently has zero gap. Permit only a constant 4px gap between these
three buttons (`gap-1`, adding 8px total selector width) to keep each 2px outline
on the actual parent background; do not ignore the intersecting neighbor paint
or change surrounding section layout.
The shared resolver may prove containment using the actual circular corner arcs
and straight edges of that outside outline and its rounded parent background,
rather than the conservative square corner of its enclosing rectangle. Accept
only finite pixel radii, uniform parent border widths, no overlapping corner
radii, and containment of every point of each relevant arc (analytical extrema,
not sparse sampling). Unsupported/elliptical radii, clipping, foreign paint,
blurred/competing indicators and ambiguous backgrounds must still fail closed.
Add positive rounded-parent and negative escaping-arc fixtures before applying
the production focus classes. Re-review this clarification and resulting code;
retain the real Tab-driven six-button/mode assertions and both 3:1 thresholds.

**Files:** `frontend/tests/theme-primitives.spec.ts`, `theme-dark-surfaces.spec.ts`, `theme-secondary-surfaces.spec.ts`, `frontend/scripts/check-theme-tokens.mjs`; conditional token fix only in `globals.css`/`tailwind.config.js`.

- [ ] Keep the synthetic `foreground/background`, `card-foreground/card`, `popover-foreground/popover`, muted and status/tint probes as token-unit coverage, but do not use them as the acceptance proof. In `theme-dark-surfaces.spec.ts`, add one fail-closed rendered-color resolver and use it on actual fixture-backed elements in both modes: `getByText('Welcome to meetily!').first()` on Home; the `Appearance` heading, its real `Choose how Meetily should render the interface` muted paragraph and the named Appearance buttons on Settings; and `getByText('Theme Test Meeting').first()` plus visible transcript/summary muted or status text under the existing Meeting Details fixture. For every required locator, fail if it is absent rather than silently reducing the sample.
- [ ] The resolver must operate on the selected DOM element, not a generated probe. Parse CSS colors as floating-point RGBA; build the effective backdrop through the complete element-to-`html` ancestor chain, applying every solid `background-color` alpha and each element/ancestor `opacity` at the correct group boundary, then composite the actual foreground or indicator over that backdrop. Keep linear/sRGB math and ratio inputs at full precision; round only ledger display values. Require an opaque resolved canvas. Fail closed, with the offending element/property in the assertion, for `background-image` (including gradients), replaced/image/video/canvas content behind the sample, `mix-blend-mode != normal`, non-none `filter`/`backdrop-filter`, masks/clip ambiguity, `text-shadow`, overlapping/translucent positioned content, or a blurred/multiple `box-shadow` whose sampled indicator and adjacent geometry cannot be uniquely identified. The current dark-surface scan must run this resolver for translucent candidates instead of skipping `alpha < 0.95`; unsupported candidates fail, they are not evidence of “no light island.”
- [ ] Test the resolver's math with opaque and multilayer-alpha fixtures, then require the same engine to pass the named real Home/Settings/Meeting Details elements. Require 4.5:1 for normal/muted/status text and 3:1 for meaningful icons, selection/focus/state boundaries. Helper fixture GREEN without all real-element assertions is not acceptance.
- [ ] Add negative policy fixtures in a task-owned temporary directory for representative forbidden forms (`bg-white`, fixed neutral surface/text/border, listed hard-coded light hex) and clean semantic forms. Refactor the audit only if needed to accept an injected root while keeping production CLI behavior and an empty exception allowlist. The fixture test must prove every forbidden input is found and semantic input is clean; do not weaken regexes to make the repository pass.
- [ ] Keep the representative dark-surface checks for `/`, `/settings`, and `/meeting-details?id=theme-test-meeting`, strict unknown-command fixture failures, visible ready text, and horizontal-overflow assertions. This migration does not add a route-by-route final-design matrix.
- [ ] Add an exact keyboard-focus case named `appearance selector focus indicator meets 3:1 in light and dark` to `theme-foundation.spec.ts`. In each effective mode, enter Settings and use real `Tab` key presses until a named Appearance button matches `:focus-visible`; calling `.focus()` is not acceptance. Read the focused element's rendered `outline` and every `box-shadow` layer, select the visible indicator from its actual geometry, alpha-composite its resolved RGB color, and compare it with both rendered surfaces directly adjacent to that geometry (the button's inner background and the outer selector/container background). Require at least 3:1 against each adjacent surface and fail if there is no parseable nonzero indicator, if the indicator is clipped, or if the test cannot identify both adjacent surfaces. This is a DOM/computed-color measurement of a real focused control, not a token-pair proxy. If RED, make the smallest `PreferenceSettings.tsx` focus-visible class correction and rerun this named case before the wider suite; do not change shared primitives or the palette speculatively.
- [ ] Run:

```bash
pnpm --dir $THEME_TASK_ROOT/frontend test:theme-audit
pnpm --dir $THEME_TASK_ROOT/frontend exec playwright test tests/theme-foundation.spec.ts --grep='appearance selector focus indicator meets 3:1 in light and dark' --project=chromium --reporter=line
pnpm --dir $THEME_TASK_ROOT/frontend exec playwright test tests/theme-primitives.spec.ts tests/theme-dark-surfaces.spec.ts tests/theme-secondary-surfaces.spec.ts --project=chromium --reporter=line
```

For a real failing contrast/policy contract, change only the owning semantic token/audit logic, not scattered feature classes. Preserve functional meanings (recording/destructive red, selection/info blue, warning yellow, success green) and verify affected pairs again. No visual redesign.

---

### Task 4: Browser regression and evidence boundary

**Files:** `frontend/tests/theme-shell.spec.ts`, `frontend/tests/e2e-build-boundary.test.mjs`; no snapshot update expected.

- [ ] Pin Tauri configuration: app-theme permission present, no static window theme, and frontend maps `system` to `null`. Pin selector accessibility: three named buttons, exactly one pressed, keyboard focus visible, no state communicated solely by fill color.
- [ ] Treat the twelve-transition prepaint matrix, zero hydration diagnostics, and absence of a post-hydration correction mutation from Task 2 as a required browser gate. Repeat it in the focused suite below; do not substitute a single dark-mode screenshot or a post-load `html.dark` assertion.
- [ ] Run the focused/full browser gates from the exact task root:

```bash
(cd $THEME_TASK_ROOT/frontend && node --test --test-name-pattern='native application theme|native startup appearance' tests/e2e-build-boundary.test.mjs)
pnpm --dir $THEME_TASK_ROOT/frontend exec playwright test tests/theme-foundation.spec.ts tests/theme-primitives.spec.ts tests/theme-dark-surfaces.spec.ts tests/theme-secondary-surfaces.spec.ts tests/theme-shell.spec.ts --project=chromium --reporter=line
pnpm --dir $THEME_TASK_ROOT/frontend test:e2e
```

The Node subshell must report `$THEME_TASK_ROOT/frontend` as both the test process cwd and spawned-child cwd; zero selected tests or resolution of `next.config.js` from another checkout is failure. Every pnpm command remains rooted by its exact `--dir`, and the repository script below self-roots from its own absolute path.

If the managed sandbox returns `listen EPERM` for `127.0.0.1:3118`, request normal approved local-loopback execution. If approval is unavailable, record browser gates `NOT_COMPLETED` and do not commit/integrate/close; do not bypass the boundary or substitute historical output.
- [ ] Inspect existing shell screenshots without regenerating them. A changed screenshot fails until reviewed; do not use `--update-snapshots` in this task.

Browser PASS proves DOM/CSS behavior and mocked IPC intent. It does not prove native macOS appearance.

---

### Task 5: Actual Tauri/macOS acceptance

**Files:** verification only; no tracked evidence required.

- [ ] Build/run the exact task worktree through the normal Tauri development path without changing host settings programmatically:

```bash
pnpm --dir $THEME_TASK_ROOT/frontend typecheck
pnpm --dir $THEME_TASK_ROOT/frontend build
pnpm --dir $THEME_TASK_ROOT/frontend tauri:dev
```

The interactive run may be performed only when GUI execution is authorized. Record exact source SHA and app identity.

- [ ] In the actual app, manually observe `system`, `light`, `dark`; relaunch persistence; live macOS Appearance change in `system`; web content, native title bar/traffic lights, popovers/dialogs, focused/inactive window; keyboard-only selector operation. Then the operator, never automation, sets each exact macOS accessibility row in turn: (1) normal, (2) Increase Contrast, (3) Reduce Transparency, and (4) both enabled. Confirm the active row in System Settings before measuring and restore the operator's original settings afterward only under their control.
- [ ] For every one of those four rows and for both effective light and dark app modes, use Accessibility Inspector's contrast audit/color measurement or an equivalently documented on-screen pixel/color instrument on the authorized real Tauri window. Record macOS version, app SHA/identity, mode/settings, active/inactive window state, exact element and adjacent rendered surface, measured foreground/indicator and backdrop values, ratio, threshold, and PASS/FAIL. At minimum measure the real Home welcome text, Settings Appearance heading and muted description, Meeting Details title and a visible muted/status label, plus the keyboard-focused Appearance button indicator against both adjacent surfaces. Normal text requires 4.5:1; the focus indicator and meaningful state boundary require 3:1. For Reduce Transparency rows, record whether each translucent surface became opaque; if not, measure the fully composited result and apply the same thresholds. Native title bar/traffic-light/inactive-window observations are separate ledger entries and never substitute for the content measurements.
- [ ] A screenshot without tool-backed sampled values is context only. If Accessibility Inspector or an equivalent measurement method is unavailable, a required element cannot be rendered, a backing layer cannot be measured, or any one of the eight mode/settings rows lacks a required ratio, mark that row and the actual-macOS gate `NOT_COMPLETED`; a below-threshold measured ratio is `FAIL`. Browser mocks, token values, and unaudited visual judgment cannot substitute, and FORK-039 cannot close.

---

### Task 6: Full verification, review, and one product commit

The commit requires explicit applicable authorization after review. `$THEME_TASK_BRANCH` and `$THEME_BASE_SHA` must be recorded from that admission; they are not predefined branch choices.

**Files:** exact allowlist only plus ignored report/review artifacts.

- [ ] Run full gates:

```bash
pnpm --dir $THEME_TASK_ROOT/frontend test:theme-audit
pnpm --dir $THEME_TASK_ROOT/frontend typecheck
pnpm --dir $THEME_TASK_ROOT/frontend test:frontend
(cd $THEME_TASK_ROOT/frontend && node --test --test-name-pattern='native application theme|native startup appearance' tests/e2e-build-boundary.test.mjs)
pnpm --dir $THEME_TASK_ROOT/frontend test:e2e
pnpm --dir $THEME_TASK_ROOT/frontend build
git -C $THEME_TASK_ROOT diff --check
$THEME_TASK_ROOT/scripts/check.sh --full
```

Every scoped command and the repository's canonical `./scripts/check.sh --full` must actually run and pass on the exact diff. The explicit Node subshell must use the task frontend as both parent and spawned-child cwd; all pnpm gates use the exact frontend `--dir`; the absolute repository-script invocation self-roots and preserves its own stage order and Xcode selection. Do not set `MEETILY_CHECK_NO_XCODE=1` or skip a native stage. Record cwd/root, stage, duration, exit code, selected developer directory/tool versions, selected-test count, and warnings. A product assertion, lint, typecheck, browser, build, Rust, or policy failure is `FAIL`. A proven missing/unavailable tool, Xcode installation, simulator/runtime, or managed-host capability that prevents a required stage from executing is `NOT_COMPLETED` for that stage and for the canonical gate, not PASS; stop before commit/integration. A warning is never promoted to PASS without the command's zero exit and the expected assertions.

- [ ] Write ignored `.superpowers/sdd/macos-fork/fork-039-implementation-report.md` with exact base/plan SHA, PR lineage, changed-path classification, every RED or initial GREEN characterization, full commands/results, browser/native evidence separation, HIG checks, and limitations.
- [ ] Verify changed paths are a subset of the exact rewritten allowlist, no snapshots/manifests/lockfiles changed, no untracked product files, and no nested repository/submodule change. Stage only admitted product paths, freeze `git diff --cached --binary` to `/private/tmp/fork-039-precommit.diff`, hash it, and confirm unstaged/untracked product state is empty.
- [ ] Obtain independent SPEC review of every success criterion and independent QUALITY review of correctness, async races, contrast math, accessibility, false fixtures, and scope. Fix only within the allowlist, rerun affected and full gates, refreeze, and repeat until both report PASS, Critical 0, Important 0.
- [ ] Create exactly one non-merge product commit:

```bash
git -C $THEME_TASK_ROOT commit -m "test(theme): freeze semantic appearance contract"
```

If a minimal production correction was needed, use `fix(theme): preserve semantic appearance contract`. Confirm the commit parent is the rewritten base and its tree is byte-identical to the reviewed staged tree. Rerun the same scoped commands and `$THEME_TASK_ROOT/scripts/check.sh --full` against the committed tree; a post-commit failure invalidates the reviews and commit acceptance.

---

### Task 7: Local fast-forward integration and separate PRD closeout

Integration and documentation commit each require explicit applicable authorization.

- [ ] Reconfirm integration destination has not moved and is clean. If it moved, stop, rebase/recreate against the new base, rerun all gates/reviews, and produce a new single commit. Never merge divergent history.
- [ ] With the integration owner and no parallel writer, integrate locally:

```bash
git -C $ADMITTED_INTEGRATION_ROOT merge --ff-only $THEME_TASK_BRANCH
git -C $ADMITTED_INTEGRATION_ROOT status --short --branch
git -C $ADMITTED_INTEGRATION_ROOT log -1 --format='%H %P %s'
```

Rerun every scoped Task 6 command with its root replaced by the exact integrated destination. In particular run `(cd $ADMITTED_INTEGRATION_ROOT/frontend && node --test --test-name-pattern='native application theme|native startup appearance' tests/e2e-build-boundary.test.mjs)` and require that frontend as parent/child cwd, then run `$ADMITTED_INTEGRATION_ROOT/scripts/check.sh --full` without native-stage bypass. Apply the same `FAIL` versus host/tool `NOT_COMPLETED` classification. Only then may the product integration be reported complete. No push.

- [ ] In the canonical admitted documentation root `$MEETILY_REPO_ROOT/docs/prd`, re-read FORK-039 and downstream blockers, use `mdtask` to mark only FORK-039 complete with a concise evidence note, run PRD validation, independently review the documentation diff, and create a separate documentation-only commit. Do not claim FORK-046/FORK-053 complete.

## Evidence limits and stop conditions

- Local contribution commits prove contents/ancestry, not current GitHub PR state, approvals, or mergeability. A fresh GitHub API read was unavailable during planning; do not invent remote metadata.
- Historical 21-test/23-test/build results are provenance only. Acceptance requires fresh commands on the rewritten exact base/diff.
- First-paint acceptance requires the held-stylesheet twelve-transition matrix and pre-body mutation record. An eventually-correct DOM, screenshot, or hydration-complete state does not prove flash-free parser ordering; any hydration diagnostic or later theme correction is a failure.
- Contrast acceptance requires full-precision compositing of the actual named element and its complete rendered ancestor/opacity stack. Unsupported visual effects fail closed; skipped translucent elements and helper-only/token-only probes are not real-surface evidence.
- Playwright with Tauri mocks proves frontend behavior and emitted payloads, not AppKit/WebView/title-bar behavior. Only an authorized actual Tauri run proves native appearance.
- `pnpm build` proves web production compilation, not Universal packaging, signing, notarization, or release readiness. `./scripts/check.sh --full` is nevertheless the required repository-wide local gate and must not be replaced by the scoped frontend list.
- Stop for any integration movement, task/spec change, new theme owner/caller, required path outside the allowlist, snapshot drift, unexplained policy exception, unmet contrast/accessibility gate, unavailable required browser/native gate, or review Critical/Important finding.
