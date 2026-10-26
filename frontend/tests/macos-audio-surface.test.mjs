import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { validateMacosAudioSurface } from '../scripts/release-policy.mjs';

const frontendRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = path.resolve(frontendRoot, '..');
const read = (relativePath) => readFileSync(path.join(repositoryRoot, relativePath), 'utf8');

const deletedPaths = [
  'frontend/src/components/AudioBackendSelector.tsx',
  'frontend/src-tauri/src/audio/capture/backend_config.rs',
  'frontend/src-tauri/src/audio/capture/microphone.rs',
  'frontend/src-tauri/src/audio/capture/system.rs',
  'frontend/src-tauri/src/audio/devices/platform/linux.rs',
  'frontend/src-tauri/src/audio/devices/platform/windows.rs',
  'frontend/src-tauri/src/audio/core-old.rs',
  'frontend/src-tauri/src/audio/recording_commands.rs.backup',
  'frontend/src-tauri/src/audio/recording_manager.rs',
  'frontend/src-tauri/src/audio/recording_saver_old.rs',
  'frontend/src-tauri/src/audio/stream.rs',
  'frontend/src-tauri/src/audio/system_audio_commands.rs',
  'frontend/src-tauri/src/audio/system_audio_stream.rs',
  'frontend/src-tauri/src/lib_old_complex.rs',
  'frontend/src-tauri/src/audio_v2/compatibility.rs',
  'frontend/src-tauri/src/audio_v2/lib.rs',
  'frontend/src-tauri/src/audio_v2/limiter.rs',
  'frontend/src-tauri/src/audio_v2/mixer.rs',
  'frontend/src-tauri/src/audio_v2/normalizer.rs',
  'frontend/src-tauri/src/audio_v2/recorder.rs',
  'frontend/src-tauri/src/audio_v2/resampler.rs',
  'frontend/src-tauri/src/audio_v2/stream.rs',
  'frontend/src-tauri/src/audio_v2/sync.rs',
];

function realSurface() {
  const paths = [
    'frontend/src-tauri/src/lib.rs',
    'frontend/src-tauri/src/audio/mod.rs',
    'frontend/src-tauri/src/audio/recording_commands.rs',
    'frontend/src-tauri/src/audio/recording_session.rs',
    'frontend/src-tauri/src/audio/recording_session_state.rs',
    'frontend/src-tauri/src/audio/capture/mod.rs',
    'frontend/src-tauri/src/audio/capture/backend.rs',
    'frontend/src-tauri/src/audio/capture/macos.rs',
    'frontend/src-tauri/src/audio/permissions.rs',
    'frontend/src-tauri/src/audio/recording_saver.rs',
    'frontend/src-tauri/src/audio/incremental_saver.rs',
    'frontend/src/services/recordingService.ts',
    'frontend/src/contexts/RecordingStateContext.tsx',
    'frontend/src/hooks/meeting-details/useDiarizationRun.ts',
    'frontend/src/hooks/useRecordingStop.ts',
    'frontend/src/hooks/usePermissionCheck.ts',
    'frontend/src/contexts/RecordingPostProcessingProvider.tsx',
  ];
  return {
    existingPaths: new Set(paths.filter((entry) => existsSync(path.join(repositoryRoot, entry)))),
    sourceFiles: Object.fromEntries(paths
      .filter((entry) => existsSync(path.join(repositoryRoot, entry)))
      .map((entry) => [entry, read(entry)])),
  };
}

test('the real tree has one Rust recording owner and one macOS capture adapter', () => {
  assert.deepEqual(validateMacosAudioSurface(realSurface()), []);
});

test('all alternate capture implementations are absent', () => {
  for (const relativePath of deletedPaths) {
    assert.equal(existsSync(path.join(repositoryRoot, relativePath)), false, relativePath);
  }
});

test('raw Tracks use source namespaces while legacy recovery stays immediate', () => {
  const saver = read('frontend/src-tauri/src/audio/incremental_saver.rs');
  assert.match(saver, /\.tracks[\s\S]*microphone[\s\S]*system/);
  assert.match(saver, /join\("\.checkpoints"\)/);
  assert.match(saver, /microphone\.mp4/);
  assert.match(saver, /system\.mp4/);
  assert.match(saver, /audio\.mp4/);
  assert.match(saver, /SourceTrackSaver::recover|pub fn recover/);
  assert.doesNotMatch(saver, /contains\("microphone"\)/);
  assert.match(saver, /validate_checkpoint_sequence/);
  assert.match(saver, /validate_checkpoint_evidence/);
  assert.match(saver, /decoded_sample_count_bounded/);
});

