import { expect, test } from './fixtures/tauri-fixtures';

const initialFolder = '/tmp/Meetily Recordings';
const selectedFolder = '/tmp/Meeting Captures';

type RecordingPreferences = {
  save_folder: string;
  auto_save: boolean;
  file_format: string;
  preferred_mic_device: string | null;
  preferred_system_device: string | null;
};

type RecordingFolderState = {
  backendMutations: string[];
  openedFolders: string[];
  saveAttempts: RecordingPreferences[];
  saves: RecordingPreferences[];
  releaseSave?: () => void;
  releaseSelection?: () => void;
  selectorCalls: number;
};

async function getState(page: import('@playwright/test').Page) {
  return page.evaluate(
    () =>
      (window as typeof window & { __recordingFolderState: RecordingFolderState })
        .__recordingFolderState,
  );
}

async function openRecordingSettings(
  page: import('@playwright/test').Page,
  scenario: string,
) {
  await page.goto(`/settings?__e2e=${scenario}`);
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await page.getByRole('tab', { name: 'Recordings' }).click();
  return page.locator('[role="tabpanel"][data-state="active"]');
}

test('cancel keeps the persisted folder and performs no save', async ({
  page,
}) => {
  const panel = await openRecordingSettings(page, 'recording-folder-cancel');

  await expect(panel.getByText(initialFolder, { exact: true })).toBeVisible();
  await panel.getByRole('button', { name: 'Choose Folder' }).click();

  await expect(panel.getByText(initialFolder, { exact: true })).toBeVisible();
  const state = await getState(page);
  expect(state.saveAttempts).toEqual([]);
  expect(state.saves).toEqual([]);
});

test('success persists the complete preferences before updating context', async ({
  page,
}) => {
  const panel = await openRecordingSettings(page, 'recording-folder-success');

  await panel.getByRole('button', { name: 'Choose Folder' }).click();
  await expect(panel.getByText(selectedFolder, { exact: true })).toBeVisible();

  await panel.getByRole('button', { name: 'Open Folder' }).click();
  await page.getByRole('tab', { name: 'General' }).click();
  await expect(
    page.locator('[role="tabpanel"][data-state="active"]').getByText(selectedFolder, {
      exact: true,
    }),
  ).toBeVisible();

  const state = await getState(page);
  const expectedPreferences: RecordingPreferences = {
    save_folder: selectedFolder,
    auto_save: true,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null,
  };
  expect(state.saveAttempts).toEqual([expectedPreferences]);
  expect(state.saves).toEqual([expectedPreferences]);
  expect(state.openedFolders).toEqual([selectedFolder]);
});

test('save failure leaves the previous folder visible and reports the error', async ({
  page,
}) => {
  const panel = await openRecordingSettings(page, 'recording-folder-failure');

  await panel.getByRole('button', { name: 'Choose Folder' }).click();

  await expect(page.getByText('Failed to save recording folder')).toBeVisible();
  await expect(panel.getByText(initialFolder, { exact: true })).toBeVisible();
  await expect(panel.getByText(selectedFolder, { exact: true })).toHaveCount(0);
  await panel.getByRole('button', { name: 'Open Folder' }).click();
  const persistedPreferences = await page.evaluate(async () => {
    const invoke = (
      window as typeof window & {
        __TAURI_INTERNALS__: {
          invoke: (command: string) => Promise<RecordingPreferences>;
        };
      }
    ).__TAURI_INTERNALS__.invoke;
    return invoke('get_recording_preferences');
  });
  const state = await getState(page);
  expect(state.saveAttempts).toHaveLength(1);
  expect(state.saves).toEqual([]);
  expect(state.backendMutations).toEqual([selectedFolder, initialFolder]);
  expect(state.openedFolders).toEqual([initialFolder]);
  expect(persistedPreferences.save_folder).toBe(initialFolder);
});

test('open folder failure is reported with an accessible error toast', async ({
  page,
}) => {
  const panel = await openRecordingSettings(page, 'recording-folder-open-failure');

  await panel.getByRole('button', { name: 'Open Folder' }).click();

  await expect(page.getByText('Failed to open recordings folder')).toBeVisible();
  await expect(page.getByText('Unable to open persisted recording folder')).toBeVisible();
});

test('folder controls remain disabled until the preference save completes', async ({
  page,
}) => {
  const panel = await openRecordingSettings(page, 'recording-folder-pending');
  const chooseButton = panel.getByRole('button', { name: 'Choose Folder' });
  const openButton = panel.getByRole('button', { name: 'Open Folder' });

  await chooseButton.click();

  await expect(chooseButton).toBeDisabled();
  await expect(openButton).toBeDisabled();
  await expect(panel.getByText(initialFolder, { exact: true })).toBeVisible();
  await page.evaluate(() => {
    (
      window as typeof window & { __recordingFolderState: RecordingFolderState }
    ).__recordingFolderState.releaseSave?.();
  });

  await expect(panel.getByText(selectedFolder, { exact: true })).toBeVisible();
  await expect(chooseButton).toBeEnabled();
  await expect(openButton).toBeEnabled();
});

test('pending picker merges the latest preferences and rejects duplicate choose', async ({
  page,
}) => {
  const panel = await openRecordingSettings(page, 'recording-folder-picker-pending');
  const chooseButton = panel.getByRole('button', { name: 'Choose Folder' });
  const openButton = panel.getByRole('button', { name: 'Open Folder' });

  await chooseButton.click();
  await expect(chooseButton).toBeDisabled();
  await expect(openButton).toBeDisabled();
  await expect(panel.getByRole('switch').first()).toBeDisabled();

  await chooseButton.evaluate((button) => {
    button.removeAttribute('disabled');
    button.click();
  });
  expect((await getState(page)).selectorCalls).toBe(1);

  await page.evaluate(async ({ initialFolder }) => {
    const invoke = (
      window as typeof window & {
        __TAURI_INTERNALS__: {
          invoke: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__.invoke;
    const externallySaved = {
      save_folder: initialFolder,
      auto_save: false,
      file_format: 'mp4',
      preferred_mic_device: null,
      preferred_system_device: null,
    };
    await invoke('set_recording_preferences', { preferences: externallySaved });
    await invoke('plugin:event|emit', {
      event: 'recording-preferences-updated',
      payload: externallySaved,
    });
  }, { initialFolder });
  await page.waitForFunction(
    () =>
      (window as typeof window & { __recordingFolderState: RecordingFolderState })
        .__recordingFolderState.saves.length === 1,
  );

  await page.evaluate(() => {
    (window as typeof window & { __recordingFolderState: RecordingFolderState })
      .__recordingFolderState.releaseSelection?.();
  });
  await page.waitForFunction(
    () =>
      (window as typeof window & { __recordingFolderState: RecordingFolderState })
        .__recordingFolderState.saves.length === 2,
  );

  const state = await getState(page);
  expect(state.saves.at(-1)).toEqual({
    save_folder: selectedFolder,
    auto_save: false,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null,
  });
  expect(state.selectorCalls).toBe(1);
});
