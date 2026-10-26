# Updated Dark Theme Universal Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the known-good full application theme to `build/universal-macos-stable`, adopt the updated neutral palette and native macOS theme synchronization, and retain complete semantic/static regression coverage.

**Architecture:** Start from the stable `v0.4.0` integration branch and merge the clean `feature/meetily-0.4.0-dark-theme` line as the implementation foundation. Apply the newer palette and native-theme behavior as small test-first deltas. Preserve the full semantic migration and global token audit from the clean branch instead of copying the dirty `codex/dark-theme-0.4.1` worktree wholesale.

**Tech Stack:** Next.js 14, React 18, Tailwind CSS 3.4 class dark mode, Tauri 2 JavaScript API, Playwright 1.60, pnpm.

---

### Task 1: Import the known-good theme foundation

**Files:**
- Merge source: `feature/meetily-0.4.0-dark-theme`
- Verify: `frontend/src/app/globals.css`
- Verify: `frontend/src/lib/theme.ts`
- Verify: `frontend/src/components/ThemeScript.tsx`
- Verify: `frontend/scripts/check-theme-tokens.mjs`
- Verify: `frontend/tests/theme-*.spec.ts`

- [ ] **Step 1: Merge the clean theme branch**

Run:

```bash
git merge --no-ff feature/meetily-0.4.0-dark-theme -m "merge: port complete theme foundation"
```

Expected: clean merge; no files from the dirty `codex/dark-theme-0.4.1` checkout are added.

- [ ] **Step 2: Install the merged dependency graph**

Run:

```bash
cd frontend
pnpm install --frozen-lockfile
```

Expected: install succeeds without changing `pnpm-lock.yaml`.

- [ ] **Step 3: Verify the imported theme audit**

Run:

```bash
pnpm test:theme-audit
```

Expected: exit 0 and no fixed light-only neutral utilities reported under `frontend/src`.

- [ ] **Step 4: Verify the imported browser tests and build**

Run:

```bash
pnpm test:e2e
pnpm build
```

Expected: all Playwright tests pass and the production build exits 0.

### Task 2: Specify the updated palette and semantic status contract

**Files:**
- Modify: `frontend/tests/theme-primitives.spec.ts`
- Modify: `frontend/src/app/globals.css`
- Modify: `frontend/tailwind.config.js`

- [ ] **Step 1: Add failing assertions for the updated neutral palette**

Add assertions that the dark token block contains the updated neutral values while retaining semantic status variables:

```ts
expect(css).toContain('--background: 0 0% 3.9%');
expect(css).toContain('--card: 0 0% 3.9%');
expect(css).toContain('--popover: 0 0% 3.9%');
expect(css).toContain('--border: 0 0% 14.9%');
expect(css).toContain('--input: 0 0% 14.9%');
expect(css).toContain('--info:');
expect(css).toContain('--success:');
expect(css).toContain('--warning:');
expect(css).toContain('--recording:');
expect(css).toContain('--overlay:');
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
pnpm exec playwright test tests/theme-primitives.spec.ts
```

Expected: FAIL because the clean theme branch still uses elevated dark card/popover values.

- [ ] **Step 3: Apply the updated neutral palette without removing status semantics**

Set the dark neutral tokens to the approved values:

```css
.dark {
  --background: 0 0% 3.9%;
  --card: 0 0% 3.9%;
  --popover: 0 0% 3.9%;
  --secondary: 0 0% 14.9%;
  --muted: 0 0% 14.9%;
  --accent: 0 0% 14.9%;
  --border: 0 0% 14.9%;
  --input: 0 0% 14.9%;
  --ring: 0 0% 83.1%;
}
```

Keep the existing theme-aware `info`, `success`, `warning`, `recording`, `overlay`, and foreground pairs. Keep Tailwind aliases for these semantic tokens.

- [ ] **Step 4: Verify GREEN and run the static audit**

Run:

```bash
pnpm exec playwright test tests/theme-primitives.spec.ts
pnpm test:theme-audit
```

Expected: both commands pass.

- [ ] **Step 5: Commit the palette delta**

```bash
git add frontend/src/app/globals.css frontend/tailwind.config.js frontend/tests/theme-primitives.spec.ts
git commit -m "feat(theme): adopt updated neutral palette"
```

### Task 3: Add safe storage and native macOS theme synchronization

**Files:**
- Modify: `frontend/src/lib/theme.ts`
- Modify: `frontend/src/components/ThemeScript.tsx`
- Modify: `frontend/src/contexts/ConfigContext.tsx`
- Modify: `frontend/tests/theme-foundation.spec.ts`
- Modify: `frontend/tests/theme-shell.spec.ts`

- [ ] **Step 1: Add failing tests for safe storage and native synchronization**

Add a unit assertion for a storage failure fallback and a browser assertion for native theme calls:

```ts
expect(readStoredThemeMode()).toBe('system');
```

The browser fixture must record `plugin:app|set_app_theme` calls and assert:

```ts
expect(nativeThemeCalls.at(-1)).toBe('light');
expect(nativeThemeCalls.at(-1)).toBeNull();
```

- [ ] **Step 2: Run the tests and verify RED**

Run:

```bash
pnpm exec playwright test tests/theme-foundation.spec.ts tests/theme-shell.spec.ts
```

Expected: FAIL because the clean theme context does not yet call Tauri `setTheme` and does not expose the storage helper.

- [ ] **Step 3: Add the minimal storage helper**

Add to `frontend/src/lib/theme.ts`:

