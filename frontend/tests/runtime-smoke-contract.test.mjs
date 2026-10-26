import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const repositoryRoot = path.resolve(import.meta.dirname, '../..');

function readRepositoryFile(relativePath) {
  const absolutePath = path.join(repositoryRoot, relativePath);
  assert.equal(existsSync(absolutePath), true, `${relativePath} must exist`);
  return readFileSync(absolutePath, 'utf8');
}

test('local model smoke is explicit, offline, temporary, and runs exact ignored tests', () => {
  const script = readRepositoryFile('scripts/smoke-local-models.sh');

  for (const requiredVariable of [
    'MEETILY_TEST_MODELS_DIR',
    'MEETILY_LLAMA_HELPER',
    'MEETILY_TEST_SUMMARY_MODEL_PATH',
    'MEETILY_TEST_SUMMARY_MODEL_NAME',
    'MEETILY_TEST_SUMMARY_EXPECTED_TEXT',
  ]) {
    assert.match(script, new RegExp(`required_env ${requiredVariable}`));
  }

  assert.match(script, /mktemp -d/);
  assert.match(script, /trap cleanup EXIT/);
  assert.match(script, /\/usr\/bin\/say/);
  assert.match(script, /MEETILY_TEST_SAY_VOICE:-Samantha/);
  assert.match(script, /say -v "\$say_voice"/);
  assert.match(script, /\$1 == voice && \$2 ~ \/\^en_\//);
  assert.match(script, /speech_bytes=.*stat -f %z/);
  assert.match(script, /speech_bytes < 16000/);
  assert.match(script, /parakeet_installed_model_transcribes_known_speech/);
  assert.match(script, /local_summary_helper_generates_expected_text/);
  assert.doesNotMatch(script, /MEETILY_TEST_SUMMARY_TEMPLATE/);
  assert.match(script, /if cargo test[\s\S]*parakeet_installed_model_transcribes_known_speech/);
  assert.match(script, /if cargo test[\s\S]*local_summary_helper_generates_expected_text/);
  assert.match(script, /model_failures > 0/);
  assert.doesNotMatch(script, /\b(curl|wget)\b/);
});

test('local model Rust harness keeps real model tests ignored and unit-tests validation', () => {
  const source = readRepositoryFile('frontend/src-tauri/tests/local_model_runtime.rs');

  assert.match(source, /#\[ignore = "requires explicit local Parakeet model and speech fixture"\]/);
  assert.match(source, /async fn parakeet_installed_model_transcribes_known_speech/);
  assert.match(source, /#\[ignore = "requires explicit local llama-helper and summary model"\]/);
  assert.match(source, /async fn local_summary_helper_generates_expected_text/);
  assert.match(source, /fn normalization_is_case_and_punctuation_insensitive/);
  assert.match(source, /fn response_validation_rejects_control_marker_leakage/);
  assert.match(source, /kill_on_drop\(true\)/);
  assert.match(source, /timeout\(/);
  assert.match(source, /const STARTUP_TIMEOUT: Duration = Duration::from_secs\(90\)/);
  assert.match(source, /const GENERATION_TIMEOUT: Duration/);
  assert.match(source, /const SUMMARY_HELPER_CONTEXT_CAP: u32 = 1024/);
  assert.match(source, /get_model_by_name/);
  assert.match(source, /sanitize_for_llama_helper/);
  assert.match(source, /model_def\.stop_tokens|sampling\.stop_tokens/);
  assert.doesNotMatch(source, /MEETILY_TEST_SUMMARY_TEMPLATE/);
  assert.match(source, /helper\/model compatibility.*not full production summary/i);
  assert.match(source, /response_validation_rejects_reordered_expected_words/);
  assert.match(source, /response_validation_rejects_production_template_marker_leakage/);
});

test('macOS hardware smoke is opt-in and CoreAudio capture is bounded and rejects silence', () => {
  const script = readRepositoryFile('scripts/smoke-macos-audio.sh');
  const discovery = readRepositoryFile(
    'frontend/src-tauri/src/audio/devices/discovery.rs',
  );
  const capture = readRepositoryFile(
    'frontend/src-tauri/src/audio/capture/core_audio.rs',
  );

  assert.match(script, /MEETILY_RUN_HARDWARE_TESTS/);
  assert.match(script, /MEETILY_PACKAGED_APP/);
  assert.match(script, /capture_test_binary/);
  assert.match(script, /ditto .*MEETILY_PACKAGED_APP.*smoke_app/);
  assert.match(script, /codesign .*--sign -/);
  assert.match(script, /\/usr\/bin\/afplay/);
  assert.match(script, /macos_system_audio_devices_have_valid_names/);
  assert.doesNotMatch(script, /audio::system_audio_commands::tests::/);
  assert.match(script, /test_core_audio_capture/);
  assert.match(script, /run_ignored_test/);
  assert.match(script, /running \[1-9\]\[0-9\]\* tests\?/);
  assert.match(script, /test result: ok\\\. \[1-9\]\[0-9\]\* passed/);
  assert.doesNotMatch(script, /tccutil/);
  assert.match(script, /audio_failures=0/);
  assert.match(
    script,
    /if run_ignored_test[\s\S]*cargo test -p meetily --lib \"\$enumeration_test\"/,
  );
  assert.doesNotMatch(
    script,
    /audio::system_audio_commands::tests::macos_system_audio_devices_have_valid_names/,
  );
  assert.match(
    script,
    /if run_ignored_test[\s\S]*"\$smoke_executable" "\$capture_test"/,
  );
  assert.match(script, /stop_tone[\s\S]*OVERALL FAIL:/);
  assert.match(script, /OVERALL NOT_RUN:.*microphone/i);
  assert.match(script, /if \(\(audio_failures > 0\)\); then/);
  assert.match(script, /OVERALL FAIL:[\s\S]*exit 1[\s\S]*OVERALL NOT_RUN:/);
  assert.match(script, /exit 2\s*$/);
  assert.match(
    discovery,
    /#\[ignore = "requires real macOS audio hardware"\][\s\S]*fn macos_system_audio_devices_have_valid_names/,
  );
  assert.match(discovery, /block_on\(list_audio_devices\(\)\)/);
  assert.match(discovery, /DeviceType::Output/);
  assert.match(discovery, /no output devices/);
  assert.match(discovery, /empty name/);
  assert.match(discovery, /system_audio_device_validation_rejects_input_only_list/);

  assert.match(capture, /std::thread::spawn/);
  assert.match(capture, /recv_timeout\(Duration::from_secs\(15\)\)/);
  assert.match(capture, /samples\.len\(\) < 48_000/);
  assert.match(capture, /rms > 0\.001/);
  assert.match(capture, /peak > 0\.01/);
});

test('macOS smoke harness rejects zero-test output for both system checks', () => {
  const scriptPath = path.join(repositoryRoot, 'scripts/smoke-macos-audio.sh');
  const result = spawnSync('bash', [scriptPath], {
    cwd: repositoryRoot,
    env: { ...process.env, MEETILY_AUDIO_SMOKE_HARNESS: '1' },
    encoding: 'utf8',
  });

  assert.equal(result.error, undefined, result.error?.message);
  assert.equal(
    result.status,
    0,
    `${result.stdout}\n${result.stderr}`,
  );
  assert.match(
    `${result.stdout}\n${result.stderr}`,
    /PASS: smoke harness rejects zero-test results for enumeration and capture\./,
  );
});
