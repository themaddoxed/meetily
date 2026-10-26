import { expect, test } from './fixtures/tauri-fixtures';
import { createNativeThemeSyncController } from '../src/lib/theme';

type NativeTheme = 'light' | 'dark' | null;

type NativeThemeTestWindow = typeof window & {
  __failNativeTheme?: boolean;
  __nativeThemeCalls?: NativeTheme[];
  __nativeThemeDelays?: Partial<Record<'light' | 'dark' | 'system', number>>;
  __nativeThemeFailures?: number;
  __nativeThemeStarts?: NativeTheme[];
};

async function initializeNativeThemeHarness(
  page: Parameters<Parameters<typeof test>[1]>[0]['page'],
  storedTheme?: 'light' | 'dark' | 'system',
) {
  await page.addInitScript((theme) => {
    const state = window as NativeThemeTestWindow;
    state.__nativeThemeCalls = [];
    state.__nativeThemeFailures = 0;
    state.__nativeThemeStarts = [];
    if (theme) localStorage.setItem('themeMode', theme);
  }, storedTheme);
}

test('disposing native theme sync invalidates a delayed module load', async () => {
  let markLoaderStarted: (() => void) | undefined;
  const loaderStarted = new Promise<void>((resolve) => {
    markLoaderStarted = resolve;
  });
  let resolveLoader:
    | ((setter: (theme: NativeTheme) => Promise<void>) => void)
    | undefined;
  const loadSetTheme = new Promise<(theme: NativeTheme) => Promise<void>>(
    (resolve) => {
      resolveLoader = resolve;
    },
  );
  const calls: NativeTheme[] = [];
  const controller = createNativeThemeSyncController({
    loadSetTheme: () => {
      markLoaderStarted?.();
      return loadSetTheme;
    },
  });

  const pending = controller.sync('dark');
  await loaderStarted;
  controller.dispose();
  resolveLoader?.(async (theme) => {
    calls.push(theme);
  });
  await pending;

  expect(calls).toEqual([]);
});

test('applies stored dark mode before DOMContentLoaded', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('themeMode', 'dark');

    const originalToggle = DOMTokenList.prototype.toggle;
    DOMTokenList.prototype.toggle = function toggle(token, force) {
      const result = originalToggle.call(this, token, force);
      if (
        token === 'dark' &&
        force === true &&
        this === document.documentElement?.classList
      ) {
        (
          window as typeof window & {
            __darkAppliedBeforeBody?: boolean;
          }
        ).__darkAppliedBeforeBody = document.body === null;
      }
      return result;
    };
  });

  await page.goto('/settings');

  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as typeof window & {
              __darkAppliedBeforeBody?: boolean;
            }
          ).__darkAppliedBeforeBody,
      ),
    )
    .toBe(true);
  await expect(page.locator('html')).toHaveClass(/dark/);
});

test('system mode follows the browser color scheme', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(() => localStorage.setItem('themeMode', 'system'));
  await page.goto('/settings');
  await expect(page.locator('html')).toHaveClass(/dark/);

  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).not.toHaveClass(/dark/);
});

test('clicking Dark persists the theme across reloads', async ({ page }) => {
  await page.goto('/settings');

  await page.getByRole('button', { name: 'Dark', exact: true }).click();

  await expect(page.locator('html')).toHaveClass(/dark/);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem('themeMode')))
    .toBe('dark');

  await page.reload();

  await expect(page.locator('html')).toHaveClass(/dark/);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem('themeMode')))
    .toBe('dark');
});

test('native theme IPC receives light, dark, and null system selections', async ({
  page,
}) => {
  await initializeNativeThemeHarness(page, 'system');
  await page.goto('/settings');

  await expect(page.getByRole('heading', { name: 'Appearance' })).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as NativeThemeTestWindow).__nativeThemeCalls,
    ),
  ).toEqual([]);

  for (const [button, expected] of [
    ['Light', 'light'],
    ['Dark', 'dark'],
    ['System', null],
  ] as const) {
    await page.getByRole('button', { name: button, exact: true }).click();
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as NativeThemeTestWindow).__nativeThemeCalls?.at(-1),
        ),
      )
      .toBe(expected);
  }

  expect(
    await page.evaluate(
      () => (window as NativeThemeTestWindow).__nativeThemeCalls,
    ),
  ).toEqual(['light', 'dark', null]);
});

test('latest theme selection wins over initial native synchronization', async ({
  page,
}) => {
  await initializeNativeThemeHarness(page, 'dark');
  await page.addInitScript(() => {
    (window as NativeThemeTestWindow).__nativeThemeDelays = { dark: 250 };
  });
  await page.goto('/settings');

  await expect
    .poll(() =>
      page.evaluate(
        () => (window as NativeThemeTestWindow).__nativeThemeStarts,
      ),
    )
    .toContain('dark');
  await page.getByRole('button', { name: 'Light', exact: true }).click();

  await expect
    .poll(() =>
      page.evaluate(
        () => (window as NativeThemeTestWindow).__nativeThemeCalls?.length,
      ),
    )
    .toBe(2);
  expect(
    await page.evaluate(
      () => (window as NativeThemeTestWindow).__nativeThemeCalls,
    ),
  ).toEqual(['dark', 'light']);
  await expect(page.locator('html')).not.toHaveClass(/dark/);
});

