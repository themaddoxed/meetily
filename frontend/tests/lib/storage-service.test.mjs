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
  'storageService.ts',
);

function loadStorageService(invocations) {
  const source = fs.readFileSync(servicePath, 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  const require = (specifier) => {
    if (specifier !== '@tauri-apps/api/core') {
      throw new Error(`Unexpected storage service dependency: ${specifier}`);
    }
    return {
      invoke: async (command, payload) => {
        invocations.push({ command, payload });
        return { meeting_id: payload.meetingId ?? 'generated-id' };
      },
    };
  };

  vm.runInNewContext(compiled, {
    exports: commonJsModule.exports,
    module: commonJsModule,
    require,
  });
  return new commonJsModule.exports.StorageService();
}

test('saveMeeting passes a provided id to Tauri using the camelCase meetingId key', async () => {
  const invocations = [];
  const service = loadStorageService(invocations);

  await service.saveMeeting('Recovered', [], '/tmp/recovered', 'stable-meeting-id');

  assert.deepEqual(JSON.parse(JSON.stringify(invocations)), [{
    command: 'api_save_transcript',
    payload: {
      meetingTitle: 'Recovered',
      transcripts: [],
      folderPath: '/tmp/recovered',
      meetingId: 'stable-meeting-id',
    },
  }]);
});

test('saveMeeting omits meetingId for existing random-id callers', async () => {
  const invocations = [];
  const service = loadStorageService(invocations);

  await service.saveMeeting('Normal', [], null);

  assert.equal(Object.hasOwn(invocations[0].payload, 'meetingId'), false);
});
