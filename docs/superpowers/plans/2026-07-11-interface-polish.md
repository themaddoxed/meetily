# Meetily Interface Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the approved complete interface-polish layer on the Universal macOS branch and produce a newly verified ad-hoc Universal DMG.

**Architecture:** Add semantic surface/shadow utilities and shared motion variants, then migrate shared primitives and selected product surfaces in small TDD batches. Keep motion out of long lists, transcripts, and generated summaries; enforce the policy with static and browser-level Playwright tests.

**Tech Stack:** Next.js 14, React 18, TypeScript, Tailwind CSS 3, Radix UI, Framer Motion 11, Playwright, Tauri 2.

---

## File Structure

- Create `frontend/src/lib/motion.ts`: shared reduced-motion-aware variants for onboarding and contextual icon swaps.
- Create `frontend/tests/interface-polish-policy.spec.ts`: static policy for shadows, transitions, wrapping, numerals, and motion constants.
- Create `frontend/tests/interface-polish-runtime.spec.ts`: computed hit-area, radius, shadow, and reduced-motion checks.
- Modify `frontend/src/app/globals.css`: light/dark semantic surface shadows and reusable surface classes.
- Modify `frontend/src/components/ui/button.tsx`: 40 px geometry, press scale, and reduced-motion behavior.
- Modify `frontend/src/components/ui/{dialog,sheet,popover,dropdown-menu,tabs,accordion,progress}.tsx`: role-based radii, wrapping, close-button hit areas, and explicit transitions.
- Modify onboarding components under `frontend/src/components/onboarding/`: step presence, staggered content, concentric cards, hit areas, and explicit transitions.
- Modify recording/status/toast components: contextual icon animation, tabular values, 40 px controls, and explicit transitions.
- Modify the exact application files listed in Task 3 to remove all remaining `transition-all`.

### Task 1: Establish the polish policy and semantic surface tokens

**Files:**
- Create: `frontend/tests/interface-polish-policy.spec.ts`
- Modify: `frontend/src/app/globals.css`

- [ ] **Step 1: Write the failing static policy test**

Create a Playwright test that reads application source and asserts:

```ts
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { expect, test } from '@playwright/test';

const read = (relativePath: string) =>
  readFileSync(path.resolve(relativePath), 'utf8');

const collectSourceFiles = (directory: string): string[] =>
  readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) return collectSourceFiles(entryPath);
      return /\.(css|ts|tsx)$/.test(entry.name) ? [entryPath] : [];
    })
    .sort();

const SOURCE_FILES = collectSourceFiles(path.resolve('src'));

test('defines adaptive semantic surface shadows', () => {
  const css = read('src/app/globals.css');
  expect(css).toContain('--shadow-surface:');
  expect(css).toContain('--shadow-surface-hover:');
  expect(css).toContain('.surface-card');
  expect(css).toContain('.dark {');
});

test('application source contains no transition-all utilities', () => {
  const offenders = SOURCE_FILES.filter((file) =>
    readFileSync(file, 'utf8').includes('transition-all'),
  );
  expect(offenders).toEqual([]);
});
```

- [ ] **Step 2: Run the policy test and verify RED**

Run: `pnpm exec playwright test tests/interface-polish-policy.spec.ts`

Expected: FAIL because the shadow variables/utilities are absent and 36 source occurrences still contain `transition-all`.

- [ ] **Step 3: Add the semantic shadow variables and utilities**

Add to the light and dark token blocks in `globals.css`:

```css
:root {
  --shadow-surface:
    0 0 0 1px rgb(0 0 0 / 0.06),
    0 1px 2px -1px rgb(0 0 0 / 0.06),
    0 2px 4px rgb(0 0 0 / 0.04);
  --shadow-surface-hover:
    0 0 0 1px rgb(0 0 0 / 0.08),
    0 1px 2px -1px rgb(0 0 0 / 0.08),
    0 2px 4px rgb(0 0 0 / 0.06);
}

.dark {
  --shadow-surface: 0 0 0 1px rgb(255 255 255 / 0.08);
  --shadow-surface-hover: 0 0 0 1px rgb(255 255 255 / 0.13);
}

@layer components {
  .surface-card {
    @apply rounded-xl bg-card text-card-foreground;
    box-shadow: var(--shadow-surface);
  }

  .surface-card-interactive {
    @apply surface-card transition-[box-shadow] duration-150 ease-out;
  }

  .surface-card-interactive:hover {
    box-shadow: var(--shadow-surface-hover);
  }
}
```

