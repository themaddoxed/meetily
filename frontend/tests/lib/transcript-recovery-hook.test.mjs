import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

const hookPath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
  'src',
  'hooks',
  'useTranscriptRecovery.ts',
);

function deferred() {
  let resolve;
  const promise = new Promise((complete) => {
    resolve = complete;
  });
  return { promise, resolve };
}

async function waitFor(predicate, message) {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.fail(message);
}

function recoveryMeeting(meetingId, folderPath) {
  return {
    meetingId,
    title: meetingId,
    startTime: Date.now() - 60_000,
    lastUpdated: Date.now() - 30_000,
    transcriptCount: 1,
    savedToSQLite: false,
    folderPath,
  };
}

function loadHook({
  getAllMeetings,
  invoke,
  getTranscripts = async () => [],
  getMeetingMetadata = async () => null,
  getOrCreateSQLiteMeetingId = async (_meetingId, proposedId) => proposedId,
  markMeetingSaved = async () => undefined,
  deleteMeeting = async () => undefined,
  saveMeeting,
  applyPinnedSummaryLanguage = async () => undefined,
  randomUUID = () => 'proposed-sqlite-id',
}) {
  const stateSlots = [];
  const stateSetters = [];
  const refSlots = [];
  const effectSlots = [];
  const setterCalls = [];
  let stateIndex = 0;
  let refIndex = 0;
  let effectIndex = 0;

  const react = {
    useState(initialValue) {
      const slot = stateIndex;
      stateIndex += 1;
      if (!(slot in stateSlots)) stateSlots[slot] = initialValue;
      stateSetters[slot] ??= (nextValue) => {
        setterCalls.push(slot);
        stateSlots[slot] =
          typeof nextValue === 'function'
            ? nextValue(stateSlots[slot])
            : nextValue;
      };
      return [stateSlots[slot], stateSetters[slot]];
    },
    useRef(initialValue) {
      const slot = refIndex;
      refIndex += 1;
      refSlots[slot] ??= { current: initialValue };
      return refSlots[slot];
    },
    useCallback(callback) {
      return callback;
    },
    useEffect(effect, dependencies) {
      const slot = effectIndex;
      effectIndex += 1;
      const previous = effectSlots[slot];
      const unchanged =
        previous &&
        dependencies?.length === previous.dependencies?.length &&
        dependencies.every((value, index) =>
          Object.is(value, previous.dependencies[index]),
        );
      if (unchanged) return;
      previous?.cleanup?.();
      effectSlots[slot] = {
        cleanup: effect() ?? undefined,
        dependencies,
      };
    },
  };

  const source = fs.readFileSync(hookPath, 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  const indexedDBService = {
    getAllMeetings,
    getTranscripts,
    getMeetingMetadata,
    getOrCreateSQLiteMeetingId,
    markMeetingSaved,
    deleteMeeting,
  };
  const require = (specifier) => {
    const modules = {
      react,
      '@tauri-apps/api/core': { invoke },
      '@/services/indexedDBService': { indexedDBService },
      '@/services/storageService': { storageService: { saveMeeting } },
      '@/lib/summary-language-preferences': {
        applyPinnedSummaryLanguageToMeeting: applyPinnedSummaryLanguage,
      },
      sonner: { toast: { warning: () => undefined } },
    };
    if (!(specifier in modules)) {
      throw new Error(`Unexpected recovery hook dependency: ${specifier}`);
    }
    return modules[specifier];
  };
  const testConsole = {
    ...console,
    error: () => undefined,
    warn: () => undefined,
  };

  vm.runInNewContext(compiled, {
    console: testConsole,
    exports: commonJsModule.exports,
    module: commonJsModule,
    require,
    crypto: { randomUUID },
    sessionStorage: { getItem: () => null },
  });

  function render() {
    stateIndex = 0;
    refIndex = 0;
    effectIndex = 0;
    return commonJsModule.exports.useTranscriptRecovery();
  }

  return {
    result: render(),
    read() {
      const result = render();
      return {
        isLoading: result.isLoading,
        meetings: Array.from(result.recoverableMeetings, (meeting) => ({
          meetingId: meeting.meetingId,
          folderPath: meeting.folderPath,
        })),
      };
    },
    render,
    setterCallCount() {
      return setterCalls.length;
    },
    unmount() {
      for (const effect of effectSlots) effect?.cleanup?.();
    },
  };
}

test('a delayed scan cannot overwrite a faster newer result', async () => {
  const firstAudioCheck = deferred();
  const oldMeeting = recoveryMeeting('old-meeting', '/tmp/old-meeting');
  const newMeeting = recoveryMeeting('new-meeting', '/tmp/new-meeting');
  let meetingRead = 0;
  let firstInvokeStarted = false;
  const hook = loadHook({
    getAllMeetings: async () => (meetingRead++ === 0 ? [oldMeeting] : [newMeeting]),
    invoke: async (_command, { meetingFolder }) => {
      if (meetingFolder === oldMeeting.folderPath) {
        firstInvokeStarted = true;
        return firstAudioCheck.promise;
      }
      return true;
    },
  });

  const firstCheck = hook.result.checkForRecoverableTranscripts();
  await waitFor(() => firstInvokeStarted, 'first audio check did not start');
  await hook.render().checkForRecoverableTranscripts();
  assert.deepEqual(hook.read(), {
    isLoading: false,
    meetings: [{ meetingId: newMeeting.meetingId, folderPath: newMeeting.folderPath }],
  });

  firstAudioCheck.resolve(false);
  await firstCheck;

  assert.deepEqual(hook.read(), {
    isLoading: false,
    meetings: [{ meetingId: newMeeting.meetingId, folderPath: newMeeting.folderPath }],
  });
});

test('a stale completion cannot clear loading while a newer scan is pending', async () => {
  const firstAudioCheck = deferred();
  const secondAudioCheck = deferred();
  const firstMeeting = recoveryMeeting('first-meeting', '/tmp/first-meeting');
  const secondMeeting = recoveryMeeting('second-meeting', '/tmp/second-meeting');
  let meetingRead = 0;
  let invokeCount = 0;
  const hook = loadHook({
    getAllMeetings: async () =>
      meetingRead++ === 0 ? [firstMeeting] : [secondMeeting],
    invoke: async () => {
      invokeCount += 1;
      return invokeCount === 1 ? firstAudioCheck.promise : secondAudioCheck.promise;
    },
  });

  const firstCheck = hook.result.checkForRecoverableTranscripts();
  await waitFor(() => invokeCount === 1, 'first audio check did not start');
  const secondCheck = hook.render().checkForRecoverableTranscripts();
  await waitFor(() => invokeCount === 2, 'second audio check did not start');

  firstAudioCheck.resolve(false);
  await firstCheck;
  assert.deepEqual(hook.read(), { isLoading: true, meetings: [] });

  secondAudioCheck.resolve(true);
  await secondCheck;
  assert.deepEqual(hook.read(), {
    isLoading: false,
    meetings: [
      { meetingId: secondMeeting.meetingId, folderPath: secondMeeting.folderPath },
    ],
  });
});

test('an in-flight scan performs no state updates after unmount', async () => {
  const audioCheck = deferred();
  const pendingMeeting = recoveryMeeting('pending-meeting', '/tmp/pending-meeting');
  let invokeStarted = false;
  const hook = loadHook({
    getAllMeetings: async () => [pendingMeeting],
    invoke: async () => {
      invokeStarted = true;
      return audioCheck.promise;
    },
  });

  const check = hook.result.checkForRecoverableTranscripts();
  await waitFor(() => invokeStarted, 'audio check did not start');
  hook.unmount();
  const callsAtUnmount = hook.setterCallCount();

  audioCheck.resolve(true);
  await check;

  assert.equal(hook.setterCallCount(), callsAtUnmount);
});

test('a permanent save failure keeps recovery checkpoints untouched', async () => {
  const recovery = recoveryMeeting('failed-save', '/tmp/failed-save');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Persist this transcript', timestamp: '10:00:00', sequenceId: 1 },
    ],
    markMeetingSaved: async () => calls.push('mark-saved'),
    getOrCreateSQLiteMeetingId: async (_meetingId, proposedId) => {
      calls.push(`allocate:${proposedId}`);
      return proposedId;
    },
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}`);
      throw new Error('SQLite save failed');
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') return { status: 'success' };
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await assert.rejects(
    hook.result.recoverMeeting(recovery.meetingId),
    /SQLite save failed/,
  );

  assert.deepEqual(calls, [
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id',
  ]);
});

test('an IndexedDB allocation failure prevents any SQLite save attempt', async () => {
  const recovery = recoveryMeeting('allocation-failed', '/tmp/allocation-failed');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Do not duplicate me', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async () => {
      calls.push('allocate');
      throw new Error('IndexedDB write failed');
    },
    saveMeeting: async () => calls.push('save-meeting'),
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        return { status: 'success', audio_file_path: '/tmp/allocation-failed/recovered.wav' };
      }
      return true;
    },
  });

  await assert.rejects(
    hook.result.recoverMeeting(recovery.meetingId),
    /IndexedDB write failed/,
  );
  assert.deepEqual(calls, ['allocate']);
});

test('an IndexedDB transcript request error blocks recovery even when audio is recoverable', async () => {
  const recovery = recoveryMeeting('transcript-read-failed', '/tmp/transcript-read-failed');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => {
      throw new Error('IndexedDB transcript request failed');
    },
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push('save-meeting');
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        calls.push('recover-audio');
        return { status: 'success', audio_file_path: '/tmp/transcript-read-failed/audio.wav' };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await assert.rejects(
    hook.result.recoverMeeting(recovery.meetingId),
    /IndexedDB transcript request failed/,
  );
  assert.deepEqual(calls, []);
});

test('a malformed IndexedDB transcript row blocks recovery even when audio is recoverable', async () => {
  const recovery = recoveryMeeting('transcript-malformed', '/tmp/transcript-malformed');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => {
      throw new Error('Malformed IndexedDB transcript row');
    },
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push('save-meeting');
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        calls.push('recover-audio');
        return { status: 'success', audio_file_path: '/tmp/transcript-malformed/audio.wav' };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await assert.rejects(
    hook.result.recoverMeeting(recovery.meetingId),
    /Malformed IndexedDB transcript row/,
  );
  assert.deepEqual(calls, []);
});

test('a failed audio recovery keeps the saved transcript retryable and reuses its SQLite id', async () => {
  const recovery = recoveryMeeting('audio-failed', '/tmp/audio-failed');
  const calls = [];
  let audioAttempt = 0;
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Transcript survives audio failure', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async (_meetingId, proposedId) => {
      const sqliteMeetingId = recovery.sqliteMeetingId ?? proposedId;
      calls.push(`allocate:${sqliteMeetingId}`);
      recovery.sqliteMeetingId = sqliteMeetingId;
      return sqliteMeetingId;
    },
    markMeetingSaved: async () => {
      calls.push('mark-saved');
      recovery.savedToSQLite = true;
    },
    saveMeeting: async (_title, transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}:${transcripts[0].text}`);
      return { meeting_id: meetingId };
    },
    applyPinnedSummaryLanguage: async (meetingId) => calls.push(`language:${meetingId}`),
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        audioAttempt += 1;
        calls.push(`recover-audio:${audioAttempt}`);
        if (audioAttempt === 1) throw new Error('Audio checkpoint is damaged');
        return {
          status: 'success',
          audio_file_path: '/tmp/audio-failed/recovered.wav',
        };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await hook.result.checkForRecoverableTranscripts();
  const firstResult = await hook.render().recoverMeeting(recovery.meetingId);

  assert.equal(firstResult.success, true);
  assert.equal(firstResult.meetingId, 'proposed-sqlite-id');
  assert.equal(firstResult.audioRecoveryStatus?.status, 'failed');
  assert.deepEqual(calls, [
    'recover-audio:1',
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id:Transcript survives audio failure',
    'language:proposed-sqlite-id',
  ]);
  assert.equal(recovery.savedToSQLite, false);
  assert.deepEqual(hook.read().meetings, [
    { meetingId: recovery.meetingId, folderPath: recovery.folderPath },
  ]);

  const secondResult = await hook.render().recoverMeeting(recovery.meetingId);

  assert.equal(secondResult.success, true);
  assert.equal(secondResult.meetingId, 'proposed-sqlite-id');
  assert.equal(secondResult.audioRecoveryStatus?.status, 'success');
  assert.deepEqual(calls, [
    'recover-audio:1',
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id:Transcript survives audio failure',
    'language:proposed-sqlite-id',
    'recover-audio:2',
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id:Transcript survives audio failure',
    'language:proposed-sqlite-id',
    'mark-saved',
    'cleanup-checkpoints',
  ]);
  assert.deepEqual(hook.read().meetings, []);
});

