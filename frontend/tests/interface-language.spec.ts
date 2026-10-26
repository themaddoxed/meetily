import { expect, test } from './fixtures/tauri-fixtures';

type LanguageStorage = {
  interfaceLanguage: string | null;
  primaryLanguage: string | null;
  summaryLanguageDefault: string | null;
};

async function readLanguageStorage(page: import('@playwright/test').Page) {
  return page.evaluate((): LanguageStorage => ({
    interfaceLanguage: localStorage.getItem('interfaceLanguage'),
    primaryLanguage: localStorage.getItem('primaryLanguage'),
    summaryLanguageDefault: localStorage.getItem('summaryLanguageDefault'),
  }));
}

test('interface language switches in place, persists, and leaves speech and summary languages unchanged', async ({
  page,
}) => {
  await page.addInitScript(() => {
    if (localStorage.getItem('interfaceLanguage') === null) {
      localStorage.setItem('interfaceLanguage', 'en');
    }
    if (localStorage.getItem('primaryLanguage') === null) {
      localStorage.setItem('primaryLanguage', 'de');
    }
    if (localStorage.getItem('summaryLanguageDefault') === null) {
      localStorage.setItem('summaryLanguageDefault', 'fr');
    }

    document.documentElement.dataset.e2eInterfaceDocumentMarker = crypto.randomUUID();
  });

  await page.goto('/settings');

  const languageSelector = page.getByTestId('interface-language-selector');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await expect(languageSelector).toHaveValue('en');

  const beforeSwitch = await page.evaluate(() => ({
    marker: document.documentElement.dataset.e2eInterfaceDocumentMarker,
    url: window.location.href,
  }));
  const independentLanguages = await readLanguageStorage(page);

  await languageSelector.selectOption('ru');

  await expect(page.getByRole('heading', { name: 'Настройки', exact: true })).toBeVisible();
  await expect(languageSelector).toHaveValue('ru');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toBe('ru');

  const afterSwitch = await page.evaluate(() => ({
    marker: document.documentElement.dataset.e2eInterfaceDocumentMarker,
    url: window.location.href,
  }));
  expect(afterSwitch).toEqual(beforeSwitch);
  expect(await readLanguageStorage(page)).toEqual({
    interfaceLanguage: 'ru',
    primaryLanguage: independentLanguages.primaryLanguage,
    summaryLanguageDefault: independentLanguages.summaryLanguageDefault,
  });

  await page.reload();

  await expect(page.getByRole('heading', { name: 'Настройки', exact: true })).toBeVisible();
  await expect(page.getByTestId('interface-language-selector')).toHaveValue('ru');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toBe('ru');
  expect(await readLanguageStorage(page)).toEqual({
    interfaceLanguage: 'ru',
    primaryLanguage: independentLanguages.primaryLanguage,
    summaryLanguageDefault: independentLanguages.summaryLanguageDefault,
  });

  await page.getByTestId('interface-language-selector').selectOption('en');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toBe('en');
});