```ts
export function readStoredThemeMode(): ThemeMode {
  if (typeof window === 'undefined') return 'system';
  try {
    return parseThemeMode(window.localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return 'system';
  }
}
```

Use this helper for initial context state. Keep `ThemeScript` responsible for applying the effective theme before hydration.

- [ ] **Step 4: Synchronize the native Tauri theme**

Add one guarded effect in `ConfigContext`:

```ts
const ownsNativeThemeRef = useRef(false);
const nativeThemeSeqRef = useRef(0);

useEffect(() => {
  if (!ownsNativeThemeRef.current && themeMode === 'system') return;
  ownsNativeThemeRef.current = true;
  const seq = ++nativeThemeSeqRef.current;

  void import('@tauri-apps/api/app')
    .then(({ setTheme }) => {
      if (seq !== nativeThemeSeqRef.current) return;
      return setTheme(themeMode === 'system' ? null : themeMode);
    })
    .catch((error) => {
      console.warn('[ConfigContext] Failed to sync native window theme:', error);
    });
}, [themeMode]);
```

- [ ] **Step 5: Verify GREEN**

Run:

```bash
pnpm exec playwright test tests/theme-foundation.spec.ts tests/theme-shell.spec.ts
```

Expected: all selected tests pass.

- [ ] **Step 6: Commit native theme synchronization**

```bash
git add frontend/src/lib/theme.ts frontend/src/components/ThemeScript.tsx frontend/src/contexts/ConfigContext.tsx frontend/tests/theme-foundation.spec.ts frontend/tests/theme-shell.spec.ts
git commit -m "feat(theme): sync web and native macOS appearance"
```

### Task 4: Expand coverage for secondary user-facing surfaces

**Files:**
- Create: `frontend/tests/theme-secondary-surfaces.spec.ts`
- Verify: `frontend/src/components/shared/DownloadProgressToast.tsx`
- Verify: `frontend/src/components/ImportAudio/ImportAudioDialog.tsx`
- Verify: `frontend/src/components/onboarding/steps/WelcomeStep.tsx`
- Verify: `frontend/src/components/onboarding/steps/DownloadProgressStep.tsx`
- Verify: `frontend/src/components/AISummary/index.tsx`

- [ ] **Step 1: Add a static regression test for the reviewed surfaces**

The test reads the five source files and rejects fixed neutral utilities:

```ts
const fixedNeutral = /\b(?:bg-white|bg-black|(?:bg|text|border|ring)-(?:gray|slate|zinc|neutral|stone)-\d{2,3})\b/g;
for (const file of files) {
  expect(await readFile(file, 'utf8')).not.toMatch(fixedNeutral);
}
```

- [ ] **Step 2: Run the test and verify its state**

Run:

```bash
pnpm exec playwright test tests/theme-secondary-surfaces.spec.ts
```

Expected after the clean theme merge: PASS. If it fails, port only the missing semantic classes from `feature/meetily-0.4.0-dark-theme`; do not copy unrelated dirty-worktree code.

- [ ] **Step 3: Run all theme gates**

Run:

```bash
pnpm test:theme-audit
pnpm exec playwright test tests/theme-foundation.spec.ts tests/theme-primitives.spec.ts tests/theme-shell.spec.ts tests/theme-secondary-surfaces.spec.ts
pnpm build
```

Expected: every command exits 0.

- [ ] **Step 4: Commit the coverage gate**

```bash
git add frontend/tests/theme-secondary-surfaces.spec.ts
git commit -m "test(theme): cover secondary application surfaces"
```

### Task 5: Add and verify the Universal macOS build entry point

**Files:**
- Modify: `frontend/package.json`
- Create: `frontend/tests/universal-build-config.spec.ts`

- [ ] **Step 1: Add a failing build-script test**

```ts
import packageJson from '../package.json';

test('defines the Universal macOS Tauri build command', () => {
  expect(packageJson.scripts['tauri:build:universal']).toBe(
    'tauri build --target universal-apple-darwin',
  );
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
pnpm exec playwright test tests/universal-build-config.spec.ts
```

Expected: FAIL because the script does not exist.

- [ ] **Step 3: Add the minimal package script**

Add to `frontend/package.json`:

```json
"tauri:build:universal": "tauri build --target universal-apple-darwin"
```

- [ ] **Step 4: Verify GREEN and inspect installed Rust targets**

Run:

```bash
pnpm exec playwright test tests/universal-build-config.spec.ts
rustup target list --installed
```

Expected: the test passes; both `aarch64-apple-darwin` and `x86_64-apple-darwin` are listed before attempting the artifact build.

- [ ] **Step 5: Commit the Universal build entry point**

```bash
git add frontend/package.json frontend/tests/universal-build-config.spec.ts
git commit -m "build(macos): add Universal Tauri target"
```

### Task 6: Final theme slice verification

**Files:**
- Verify only; no planned production changes.

- [ ] **Step 1: Run frontend gates**

```bash
cd frontend
pnpm test:theme-audit
pnpm test:e2e
pnpm build
```

Expected: all commands pass with zero failures.

- [ ] **Step 2: Run repository hygiene checks**

```bash
cd ..
git diff --check origin/main...HEAD
git status --short
```

Expected: no whitespace errors and no uncommitted files.

- [ ] **Step 3: Record the next build gate**

Do not claim a Universal artifact exists until `pnpm tauri:build:universal`, `lipo -info`, and ad-hoc `codesign --verify --deep --strict` pass in the later packaging slice.