test('partial audio recovery keeps the saved transcript and checkpoints retryable', async () => {
  const recovery = recoveryMeeting('audio-partial', '/tmp/audio-partial');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Transcript survives partial audio', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async (_meetingId, proposedId) => {
      calls.push(`allocate:${proposedId}`);
      recovery.sqliteMeetingId = proposedId;
      return proposedId;
    },
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}`);
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        return {
          status: 'partial',
          audio_file_path: '/tmp/audio-partial/recovered.wav',
        };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await hook.result.checkForRecoverableTranscripts();
  const result = await hook.render().recoverMeeting(recovery.meetingId);

  assert.equal(result.audioRecoveryStatus?.status, 'partial');
  assert.deepEqual(calls, [
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id',
  ]);
  assert.deepEqual(hook.read().meetings, [
    { meetingId: recovery.meetingId, folderPath: recovery.folderPath },
  ]);
});

test('malformed audio success without a file keeps recovery data retryable', async () => {
  const recovery = recoveryMeeting('audio-malformed-success', '/tmp/audio-malformed-success');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Transcript survives malformed success', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async (_meetingId, proposedId) => {
      calls.push(`allocate:${proposedId}`);
      recovery.sqliteMeetingId = proposedId;
      return proposedId;
    },
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}`);
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') return { status: 'success' };
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await hook.result.checkForRecoverableTranscripts();
  const result = await hook.render().recoverMeeting(recovery.meetingId);

  assert.equal(result.audioRecoveryStatus?.status, 'success');
  assert.deepEqual(calls, [
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id',
  ]);
  assert.deepEqual(hook.read().meetings, [
    { meetingId: recovery.meetingId, folderPath: recovery.folderPath },
  ]);
});

