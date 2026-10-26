#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

[[ $(uname -s) == Darwin ]] || fail 'fixture generation requires macOS'
[[ -x /usr/bin/say ]] || fail '/usr/bin/say is unavailable'

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P) ||
  fail 'unable to resolve script directory'
repo_root=$(git -C "$script_dir/.." rev-parse --show-toplevel 2>/dev/null) ||
  fail 'script is not inside a Git worktree'
output_path=${1:-$repo_root/artifacts/diarization-fixture/two-speaker-16k.wav}
voice_a=${MEETILY_DIARIZATION_VOICE_A:-Samantha}
voice_b=${MEETILY_DIARIZATION_VOICE_B:-Daniel}
ffmpeg=${MEETILY_FFMPEG:-$(command -v ffmpeg || true)}

[[ -n $ffmpeg && -x $ffmpeg ]] ||
  fail 'no local FFmpeg found; set MEETILY_FFMPEG to an existing executable'

available_voices=$(/usr/bin/say -v '?' 2>/dev/null) || fail 'unable to list macOS voices'
for voice in "$voice_a" "$voice_b"; do
  if ! awk -v voice="$voice" '$1 == voice && $2 ~ /^en_/ { found = 1 } END { exit !found }' \
    <<<"$available_voices"; then
    fail "English macOS voice '$voice' is unavailable"
  fi
done
[[ $voice_a != "$voice_b" ]] || fail 'the two fixture voices must be different'

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/meetily-diarization-fixture.XXXXXX") ||
  fail 'unable to create temporary fixture directory'
cleanup() {
  rm -rf -- "$temp_dir"
}
trap cleanup EXIT

mkdir -p -- "$(dirname -- "$output_path")"

texts=(
  'Hello team. I am Samantha. Today I will review the product release timeline and confirm the testing plan.'
  'Thanks Samantha. I am Daniel. I will check the installer, permissions, and the final release notes.'
  'The automatic checks must finish before we approve the build. I will record every remaining blocker.'
  'I will test the recording workflow and verify that existing transcripts remain safe after an error.'
)
voices=("$voice_a" "$voice_b" "$voice_a" "$voice_b")

for index in 0 1 2 3; do
  aiff_path=$temp_dir/turn-$index.aiff
  wav_path=$temp_dir/turn-$index.wav
  /usr/bin/say -v "${voices[$index]}" -r 165 -o "$aiff_path" -- "${texts[$index]}"
  "$ffmpeg" -hide_banner -loglevel error -y -i "$aiff_path" \
    -ar 16000 -ac 1 -c:a pcm_s16le "$wav_path"
done

silence_path=$temp_dir/silence.wav
"$ffmpeg" -hide_banner -loglevel error -y \
  -f lavfi -i anullsrc=r=16000:cl=mono -t 1.25 \
  -ar 16000 -ac 1 -c:a pcm_s16le "$silence_path"

concat_list=$temp_dir/concat.txt
for index in 0 1 2 3; do
  printf "file '%s'\n" "$temp_dir/turn-$index.wav" >> "$concat_list"
  if [[ $index -lt 3 ]]; then
    printf "file '%s'\n" "$silence_path" >> "$concat_list"
  fi
done

temporary_output=$temp_dir/two-speaker-16k.wav
"$ffmpeg" -hide_banner -loglevel error -y \
  -f concat -safe 0 -i "$concat_list" \
  -ar 16000 -ac 1 -c:a pcm_s16le "$temporary_output"
[[ -s $temporary_output ]] || fail 'FFmpeg produced an empty fixture'
mv -f -- "$temporary_output" "$output_path"

printf 'PASS: generated deterministic two-speaker fixture\n'
printf 'VOICE_A=%s\nVOICE_B=%s\nOUTPUT=%s\n' "$voice_a" "$voice_b" "$output_path"
