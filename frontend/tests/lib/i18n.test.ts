import { afterEach, describe, expect, it } from 'vitest';
import {
  EN_MESSAGES,
  RU_MESSAGES,
  normalizeLocale,
  translateMessage,
} from '@/lib/i18n/catalog';

describe('interface language catalog', () => {
  const fallbackKey = 'settings.title';
  const originalFallback = RU_MESSAGES[fallbackKey];

  afterEach(() => {
    if (originalFallback === undefined) {
      delete RU_MESSAGES[fallbackKey];
    } else {
      RU_MESSAGES[fallbackKey] = originalFallback;
    }
  });

  it('normalizes unsupported persisted locales to English', () => {
    expect(normalizeLocale('de')).toBe('en');
    expect(normalizeLocale(null)).toBe('en');
    expect(normalizeLocale('ru')).toBe('ru');
  });

  it('interpolates variables without changing the selected language', () => {
    expect(translateMessage('ru', 'progress.chunksCompleted', { completed: 2, total: 5 }))
      .toBe('Завершено сегментов: 2 из 5');
  });

  it('interpolates both microphone names in route-change messages', () => {
    const variables = {
      previousDevice: 'Studio Microphone',
      currentDevice: 'Built-in Microphone',
    };
    expect(translateMessage('en', 'recording.microphoneDisconnectedContinues', variables))
      .toBe('Microphone changed from Studio Microphone to Built-in Microphone. Recording continues on the new microphone.');
    expect(translateMessage('ru', 'recording.microphoneDisconnectedContinues', variables))
      .toBe('Микрофон изменился: «Studio Microphone» → «Built-in Microphone». Запись продолжается через новый микрофон.');
  });

  it('falls back to readable English when a Russian entry is unavailable', () => {
    delete RU_MESSAGES[fallbackKey];

    expect(translateMessage('ru', fallbackKey)).toBe(EN_MESSAGES[fallbackKey]);
  });
});
