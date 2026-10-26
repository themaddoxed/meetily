'use client';

import { createContext, createElement, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import {
  DEFAULT_LOCALE,
  INTERFACE_LANGUAGE_STORAGE_KEY,
  type Locale,
  type MessageKey,
  type MessageVariables,
  normalizeLocale,
  translateMessage,
} from '@/lib/i18n/catalog';

export { DEFAULT_LOCALE, normalizeLocale, translateMessage } from '@/lib/i18n/catalog';
export type { Locale, MessageKey, MessageVariables } from '@/lib/i18n/catalog';

interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: MessageKey, variables?: MessageVariables) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function readStoredLocale(): Locale {
  if (typeof window === 'undefined') return DEFAULT_LOCALE;
  try {
    return normalizeLocale(window.localStorage.getItem(INTERFACE_LANGUAGE_STORAGE_KEY));
  } catch {
    return DEFAULT_LOCALE;
  }
}

export function I18nProvider({ children }: { children: ReactNode }) {
  // Keep the server and first client render in English; hydrate localStorage in
  // an effect so Next.js does not compare different server/client markup.
  const [locale, setLocaleState] = useState<Locale>(DEFAULT_LOCALE);

  useEffect(() => {
    const storedLocale = readStoredLocale();
    setLocaleState(storedLocale);
    document.documentElement.lang = storedLocale;
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const setLocale = useCallback((nextLocale: Locale) => {
    const safeLocale = normalizeLocale(nextLocale);
    setLocaleState(safeLocale);
    try {
      window.localStorage.setItem(INTERFACE_LANGUAGE_STORAGE_KEY, safeLocale);
    } catch {
      // Interface language still applies for this session when storage is unavailable.
    }
  }, []);

  const t = useCallback((key: MessageKey, variables?: MessageVariables) => (
    translateMessage(locale, key, variables)
  ), [locale]);

  const value = useMemo(() => ({ locale, setLocale, t }), [locale, setLocale, t]);

  return createElement(I18nContext.Provider, { value }, children);
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}
