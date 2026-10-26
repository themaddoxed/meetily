import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (relativePath) =>
  readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');

test('meeting details exposes one independently busy speaker detection action', () => {
  const buttons = read('src/components/MeetingDetails/TranscriptButtonGroup.tsx');
  const panel = read('src/components/MeetingDetails/TranscriptPanel.tsx');
  const page = read('src/app/meeting-details/page-content.tsx');

  assert.match(buttons, /Detect speakers|Определить спикеров/);
  assert.match(buttons, /onRunDiarization/);
  assert.match(buttons, /isDiarizationRunning/);
  assert.match(buttons, /diarizationProgress/);
  assert.match(
    buttons,
    /disabled=\{[^}]*transcriptCount[^}]*isRecording[^}]*isDiarizationRunning/,
  );
  assert.match(buttons, /aria-busy=\{isDiarizationRunning\}/);
  assert.match(panel, /onRunDiarization/);
  assert.match(page, /useDiarizationRun/);
  assert.match(page, /onRefetchTranscripts/);
});

test('hook scopes progress and completion to the current meeting', () => {
  const hook = read('src/hooks/meeting-details/useDiarizationRun.ts');

  assert.equal((hook.match(/invoke<[^>]+>\('run_meeting_diarization'/g) ?? []).length, 1);
  assert.match(hook, /listen<DiarizationProgress>\('meeting-diarization-progress'/);
  assert.match(hook, /payload\.meetingId !== meetingId/);
  assert.match(hook, /unlisten/);
  assert.match(hook, /runTokenRef|ignore/);
  assert.match(hook, /onRefetchTranscripts/);
  assert.match(hook, /toast\.success/);
  assert.match(hook, /feature_disabled/);
  assert.match(hook, /missing_recording/);
  assert.match(hook, /insufficient_speech/);
  assert.match(hook, /already_running/);
  assert.match(hook, /model_invalid/);
  assert.match(hook, /persistence_failed/);
  assert.doesNotMatch(hook, /invoke<boolean>\('is_recording'/);
  assert.doesNotMatch(hook, /listen\('recording-(started|stopped)'/);
  assert.doesNotMatch(hook, /setIsRecording/);
  assert.match(hook, /useRecordingState/);
  assert.match(hook, /snapshot/);
});

test('backend registers only the production command and bounded progress event', () => {
  const lib = read('src-tauri/src/lib.rs');
  const coordinator = read('src-tauri/src/diarization/coordinator.rs');

  assert.match(lib, /diarization::coordinator::run_meeting_diarization/);
  assert.match(coordinator, /SherpaDiarizationRuntime::from_resource_root/);
  assert.match(coordinator, /meeting-diarization-progress/);
  assert.match(coordinator, /progress:\s*u8/);
  assert.doesNotMatch(lib, /inject_diarization|fake_diarization|apply_engine_segments/);
});