test('an audio-only interrupted meeting is saved when checkpoint recovery succeeds', async () => {
  const recovery = recoveryMeeting('audio-only', '/tmp/audio-only');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [],
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async (_title, transcripts, folderPath, meetingId) => {
      calls.push(`save-meeting:${transcripts.length}:${folderPath}:${meetingId}`);
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        calls.push('recover-audio');
        return {
          status: 'success',
          audio_file_path: '/tmp/audio-only/audio.mp4',
        };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  const result = await hook.result.recoverMeeting(recovery.meetingId);

  assert.equal(result.success, true);
  assert.equal(result.meetingId, 'proposed-sqlite-id');
  assert.deepEqual(calls, [
    'recover-audio',
    'save-meeting:0:/tmp/audio-only:proposed-sqlite-id',
    'mark-saved',
    'cleanup-checkpoints',
  ]);
});

test('an empty transcript with failed audio recovery keeps all recovery data pending', async () => {
  const recovery = recoveryMeeting('empty-and-failed', '/tmp/empty-and-failed');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [],
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async () => {
      calls.push('save-meeting');
      return { meeting_id: 'must-not-save' };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        calls.push('recover-audio');
        throw new Error('Audio checkpoint is damaged');
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await assert.rejects(
    hook.result.recoverMeeting(recovery.meetingId),
    /No transcript or recoverable audio found/,
  );
  assert.deepEqual(calls, ['recover-audio']);
});

test('checkpoint cleanup starts only after permanent save and recovery marking', async () => {
  const recovery = recoveryMeeting('ordered-cleanup', '/tmp/ordered-cleanup');
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Ordered transcript', timestamp: '10:00:00', sequenceId: 1 },
    ],
    markMeetingSaved: async () => calls.push('mark-saved'),
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}`);
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        return { status: 'success', audio_file_path: '/tmp/ordered-cleanup/recovered.wav' };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await hook.result.recoverMeeting(recovery.meetingId);

  assert.deepEqual(calls, [
    'save-meeting:proposed-sqlite-id',
    'mark-saved',
    'cleanup-checkpoints',
  ]);
});

test('a mark-saved failure keeps the recovery item and checkpoints intact', async () => {
  const recovery = {
    ...recoveryMeeting('mark-failed', '/tmp/mark-failed'),
    sqliteMeetingId: 'already-persisted-id',
  };
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Already persisted', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async () => recovery.sqliteMeetingId,
    markMeetingSaved: async () => {
      calls.push('mark-saved');
      throw new Error('IndexedDB mark failed');
    },
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}`);
      return { meeting_id: meetingId };
    },
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        calls.push('recover-audio');
        return { status: 'success', audio_file_path: '/tmp/mark-failed/recovered.wav' };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  await hook.result.checkForRecoverableTranscripts();
  await assert.rejects(
    hook.render().recoverMeeting(recovery.meetingId),
    /IndexedDB mark failed/,
  );

  assert.deepEqual(calls, [
    'recover-audio',
    'save-meeting:already-persisted-id',
    'mark-saved',
  ]);
  assert.deepEqual(hook.read().meetings, [
    { meetingId: recovery.meetingId, folderPath: recovery.folderPath },
  ]);
});

