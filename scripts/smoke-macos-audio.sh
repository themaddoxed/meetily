#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

run_ignored_test() {
  local label=$1
  local expected_test=$2
  local output_path=$3
  shift 3

  if "$@" >"$output_path" 2>&1; then
    cat "$output_path"
  else
    cat "$output_path" >&2
    return 1
  fi

  if ! grep -Eq '^running [1-9][0-9]* tests?$' "$output_path"; then
    printf 'FAIL: %s matched zero tests (expected %s).\n' "$label" "$expected_test" >&2
    return 1
  fi
  if ! grep -Fqx -- "test ${expected_test} ... ok" "$output_path"; then
    printf 'FAIL: %s did not execute the expected test: %s.\n' "$label" "$expected_test" >&2
    return 1
  fi
  if ! grep -Eq '^test result: ok\. [1-9][0-9]* passed;' "$output_path"; then
    printf 'FAIL: %s did not report a passing test result.\n' "$label" >&2
    return 1
  fi
}

run_harness_self_test() {
  local fixture_dir
  fixture_dir=$(mktemp -d "${TMPDIR:-/tmp}/meetily-audio-smoke-harness.XXXXXX") ||
    fail "unable to create smoke harness fixture directory"

  local enumeration_test=audio::devices::discovery::tests::macos_system_audio_devices_have_valid_names
  local capture_test=audio::capture::core_audio::tests::test_core_audio_capture
  local zero_fixture=$fixture_dir/zero-tests.log
  local passing_fixture=$fixture_dir/passing-test.log
  local zero_result=$fixture_dir/zero-result.log
  local passing_result=$fixture_dir/passing-result.log

  printf '%s\n' \
    'running 0 tests' \
    'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out' \
    >"$zero_fixture"
  if run_ignored_test 'enumeration zero-test fixture' "$enumeration_test" "$zero_result" \
    cat "$zero_fixture"; then
    rm -rf -- "$fixture_dir"
    fail 'the harness accepted zero enumeration tests'
  fi
  if run_ignored_test 'capture zero-test fixture' "$capture_test" "$zero_result" \
    cat "$zero_fixture"; then
    rm -rf -- "$fixture_dir"
    fail 'the harness accepted zero capture tests'
  fi

  printf '%s\n' \
    'running 1 test' \
    "test $enumeration_test ... ok" \
    'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' \
    >"$passing_fixture"
  run_ignored_test 'enumeration passing fixture' "$enumeration_test" "$passing_result" \
    cat "$passing_fixture" || {
    rm -rf -- "$fixture_dir"
    fail 'the harness rejected a passing enumeration test'
  }

  printf '%s\n' \
    'running 1 test' \
    "test $capture_test ... ok" \
    'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out' \
    >"$passing_fixture"
  run_ignored_test 'capture passing fixture' "$capture_test" "$passing_result" \
    cat "$passing_fixture" || {
    rm -rf -- "$fixture_dir"
    fail 'the harness rejected a passing capture test'
  }

  rm -rf -- "$fixture_dir"
  printf 'PASS: smoke harness rejects zero-test results for enumeration and capture.\n'
}

if [[ ${MEETILY_AUDIO_SMOKE_HARNESS:-} == 1 ]]; then
  run_harness_self_test
  exit 0
fi

resolve_ffmpeg() {
  local candidate

  if [[ -n ${MEETILY_FFMPEG:-} ]]; then
    [[ -x $MEETILY_FFMPEG ]] || fail "MEETILY_FFMPEG is not executable: $MEETILY_FFMPEG"
    printf '%s\n' "$MEETILY_FFMPEG"
    return
  fi

  if command -v ffmpeg >/dev/null 2>&1; then
    command -v ffmpeg
    return
  fi

  for candidate in \
    "$repo_root/frontend/src-tauri/binaries/ffmpeg-universal-apple-darwin" \
    "$repo_root/frontend/src-tauri/binaries/ffmpeg-$(uname -m)-apple-darwin"; do
    if [[ -x $candidate ]]; then
      printf '%s\n' "$candidate"
      return
    fi
  done

  fail "no local FFmpeg found; set MEETILY_FFMPEG to an existing executable"
}

[[ ${MEETILY_RUN_HARDWARE_TESTS:-} == 1 ]] || {
  printf 'NOT_RUN: set MEETILY_RUN_HARDWARE_TESTS=1 only on the Mac being tested.\n' >&2
  exit 2
}
[[ $(uname -s) == Darwin ]] || {
  printf 'NOT_RUN: macOS audio smoke requires a real Mac.\n' >&2
  exit 2
}
[[ -x /usr/bin/afplay ]] || fail "/usr/bin/afplay is unavailable"
[[ -d ${MEETILY_PACKAGED_APP:-} ]] ||
  fail "MEETILY_PACKAGED_APP must point to the exact packaged .app being tested"
[[ -f $MEETILY_PACKAGED_APP/Contents/Info.plist ]] ||
  fail "MEETILY_PACKAGED_APP has no Contents/Info.plist: $MEETILY_PACKAGED_APP"

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P) ||
  fail "unable to resolve script directory"
