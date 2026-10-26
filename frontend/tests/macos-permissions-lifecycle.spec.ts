import { expect, test } from './fixtures/tauri-fixtures';

type CommandJournalEntry = {
  command: string;
  payload?: unknown;
};

type BrowserAudioPermissionState = {
  microphone: 'not_determined' | 'authorized' | 'denied' | 'restricted' | 'unavailable';
  systemAudio: 'unknown' | 'available' | 'denied' | 'unavailable';
};

async function commandCalls(page: import('@playwright/test').Page, command: string) {
  return page.evaluate((commandName) =>
    (window as typeof window & { __tauriCommandJournal: CommandJournalEntry[] })
      .__tauriCommandJournal.filter((entry) => entry.command === commandName).length,
  command);
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'userAgent', {
      configurable: true,
      value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
    });
    Object.defineProperty(navigator, 'platform', {
      configurable: true,
      value: 'MacIntel',
    });
  });
});

test('microphone authorization enables Finish while system audio remains optional', async ({ page }) => {
  await page.goto('/?__e2e=permissions-onboarding');
  await expect(page.getByRole('heading', { name: 'Grant Permissions' })).toBeVisible();

  const finish = page.getByRole('button', { name: 'Finish Setup' });
  await expect(finish).toBeDisabled();
  await page.getByText('Microphone', { exact: true }).locator('xpath=../../..')
    .getByRole('button', { name: 'Enable' }).click();

  await expect(finish).toBeEnabled();
  await expect(page.getByText('System Audio (Optional)', { exact: true })).toBeVisible();
  await expect(page.getByText(/restart/i)).toHaveCount(0);
  await expect(page.getByText(/Screen Recording|BlackHole/i)).toHaveCount(0);
  expect(await commandCalls(page, 'request_microphone_permission')).toBe(1);
  expect(await commandCalls(page, 'request_system_audio_permission')).toBe(0);
  expect(await commandCalls(page, 'get_audio_devices')).toBe(0);
});

test('returning from settings refreshes permission state without reload', async ({ page }) => {
  await page.goto('/?__e2e=permissions-focus');
  await expect(page.getByText('Access needs attention', { exact: true })).toBeVisible();

  await page.evaluate(() => {
    const state = window as typeof window & {
      __audioPermissionState: BrowserAudioPermissionState;
    };
    state.__audioPermissionState.microphone = 'authorized';
    window.dispatchEvent(new Event('focus'));
  });

  await expect(page.getByText('Access Granted', { exact: true })).toBeVisible();
  expect(await commandCalls(page, 'get_audio_permission_snapshot')).toBeGreaterThanOrEqual(2);
  expect(await commandCalls(page, 'request_microphone_permission')).toBe(0);
});

test('Russian onboarding translates denied microphone attention status', async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem('interfaceLanguage', 'ru');
  });
  await page.goto('/?__e2e=permissions-focus');

  await expect(page.getByRole('heading', { name: 'Выдайте разрешения' })).toBeVisible();
  await expect(page.getByText('Требуется внимание к доступу', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Открыть настройки' })).toHaveCount(2);
});

