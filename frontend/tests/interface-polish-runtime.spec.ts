import { expect, test } from './fixtures/tauri-fixtures';

test('appearance controls expose a 40px hit area', async ({ page }) => {
  await page.goto('/settings');

  for (const name of ['System', 'Light', 'Dark']) {
    const button = page.getByRole('button', { name, exact: true });
    await expect(button).toBeVisible();
    const box = await button.boundingBox();

    expect(box?.height).toBeGreaterThanOrEqual(40);
    expect(box?.width).toBeGreaterThanOrEqual(40);
  }
});

test('shared buttons disable press transitions when reduced motion is requested', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/settings');

  const button = page.getByRole('button', { name: 'Dark', exact: true });
  await expect(button).toBeVisible();
  await expect(button).toHaveCSS('min-height', '40px');
  await expect(button).toHaveCSS('transition-property', 'none');
});

test('settings headings and short descriptions use intentional wrapping', async ({
  page,
}) => {
  await page.goto('/settings');

  await expect(page.getByRole('heading', { name: 'Appearance' })).toHaveCSS(
    'text-wrap',
    'balance',
  );
  await expect(
    page.getByText('Choose how Meetily should render the interface'),
  ).toHaveCSS('text-wrap', 'pretty');
});

test('onboarding removes transform and blur under reduced motion', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/?__e2e=onboarding');

  const title = page.getByRole('heading', { name: 'Welcome to Meetily' });
  await expect(title).toBeVisible();
  await expect(title).toHaveCSS('transform', 'none');
  await expect(title).toHaveCSS('filter', 'none');
});
