import { existsSync, readdirSync, readFileSync } from 'node:fs';
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

const sourceFiles = collectSourceFiles(path.resolve('src'));

test('defines adaptive semantic surface shadows', () => {
  const css = read('src/app/globals.css');

  expect(css).toContain('--shadow-surface:');
  expect(css).toContain('--shadow-surface-hover:');
  expect(css).toContain('.surface-card');
  expect(css).toContain('.surface-card-interactive');
});

test('application source contains no transition-all utilities', () => {
  const offenders = sourceFiles
    .filter((file) => readFileSync(file, 'utf8').includes('transition-all'))
    .map((file) => path.relative(process.cwd(), file));

  expect(offenders).toEqual([]);
});

test('scale states transition the Tailwind 3 transform property', () => {
  const offenders = sourceFiles.flatMap((file) => {
    const source = readFileSync(file, 'utf8');
    return [...source.matchAll(/transition-\[([^\]]*\bscale\b[^\]]*)\]/g)].map(
      (match) => `${path.relative(process.cwd(), file)}: ${match[0]}`,
    );
  });

  expect(offenders).toEqual([]);
});

test('shared controls define tactile and accessible geometry', () => {
  const button = read('src/components/ui/button.tsx');
  const dialog = read('src/components/ui/dialog.tsx');

  expect(button).toContain('active:scale-[0.96]');
  expect(button).toContain('motion-reduce:transform-none');
  expect(button).toContain('default: "h-10');
  expect(button).toContain('icon: "h-10 w-10');
  expect(dialog).toContain('rounded-2xl');
  expect(dialog).toContain('h-10 w-10');
  expect(dialog).toContain('text-balance');
  expect(dialog).toContain('text-pretty');
});

test('short UI copy and dynamic values use stable typography', () => {
  const preferences = read('src/components/PreferenceSettings.tsx');
  const onboarding = read('src/components/onboarding/OnboardingContainer.tsx');
  const recording = read('src/components/RecordingStatusBar.tsx');
  const progress = read('src/components/ModelDownloadProgress.tsx');
  const toast = read('src/components/shared/DownloadProgressToast.tsx');

  expect(preferences).toContain('text-balance');
  expect(preferences).toContain('text-pretty');
  expect(onboarding).toContain('text-balance');
  expect(onboarding).toContain('text-pretty');
  expect(recording).toContain('tabular-nums');
  expect(progress).toContain('tabular-nums');
  expect(toast).toContain('tabular-nums');
});

test('onboarding motion is shared, staggered, and reduced-motion aware', () => {
  const motionPath = 'src/lib/motion.ts';
  const motion = existsSync(path.resolve(motionPath)) ? read(motionPath) : '';
  const flow = read('src/components/onboarding/OnboardingFlow.tsx');
  const container = read('src/components/onboarding/OnboardingContainer.tsx');

  expect(motion).toContain('staggerChildren: 0.1');
  expect(motion).toContain('y: 12');
  expect(motion).toContain('y: -12');
  expect(motion).toContain("filter: 'blur(4px)'");
  expect(motion).toContain('duration: 0.15');
  expect(motion).toContain('bounce: 0');
  expect(flow).toContain('initial={false}');
  expect(flow).toContain('mode="wait"');
  expect(flow).toContain('useLiveReducedMotion');
  expect(container).toContain('createStepItemVariants');
  expect(container).not.toContain('animate-fade-in-up');
});

test('reduced-motion preference uses a hydration-safe external store', () => {
  const hook = read('src/hooks/use-live-reduced-motion.ts');

  expect(hook).toContain('useSyncExternalStore');
  expect(hook).toContain("const reducedMotionQuery = '(prefers-reduced-motion: reduce)'");
  expect(hook).toContain('window.matchMedia(reducedMotionQuery)');
  expect(hook).toContain("mediaQuery.addEventListener('change', callback)");
  expect(hook).toContain("mediaQuery.removeEventListener('change', callback)");
  expect(hook).toContain('const getServerSnapshot = () => false');
  expect(hook).not.toContain('framer-motion');
});

test('contextual status icons cross-fade without changing layout', () => {
  const helperPath = 'src/components/ui/contextual-icon.tsx';
  const helper = existsSync(path.resolve(helperPath)) ? read(helperPath) : '';

  expect(helper).toContain('AnimatePresence initial={false}');
  expect(helper).toContain('mode="popLayout"');
  expect(helper).toContain('useLiveReducedMotion');
  expect(helper).toContain('scale: 0.25');
  expect(helper).toContain("filter: 'blur(4px)'");
  expect(helper).toContain('iconSpring');

  for (const component of [
    'src/components/onboarding/shared/ProgressIndicator.tsx',
    'src/components/shared/DownloadProgressToast.tsx',
    'src/components/RecordingStatusBar.tsx',
  ]) {
    expect(read(component)).toContain('ContextualIcon');
  }
});

test('contextual icons keep status semantics on their interactive caller', () => {
  const helperPath = 'src/components/ui/contextual-icon.tsx';
  const helper = existsSync(path.resolve(helperPath)) ? read(helperPath) : '';
  const progress = read('src/components/onboarding/shared/ProgressIndicator.tsx');

  expect(helper).toContain('aria-hidden="true"');
  expect(progress).toContain('aria-label={`Step ${step}');
  expect(progress).toContain("aria-current={isActive ? 'step' : undefined}");
});