test('system audio denial shows microphone-only guidance and keeps recording controls available', async ({ page }) => {
  await page.goto('/?__e2e=permissions-system-audio');

  await expect(page.getByText('Audio Capture', { exact: true })).toBeVisible();
  await expect(page.getByText('Recording will continue with microphone only.')).toBeVisible();
  await expect(page.getByText(/restart|Screen Recording|BlackHole/i)).toHaveCount(0);
  await expect(page.locator('div.fixed.bottom-12 button').first()).toBeVisible();
  expect(await commandCalls(page, 'get_audio_permission_snapshot')).toBe(1);
  expect(await commandCalls(page, 'get_audio_devices')).toBe(0);

  await page.getByRole('button', { name: 'Try Again' }).click();
  expect(await commandCalls(page, 'request_system_audio_permission')).toBe(1);
  await expect(page.getByText('System audio capture is not confirmed.', { exact: false })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open Audio Capture Settings' })).toBeVisible();
});

test('main microphone request resolves a not-determined permission state', async ({ page }) => {
  await page.goto('/?__e2e=permissions-main-microphone');

  await page.getByRole('button', { name: 'Enable Microphone' }).click();

  expect(await commandCalls(page, 'request_microphone_permission')).toBe(1);
  await expect(page.getByText('Microphone Permission Required')).toHaveCount(0);
});

test('system audio retry keeps successful stream access unconfirmed', async ({ page }) => {
  await page.goto('/?__e2e=permissions-system-audio-unknown');

  await page.getByRole('button', { name: 'Try Again' }).click();

  expect(await commandCalls(page, 'request_system_audio_permission')).toBe(1);
  await expect(page.getByText('System audio capture is not confirmed.', { exact: false })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open Audio Capture Settings' })).toBeVisible();
});

test('system audio retry keeps unavailable access honest and retryable', async ({ page }) => {
  await page.goto('/?__e2e=permissions-system-audio-unavailable');

  const retry = page.getByRole('button', { name: 'Try Again' });
  await retry.click();

  expect(await commandCalls(page, 'request_system_audio_permission')).toBe(1);
  await expect(page.getByText('Audio Capture', { exact: true })).toBeVisible();
  await expect(retry).toBeVisible();
  await expect(retry).toBeEnabled();
});

test('onboarding keeps denied system audio optional while offering settings and retry', async ({ page }) => {
  await page.goto('/?__e2e=permissions-onboarding-system-audio-denied');

  await expect(page.getByRole('button', { name: 'Finish Setup' })).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Open Settings' })).toBeVisible();
  await page.getByRole('button', { name: 'Try Again' }).click();

  expect(await commandCalls(page, 'request_system_audio_permission')).toBe(1);
  await expect(page.getByText('Access Granted', { exact: true })).toHaveCount(1);
  await expect(page.getByText('System audio capture is not confirmed.', { exact: false })).toBeVisible();
});

test('route change notification remains visible for eight seconds without changing the saved preference', async ({ page }) => {
  await page.clock.install();
  await page.goto('/?__e2e=permissions-audio-events');
  await expect(page.locator('div.fixed.bottom-12')).toBeVisible();

  const savedPreference = await page.evaluate(() => {
    const read = (window as typeof window & {
      __getSavedMicrophonePreference?: () => string;
    }).__getSavedMicrophonePreference;
    if (!read) throw new Error('Saved microphone preference helper is unavailable');
    return read();
  });
  const routeRefreshesBefore = await commandCalls(page, 'get_recording_session');
  await page.evaluate(async () => {
    const emitRouteChanged = (window as typeof window & {
      __emitRecordingAudioRouteChanged?: (currentDevice: string) => Promise<void>;
    }).__emitRecordingAudioRouteChanged;
    if (!emitRouteChanged) throw new Error('Route change helper is unavailable');
    await emitRouteChanged('Built-in Microphone');
  });
  expect(await commandCalls(page, 'get_recording_session')).toBe(
    routeRefreshesBefore + 1,
  );

  const notification = page.getByText(
    'Microphone changed from Studio Microphone to Built-in Microphone. Recording continues on the new microphone.',
    { exact: true },
  );
  await expect(notification).toBeVisible();
  await page.clock.fastForward(7_999);
  await expect(notification).toBeVisible();
  expect(await page.evaluate(() => {
    const read = (window as typeof window & {
      __getSavedMicrophonePreference?: () => string;
    }).__getSavedMicrophonePreference;
    return read?.();
  })).toBe(savedPreference);
});

test('Russian route change notification includes both microphone names', async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem('interfaceLanguage', 'ru');
  });
  await page.goto('/?__e2e=permissions-audio-events');
  await expect(page.locator('div.fixed.bottom-12')).toBeVisible();

  await page.evaluate(async () => {
    const emitRouteChanged = (window as typeof window & {
      __emitRecordingAudioRouteChanged?: (currentDevice: string) => Promise<void>;
    }).__emitRecordingAudioRouteChanged;
    if (!emitRouteChanged) throw new Error('Route change helper is unavailable');
    await emitRouteChanged('Built-in Microphone');
  });

  await expect(page.getByText(
    'Микрофон изменился: «Studio Microphone» → «Built-in Microphone». Запись продолжается через новый микрофон.',
    { exact: true },
  )).toBeVisible();
});

test('system degradation warns that microphone capture continues and keeps controls active', async ({ page }) => {
  await page.goto('/?__e2e=permissions-audio-events');
  const controls = page.locator('div.fixed.bottom-12');
  await expect(controls).toBeVisible();

  await page.evaluate(async () => {
    const emitDegraded = (window as typeof window & {
      __emitRecordingAudioDegraded?: () => Promise<void>;
    }).__emitRecordingAudioDegraded;
    if (!emitDegraded) throw new Error('System degradation helper is unavailable');
    await emitDegraded();
  });

  await expect(page.getByText(
    'System audio stopped. Recording continues with the microphone.',
    { exact: true },
  )).toBeVisible();
  // The recovery dialog is intentionally still open in this fixture, so the
  // controls are checked at the DOM level rather than through the dialog's
  // accessibility tree. Pause/stop plus microphone/system source controls
  // must all remain enabled after degradation.
  await expect(controls.locator('button:enabled')).toHaveCount(4);
});


test('unconfirmed system audio stays visible during recording without starting another probe', async ({ page }) => {
  await page.goto('/?__e2e=permissions-audio-events');
  const recovery = page.getByRole('dialog', { name: 'Recover Interrupted Meetings' });
  await expect(recovery).toBeVisible();
  await recovery.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Pause recording', exact: true })).toBeVisible();
  await page.evaluate(() => {
    const state = window as typeof window & { __audioPermissionState: BrowserAudioPermissionState };
    state.__audioPermissionState.systemAudio = 'unknown';
    window.dispatchEvent(new Event('focus'));
  });
  await expect(page.getByText('System audio capture is not confirmed.', { exact: false })).toBeVisible();
  await expect(page.getByText('Recording will continue with microphone only.')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Try Again' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Open Audio Capture Settings' })).toBeVisible();
  expect(await commandCalls(page, 'request_system_audio_permission')).toBe(0);
});