test('the production stop path preserves the required acknowledgement and durability order', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  const orderedTokens = [
    'CaptureControl::Stop',
    'recv_timeout',
    'capture.join_once()',
    'closed_sources',
    'microphone_join',
    'force_flush_and_stop',
    'SourceTrackSaver::finalize',
    'captured_kinds',
    'RecordingSessionAction::FinalizationVerified',
  ];
  let previous = -1;
  for (const token of orderedTokens) {
    const current = session.indexOf(token, previous + 1);
    assert.ok(current > previous, `${token} must follow the prior stop stage`);
    previous = current;
  }
  assert.match(session, /production_stop_helper_acknowledges_then_drops_owner_before_returning_joined/);
  assert.match(session, /Interrupted[\s\S]*SourceTrackSaver::recover[\s\S]*FinalizationVerified/);
  assert.match(read('frontend/src-tauri/src/lib.rs'), /recover_recording_session/);
  assert.match(read('frontend/src/services/recordingService.ts'), /invoke\('recover_recording_session'/);
});

test('normal transcript save marks the Rust session identity before recovery can rescan it', () => {
  const stop = read('frontend/src/hooks/useRecordingStop.ts');
  const context = read('frontend/src/contexts/TranscriptContext.tsx');
  assert.match(stop, /markMeetingAsSaved\(sessionSnapshot\.sessionId\)/);
  assert.match(context, /markMeetingAsSaved: \(meetingId\?: string\) => Promise<void>/);
  assert.match(context, /const meetingId = meetingIdOverride \?\? currentMeetingId/);
});

test('FORK-044 transcript emission behavior remains intact', () => {
  const worker = read('frontend/src-tauri/src/audio/transcription/worker.rs');
  assert.match(worker, /fn should_emit_transcript\(transcript: &str, _confidence: Option<f32>\) -> bool/);
  assert.match(worker, /!transcript\.trim\(\)\.is_empty\(\)/);
  assert.match(worker, /if should_emit_transcript\(&transcript, confidence_opt\)/);
  for (const name of [
    'accepts_reviewed_short_nonempty_transcripts',
    'emits_nonempty_transcript_independent_of_confidence_metadata',
    'rejects_ascii_and_unicode_whitespace_only_transcripts',
  ]) assert.match(worker, new RegExp(`fn ${name}\\(`));
});

test('a stale stop id cannot take or drop the active recording owner', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(session, /fn take_matching_active<T>\([\s\S]*as_ref\(\)[\s\S]*session_id[\s\S]*active\s*\.take\(\)/);
  assert.match(session, /wrong_session_stop_preserves_active_owner/);
});

test('default system capture intent is independent from an optional device preference', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  const service = read('frontend/src/services/recordingService.ts');
  assert.match(session, /capture_system:\s*bool/);
  assert.match(session, /if request\.capture_system\s*\{/);
  assert.doesNotMatch(session, /if request\.system_device\.is_some\(\)\s*\{/);
  assert.match(service, /captureSystem\?:\s*boolean/);
  assert.match(read('frontend/src-tauri/src/audio/capture/backend.rs'), /default_system_capture_uses_no_device_override/);
});

test('paused CoreAudio is discarded and system termination keeps microphone capture', () => {
  const macos = read('frontend/src-tauri/src/audio/capture/macos.rs');
  const coreAudio = read('frontend/src-tauri/src/audio/capture/core_audio.rs');
  assert.match(macos, /discard_system_samples/);
  assert.match(macos, /if self\.paused\.load\([^)]*\)[\s\S]*discard_system_samples/);
  assert.match(coreAudio, /pub fn should_terminate\(/);
  const terminationBranch = macos.match(/if stream\.should_terminate\(\)\s*\{[\s\S]*?\n\s*}/)?.[0] ?? '';
  const degradation = macos.match(/fn degrade_system_audio[\s\S]*?\n    }\n/)?.[0] ?? '';
  assert.match(terminationBranch, /degrade_system_audio/);
  assert.match(degradation, /system_stream\.take\(\)[\s\S]*system_sinks\.take\(\)/);
  assert.match(degradation, /CaptureRuntimeEvent::SystemAudioDegraded/);
  assert.doesNotMatch(degradation, /microphone_stream\.take\(\)/);
  assert.match(coreAudio, /paused_system_audio_is_discarded_instead_of_replayed/);
});

