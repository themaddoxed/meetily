import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (relativePath) =>
  readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');

test('both transcript list modes render the same speaker-aware segment component', () => {
  const transcript = read('src/components/VirtualizedTranscriptView.tsx');
  const segmentCalls = transcript.match(/<TranscriptSegment/g) ?? [];

  assert.ok(segmentCalls.length >= 2);
  assert.match(transcript, /speaker_label/);
  assert.match(transcript, /speaker_color/);
  assert.match(transcript, /diarization_status/);
  assert.match(transcript, /Multiple speakers/);
  assert.match(transcript, /SpeakerLabelEditor/);
});

test('speaker editor validates, saves, cancels, and ignores duplicate submission', () => {
  const editor = read('src/components/MeetingDetails/SpeakerLabelEditor.tsx');

  assert.match(editor, /rename_speaker/);
  assert.match(editor, /label\.length|Array\.from|\.chars/);
  assert.match(editor, /128/);
  assert.match(editor, /event\.key === 'Enter'/);
  assert.match(editor, /event\.key === 'Escape'/);
  assert.match(editor, /isSaving/);
  assert.match(editor, /disabled=\{isSaving\}/);
  assert.match(editor, /onRenamed/);
  assert.match(editor, /toast\.success/);
  assert.match(editor, /toast\.error/);
});

test('meeting panel wires successful rename to transcript refetch', () => {
  const panel = read('src/components/MeetingDetails/TranscriptPanel.tsx');

  assert.match(panel, /meetingId=\{meetingId\}/);
  assert.match(panel, /onSpeakerRenamed=\{onRefetchTranscripts\}/);
});
