import { expect, test } from './fixtures/tauri-fixtures';

type QuestionWindow = Window & {
  __TAURI_INTERNALS__: { invoke: (command: string, payload?: Record<string, unknown>, options?: unknown) => Promise<unknown> };
  __questionRequests: Record<string, unknown>[];
  __questionCancels: Record<string, unknown>[];
  __resolveQuestion?: () => void;
  __resolveQuestionConfiguration?: () => void;
  __emitTauriEvent: (event: string, payload: Record<string, unknown>) => Promise<void>;
};

async function openQuestions(page: import('@playwright/test').Page, mode: 'answer' | 'network' | 'pending') {
  await page.goto('/settings');
  await page.getByRole('button', { name: 'Expand sidebar', exact: true }).click();
  await page.evaluate(({ mode }) => {
    const w = window as QuestionWindow;
    const original = w.__TAURI_INTERNALS__.invoke;
    w.__questionRequests = []; w.__questionCancels = [];
    const configuration = { provider: mode === 'network' ? 'openai' : 'builtin-ai', model: mode === 'network' ? 'fixture-network' : 'qwen3.5:2b', transport: mode === 'network' ? 'network' : 'local', fingerprint: 'fixture-config' };
    const transcripts = Array.from({ length: 160 }, (_, index) => ({ id: `question-target-${index}`, text: index === 120 ? 'We agreed to ship on Friday.' : `Transcript row ${index}`, timestamp: '09:00:00', audio_start_time: index * 5, audio_end_time: index * 5 + 4, duration: 4, confidence: 0.98 }));
    const response = { meetingId: 'theme-test-meeting', configuration, content: mode === 'network'
      ? { insufficientEvidence: true, claims: [] }
      : { insufficientEvidence: false, claims: [{ text: 'Shipping is Friday.', citations: [{ transcriptId: 'question-target-120', quote: 'We agreed to ship on Friday.', href: '/meeting-details?id=theme-test-meeting&source=question&transcriptId=question-target-120&at=600' }] }] } };
    w.__TAURI_INTERNALS__.invoke = async (command, payload, options) => {
      if (command === 'api_search_transcripts') return [{ id: 'theme-test-meeting', title: 'Theme Test Meeting', matchContext: 'Selected question meeting', timestamp: '2026-06-20T09:00:00Z', sourceType: 'title' }];
      if (command === 'api_get_meeting_question_configuration') return configuration;
      if (command === 'api_get_meeting_transcripts') {
        const offset = Number(payload?.offset ?? 0); const limit = Number(payload?.limit ?? 50);
        return { transcripts: transcripts.slice(offset, offset + limit), has_more: offset + limit < transcripts.length, total_count: transcripts.length };
      }
      if (command === 'api_ask_meeting_question') {
        w.__questionRequests.push(payload ?? {});
        if (mode === 'pending') return new Promise(resolve => { w.__resolveQuestion = () => resolve(response); });
        return response;
      }
      if (command === 'api_cancel_meeting_question') { w.__questionCancels.push(payload ?? {}); return null; }
      return original(command, payload, options);
    };
  }, { mode });
  await page.getByPlaceholder('Search meeting content...').fill('questions');
  await page.getByRole('button', { name: 'Open Meeting match in Theme Test Meeting' }).click();
  await expect(page.getByRole('region', { name: 'Ask this meeting' })).toBeVisible({ timeout: 30_000 });
}

test('selected meeting answer shows provider and opens a deep transcript source', async ({ page }) => {
  test.setTimeout(90_000); await openQuestions(page, 'answer');
  await expect(page.getByTestId('question-provider')).toContainText('builtin-ai · qwen3.5:2b · Local');
  await page.getByLabel('Question about this meeting').fill('When do we ship?');
  await page.getByRole('button', { name: 'Ask meeting', exact: true }).click();
  await expect(page.getByText('Shipping is Friday.', { exact: true })).toBeVisible();
  const requests = await page.evaluate(() => (window as QuestionWindow).__questionRequests);
  expect(requests).toHaveLength(1); expect(requests[0].meetingId).toBe('theme-test-meeting');
  expect(Object.keys(requests[0]).sort()).toEqual(['configurationFingerprint', 'meetingId', 'networkConsent', 'question', 'requestId']);
  await page.getByRole('link', { name: 'Open source transcript' }).click();
  await expect(page).toHaveURL(/source=question&transcriptId=question-target-120&at=600/);
  const target = page.locator('#segment-question-target-120');
  await expect(target).toHaveAttribute('aria-current', 'location', { timeout: 30_000 });
  await expect(target).toBeInViewport({ timeout: 30_000 });
  await page.screenshot({ path: '/private/tmp/meetily-question-source.png', fullPage: false });
});

test('network question requires explicit consent and displays insufficient evidence', async ({ page }) => {
  test.setTimeout(90_000); await openQuestions(page, 'network');
  await expect(page.getByTestId('question-provider')).toContainText('Network provider');
  await page.getByLabel('Question about this meeting').fill('Who agreed to a different date?');
  const button = page.getByRole('button', { name: 'Ask meeting', exact: true });
  await expect(button).toBeDisabled();
  expect(await page.evaluate(() => (window as QuestionWindow).__questionRequests.length)).toBe(0);
  await page.getByRole('checkbox', { name: /Send this question/ }).check();
  await button.click();
  await expect(page.getByText('There is not enough evidence in this meeting to answer.', { exact: true })).toBeVisible();
  expect(await page.evaluate(() => (window as QuestionWindow).__questionRequests[0].networkConsent)).toBe(true);
  await page.screenshot({ path: '/private/tmp/meetily-question-panel.png', fullPage: false });
});