- [ ] **Step 4: Run the focused token assertion**

Temporarily run only the token test with `-g "adaptive semantic surface shadows"`.

Expected: PASS. The no-`transition-all` test remains RED until Task 3.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/app/globals.css frontend/tests/interface-polish-policy.spec.ts
git commit -m "test(ui): define interface polish policy"
```

### Task 2: Polish shared UI primitives

**Files:**
- Modify: `frontend/tests/interface-polish-policy.spec.ts`
- Create: `frontend/tests/interface-polish-runtime.spec.ts`
- Modify: `frontend/src/components/ui/button.tsx`
- Modify: `frontend/src/components/ui/dialog.tsx`
- Modify: `frontend/src/components/ui/sheet.tsx`
- Modify: `frontend/src/components/ui/popover.tsx`
- Modify: `frontend/src/components/ui/dropdown-menu.tsx`

- [ ] **Step 1: Write failing primitive policy assertions**

Assert source contains these exact contracts:

```ts
expect(read('src/components/ui/button.tsx')).toContain('active:scale-[0.96]');
expect(read('src/components/ui/button.tsx')).toContain('motion-reduce:transform-none');
expect(read('src/components/ui/button.tsx')).toContain('h-10');
expect(read('src/components/ui/dialog.tsx')).toContain('rounded-2xl');
expect(read('src/components/ui/dialog.tsx')).toContain('h-10 w-10');
```

Add runtime checks on `/settings` that locate representative buttons and assert `boundingBox().height >= 40` and `boundingBox().width >= 40` for icon-only buttons.

- [ ] **Step 2: Run primitive tests and verify RED**

Run: `pnpm exec playwright test tests/interface-polish-policy.spec.ts tests/interface-polish-runtime.spec.ts`

Expected: FAIL on 36 px button and compact close controls.

- [ ] **Step 3: Update the shared button**

Change the base/size classes to:

```ts
const buttonVariants = cva(
  'inline-flex min-h-10 items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-[color,background-color,border-color,box-shadow,scale] duration-150 ease-out active:scale-[0.96] motion-reduce:transform-none motion-reduce:transition-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:scale-100 disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0',
  {
    variants: {
      size: {
        default: 'h-10 px-4 py-2',
        sm: 'h-10 px-3 text-xs',
        lg: 'h-11 px-8',
        icon: 'h-10 w-10 p-0',
      },
    },
  },
);
```

Keep all existing color variants unchanged.

- [ ] **Step 4: Update overlay primitives**

- Dialog content: `rounded-2xl`, existing border, semantic shadow.
- Sheet content: retain structural side border, use `shadow-[var(--shadow-surface)]` and explicit transform transition.
- Popover/dropdown: `rounded-xl`, semantic shadow.
- Dialog/sheet close controls: `right-2 top-2 h-10 w-10 rounded-full`; content headers reserve `pr-10` so controls do not overlap title text.
- Dialog titles add `text-balance`; descriptions add `text-pretty`.

- [ ] **Step 5: Run primitive tests and verify GREEN**

Expected: focused policy/runtime tests pass in light, dark, and reduced-motion emulation.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/ui frontend/tests/interface-polish-policy.spec.ts frontend/tests/interface-polish-runtime.spec.ts
git commit -m "feat(ui): polish shared primitives"
```

### Task 3: Replace every broad transition with explicit properties

**Files:**
- Modify: all files reported by `rg -l 'transition-all' frontend/src`
- Test: `frontend/tests/interface-polish-policy.spec.ts`

- [ ] **Step 1: Confirm the policy test remains RED**

Run: `pnpm exec playwright test tests/interface-polish-policy.spec.ts -g "no transition-all"`

Expected: FAIL with the sorted list of offending files.

- [ ] **Step 2: Apply the exact replacement map**

Use these mappings based on the properties each component changes:

| Files / surfaces | Replacement |
|---|---|
| Progress widths in `UpdateDialog`, `DownloadProgressStep`, `BuiltInModelManager`, `DownloadProgressToast`, `ImportAudioDialog`, `ModelSettingsModal`, `RetranscribeDialog`, `ModelDownloadProgress`, `ChunkProgressDisplay`, `ui/progress` | `transition-[width]` or `transition-transform` when Radix progress uses translate |
| Switch knob in `SettingsModal` | `after:transition-transform` |
| `ProgressIndicator` dots and lines | `transition-[width,height,background-color,color,scale,opacity,filter]` split between containers/icons |
| `PermissionRow`, `AudioBackendSelector`, model cards, AISummary blocks, Sidebar rows | explicit `transition-[color,background-color,border-color,box-shadow,scale,opacity]`, removing properties not changed by that element |
| Onboarding arrows | `transition-[color,background-color,box-shadow,scale,opacity]` |
| `MainContent` and Sidebar width | `transition-[width,margin]` or `transition-[width]` according to their state expression |
| Audio meters and recording bars | `transition-[height,background-color]`, `transition-[left,background-color]`, or `transition-[transform,background-color]` matching inline style updates |
| Tabs and accordion | `transition-[color,background-color,box-shadow]` and `transition-[color]`; keep the chevron's existing rotate utility |
| Compliance notification | `transition-[opacity,transform]` |

