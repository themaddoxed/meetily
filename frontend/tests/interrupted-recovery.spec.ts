import { expect, test } from './fixtures/tauri-fixtures';
import type { Page } from '@playwright/test';

type RecoveryMeeting = {
  meetingId: string;
  title: string;
  startTime: number;
  lastUpdated: number;
  transcriptCount: number;
  savedToSQLite: false;
  folderPath?: string;
};

async function seedRecoveryMeeting(
  page: Page,
  meeting: RecoveryMeeting,
  activeMeetingId?: string,
  transcriptTexts: string[] = [`Transcript for ${meeting.title}`],
) {
  await page.goto('/settings');
  await page.evaluate(async ({ meeting, activeMeetingId, transcriptTexts }) => {
    await new Promise<void>((resolve, reject) => {
      const request = indexedDB.open('MeetilyRecoveryDB', 3);

      request.onerror = () => reject(request.error);
      request.onupgradeneeded = () => {
        const database = request.result;
        if (!database.objectStoreNames.contains('meetings')) {
          const meetings = database.createObjectStore('meetings', {
            keyPath: 'meetingId',
          });
          meetings.createIndex('lastUpdated', 'lastUpdated', { unique: false });
          meetings.createIndex('savedToSQLite', 'savedToSQLite', { unique: false });
        }
        if (!database.objectStoreNames.contains('transcripts')) {
          const transcripts = database.createObjectStore('transcripts', {
            keyPath: 'id',
            autoIncrement: true,
          });
          transcripts.createIndex('meetingId', 'meetingId', { unique: false });
          transcripts.createIndex('storedAt', 'storedAt', { unique: false });
        }
        if (!database.objectStoreNames.contains('migrationOperations')) {
          database.createObjectStore('migrationOperations', { keyPath: 'operationId' });
        }
      };
      request.onsuccess = () => {
        const database = request.result;
        const transaction = database.transaction(
          ['meetings', 'transcripts'],
          'readwrite',
        );
        transaction.objectStore('meetings').put(meeting);
        transcriptTexts.forEach((text, sequenceId) => {
          transaction.objectStore('transcripts').add({
            meetingId: meeting.meetingId,
            text,
            timestamp: new Date(meeting.lastUpdated + sequenceId * 1_000).toISOString(),
            confidence: 0.98,
            sequenceId,
            storedAt: meeting.lastUpdated + sequenceId * 1_000,
          });
        });
        transaction.onerror = () => reject(transaction.error);
        transaction.oncomplete = () => {
          database.close();
          resolve();
        };
      };
    });

    sessionStorage.clear();
    if (activeMeetingId) {
      sessionStorage.setItem('indexeddb_current_meeting_id', activeMeetingId);
    }
  }, { meeting, activeMeetingId, transcriptTexts });
}

async function readStoredTranscriptTexts(page: Page, meetingId: string) {
  return page.evaluate(async (id) => {
    return new Promise<string[]>((resolve, reject) => {
      const request = indexedDB.open('MeetilyRecoveryDB', 3);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const database = request.result;
        const transaction = database.transaction('transcripts', 'readonly');
        const query = transaction.objectStore('transcripts').index('meetingId').getAll(id);
        query.onerror = () => reject(query.error);
        query.onsuccess = () => {
          const rows = query.result as Array<{ text: string; sequenceId: number }>;
          rows.sort((left, right) => left.sequenceId - right.sequenceId);
          database.close();
          resolve(rows.map((row) => row.text));
        };
      };
    });
  }, meetingId);
}

async function emitTauriEvent(page: Page, event: string, payload: unknown) {
  await page.evaluate(async ({ event, payload }) => {
    const emitter = (
      window as typeof window & {
        __emitTauriEvent?: (name: string, value: unknown) => Promise<void>;
      }
    ).__emitTauriEvent;
    if (!emitter) throw new Error('Tauri E2E event emitter is unavailable');
    await emitter(event, payload);
  }, { event, payload });
}

async function seedMalformedRecoveryMeeting(page: Page, lastUpdated: number) {
  await page.evaluate(async (timestamp) => {
    await new Promise<void>((resolve, reject) => {
      const request = indexedDB.open('MeetilyRecoveryDB', 3);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const database = request.result;
        const transaction = database.transaction('meetings', 'readwrite');
        transaction.objectStore('meetings').put({
          meetingId: 'malformed-recovery-row',
          title: 'Malformed recovery row',
          startTime: timestamp - 60_000,
          lastUpdated: String(timestamp),
          transcriptCount: 1,
          savedToSQLite: false,
        });
        transaction.onerror = () => reject(transaction.error);
        transaction.oncomplete = () => {
          database.close();
          resolve();
        };
      };
    });
    sessionStorage.clear();
  }, lastUpdated);
}

function meeting(
  meetingId: string,
  title: string,
  lastUpdated: number,
  folderPath?: string,
  transcriptCount = 1,
): RecoveryMeeting {
  return {
    meetingId,
    title,
    startTime: lastUpdated - 60_000,
    lastUpdated,
    transcriptCount,
    savedToSQLite: false,
    folderPath,
  };
}

test('shows a just-crashed meeting on the first relaunch', async ({ page }) => {
  const relaunchedAt = Date.now();
  const justCrashed = meeting(
    'meeting-just-crashed',
    'Just Crashed Meeting',
    relaunchedAt - 250,
  );
  await seedRecoveryMeeting(page, justCrashed);
  await page.addInitScript((now) => {
    Date.now = () => now;
  }, relaunchedAt);

  await page.goto('/');

  await expect(
    page.getByRole('heading', { name: 'Recover Interrupted Meetings' }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: justCrashed.title }),
  ).toBeVisible();
});

