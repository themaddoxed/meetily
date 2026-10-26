import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

const servicePath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
  'src',
  'services',
  'indexedDBService.ts',
);

async function loadServiceHarness() {
  const getRequest = {};
  const putRequest = {};
  const getAllRequest = {};
  const store = {
    get() {
      return getRequest;
    },
    put() {
      return putRequest;
    },
    index() {
      return {
        getAll() {
          return getAllRequest;
        },
      };
    },
  };
  const transaction = {
    error: null,
    abortCalls: 0,
    objectStore() {
      return store;
    },
    abort() {
      this.abortCalls += 1;
      this.onabort?.();
    },
  };
  const database = {
    transaction() {
      return transaction;
    },
  };
  const openRequest = {};
  const indexedDB = {
    open() {
      return openRequest;
    },
  };

  const source = fs.readFileSync(servicePath, 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  vm.runInNewContext(compiled, {
    console: { log() {}, warn() {}, error() {} },
    exports: commonJsModule.exports,
    module: commonJsModule,
    indexedDB,
  });

  const service = commonJsModule.exports.indexedDBService;
  const initialization = service.init();
  openRequest.result = database;
  openRequest.onsuccess();
  await initialization;

  return { service, getRequest, putRequest, getAllRequest, transaction };
}

function metadata() {
  return {
    meetingId: 'recovery-id',
    title: 'Interrupted meeting',
    startTime: 1,
    lastUpdated: 2,
    transcriptCount: 1,
    savedToSQLite: false,
  };
}

function observe(promise) {
  const observation = { state: 'pending', value: undefined };
  const outcome = promise.then(
    (value) => {
      observation.state = 'resolved';
      observation.value = value;
      return value;
    },
    (error) => {
      observation.state = 'rejected';
      observation.value = error;
      throw error;
    },
  );
  return { observation, outcome };
}

test('SQLite id allocation resolves only after the IndexedDB transaction commits', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const allocation = observe(
    service.getOrCreateSQLiteMeetingId('recovery-id', 'proposed-id'),
  );

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.onsuccess?.();
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(allocation.observation.state, 'pending');

  transaction.oncomplete();
  assert.equal(await allocation.outcome, 'proposed-id');
});

test('SQLite id allocation rejects when the IndexedDB transaction aborts after put success', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const allocation = service.getOrCreateSQLiteMeetingId('recovery-id', 'proposed-id');

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.onsuccess?.();
  transaction.error = new Error('commit aborted');
  transaction.onabort?.();

  await assert.rejects(allocation, /commit aborted/);
});

test('SQLite id allocation rejects when the IndexedDB transaction errors after put success', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const allocation = service.getOrCreateSQLiteMeetingId('recovery-id', 'proposed-id');

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.onsuccess?.();
  transaction.error = new Error('commit error');
  transaction.onerror?.();

  await assert.rejects(allocation, /commit error/);
});

test('markMeetingSaved resolves only after the IndexedDB transaction commits', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const marking = observe(service.markMeetingSaved('recovery-id'));

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.onsuccess?.();
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(marking.observation.state, 'pending');

  transaction.oncomplete();
  await marking.outcome;
});

test('markMeetingSaved rejects when its transaction aborts after put success', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const marking = service.markMeetingSaved('recovery-id');

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.onsuccess?.();
  transaction.error = new Error('mark commit aborted');
  transaction.onabort?.();

  await assert.rejects(marking, /mark commit aborted/);
});

test('markMeetingSaved rejects when its transaction errors after put success', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const marking = service.markMeetingSaved('recovery-id');

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.onsuccess?.();
  transaction.error = new Error('mark commit error');
  transaction.onerror?.();

  await assert.rejects(marking, /mark commit error/);
});

test('markMeetingSaved rejects and aborts when its metadata request fails', async () => {
  const { service, getRequest, transaction } = await loadServiceHarness();
  const marking = service.markMeetingSaved('recovery-id');

  getRequest.error = new Error('mark read failed');
  getRequest.onerror();

  await assert.rejects(marking, /mark read failed/);
  assert.equal(transaction.abortCalls, 1);
});

test('markMeetingSaved rejects and aborts when its put request fails', async () => {
  const { service, getRequest, putRequest, transaction } = await loadServiceHarness();
  const marking = service.markMeetingSaved('recovery-id');

  getRequest.result = metadata();
  getRequest.onsuccess();
  putRequest.error = new Error('mark write failed');
  putRequest.onerror();

  await assert.rejects(marking, /mark write failed/);
  assert.equal(transaction.abortCalls, 1);
});

function validTranscriptRow() {
  return {
    id: 1,
    meetingId: 'recovery-id',
    text: 'Primary transcript',
    timestamp: '2026-07-16T10:00:00Z',
    confidence: 0.95,
    sequenceId: 1,
    storedAt: 10,
  };
}

test('getTranscripts rejects an IndexedDB request error instead of returning an empty transcript', async () => {
  const { service, getAllRequest } = await loadServiceHarness();
  const read = service.getTranscripts('recovery-id');

  getAllRequest.error = new Error('transcript read failed');
  getAllRequest.onerror();

  await assert.rejects(read, /transcript read failed/);
});

test('getTranscripts rejects the entire read when any stored transcript row is malformed', async () => {
  const { service, getAllRequest } = await loadServiceHarness();
  const read = service.getTranscripts('recovery-id');

  getAllRequest.result = [validTranscriptRow(), { meetingId: 'recovery-id' }];
  getAllRequest.onsuccess();

  await assert.rejects(read, /malformed/i);
});

test('getTranscripts preserves a legitimate empty transcript result', async () => {
  const { service, getAllRequest } = await loadServiceHarness();
  const read = service.getTranscripts('recovery-id');

  getAllRequest.result = [];
  getAllRequest.onsuccess();

  assert.deepEqual(await read, []);
});
