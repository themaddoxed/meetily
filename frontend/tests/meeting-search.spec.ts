import { expect, test } from './fixtures/tauri-fixtures';

type SearchWindow = Window & {
  __TAURI_INTERNALS__: {
    invoke: (command: string, payload?: Record<string, unknown>, options?: unknown) => Promise<unknown>;
  };
  __searchPageOffsets: number[];
};

for (const sourceType of ['title', 'summary', 'transcript'] as const) {
  test(`search opens the visible ${sourceType} source`, async ({ page }) => {
    test.setTimeout(90_000);
    await page.goto('/settings');
    await page.getByRole('button', { name: 'Expand sidebar', exact: true }).click();
    await expect(page.getByPlaceholder('Search meeting content...')).toBeVisible();
    await page.evaluate(({ sourceType }) => {
      const browserWindow = window as SearchWindow;
      const original = browserWindow.__TAURI_INTERNALS__.invoke;
      browserWindow.__searchPageOffsets = [];
      const transcripts = Array.from({ length: 160 }, (_, index) => ({
        id: `search-target-${index}`,
        text: index === 120 ? 'Decision beyond the first transcript page.' : `Transcript row ${index}`,
        timestamp: '09:00:00',
        audio_start_time: index * 5,
        audio_end_time: index * 5 + 4,
        duration: 4,
        confidence: 0.98,
      }));
      browserWindow.__TAURI_INTERNALS__.invoke = async (command, payload, options) => {
        if (command === 'api_search_transcripts') {
          return [{
            id: 'theme-test-meeting',
            title: 'Theme Test Meeting',
            matchContext: `Visible ${sourceType} match`,
            timestamp: '2026-06-20T09:00:00Z',
            sourceType,
            ...(sourceType === 'transcript' ? { sourceId: 'search-target-120', audioStartTime: 600 } : {}),
          }];
        }
        if (command === 'api_get_meeting_transcripts') {
          const offset = Number(payload?.offset ?? 0);
          const limit = Number(payload?.limit ?? 50);
          browserWindow.__searchPageOffsets.push(offset);
          return {
            transcripts: transcripts.slice(offset, offset + limit),
            has_more: offset + limit < transcripts.length,
            total_count: transcripts.length,
          };
        }
        return original(command, payload, options);
      };
    }, { sourceType });
    await page.getByPlaceholder('Search meeting content...').fill(`decision-${sourceType}`);
    const label = sourceType === 'title' ? 'Meeting' : sourceType === 'summary' ? 'Summary' : 'Transcript';
    await page.getByRole('button', { name: `Open ${label} match in Theme Test Meeting` }).click();
    await expect(page).toHaveURL(/meeting-details\?id=theme-test-meeting&source=search/, { timeout: 30_000 });
    if (sourceType === 'transcript') {
      await expect(page).toHaveURL(/transcriptId=search-target-120&at=600/);
      const target = page.locator('#segment-search-target-120');
      await expect(target).toHaveAttribute('aria-current', 'location', { timeout: 30_000 });
      await expect(target).toBeInViewport({ timeout: 30_000 });
      const offsets = await page.evaluate(() => (window as SearchWindow).__searchPageOffsets);
      expect(offsets.some(offset => offset > 0)).toBe(true);
      const scrollTop = await target.evaluate(element => {
        let parent = element.parentElement;
        while (parent && !(parent.scrollHeight > parent.clientHeight
          && ['auto', 'scroll'].includes(getComputedStyle(parent).overflowY))) {
          parent = parent.parentElement;
        }
        return parent?.scrollTop ?? 0;
      });
      expect(scrollTop).toBeGreaterThan(0);
    } else {
      await expect(page.getByText('Use semantic tokens.', { exact: true })).toBeVisible();
    }
  });
}
