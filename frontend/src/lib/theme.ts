export type ThemeMode = 'light' | 'dark' | 'system';
export type EffectiveTheme = 'light' | 'dark';

type NativeThemeSetter = (theme: EffectiveTheme | null) => Promise<void>;

export interface NativeThemeSyncController {
  dispose: () => void;
  sync: (mode: ThemeMode) => Promise<void>;
}

export const THEME_STORAGE_KEY = 'themeMode';

export function parseThemeMode(value: string | null | undefined): ThemeMode {
  return value === 'light' || value === 'dark' || value === 'system'
    ? value
    : 'system';
}

export function readStoredThemeMode(): ThemeMode {
  if (typeof window === 'undefined') return 'system';

  try {
    return parseThemeMode(window.localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return 'system';
  }
}

export function createNativeThemeSyncController(
  options: {
    loadSetTheme?: () => Promise<NativeThemeSetter>;
    onError?: (error: unknown) => void;
  } = {},
): NativeThemeSyncController {
  const loadSetTheme =
    options.loadSetTheme ??
    (async () => {
      const { setTheme } = await import('@tauri-apps/api/app');
      return setTheme;
    });
  const onError = options.onError ?? (() => {});
  let disposed = false;
  let requestSequence = 0;
  let queue = Promise.resolve();

  return {
    dispose() {
      disposed = true;
      requestSequence += 1;
    },
    sync(mode) {
      if (disposed) return queue;

      const sequence = ++requestSequence;
      const applyTheme = async () => {
        try {
          const setTheme = await loadSetTheme();
          if (disposed || sequence !== requestSequence) return;
          await setTheme(mode === 'system' ? null : mode);
        } catch (error) {
          if (!disposed) onError(error);
        }
      };

      queue = queue.then(applyTheme, applyTheme);
      return queue;
    },
  };
}

export function resolveEffectiveTheme(
  mode: ThemeMode,
  prefersDark: boolean,
): EffectiveTheme {
  if (mode === 'system') {
    return prefersDark ? 'dark' : 'light';
  }

  return mode;
}

export function applyEffectiveTheme(theme: EffectiveTheme) {
  if (typeof document === 'undefined') return;

  const root = document.documentElement;
  root.classList.toggle('dark', theme === 'dark');
  root.style.colorScheme = theme;
}