test('cancelled request cannot restore a late answer', async ({ page }) => {
  test.setTimeout(90_000); await openQuestions(page, 'pending');
  await page.getByLabel('Question about this meeting').fill('When?');
  await page.getByRole('button', { name: 'Ask meeting', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Cancel answer' })).toBeVisible();
  await page.getByRole('button', { name: 'Cancel answer' }).click();
  await page.evaluate(() => (window as QuestionWindow).__resolveQuestion?.());
  await expect(page.getByText('Shipping is Friday.', { exact: true })).not.toBeVisible();
  expect(await page.evaluate(() => (window as QuestionWindow).__questionCancels.length)).toBe(1);
});

test('switching meetings cancels the old request and rejects its late answer', async ({ page }) => {
  test.setTimeout(90_000); await openQuestions(page, 'pending');
  await page.getByLabel('Question about this meeting').fill('When?');
  await page.getByRole('button', { name: 'Ask meeting', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Cancel answer' })).toBeVisible();
  const request = await page.evaluate(() => (window as QuestionWindow).__questionRequests[0]);
  await page.evaluate(() => {
    const w = window as QuestionWindow; const original = w.__TAURI_INTERNALS__.invoke;
    w.__TAURI_INTERNALS__.invoke = async (command, payload, options) => {
      if (command === 'api_search_transcripts') return [{ id: 'neighbor-meeting', title: 'Another meeting', matchContext: 'Different meeting', timestamp: '2026-06-20T10:00:00Z', sourceType: 'title' }];
      if (command === 'api_get_meetings') {
        const values = await original(command, payload, options) as Record<string, unknown>[];
        return [...values, { ...values[0], id: 'neighbor-meeting', title: 'Another meeting' }];
      }
      if (command === 'api_get_meeting_metadata' && payload?.meetingId === 'neighbor-meeting') {
        const value = await original(command, payload, options) as Record<string, unknown>;
        return { ...value, id: 'neighbor-meeting', title: 'Another meeting' };
      }
      return original(command, payload, options);
    };
  });
  await page.evaluate(() => (window as QuestionWindow).__emitTauriEvent('meeting-lifecycle-updated', {}));
  await page.getByPlaceholder('Search meeting content...').fill('another');
  await page.getByRole('button', { name: 'Open Meeting match in Another meeting' }).click();
  await expect(page).toHaveURL(/id=neighbor-meeting/);
  await expect(page.getByLabel('Question about this meeting')).toHaveValue('');
  await expect.poll(async () => page.evaluate(() => (window as QuestionWindow).__questionCancels.length)).toBeGreaterThan(0);
  const cancels = await page.evaluate(() => (window as QuestionWindow).__questionCancels);
  expect(cancels.some(value => value.meetingId === request.meetingId && value.requestId === request.requestId)).toBe(true);
  await page.evaluate(() => (window as QuestionWindow).__resolveQuestion?.());
  await expect(page.getByText('Shipping is Friday.', { exact: true })).not.toBeVisible();
});

for (const code of ['meeting_changed', 'model_configuration_changed'] as const) {
  test(`native ${code} is visible and a fresh request can recover`, async ({ page }) => {
    test.setTimeout(90_000); await openQuestions(page, 'answer');
    await page.evaluate(({ code }) => {
      const w = window as QuestionWindow; const original = w.__TAURI_INTERNALS__.invoke;
      let failed = false;
      const current = { provider: 'openai', model: 'fixture-new-model', transport: 'network', fingerprint: 'new-config' };
      w.__TAURI_INTERNALS__.invoke = async (command, payload, options) => {
        if (command === 'api_ask_meeting_question' && !failed) { failed = true; throw code; }
        if (code === 'model_configuration_changed' && failed && command === 'api_get_meeting_question_configuration') return new Promise(resolve => { w.__resolveQuestionConfiguration = () => resolve(current); });
        const value = await original(command, payload, options);
        if (code === 'model_configuration_changed' && failed && command === 'api_ask_meeting_question') {
          if (payload?.configurationFingerprint !== current.fingerprint || !payload.networkConsent) throw 'network_consent_required';
          return { ...value as Record<string, unknown>, configuration: current };
        }
        return value;
      };
    }, { code });
    await page.getByLabel('Question about this meeting').fill('When?');
    const button = page.getByRole('button', { name: 'Ask meeting', exact: true });
    await button.click();
    if (code === 'meeting_changed') {
      await expect(page.getByRole('region', { name: 'Ask this meeting' }).getByRole('alert')).toContainText('The meeting changed during generation');
    } else {
      await expect(page.getByRole('region', { name: 'Ask this meeting' }).getByRole('alert')).toContainText('The model settings changed');
      await expect(page.getByRole('button', { name: 'Cancel answer' })).not.toBeVisible();
      await expect(page.getByRole('region', { name: 'Ask this meeting' }).locator('button[type=submit]')).toBeDisabled();
      await page.evaluate(() => (window as QuestionWindow).__resolveQuestionConfiguration?.());
      await expect(page.getByTestId('question-provider')).toContainText('openai · fixture-new-model · Network');
      await expect(button).toBeDisabled();
      await page.getByRole('checkbox', { name: /Send this question/ }).check();
    }
    await expect(button).toBeEnabled(); await button.click();
    await expect(page.getByText('Shipping is Friday.', { exact: true })).toBeVisible();
  });
}