test('CoreAudio startup and stop workers are owned and joined before capture stop returns', () => {
  const macos = read('frontend/src-tauri/src/audio/capture/macos.rs');
  const stop = macos.match(/fn stop\(&mut self\)[\s\S]*?\n    }\n/)?.[0] ?? '';
  const startupCleanup = macos.match(/fn finish_system_start_before_stop\(&mut self\)[\s\S]*?\n    }\n/)?.[0] ?? '';
  const backendDrop = macos.match(/impl Drop for MacosCaptureBackend[\s\S]*?\n}\n/)?.[0] ?? '';
  const receiverReceiveIndex = startupCleanup.indexOf('receiver.recv()');
  const receiverDropIndex = startupCleanup.indexOf('drop(receiver)');
  const startupJoinIndex = startupCleanup.indexOf('system_start_worker.take()');
  const degradation = macos.match(/fn degrade_system_audio[\s\S]*?\n    }\n/)?.[0] ?? '';

  assert.deepEqual({
    ownsStartupWorker: /system_start_worker:\s*Option<thread::JoinHandle<Result<\(\), CoreAudioSetupError>>>/.test(macos),
    ownsStopWorker: /system_stop_worker:\s*Option<thread::JoinHandle<Result<\(\), CoreAudioSetupError>>>/.test(macos),
    consumesAndDropsStartupReceiverBeforeJoin: receiverReceiveIndex >= 0
      && receiverReceiveIndex < receiverDropIndex
      && receiverDropIndex < startupJoinIndex,
    stopWaitsForStartupResolution: /finish_system_start_before_stop/.test(stop),
    joinsStartupWorker: /system_start_worker\.take\(\)[\s\S]*?join_core_audio_worker/.test(startupCleanup),
    joinsStopWorker: /system_stop_worker\.take\(\)[\s\S]*?join_core_audio_worker/.test(stop),
    dropJoinsStartupAndStopWorkers: /finish_system_start_before_stop[\s\S]*?system_stop_worker\.take\(\)[\s\S]*?join_core_audio_worker/.test(backendDrop),
    degradationStopsStream: /stream\.stop_capture\(\)/.test(degradation),
    joinPropagatesWorkerResult: /worker\.join\(\)[\s\S]*?CaptureFailed/.test(macos),
  }, {
    ownsStartupWorker: true,
    ownsStopWorker: true,
    consumesAndDropsStartupReceiverBeforeJoin: true,
    stopWaitsForStartupResolution: true,
    joinsStartupWorker: true,
    joinsStopWorker: true,
    dropJoinsStartupAndStopWorkers: true,
    degradationStopsStream: true,
    joinPropagatesWorkerResult: true,
  });
});

test('source-only recovery remains partial until playback-compatible audio exists', () => {
  const saver = read('frontend/src-tauri/src/audio/incremental_saver.rs');
  const sourceRecovery = saver.match(/if let Some\(track\) = recovered_tracks\.first\(\)[\s\S]*?\n        }/)?.[0] ?? '';

  assert.equal(/status:\s*"partial"\.to_string\(\)/.test(sourceRecovery), true);
  assert.match(sourceRecovery, /no playback-compatible audio\.mp4 was created/);
});

test('legacy checkpoint recovery remains partial when raw source checkpoints coexist', () => {
  const saver = read('frontend/src-tauri/src/audio/incremental_saver.rs');
  assert.match(
    saver,
    /Ok\(output\) if output\.status\.success\(\)[\s\S]*?source_checkpoint_file_count\(&folder_path\)\?[\s\S]*?keep_recovery_partial_for_source_checkpoints/,
  );
});

