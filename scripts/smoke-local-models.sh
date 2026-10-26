#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

required_env() {
  local name=$1
  [[ -n ${!name:-} ]] || fail "$name must be set explicitly"
}

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

[[ $(uname -s) == Darwin ]] || {
  printf 'NOT_RUN: local model smoke currently requires macOS speech fixture generation.\n' >&2
  exit 2
}

required_env MEETILY_TEST_MODELS_DIR
required_env MEETILY_LLAMA_HELPER
required_env MEETILY_TEST_SUMMARY_MODEL_PATH
required_env MEETILY_TEST_SUMMARY_MODEL_NAME
required_env MEETILY_TEST_SUMMARY_EXPECTED_TEXT

[[ -d $MEETILY_TEST_MODELS_DIR ]] ||
  fail "MEETILY_TEST_MODELS_DIR is not a directory: $MEETILY_TEST_MODELS_DIR"
[[ -x $MEETILY_LLAMA_HELPER ]] ||
  fail "MEETILY_LLAMA_HELPER is not executable: $MEETILY_LLAMA_HELPER"
[[ -f $MEETILY_TEST_SUMMARY_MODEL_PATH ]] ||
  fail "MEETILY_TEST_SUMMARY_MODEL_PATH is not a file: $MEETILY_TEST_SUMMARY_MODEL_PATH"
[[ -x /usr/bin/say ]] || fail "/usr/bin/say is unavailable"
say_voice=${MEETILY_TEST_SAY_VOICE:-Samantha}
available_voices=$(/usr/bin/say -v '?' 2>/dev/null) ||
  fail "unable to list installed macOS voices"
if ! awk -v voice="$say_voice" '$1 == voice && $2 ~ /^en_/ { found = 1 } END { exit !found }' \
  <<<"$available_voices"; then
  fail "macOS English voice '$say_voice' is unavailable; install it or set MEETILY_TEST_SAY_VOICE to an installed English voice"
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P) ||
  fail "unable to resolve script directory"
repo_root=$(git -C "$script_dir/.." rev-parse --show-toplevel 2>/dev/null) ||
  fail "script is not inside a Git worktree"
ffmpeg=$(resolve_ffmpeg)
temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/meetily-local-model-smoke.XXXXXX") ||
  fail "unable to create temporary fixture directory"

cleanup() {
  rm -rf -- "$temp_dir"
}
trap cleanup EXIT

speech_text=${MEETILY_TEST_SPEECH_TEXT:-"Meetily records local meeting notes safely on Friday"}
speech_aiff=$temp_dir/known-speech.aiff
speech_wav=$temp_dir/known-speech.wav

/usr/bin/say -v "$say_voice" -o "$speech_aiff" -- "$speech_text" ||
  fail "macOS say could not create the temporary speech fixture"
"$ffmpeg" -hide_banner -loglevel error -y -i "$speech_aiff" -ac 1 -ar 16000 "$speech_wav" ||
  fail "local FFmpeg could not convert the temporary speech fixture"
[[ -s $speech_wav ]] || fail "temporary speech fixture is empty"
speech_bytes=$(stat -f %z "$speech_wav") || fail "unable to inspect temporary speech fixture"
if ((speech_bytes < 16000)); then
  fail "temporary speech fixture contains too little audio ($speech_bytes bytes); run on an interactive Mac with a working system voice"
fi

export DEVELOPER_DIR=${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}
export MEETILY_TEST_AUDIO_PATH=$speech_wav
export MEETILY_TEST_EXPECTED_TEXT=$speech_text

cd "$repo_root"
model_failures=0

printf 'RUN: local Parakeet transcription smoke (no downloads, temporary fixture only)\n'
if cargo test -p meetily --test local_model_runtime \
  parakeet_installed_model_transcribes_known_speech --locked --offline -- \
  --ignored --exact --nocapture; then
  printf 'PASS: local Parakeet transcription sub-check.\n'
else
  printf 'FAIL: local Parakeet transcription sub-check.\n' >&2
  model_failures=$((model_failures + 1))
fi

printf 'RUN: local summary helper smoke (no downloads, explicit model only)\n'
if cargo test -p meetily --test local_model_runtime \
  local_summary_helper_generates_expected_text --locked --offline -- \
  --ignored --exact --nocapture; then
  printf 'PASS: local summary helper sub-check.\n'
else
  printf 'FAIL: local summary helper sub-check.\n' >&2
  model_failures=$((model_failures + 1))
fi

if ((model_failures > 0)); then
  printf 'FAIL: %s local model runtime sub-check(s) failed.\n' "$model_failures" >&2
  exit 1
fi

printf 'PASS: explicit local model runtime smoke completed.\n'