Do not modify Framer Motion animation objects in transcript, summary, or virtualized-list files unless the only change is removal of a Tailwind `transition-all` class.

- [ ] **Step 3: Run the static scan and verify GREEN**

Run both:

```bash
rg -n 'transition-all' frontend/src
pnpm exec playwright test tests/interface-polish-policy.spec.ts -g "no transition-all"
```

Expected: `rg` exits with no matches; policy test passes.

- [ ] **Step 4: Run existing E2E tests**

Run: `pnpm test:e2e`

Expected: all existing tests pass.

- [ ] **Step 5: Commit**

```bash
git add frontend/src frontend/tests/interface-polish-policy.spec.ts
git commit -m "refactor(ui): use explicit transitions"
```

### Task 4: Add targeted typography, numerals, and surface geometry

**Files:**
- Modify: `frontend/tests/interface-polish-policy.spec.ts`
- Modify: `frontend/src/app/settings/page.tsx`
- Modify: `frontend/src/components/onboarding/OnboardingContainer.tsx`
- Modify: `frontend/src/components/onboarding/steps/{WelcomeStep,SetupOverviewStep,PermissionsStep,DownloadProgressStep}.tsx`
- Modify: `frontend/src/components/{RecordingControls,RecordingStatusBar,ModelDownloadProgress}.tsx`
- Modify: `frontend/src/components/shared/DownloadProgressToast.tsx`

- [ ] **Step 1: Add failing typography/numeral assertions**

Assert representative files contain `text-balance`, `text-pretty`, and `tabular-nums`, and runtime-check computed `fontVariantNumeric` on recording/progress values.

- [ ] **Step 2: Run and verify RED**

Expected: zero current `tabular-nums` and no balance/pretty utilities.

- [ ] **Step 3: Apply role-based geometry**

- Onboarding outer cards: `rounded-2xl`; nested 8 px inset surfaces: `rounded-lg`.
- Settings and model standalone cards: `surface-card` or `rounded-xl` with semantic shadow.
- Preserve form control borders and divider borders.
- Do not change transcript or generated-summary geometry.

- [ ] **Step 4: Apply typography and numerals**

- Add `text-balance` to page/dialog/onboarding/card headings.
- Add `text-pretty` to short descriptions and helper copy.
- Add `tabular-nums` to recording time, elapsed time, percentage labels, and changing progress values.
- Do not add wrapping utilities to transcript, summary, logs, or code.

- [ ] **Step 5: Run focused tests and route screenshots**

Run the policy/runtime specs plus existing theme shell/settings/onboarding coverage. Review changed screenshots individually.

- [ ] **Step 6: Commit**

```bash
git add frontend/src frontend/tests
git commit -m "feat(ui): polish surfaces and typography"
```

### Task 5: Add shared reduced-motion variants and onboarding transitions

**Files:**
- Create: `frontend/src/lib/motion.ts`
- Modify: `frontend/tests/interface-polish-policy.spec.ts`
- Modify: `frontend/tests/interface-polish-runtime.spec.ts`
- Modify: `frontend/src/components/onboarding/OnboardingFlow.tsx`
- Modify: `frontend/src/components/onboarding/OnboardingContainer.tsx`

- [ ] **Step 1: Write failing motion contract tests**

Assert the motion module exports exact constants and the flow uses `AnimatePresence initial={false} mode="wait"`. Runtime test emulates `reducedMotion: 'reduce'` and verifies onboarding content has no non-identity transform.

- [ ] **Step 2: Run and verify RED**

Expected: FAIL because `src/lib/motion.ts` does not exist and onboarding switches components without presence handling.

- [ ] **Step 3: Create shared variants**