test('mixed-audio inspection counts microphone and system source checkpoints', () => {
  const saver = read('frontend/src-tauri/src/audio/incremental_saver.rs');
  const mixedInspection = saver.match(/fn inspect_existing_mixed_audio\([\s\S]*?\n}\n/)?.[0] ?? '';
  const sourceCounter = saver.match(/fn source_checkpoint_file_count\([\s\S]*?\n}\n/)?.[0] ?? '';

  assert.deepEqual({
    mixedInspectionIncludesSourceChunks: /source_checkpoint_file_count\(folder\)/.test(mixedInspection),
    checksSourceNamespaces: /\.join\("\.tracks"\)[\s\S]*?\.join\("\.checkpoints"\)/.test(sourceCounter),
    checksMicrophoneAndSystem: /microphone/.test(sourceCounter) && /system/.test(sourceCounter),
    includesSourceChunksInCount: /chunk_count/.test(mixedInspection) && /"partial"/.test(mixedInspection),
  }, {
    mixedInspectionIncludesSourceChunks: true,
    checksSourceNamespaces: true,
    checksMicrophoneAndSystem: true,
    includesSourceChunksInCount: true,
  });
});

test('pause or resume failure shuts down and joins the owner deterministically', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  const backend = read('frontend/src-tauri/src/audio/capture/backend.rs');
  assert.match(session, /shutdown_after_control_failure/);
  assert.match(session, /control_failure_stops_and_joins_before_returning/);
  assert.match(backend, /owner_exit_cause/);
  assert.match(backend, /pump_failure_is_reported_and_owner_remains_joinable/);
});

test('start resolves persisted destination and auto-save unless explicitly overridden', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(session, /load_recording_preferences/);
  assert.match(session, /request\.save_path[\s\S]*preferences\.save_folder/);
  assert.match(session, /request\.auto_save[\s\S]*preferences\.auto_save/);
  assert.match(session, /persisted_start_preferences_respect_explicit_overrides/);
  assert.match(session, /pub auto_save:\s*bool/);
});

test('Rust model preflight completes before recording resources are created', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(session, /public_model_preflight_failure_creates_no_recording_resources/);
});

test('compatibility transcript history projects real session-owned updates', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  const commands = read('frontend/src-tauri/src/audio/recording_commands.rs');
  const worker = read('frontend/src-tauri/src/audio/transcription/worker.rs');
  assert.match(session, /projected_transcript_history/);
  assert.match(worker, /transcript_history/);
  assert.match(worker, /transcript_history\.lock\(\)/);
  assert.match(commands, /RecordingSession::projected_transcript_history/);
  assert.doesNotMatch(commands, /get_transcript_history[\s\S]{0,300}Vec::new\(\)/);
  assert.match(worker, /partial_whisper_updates_are_kept_for_history_reload/);
  assert.match(worker, /speech_detection_is_reset_for_each_session/);
});

test('removed device compatibility commands are neither registered stubs nor frontend callers', () => {
  const commands = read('frontend/src-tauri/src/audio/recording_commands.rs');
  const lib = read('frontend/src-tauri/src/lib.rs');
  for (const command of ['poll_audio_device_events', 'get_reconnection_status', 'attempt_device_reconnect']) {
    assert.doesNotMatch(commands, new RegExp(`fn ${command}\\(`));
    assert.doesNotMatch(lib, new RegExp(`recording_commands::${command}`));
  }
  assert.match(commands, /get_active_audio_output/);
});

test('frontend recording failures branch on contract code and state, never message text', () => {
  const controls = read('frontend/src/components/RecordingControls.tsx');
  const service = read('frontend/src/services/recordingService.ts');
  assert.match(service, /function contractErrorFrom/);
  assert.match(controls, /contract\?\.code === 'device_unavailable'/);
  assert.match(controls, /contract\?\.code === 'permission_denied'/);
  assert.match(controls, /contract\?\.code === 'not_found' && !contract\.currentState/);
  assert.doesNotMatch(controls, /error\.message\.includes\(/);
});

test('failed start retains its idempotent identity and failed snapshot', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  const identityInsert = session.indexOf('insert_start_identity');
  const firstResource = session.indexOf('runtime.validate_model().await');
  assert.ok(identityInsert >= 0 && firstResource >= 0 && identityInsert < firstResource);
  assert.match(session, /record_start_failure/);
  assert.match(session, /failed_start_reuses_same_session_identity/);
});

test('tray state is projected only from the Rust-confirmed returned snapshot', () => {
  const tray = read('frontend/src-tauri/src/tray.rs');
  assert.match(tray, /Ok\(snapshot\)[\s\S]*update_tray_from_snapshot\([^;]*&snapshot\)/);
  assert.match(tray, /tray_pause_resume_projects_returned_snapshot_only/);
});

