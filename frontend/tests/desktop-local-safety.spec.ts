import { expect, test } from './fixtures/tauri-fixtures';

type CommandJournalEntry = {
  command: string;
  payload?: unknown;
};

async function commandJournal(page: import('@playwright/test').Page) {
  return page.evaluate(
    () =>
      (
        window as typeof window & {
          __tauriCommandJournal?: CommandJournalEntry[];
        }
      ).__tauriCommandJournal ?? [],
  );
}

test('missing local summary model preserves the saved transcript and summary', async ({
  page,
}) => {
  await page.goto(
    '/meeting-details?id=theme-test-meeting&__e2e=missing-local-summary-model',
  );

  const transcript = page.getByText('We approved the dark theme.', { exact: true });
  const savedSummary = page.getByText('Use semantic tokens.', { exact: true });
  await expect(transcript).toBeVisible();
  await expect(savedSummary).toBeVisible();

  await page.getByRole('button', { name: 'Regenerate AI Summary', exact: true }).click();

  await expect(page.getByText('Model not downloaded', { exact: true })).toBeVisible();
  await expect(page.getByRole('dialog')).toContainText('Model Settings');
  await expect(transcript).toBeVisible();
  await expect(savedSummary).toBeVisible();

  const journal = await commandJournal(page);
  expect(journal).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        command: 'builtin_ai_is_model_ready',
        payload: expect.objectContaining({ modelName: 'qwen3.5:4b' }),
      }),
      expect.objectContaining({
        command: 'builtin_ai_get_model_info',
        payload: expect.objectContaining({ modelName: 'qwen3.5:4b' }),
      }),
    ]),
  );

  const forbiddenMutationCommands = new Set([
    'api_process_transcript',
    'api_save_meeting_summary',
    'api_save_meeting_summary_language',
    'api_save_meeting_detected_summary_language',
    'api_save_transcript_config',
    'api_save_transcript',
    'save_transcript',
  ]);
  const unsafeMutations = journal.filter(
    ({ command }) =>
      forbiddenMutationCommands.has(command) || /export/i.test(command),
  );
  expect(
    unsafeMutations,
    `missing-model flow must not mutate saved meeting data: ${JSON.stringify(unsafeMutations)}`,
  ).toEqual([]);
});

test('offline-startup observes no WebView egress or updater command in the browser harness (not packaged offline evidence)', async ({
  context,
  page,
}) => {
  const externalHttpRequests: string[] = [];
  const externalWebSockets: string[] = [];
  const pageErrors: string[] = [];
  const appOrigin = 'http://127.0.0.1:3118';

  await context.route(/^https?:\/\//, async (route) => {
    const requestUrl = route.request().url();
    if (new URL(requestUrl).origin === appOrigin) {
      await route.continue();
      return;
    }

    externalHttpRequests.push(requestUrl);
    await route.abort('blockedbyclient');
  });
  page.on('websocket', (socket) => {
    const socketUrl = new URL(socket.url());
    if (
      socketUrl.hostname !== '127.0.0.1' ||
      socketUrl.port !== '3118'
    ) {
      externalWebSockets.push(socket.url());
    }
  });
  page.on('pageerror', (error) => pageErrors.push(error.message));

  await page.goto(
    '/meeting-details?id=theme-test-meeting&__e2e=offline-startup',
  );

  await expect(
    page.getByText('We approved the dark theme.', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Use semantic tokens.', { exact: true })).toBeVisible();
  await page.waitForTimeout(2_250);
  const journal = await page.evaluate(() => (
    window as typeof window & {
      __tauriCommandJournal?: CommandJournalEntry[];
    }
  ).__tauriCommandJournal ?? []);

  await expect(
    page.getByText('We approved the dark theme.', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Use semantic tokens.', { exact: true })).toBeVisible();

  expect(
    journal.filter(({ command }) => command.startsWith('plugin:updater|')),
  ).toEqual([]);
  expect(externalHttpRequests).toEqual([]);
  expect(externalWebSockets).toEqual([]);
  expect(pageErrors).toEqual([]);
});