test('native theme mock rejects missing and invalid payloads', async ({ page }) => {
  await page.goto('/settings');

  const errors = await page.evaluate(async () => {
    const invoke = (
      window as typeof window & {
        __TAURI_INTERNALS__: {
          invoke: (command: string, payload: unknown) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__.invoke;

    return Promise.all(
      [{}, { theme: undefined }, { theme: 'sepia' }].map(async (payload) => {
        try {
          await invoke('plugin:app|set_app_theme', payload);
          return 'accepted';
        } catch (error) {
          return error instanceof Error ? error.message : String(error);
        }
      }),
    );
  });

  expect(errors).toEqual([
    '[E2E fixture invalid] set_app_theme requires light, dark, or null',
    '[E2E fixture invalid] set_app_theme requires light, dark, or null',
    '[E2E fixture invalid] set_app_theme requires light, dark, or null',
  ]);
});

test('native theme failures do not block the selected web theme', async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  await initializeNativeThemeHarness(page, 'system');
  await page.addInitScript(() => {
    (window as NativeThemeTestWindow).__failNativeTheme = true;
  });
  await page.goto('/settings');

  await page.getByRole('button', { name: 'Dark', exact: true }).click();

  await expect(page.locator('html')).toHaveClass(/dark/);
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as NativeThemeTestWindow).__nativeThemeFailures,
      ),
    )
    .toBe(1);
  expect(pageErrors).toEqual([]);
});

test('theme selector exposes and updates its selected state', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('themeMode', 'system'));
  await page.goto('/settings');

  const light = page.getByRole('button', { name: 'Light', exact: true });
  const dark = page.getByRole('button', { name: 'Dark', exact: true });
  const system = page.getByRole('button', { name: 'System', exact: true });

  await expect(light).toHaveAttribute('aria-pressed', 'false');
  await expect(dark).toHaveAttribute('aria-pressed', 'false');
  await expect(system).toHaveAttribute('aria-pressed', 'true');

  await dark.click();

  await expect(light).toHaveAttribute('aria-pressed', 'false');
  await expect(dark).toHaveAttribute('aria-pressed', 'true');
  await expect(system).toHaveAttribute('aria-pressed', 'false');
});

test('summary prompt storage is read only during context initialization', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const trackedWindow = window as typeof window & {
      __summaryPromptReads?: number;
    };
    trackedWindow.__summaryPromptReads = 0;

    const originalGetItem = Storage.prototype.getItem;
    Storage.prototype.getItem = function getItem(key) {
      if (key === 'summarySystemPrompt') {
        trackedWindow.__summaryPromptReads =
          (trackedWindow.__summaryPromptReads ?? 0) + 1;
      }
      return originalGetItem.call(this, key);
    };
  });

  await page.goto('/settings');
  await expect(page.getByRole('heading', { name: 'Appearance' })).toBeVisible();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();

  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as typeof window & {
              __summaryPromptReads?: number;
            }
          ).__summaryPromptReads,
      ),
    )
    .toBe(1);
});

test('invalid stored theme falls back to the dark system theme', async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(() => localStorage.setItem('themeMode', 'sepia'));

  await page.goto('/settings');

  await expect(page.locator('html')).toHaveClass(/dark/);
});

test('unavailable theme storage falls back to system without blocking render', async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));

  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(() => {
    const originalGetItem = Storage.prototype.getItem;
    Storage.prototype.getItem = function getItem(key) {
      if (key === 'themeMode') {
        throw new DOMException('Theme storage unavailable', 'SecurityError');
      }
      return originalGetItem.call(this, key);
    };
  });

  await page.goto('/settings');

  await expect(page.getByRole('heading', { name: 'Appearance' })).toBeVisible();
  await expect(page.locator('html')).toHaveClass(/dark/);
  expect(pageErrors).toEqual([]);
});

test('selected theme still applies when theme storage writes fail', async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));

  await page.emulateMedia({ colorScheme: 'light' });
  await page.addInitScript(() => {
    const originalSetItem = Storage.prototype.setItem;
    Storage.prototype.setItem = function setItem(key, value) {
      if (key === 'themeMode') {
        throw new DOMException('Theme storage unavailable', 'SecurityError');
      }
      return originalSetItem.call(this, key, value);
    };
  });

  await page.goto('/settings');
  await page.getByRole('button', { name: 'Dark', exact: true }).click();

  await expect(page.locator('html')).toHaveClass(/dark/);
  expect(pageErrors).toEqual([]);
});
