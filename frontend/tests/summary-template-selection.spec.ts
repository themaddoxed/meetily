import { expect, test } from './fixtures/tauri-fixtures';

type SummaryTemplateState = {
  availableTemplateIds: string[];
  selectedTemplateId: string;
  selectionReads: number;
  selectionSaves: string[];
  crud: string[];
  processCalls: Array<Record<string, unknown>>;
  selectionReady: boolean;
  releaseSelection?: () => void;
};

async function getState(page: import('@playwright/test').Page) {
  return page.evaluate(
    () =>
      (window as typeof window & { __summaryTemplateState: SummaryTemplateState })
        .__summaryTemplateState,
  );
}

test('summary template selection waits for the backend and survives reload', async ({ page }) => {
  await page.goto('/meeting-details?id=theme-test-meeting&__e2e=summary-template-selection-pending');

  await expect(page.locator('button[title="Select summary template"]')).toHaveCount(0);
  const beforeRelease = await getState(page);
  expect(beforeRelease.selectionReads).toBe(1);
  expect(beforeRelease.processCalls).toEqual([]);

  await page.evaluate(() => {
    const state = window as typeof window & {
      __releaseSummaryTemplateSelection?: () => void;
    };
    state.__releaseSummaryTemplateSelection?.();
  });


  await expect(page.getByRole('button', { name: 'Manage summary templates' })).toBeVisible();
  await expect(page.locator('button[title="Select summary template"]')).toBeVisible();
  const afterRelease = await getState(page);
  expect(afterRelease.selectionReady).toBe(true);
  expect(afterRelease.selectedTemplateId).toBe('standard_meeting');
});

test('summary template list and CRUD commands preserve the selected custom template', async ({
  page,
}) => {
  await page.goto('/meeting-details?id=theme-test-meeting&__e2e=summary-template-crud');
  await expect(page.getByRole('button', { name: 'Manage summary templates' })).toBeVisible();

  await page.locator('button[title="Select summary template"]').click();
  await page.getByRole('menuitem', { name: 'Tasks Only' }).click();

  const result = await page.evaluate(async () => {
    const invoke = (
      window as typeof window & {
        __TAURI_INTERNALS__: { invoke: (command: string, payload?: Record<string, unknown>) => Promise<unknown> };
      }
    ).__TAURI_INTERNALS__.invoke;
    const created = await invoke('api_create_custom_template', {
      templateId: 'browser-custom',
      templateJson: JSON.stringify({
        name: 'Browser Custom',
        description: 'Browser CRUD fixture',
        sections: [{ title: 'Tasks', instruction: 'List tasks', format: 'list' }],
      }),
    });
    await invoke('api_update_custom_template', {
      templateId: created,
      templateJson: JSON.stringify({
        name: 'Browser Custom Updated',
        description: 'Updated browser CRUD fixture',
        sections: [{ title: 'Tasks', instruction: 'List updated tasks', format: 'list' }],
      }),
    });
    await invoke('api_save_summary_template_selection', { templateId: created });
    await invoke('api_delete_custom_template', { templateId: created });
    const fallback = await invoke('api_get_summary_template_selection');
    return { created, fallback };
  });

  expect(result).toEqual({
    created: 'browser-custom',
    fallback: expect.objectContaining({ templateId: 'standard_meeting', replaced: true }),
  });

  const beforeReload = await getState(page);
  expect(beforeReload.crud).toEqual([
    'save-selection:tasks_only',
    'create:browser-custom',
    'update:browser-custom',
    'save-selection:browser-custom',
    'delete:browser-custom',
  ]);

  await page.reload();
  await expect(page.getByRole('button', { name: 'Manage summary templates' })).toBeVisible();
  const state = await getState(page);
  expect(state.availableTemplateIds).toContain('tasks_only');
  expect(state.selectedTemplateId).toBe('standard_meeting');
});

test('template editor keeps section title focus during keyboard input', async ({ page }) => {
  await page.goto('/meeting-details?id=theme-test-meeting&__e2e=summary-template-crud');
  await expect(page.getByRole('button', { name: 'Manage summary templates' })).toBeVisible();

  await page.getByRole('button', { name: 'Manage summary templates' }).click();
  const sectionTitle = page.getByRole('textbox', { name: 'Section 1 title' });
  await expect(sectionTitle).toBeVisible();

  await sectionTitle.pressSequentially('Decisions');

  await expect(sectionTitle).toHaveValue('Decisions');
});