test('parallel retries share one recovery operation and one SQLite save', async () => {
  const recovery = recoveryMeeting('parallel-recovery', '/tmp/parallel-recovery');
  const saveStarted = deferred();
  const finishSave = deferred();
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Save once', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async (_meetingId, proposedId) => {
      calls.push(`allocate:${proposedId}`);
      recovery.sqliteMeetingId = proposedId;
      return proposedId;
    },
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => {
      calls.push(`save-meeting:${meetingId}`);
      saveStarted.resolve();
      await finishSave.promise;
      return { meeting_id: meetingId };
    },
    markMeetingSaved: async () => calls.push('mark-saved'),
    invoke: async (command) => {
      if (command === 'recover_audio_from_checkpoints') {
        calls.push('recover-audio');
        return { status: 'success', audio_file_path: '/tmp/parallel-recovery/recovered.wav' };
      }
      if (command === 'cleanup_checkpoints') calls.push('cleanup-checkpoints');
      return true;
    },
  });

  const rendered = hook.result;
  const first = rendered.recoverMeeting(recovery.meetingId);
  const second = rendered.recoverMeeting(recovery.meetingId);
  await saveStarted.promise;
  finishSave.resolve();
  const [firstResult, secondResult] = await Promise.all([first, second]);

  assert.equal(firstResult.meetingId, 'proposed-sqlite-id');
  assert.equal(secondResult.meetingId, 'proposed-sqlite-id');
  assert.deepEqual(calls, [
    'recover-audio',
    'allocate:proposed-sqlite-id',
    'save-meeting:proposed-sqlite-id',
    'mark-saved',
    'cleanup-checkpoints',
  ]);
});