test('excludes the active meeting from the same browser session', async ({ page }) => {
  const control = meeting(
    'meeting-active-control',
    'Active Exclusion Control',
    Date.now() - 60_000,
  );
  const active = meeting(
    'meeting-active-session',
    'Active Session Meeting',
    Date.now() - 30_000,
  );
  await seedRecoveryMeeting(page, control);
  await seedRecoveryMeeting(page, active, active.meetingId);

  await page.goto('/');

  await expect(page.getByRole('heading', { name: control.title })).toBeVisible();
  await expect(page.getByText(active.title, { exact: true })).toHaveCount(0);
});

test('excludes meetings outside the seven-day retention window', async ({ page }) => {
  const control = meeting(
    'meeting-retention-control',
    'Retention Control Meeting',
    Date.now() - 6 * 24 * 60 * 60 * 1_000,
  );
  const expired = meeting(
    'meeting-expired',
    'Expired Meeting',
    Date.now() - 8 * 24 * 60 * 60 * 1_000,
  );
  await seedRecoveryMeeting(page, expired);
  await seedRecoveryMeeting(page, control);

  await page.goto('/');

  await expect(page.getByRole('heading', { name: control.title })).toBeVisible();
  await expect(page.getByText(expired.title, { exact: true })).toHaveCount(0);
});

test('excludes a meeting exactly at the seven-day cutoff', async ({ page }) => {
  const checkedAt = Date.now();
  const control = meeting(
    'meeting-boundary-control',
    'Boundary Control Meeting',
    checkedAt - 6 * 24 * 60 * 60 * 1_000,
  );
  const atCutoff = meeting(
    'meeting-at-cutoff',
    'Meeting At Seven Day Cutoff',
    checkedAt - 7 * 24 * 60 * 60 * 1_000,
  );
  await seedRecoveryMeeting(page, atCutoff);
  await seedRecoveryMeeting(page, control);
  await page.addInitScript((now) => {
    Date.now = () => now;
  }, checkedAt);

  await page.goto('/');

  await expect(page.getByRole('heading', { name: control.title })).toBeVisible();
  await expect(page.getByText(atCutoff.title, { exact: true })).toHaveCount(0);
});

test('keeps the audio checkpoint status when inspection succeeds', async ({ page }) => {
  const withAudio = meeting(
    'meeting-with-audio',
    'Meeting With Audio',
    Date.now() - 30_000,
    '/tmp/meeting-with-audio',
  );
  await seedRecoveryMeeting(page, withAudio);

  await page.goto('/?__e2e=recovery-audio-available');

  await expect(
    page.getByRole('heading', { name: withAudio.title }),
  ).toBeVisible();
  await expect(page.getByText('Audio available', { exact: true })).toBeVisible();
});

test('keeps the meeting but reports no audio when inspection fails', async ({ page }) => {
  const unavailableAudio = meeting(
    'meeting-audio-error',
    'Meeting With Audio Error',
    Date.now() - 30_000,
    '/tmp/meeting-audio-error',
    2,
  );
  const transcriptTexts = ['Checkpoint before audio failure', 'Checkpoint after audio failure'];
  await seedRecoveryMeeting(page, unavailableAudio, undefined, transcriptTexts);

  await page.goto('/?__e2e=recovery-audio-error');

  await expect(
    page.getByRole('heading', { name: unavailableAudio.title }),
  ).toBeVisible();
  await expect(page.getByText('No audio', { exact: true })).toBeVisible();
  await expect(page.getByText(transcriptTexts[0], { exact: true })).toBeVisible();
  await expect(page.getByText(transcriptTexts[1], { exact: true })).toBeVisible();
  await expect.poll(() => readStoredTranscriptTexts(page, unavailableAudio.meetingId))
    .toEqual(transcriptTexts);
});

test('keeps every transcript checkpoint after an STT warning, error, and relaunch', async ({ page }) => {
  const interrupted = meeting(
    'meeting-stt-error',
    'Meeting With STT Error',
    Date.now() - 30_000,
    undefined,
    2,
  );
  const transcriptTexts = ['Persisted before STT error', 'Persisted while STT was degraded'];
  await seedRecoveryMeeting(page, interrupted, undefined, transcriptTexts);

  await page.goto('/');
  await expect(page.getByRole('heading', { name: interrupted.title })).toBeVisible();
  await emitTauriEvent(page, 'transcription-warning', 'temporary decoder warning');
  await emitTauriEvent(page, 'transcription-error', {
    error: 'local STT unavailable',
    userMessage: 'Local transcription stopped',
    actionable: true,
  });

  await page.evaluate(() => sessionStorage.clear());
  await page.reload();

  await expect(page.getByRole('heading', { name: interrupted.title })).toBeVisible();
  await expect(page.getByText(transcriptTexts[0], { exact: true })).toBeVisible();
  await expect(page.getByText(transcriptTexts[1], { exact: true })).toBeVisible();
  await expect.poll(() => readStoredTranscriptTexts(page, interrupted.meetingId))
    .toEqual(transcriptTexts);
});

test('isolates a malformed recovery row so a healthy meeting remains recoverable', async ({ page }) => {
  const healthy = meeting(
    'meeting-healthy-recovery',
    'Healthy Recovery Meeting',
    Date.now() - 30_000,
  );
  await seedRecoveryMeeting(page, healthy);
  await seedMalformedRecoveryMeeting(page, Date.now() - 10_000);

  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Recover Interrupted Meetings' }))
    .toBeVisible();
  await expect(page.getByRole('heading', { name: healthy.title })).toBeVisible();
  await expect(page.getByText(`Transcript for ${healthy.title}`, { exact: true }))
    .toBeVisible();
});
