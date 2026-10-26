import { expect, test } from './fixtures/tauri-fixtures';

interface RecoveryTranscript {
  text: string;
  timestamp: string;
  source: string;
  sequence_id: number;
  chunk_start_time: number;
  is_partial: boolean;
  confidence: number;
  audio_start_time: number;
  audio_end_time: number;
  duration: number;
}

interface ReopenedTranscript {
  text: string;
  sequenceId: number;
}

interface PersistedTranscriptSequence {
  sequenceId?: number;
  sequence_id?: number;
}

interface RecoveryMetadata {
  meetingId: string;
  title: string;
  startTime: number;
  lastUpdated: number;
  transcriptCount: number;
  savedToSQLite: boolean;
  sqliteMeetingId?: string;
}

test('native transcript sequence survives IndexedDB persistence and reopen', async ({ page }) => {
  await page.goto('/settings');

  const meetingId = 'native-sequence-contract';
  const updates: RecoveryTranscript[] = [
    {
      text: 'Third native segment',
      timestamp: '2026-07-15T09:00:03.000Z',
      source: 'whisper',
      sequence_id: 30,
      chunk_start_time: 3,
      is_partial: false,
      confidence: 0.93,
      audio_start_time: 3,
      audio_end_time: 4,
      duration: 1,
    },
    {
      text: 'First native segment',
      timestamp: '2026-07-15T09:00:01.000Z',
      source: 'whisper',
      sequence_id: 10,
      chunk_start_time: 1,
      is_partial: false,
      confidence: 0.99,
      audio_start_time: 1,
      audio_end_time: 2,
      duration: 1,
    },
    {
      text: 'Second native segment',
      timestamp: '2026-07-15T09:00:02.000Z',
      source: 'whisper',
      sequence_id: 20,
      chunk_start_time: 2,
      is_partial: false,
      confidence: 0.96,
      audio_start_time: 2,
      audio_end_time: 3,
      duration: 1,
    },
  ];

  await page.evaluate(async ({ id, transcripts }) => {
    const saveTranscript = (
      window as typeof window & {
        __saveRecoveryTranscript?: (
          meetingId: string,
          transcript: RecoveryTranscript,
        ) => Promise<void>;
      }
    ).__saveRecoveryTranscript;
    if (!saveTranscript) throw new Error('IndexedDB E2E save helper is unavailable');

    for (const transcript of transcripts) {
      await saveTranscript(id, transcript);
    }
  }, { id: meetingId, transcripts: updates });

  await page.reload();

  const persistedSequences = await page.evaluate(async (id) => {
    return new Promise<PersistedTranscriptSequence[]>((resolve, reject) => {
      const request = indexedDB.open('MeetilyRecoveryDB', 3);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const database = request.result;
        const transaction = database.transaction('transcripts', 'readonly');
        const query = transaction.objectStore('transcripts').index('meetingId').getAll(id);
        query.onerror = () => reject(query.error);
        query.onsuccess = () => {
          database.close();
          resolve(query.result as PersistedTranscriptSequence[]);
        };
      };
    });
  }, meetingId);

  expect(persistedSequences.map(({ sequenceId }) => sequenceId)).toEqual([30, 10, 20]);
  expect(persistedSequences.every(({ sequence_id }) => sequence_id === undefined)).toBe(true);

  const reopened = await page.evaluate(async (id) => {
    const getTranscripts = (
      window as typeof window & {
        __getRecoveryTranscripts?: (
          meetingId: string,
        ) => Promise<ReopenedTranscript[]>;
      }
    ).__getRecoveryTranscripts;
    if (!getTranscripts) throw new Error('IndexedDB E2E read helper is unavailable');
    return getTranscripts(id);
  }, meetingId);

  expect(reopened.map(({ sequenceId }) => sequenceId)).toEqual([10, 20, 30]);
  expect(reopened.map(({ text }) => text)).toEqual([
    'First native segment',
    'Second native segment',
    'Third native segment',
  ]);
});

test('recovered SQLite meeting id survives IndexedDB persistence and reopen', async ({ page }) => {
  await page.goto('/settings');

  const metadata: RecoveryMetadata = {
    meetingId: 'sqlite-id-recovery-contract',
    title: 'Retryable recovery',
    startTime: Date.now() - 60_000,
    lastUpdated: Date.now(),
    transcriptCount: 1,
    savedToSQLite: false,
  };

  await page.evaluate(async ({ meeting, sqliteMeetingId }) => {
    const recoveryWindow = window as typeof window & {
      __saveRecoveryMetadata?: (metadata: RecoveryMetadata) => Promise<void>;
      __getOrCreateRecoverySQLiteMeetingId?: (
        meetingId: string,
        proposedId: string,
      ) => Promise<string>;
    };
    if (!recoveryWindow.__saveRecoveryMetadata) {
      throw new Error('IndexedDB E2E metadata save helper is unavailable');
    }
    if (!recoveryWindow.__getOrCreateRecoverySQLiteMeetingId) {
      throw new Error('IndexedDB E2E SQLite id helper is unavailable');
    }
    await recoveryWindow.__saveRecoveryMetadata(meeting);
    const allocated = await Promise.all([
      recoveryWindow.__getOrCreateRecoverySQLiteMeetingId(meeting.meetingId, sqliteMeetingId),
      recoveryWindow.__getOrCreateRecoverySQLiteMeetingId(meeting.meetingId, 'competing-id'),
    ]);
    if (allocated[0] !== allocated[1]) {
      throw new Error(`Concurrent allocators diverged: ${allocated.join(', ')}`);
    }
    return allocated[0];
  }, { meeting: metadata, sqliteMeetingId: 'sqlite-meeting-42' });

  await page.reload();

  const reopened = await page.evaluate(async (meetingId) => {
    const getMetadata = (
      window as typeof window & {
        __getRecoveryMetadata?: (id: string) => Promise<RecoveryMetadata | null>;
      }
    ).__getRecoveryMetadata;
    if (!getMetadata) throw new Error('IndexedDB E2E metadata read helper is unavailable');
    return getMetadata(meetingId);
  }, metadata.meetingId);

  expect(reopened).toMatchObject({
    meetingId: metadata.meetingId,
    savedToSQLite: false,
    sqliteMeetingId: 'sqlite-meeting-42',
  });
});