```ts
import type { Variants } from 'framer-motion';

export const createStepContainerVariants = (reduceMotion: boolean): Variants => ({
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: reduceMotion ? { duration: 0.1 } : { staggerChildren: 0.1 },
  },
  exit: { opacity: 0, transition: { duration: 0.15 } },
});

export const createStepItemVariants = (reduceMotion: boolean): Variants => ({
  hidden: reduceMotion
    ? { opacity: 0 }
    : { opacity: 0, y: 12, filter: 'blur(4px)' },
  visible: reduceMotion
    ? { opacity: 1, transition: { duration: 0.1 } }
    : { opacity: 1, y: 0, filter: 'blur(0px)', transition: { duration: 0.3 } },
  exit: reduceMotion
    ? { opacity: 0, transition: { duration: 0.1 } }
    : { opacity: 0, y: -12, filter: 'blur(4px)', transition: { duration: 0.15 } },
});

export const iconSpring = { type: 'spring', duration: 0.3, bounce: 0 } as const;
```

- [ ] **Step 4: Wire onboarding presence**

Use `useReducedMotion`, key the current step, wrap it in `AnimatePresence initial={false} mode="wait"`, and apply container/item variants to title, description/content, and actions. Remove duplicate CSS `animate-fade-in-up` classes to prevent double animation.

- [ ] **Step 5: Run motion tests and verify GREEN**

Verify normal motion gets the specified values and reduced motion never receives translate/scale/blur.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/motion.ts frontend/src/components/onboarding frontend/tests
git commit -m "feat(ui): add accessible onboarding motion"
```

### Task 6: Animate contextual status icons without layout shift

**Files:**
- Modify: `frontend/tests/interface-polish-policy.spec.ts`
- Modify: `frontend/src/components/onboarding/shared/ProgressIndicator.tsx`
- Modify: `frontend/src/components/shared/DownloadProgressToast.tsx`
- Modify: `frontend/src/components/RecordingStatusBar.tsx`

- [ ] **Step 1: Write failing contextual-icon assertions**

Assert each selected component uses `AnimatePresence initial={false}`, stable keyed icon states, `iconSpring`, scale `0.25`, and blur `4px`.

- [ ] **Step 2: Run and verify RED**

Expected: FAIL because the current status icons toggle immediately or use unrelated transitions.

- [ ] **Step 3: Implement stable icon swaps**

For each selected component:

```tsx
<AnimatePresence initial={false} mode="popLayout">
  <motion.span
    key={status}
    className="flex h-5 w-5 items-center justify-center"
    initial={reduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.25, filter: 'blur(4px)' }}
    animate={{ opacity: 1, scale: 1, filter: 'blur(0px)' }}
    exit={reduceMotion ? { opacity: 0 } : { opacity: 0, scale: 0.25, filter: 'blur(4px)' }}
    transition={iconSpring}
  >
    <StatusIcon />
  </motion.span>
</AnimatePresence>
```

Keep the wrapper dimensions fixed. Do not animate progress widths or virtualized rows through Framer Motion.

- [ ] **Step 4: Run policy/runtime tests and verify GREEN**

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components frontend/tests
git commit -m "feat(ui): animate contextual status icons"
```

### Task 7: Full verification and Universal build

**Files:**
- Verify only: all changed frontend and packaging files

- [ ] **Step 1: Run source-policy checks**

```bash
rg -n 'transition-all' frontend/src
pnpm test:theme-audit
```

Expected: no `transition-all`; theme audit exits 0.

- [ ] **Step 2: Run full browser suite**

Run: `pnpm test:e2e`

Expected: all Playwright tests pass in Chromium, including reduced-motion and computed geometry checks.

- [ ] **Step 3: Run production frontend build**

Run: `pnpm build`

Expected: Next production build exits 0 with all routes generated.

- [ ] **Step 4: Build the Universal ad-hoc artifact**

Run: `pnpm tauri:build:universal`

Expected output: `target/universal-apple-darwin/release/bundle/dmg/meetily_0.4.0_universal.dmg`.

- [ ] **Step 5: Verify bundle architecture, signature, and DMG**

Run `lipo -info` on `meetily`, `llama-helper`, and `ffmpeg`; all must report `x86_64 arm64`. Run `codesign --verify --deep --strict`, confirm `Signature=adhoc`, then run `hdiutil verify` and calculate SHA-256.

- [ ] **Step 6: Smoke-launch on the current Intel Mac**

Open the built `.app`, confirm the exact bundle process starts, then terminate only that process.

- [ ] **Step 7: Commit any intentional snapshot updates**

```bash
git add frontend/tests
git commit -m "test(ui): update polished interface snapshots"
```

Skip this commit when no snapshot files changed.