test('unexpected owner exit is monitored by the session and finalized as failure', () => {
  const backend = read('frontend/src-tauri/src/audio/capture/backend.rs');
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(backend, /owner_exit_rx/);
  assert.match(session, /monitor_capture_owner_exit/);
  assert.match(session, /unexpected_owner_exit_uses_session_finalization/);
});

test('stop enqueue is nonblocking before its bounded acknowledgement wait', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(session, /stop_and_join_capture[\s\S]*try_send\(CaptureControl::Stop/);
  assert.doesNotMatch(session, /stop_and_join_capture[\s\S]{0,700}\.send\(CaptureControl::Stop/);
  assert.match(session, /full_control_queue_cannot_deadlock_stop_enqueue/);
});

test('recovery retains verified partial Tracks and exposes a retryable persistence state', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(session, /verified_partial_tracks/);
  assert.match(session, /two_source_recovery_retains_verified_microphone_when_system_fails/);
  assert.match(session, /transient_recovery_failure_can_be_retried/);
});

test('final Track verification authenticates checkpoint evidence with bounded decoding', () => {
  const saver = read('frontend/src-tauri/src/audio/incremental_saver.rs');
  assert.match(saver, /struct CheckpointEvidence[\s\S]*input_sample_count[\s\S]*sha256/);
  assert.match(saver, /checkpoint hash mismatch/);
  assert.match(saver, /DECODE_VERIFY_BUFFER_BYTES/);
  assert.doesNotMatch(saver, /canonical-verification/);
  assert.match(saver, /packet_truncated_final_is_rebuilt_from_authenticated_checkpoints/);
  assert.match(saver, /corrupted_checkpoint_is_rejected_without_replacing_a_healthy_final/);
  assert.match(saver, /checkpoint_directory_io_failure_is_retryable_persistence/);
  assert.match(saver, /validate_checkpoint_evidence[\s\S]*SourceTrackRecoveryError/);
  assert.match(saver, /two_checkpoint_track_finalizes_and_recovers_with_verified_metadata/);
  assert.match(saver, /checkpoint_commands_cover_source_track_layout/);
  assert.match(saver, /checkpoint_sequence_sorts_numeric_indices_past_999/);
});

test('frontend refreshes reject older async generations and session responses', () => {
  const context = read('frontend/src/contexts/RecordingStateContext.tsx');
  assert.match(context, /refreshGenerationRef/);
  assert.match(context, /generation !== refreshGenerationRef\.current/);
  assert.match(read('frontend/tests/react-hooks-lifecycle.test.mjs'), /older_snapshot_response_cannot_overwrite_newer_session/);
});

test('application exit asks RecordingSession to shut down before app exit', () => {
  const lib = read('frontend/src-tauri/src/lib.rs');
  const tray = read('frontend/src-tauri/src/tray.rs');
  assert.match(lib, /fn begin_exit_after_recording_shutdown[\s\S]*compare_exchange[\s\S]*perform_recording_shutdown_and_exit/);
  assert.match(lib, /compare_exchange[\s\S]*latch_shutdown\(\)[\s\S]*perform_recording_shutdown_and_exit/);
  assert.match(lib, /RunEvent::ExitRequested[\s\S]*prevent_exit[\s\S]*begin_exit_after_recording_shutdown/);
  assert.match(tray, /"quit"[\s\S]*begin_exit_after_recording_shutdown/);
  assert.match(tray, /perform_recording_shutdown_and_exit/);
  assert.match(tray, /shutdown_active\(\)\.await/);
  assert.match(read('frontend/src-tauri/src/audio/recording_session.rs'), /shutdown_waits_for_an_in_flight_finalizer_before_reporting_safe_exit/);
});

test('RecordingSession owns an executable injected capture seam', () => {
  const session = read('frontend/src-tauri/src/audio/recording_session.rs');
  assert.match(session, /trait CaptureOwnerFactory/);
  assert.match(session, /new_with_capture_factory/);
  assert.match(session, /shared_start_seam_opens_and_stops_the_production_owner_topology/);
  assert.match(session, /auto_save_false_discards_audio_without_creating_a_track_namespace/);
  assert.match(session, /public_partial_start_failure_persists_failed_metadata_and_notifies_once/);
  assert.match(session, /rollback_start_tasks[\s\S]*Vec<FinalizedTrack>/);
  assert.match(session, /wrong_session_stop_preserves_active_owner/);
  assert.match(session, /control_failure_stops_and_joins_before_returning/);
});