test('transcript-only recovery keeps the item retryable after remembering its SQLite id', async () => {
  const recovery = recoveryMeeting('old-recovery-id', undefined);
  const calls = [];
  const hook = loadHook({
    getAllMeetings: async () => [recovery],
    getMeetingMetadata: async () => recovery,
    getTranscripts: async () => [
      { text: 'Recovered transcript', timestamp: '10:00:00', sequenceId: 1 },
    ],
    getOrCreateSQLiteMeetingId: async (_meetingId, proposedId) => {
      calls.push(`allocate:${proposedId}`);
      recovery.sqliteMeetingId = proposedId;
      return proposedId;
    },
    markMeetingSaved: async (meetingId) => calls.push(`mark:${meetingId}`),
    saveMeeting: async (_title, _transcripts, _folderPath, meetingId) => ({
      meeting_id: meetingId,
    }),
    applyPinnedSummaryLanguage: async (meetingId) => calls.push(`language:${meetingId}`),
    invoke: async (command) => {
      if (command === 'get_meeting_folder_path') throw new Error('No audio folder');
      return true;
    },
  });

  await hook.result.checkForRecoverableTranscripts();
  assert.deepEqual(hook.read().meetings, [
    { meetingId: recovery.meetingId, folderPath: recovery.folderPath },
  ]);

  const result = await hook.render().recoverMeeting(recovery.meetingId);

  assert.equal(result.success, true);
  assert.equal(result.meetingId, 'proposed-sqlite-id');
  assert.deepEqual(calls, [
    'allocate:proposed-sqlite-id',
    'language:proposed-sqlite-id',
  ]);
  assert.deepEqual(hook.read().meetings, [
    { meetingId: recovery.meetingId, folderPath: recovery.folderPath },
  ]);
});