repo_root=$(git -C "$script_dir/.." rev-parse --show-toplevel 2>/dev/null) ||
  fail "script is not inside a Git worktree"
ffmpeg=$(resolve_ffmpeg)
temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/meetily-macos-audio-smoke.XXXXXX") ||
  fail "unable to create temporary fixture directory"
tone_path=$temp_dir/known-system-audio.wav
smoke_app=$temp_dir/meetily-audio-smoke.app
afplay_pid=
awake_pid=

stop_tone() {
  if [[ -n $afplay_pid ]]; then
    kill "$afplay_pid" 2>/dev/null || true
    wait "$afplay_pid" 2>/dev/null || true
    afplay_pid=
  fi
}

cleanup() {
  stop_tone
  if [[ -n $awake_pid ]]; then
    kill "$awake_pid" 2>/dev/null || true
    wait "$awake_pid" 2>/dev/null || true
  fi
  rm -rf -- "$temp_dir"
}
trap cleanup EXIT

# Keep this hardware run awake; release the assertions when this shell exits,
# including an interrupted run. No permanent sleep or password settings change.
[[ -x /usr/bin/caffeinate ]] || fail "/usr/bin/caffeinate is unavailable"
/usr/bin/caffeinate -di -w "$$" &
awake_pid=$!
kill -0 "$awake_pid" 2>/dev/null || fail "unable to prevent idle sleep for this hardware run"

"$ffmpeg" -hide_banner -loglevel error -y \
  -f lavfi -i 'sine=frequency=880:sample_rate=48000:duration=20' \
  -filter:a 'volume=0.2' -ac 1 "$tone_path" ||
  fail "local FFmpeg could not create the temporary system-audio signal"
[[ -s $tone_path ]] || fail "temporary system-audio signal is empty"

export DEVELOPER_DIR=${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}
cd "$repo_root"
audio_failures=0

app_executable=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' \
  "$MEETILY_PACKAGED_APP/Contents/Info.plist") ||
  fail "unable to read CFBundleExecutable from packaged app"
[[ -n $app_executable ]] || fail "packaged app has an empty CFBundleExecutable"

printf 'RUN: build CoreAudio test binary for packaged app host\n'
capture_build_output=$(cargo test -p meetily --lib --locked --offline --no-run 2>&1) || {
  printf '%s\n' "$capture_build_output" >&2
  fail "unable to build CoreAudio test binary"
}
printf '%s\n' "$capture_build_output"
capture_test_binary=$(printf '%s\n' "$capture_build_output" |
  sed -n 's/.*Executable unittests src\/lib.rs (\(.*\))$/\1/p' |
  tail -n 1)
[[ -x $capture_test_binary ]] ||
  fail "unable to resolve executable CoreAudio test binary from cargo output"

ditto "$MEETILY_PACKAGED_APP" "$smoke_app" ||
  fail "unable to copy packaged app into temporary CoreAudio test host"
cp "$capture_test_binary" "$smoke_app/Contents/MacOS/$app_executable" ||
  fail "unable to install CoreAudio test binary into temporary app host"
codesign --force --deep --sign - --options runtime \
  --entitlements "$repo_root/frontend/src-tauri/entitlements.plist" "$smoke_app" ||
  fail "unable to ad-hoc sign temporary CoreAudio test host"
smoke_executable=$smoke_app/Contents/MacOS/$app_executable
[[ -x $smoke_executable ]] || fail "temporary CoreAudio test host is not executable"

printf 'RUN: real macOS system audio device enumeration\n'
enumeration_test=audio::devices::discovery::tests::macos_system_audio_devices_have_valid_names
if run_ignored_test \
  'real macOS system audio device enumeration' \
  "$enumeration_test" \
  "$temp_dir/device-enumeration.log" \
  cargo test -p meetily --lib "$enumeration_test" \
    --locked --offline -- --ignored --exact --nocapture; then
  printf 'PASS: real macOS system audio device enumeration.\n'
else
  printf 'FAIL: real macOS system audio device enumeration.\n' >&2
  audio_failures=$((audio_failures + 1))
fi

printf 'RUN: CoreAudio capture while a known local tone is playing\n'
/usr/bin/afplay "$tone_path" &
afplay_pid=$!
capture_test=audio::capture::core_audio::tests::test_core_audio_capture
if run_ignored_test \
  'non-silent CoreAudio capture' \
  "$capture_test" \
  "$temp_dir/core-audio-capture.log" \
  "$smoke_executable" "$capture_test" \
    --ignored --exact --nocapture; then
  printf 'PASS: non-silent CoreAudio capture.\n'
else
  printf 'FAIL: non-silent CoreAudio capture.\n' >&2
  audio_failures=$((audio_failures + 1))
fi
stop_tone

printf 'NOT_RUN: microphone non-silent capture still requires the manual smoke checklist.\n'
if ((audio_failures > 0)); then
  printf 'OVERALL FAIL: %s system-audio sub-check(s) failed; microphone remains NOT_RUN.\n' \
    "$audio_failures" >&2
  exit 1
fi

printf 'PASS: system device enumeration and non-silent CoreAudio capture completed.\n'
printf 'OVERALL NOT_RUN: microphone non-silent capture still requires the manual smoke checklist.\n'
exit 2
