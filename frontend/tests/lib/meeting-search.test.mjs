import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

function loadMeetingSearch() {
  const sourcePath = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    '..',
    '..',
    'src',
    'lib',
    'meeting-search.ts',
  );
  const compiled = ts.transpileModule(fs.readFileSync(sourcePath, 'utf8'), {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  vm.runInNewContext(compiled, {
    URLSearchParams,
    exports: commonJsModule.exports,
    module: commonJsModule,
  });
  return commonJsModule.exports;
}

test('transcript search links preserve the meeting and exact transcript location', () => {
  const {
    buildMeetingSearchHref,
    formatMeetingSearchTime,
  } = loadMeetingSearch();

  const result = {
    id: 'meeting-42',
    title: 'Planning',
    matchContext: 'Discuss launch date',
    timestamp: '2026-09-26T09:00:00Z',
    sourceType: 'transcript',
    sourceId: 'transcript-7',
    audioStartTime: 125.8,
  };

  assert.equal(
    buildMeetingSearchHref(result),
    '/meeting-details?id=meeting-42&source=search&transcriptId=transcript-7&at=125.8',
  );
  assert.equal(formatMeetingSearchTime(result), '[02:05]');
});

test('title and summary results open the meeting without a transcript anchor', () => {
  const {
    buildMeetingSearchHref,
    meetingSearchSourceLabel,
  } = loadMeetingSearch();

  const result = {
    id: 'meeting-42',
    title: 'Planning',
    matchContext: 'Planning',
    timestamp: '2026-09-26T09:00:00Z',
    sourceType: 'summary',
    sourceId: 'should-not-be-used',
    audioStartTime: 12,
  };

  assert.equal(
    buildMeetingSearchHref(result),
    '/meeting-details?id=meeting-42&source=search',
  );
  assert.equal(meetingSearchSourceLabel(result.sourceType), 'Summary');
});

test('meeting result grouping keeps the transcript location over a title hit', () => {
  const { groupMeetingSearchResults, buildMeetingSearchHref } = loadMeetingSearch();
  const grouped = groupMeetingSearchResults([
    {
      id: 'meeting-42',
      title: 'Launch planning',
      matchContext: 'Launch planning',
      timestamp: '2026-09-26T09:00:00Z',
      sourceType: 'title',
    },
    {
      id: 'meeting-42',
      title: 'Launch planning',
      matchContext: 'Discuss launch date',
      timestamp: '2026-09-26T09:00:00Z',
      sourceType: 'transcript',
      sourceId: 'transcript-7',
      audioStartTime: 125.8,
    },
  ]);

  const selected = grouped.get('meeting-42');
  assert.equal(selected?.sourceType, 'transcript');
  assert.equal(selected?.sourceId, 'transcript-7');
  assert.equal(
    buildMeetingSearchHref(selected),
    '/meeting-details?id=meeting-42&source=search&transcriptId=transcript-7&at=125.8',
  );
});
