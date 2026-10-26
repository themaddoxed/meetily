import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

function loadPagination() {
  const sourcePath = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    '..',
    '..',
    'src',
    'lib',
    'transcript-pagination.ts',
  );
  const compiled = ts.transpileModule(fs.readFileSync(sourcePath, 'utf8'), {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  vm.runInNewContext(compiled, {
    exports: commonJsModule.exports,
    module: commonJsModule,
  });
  return commonJsModule.exports;
}

test('target loading continues through multiple fast pages', async () => {
  const { loadPagesUntilTarget } = loadPagination();
  const pages = [
    [{ id: 'transcript-1' }],
    [{ id: 'transcript-2' }],
    [{ id: 'transcript-target' }],
  ];
  let pageIndex = 0;

  const found = await loadPagesUntilTarget(
    'transcript-target',
    async () => pages[pageIndex++],
    () => pageIndex < pages.length,
  );

  assert.equal(found, true);
  assert.equal(pageIndex, 3);
});

test('target loading stops when the active meeting changes during a page request', async () => {
  const { loadPagesUntilTarget } = loadPagination();
  let activeMeetingId = 'meeting-a';
  let releaseFirstPage;
  let pageRequests = 0;
  const firstPage = new Promise((resolve) => {
    releaseFirstPage = resolve;
  });

  const loadPromise = loadPagesUntilTarget(
    'transcript-from-b',
    async () => {
      pageRequests += 1;
      return firstPage;
    },
    () => true,
    () => activeMeetingId === 'meeting-a',
  );

  activeMeetingId = 'meeting-b';
  releaseFirstPage([{ id: 'transcript-from-a' }]);

  assert.equal(await loadPromise, false);
  assert.equal(pageRequests, 1);
});
