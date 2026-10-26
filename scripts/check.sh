#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
FRONTEND_DIR="$REPOSITORY_ROOT/frontend"

usage() {
  cat <<'EOF'
Usage: ./scripts/check.sh [--quick|--full|--docs|--audio|--summary]

Modes:
  --quick  Run every automatic gate except the production frontend build.
  --full   Run the complete pre-build gate (default), including pnpm build.
  --docs   Run the documentation guide contract only.
  --audio  Run the selected audio Rust tests only.
  --summary
           Run the selected summary Rust tests only.

Automatic stages:
  git diff --check <base>...HEAD (plus staged and unstaged changes)
  cd frontend && pnpm check:policy
  cd frontend && pnpm lint
  cd frontend && pnpm typecheck
  cd frontend && pnpm test:frontend
  cd frontend && pnpm exec playwright test (critical recovery/contracts)
  cd frontend && pnpm build (--full only)
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo test --test recording_recovery successful_checkpoint_recovery_preserves_neighboring_meeting_files -- --ignored (macOS only)

Targeted stages:
  --docs   cd frontend && pnpm test:docs
  --audio  cargo test -p meetily --lib audio:: (must match at least one test)
  --summary
           cargo test -p meetily --lib summary:: (must match at least one test)

This command does not package the desktop application. After a successful full
check, create and verify the packaged candidate with:
  ./scripts/release-candidate.sh
EOF
}

MODE=full
case "${1:-}" in
  "") ;;
  --quick) MODE=quick ;;
  --full) MODE=full ;;
  --docs) MODE=docs ;;
  --audio) MODE=audio ;;
  --summary) MODE=summary ;;
  --help|-h)
    usage
    exit 0
    ;;
  *)
    echo "Unknown option: $1" >&2
    usage >&2
    exit 2
    ;;
esac

if [ "$#" -gt 1 ]; then
  echo "Only one mode option is accepted." >&2
  usage >&2
  exit 2
fi

if [ -z "${DEVELOPER_DIR:-}" ] && [ "${MEETILY_CHECK_NO_XCODE:-0}" != "1" ] && [ -d /Applications/Xcode.app/Contents/Developer ]; then
  DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
  export DEVELOPER_DIR
fi

run_in_frontend() {
  (cd "$FRONTEND_DIR" && "$@")
}

FAILURES=0
STARTED_AT=$(date +%s)

run_stage() {
  stage_name=$1
  standalone_command=$2
  shift 2
  stage_started_at=$(date +%s)

  echo
  echo "RUN  $stage_name"
  echo "COMMAND: $standalone_command"

  if "$@"; then
    stage_finished_at=$(date +%s)
    echo "PASS $stage_name ($((stage_finished_at - stage_started_at))s)"
  else
    stage_status=$?
    stage_finished_at=$(date +%s)
    echo "FAIL $stage_name ($((stage_finished_at - stage_started_at))s, exit $stage_status)" >&2
    echo "FAILED COMMAND: $standalone_command" >&2
    FAILURES=$((FAILURES + 1))
  fi
}

run_filtered_cargo_test() {
  filter=$1
  scope=${filter%%::*}
  list_file=$(mktemp "${TMPDIR:-/tmp}/meetily-check-tests.XXXXXX")

  if cargo test -p meetily --lib "$filter" -- --list >"$list_file" 2>&1; then
    :
  else
    status=$?
    cat "$list_file"
    rm -f "$list_file"
    return "$status"
  fi

  cat "$list_file"
  matched_tests=$(grep -Ec ':[[:space:]]+test$' "$list_file" || true)
  rm -f "$list_file"
  if [ "$matched_tests" -eq 0 ]; then
    echo "No tests matched the $scope filter" >&2
    return 1
  fi

  cargo test -p meetily --lib "$filter" --
}

cd "$REPOSITORY_ROOT"
echo "Meetily automatic check ($MODE mode)"

run_standard_check() {
  run_stage "Patch whitespace" "./scripts/check-whitespace.sh" "$SCRIPT_DIR/check-whitespace.sh"
  run_stage "Production policy" "cd frontend && pnpm check:policy" run_in_frontend pnpm check:policy
  run_stage "Frontend lint" "cd frontend && pnpm lint" run_in_frontend pnpm lint
  run_stage "TypeScript" "cd frontend && pnpm typecheck" run_in_frontend pnpm typecheck
  run_stage "Frontend unit tests" "cd frontend && pnpm test:frontend" run_in_frontend pnpm test:frontend
  run_stage \
    "Critical browser recovery and contracts" \
    "cd frontend && pnpm exec playwright test tests/interrupted-recovery.spec.ts tests/indexeddb-transcript-contract.spec.ts tests/fixture-contracts.spec.ts tests/recording-folder-selection.spec.ts tests/desktop-local-safety.spec.ts tests/meeting-search.spec.ts --workers=2" \
    run_in_frontend pnpm exec playwright test \
      tests/interrupted-recovery.spec.ts \
      tests/indexeddb-transcript-contract.spec.ts \
      tests/fixture-contracts.spec.ts \
      tests/recording-folder-selection.spec.ts \
      tests/desktop-local-safety.spec.ts \
      tests/meeting-search.spec.ts \
      --workers=2

  if [ "$MODE" = "full" ]; then
    run_stage "Production frontend build" "cd frontend && pnpm build" run_in_frontend pnpm build
  else
    echo
    echo "SKIP Production frontend build (quick mode; run --full before packaging)"
  fi

  run_stage "Rust format" "cargo fmt --all -- --check" cargo fmt --all -- --check
  run_stage \
    "Rust clippy" \
    "cargo clippy --workspace --all-targets -- -D warnings" \
    cargo clippy --workspace --all-targets -- -D warnings
  run_stage "Rust tests" "cargo test --workspace" cargo test --workspace

  HOST_OS=$(uname -s)
  if [ "$HOST_OS" = "Darwin" ]; then
    run_stage \
      "Packaged FFmpeg recovery" \
      "cargo test --test recording_recovery successful_checkpoint_recovery_preserves_neighboring_meeting_files -- --ignored" \
      cargo test --test recording_recovery \
        successful_checkpoint_recovery_preserves_neighboring_meeting_files \
        -- --ignored
  else
    echo
    echo "SKIP Packaged FFmpeg recovery (requires macOS; detected $HOST_OS)"
  fi
}

case "$MODE" in
  docs)
    run_stage \
      "Documentation guide" \
      "cd frontend && pnpm test:docs" \
      run_in_frontend pnpm test:docs
    ;;
  audio)
    run_stage \
      "Audio tests" \
      "cargo test -p meetily --lib audio:: -- --list (must match at least one test), then cargo test -p meetily --lib audio::" \
      run_filtered_cargo_test "audio::"
    ;;
  summary)
    run_stage \
      "Summary tests" \
      "cargo test -p meetily --lib summary:: -- --list (must match at least one test), then cargo test -p meetily --lib summary::" \
      run_filtered_cargo_test "summary::"
    ;;
  *)
    run_standard_check
    ;;
esac

FINISHED_AT=$(date +%s)
echo
if [ "$FAILURES" -eq 0 ]; then
  echo "PASS Meetily automatic check ($((FINISHED_AT - STARTED_AT))s)"
  exit 0
fi

echo "FAIL Meetily automatic check: $FAILURES stage(s) failed ($((FINISHED_AT - STARTED_AT))s)" >&2
exit 1
