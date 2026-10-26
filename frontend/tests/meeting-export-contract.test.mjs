import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (relativePath) =>
  readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');

test('meeting details exposes a visible, independently busy Export action', () => {
  const buttons = read('src/components/MeetingDetails/TranscriptButtonGroup.tsx');
  const panel = read('src/components/MeetingDetails/TranscriptPanel.tsx');
  const page = read('src/app/meeting-details/page-content.tsx');

  assert.match(buttons, /FileDown/);
  assert.match(buttons, />Export</);
  assert.match(buttons, /isExporting/);
  assert.match(buttons, /disabled=\{[^}]*isExporting/);
  assert.match(buttons, /onExportMeeting/);
  assert.match(panel, /onExportMeeting/);
  assert.match(page, /useMeetingExport/);
});

test('export orchestration uses the native command and treats cancellation as normal', () => {
  const hook = read('src/hooks/meeting-details/useMeetingExport.ts');

  assert.match(hook, /export_meeting_markdown/);
  assert.match(hook, /formatMeetingMarkdown/);
  assert.match(hook, /fetchAllMeetingTranscripts/);
  assert.match(hook, /outcome\.status === 'cancelled'/);
  assert.match(hook, /toast\.success/);
  assert.match(hook, /toast\.error/);
  assert.doesNotMatch(
    hook,
    /outcome\.status === 'cancelled'[\s\S]{0,160}toast\.error/,
  );
});

test('copy and export share one complete transcript fetcher', () => {
  const copy = read('src/hooks/meeting-details/useCopyOperations.ts');
  const fetcher = read('src/lib/meeting-transcripts.ts');

  assert.match(copy, /fetchAllMeetingTranscripts/);
  assert.match(fetcher, /api_get_meeting_transcripts/);
  assert.match(fetcher, /total_count/);
});
