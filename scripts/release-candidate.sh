#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
FRONTEND_DIR="$REPOSITORY_ROOT/frontend"

usage() {
  cat <<'EOF'
Usage: ./scripts/release-candidate.sh [--diagnostic-build]

Normal mode requires a completely clean worktree and a successful full automatic
check before it builds and verifies a Universal macOS app and DMG.

  --diagnostic-build  Continue after a failed automatic check so packaging can
                      be diagnosed, including from a dirty worktree. The result
                      is always labelled UNGATED and exits nonzero.

Release evidence is written under artifacts/release-gate/<commit-sha>/.
Diagnostic evidence uses a unique diagnostic[-dirty]/<commit-sha>/run-* path.
EOF
}

MODE=normal
case "${1:-}" in
  "") ;;
  --diagnostic-build) MODE=diagnostic ;;
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
  exit 2
fi

cd "$REPOSITORY_ROOT"
WORKTREE_STATUS=$(git status --porcelain)
WORKTREE_CLEAN=true
if [ -n "$WORKTREE_STATUS" ]; then
  WORKTREE_CLEAN=false
fi

COMMIT_SHA=$(git rev-parse HEAD)
EVIDENCE_DIR="$REPOSITORY_ROOT/artifacts/release-gate/$COMMIT_SHA"
RUN_ID="normal-$COMMIT_SHA"
if [ "$MODE" = diagnostic ]; then
  evidence_category=diagnostic
  if [ "$WORKTREE_CLEAN" = false ]; then
    evidence_category=diagnostic-dirty
    echo "DIRTY worktree diagnostic packaging is enabled. The result is UNGATED and must not be released." >&2
  fi
  evidence_root="$REPOSITORY_ROOT/artifacts/release-gate/$evidence_category/$COMMIT_SHA"
  mkdir -p "$evidence_root"
  EVIDENCE_DIR=$(mktemp -d "$evidence_root/run-XXXXXX")
  RUN_ID=$(basename "$EVIDENCE_DIR")
elif [ "$WORKTREE_CLEAN" = false ]; then
  evidence_root="$REPOSITORY_ROOT/artifacts/release-gate/preflight-dirty/$COMMIT_SHA"
  mkdir -p "$evidence_root"
  EVIDENCE_DIR=$(mktemp -d "$evidence_root/run-XXXXXX")
  RUN_ID=$(basename "$EVIDENCE_DIR")
else
  mkdir -p "$EVIDENCE_DIR"
fi

AUTOMATIC_LOG="$EVIDENCE_DIR/automatic-check.log"
SEED_LOG="$EVIDENCE_DIR/artifact-seed.log"
BUILD_LOG="$EVIDENCE_DIR/universal-build.log"
VERSION_LOG="$EVIDENCE_DIR/version-resolution.log"
VERIFIER_LOG="$EVIDENCE_DIR/candidate-verifier.log"
SOURCE_FINGERPRINT_LOG="$EVIDENCE_DIR/source-fingerprint.log"
SOURCE_RECHECK_LOG="$EVIDENCE_DIR/source-recheck.log"
REPORT_PATH="$EVIDENCE_DIR/candidate-report.json"
PENDING_REPORT_PATH="$EVIDENCE_DIR/candidate-report.pending.json"
STATUS_PATH="$EVIDENCE_DIR/status.env"

# A report from an older attempt at this commit must never survive a new run.
rm -f "$REPORT_PATH" "$PENDING_REPORT_PATH"

SOURCE_FINGERPRINT=NOT_RUN
SOURCE_FINGERPRINT_STATUS=NOT_RUN
AUTOMATIC_STATUS=NOT_RUN
SEED_STATUS=NOT_RUN
BUILD_STATUS=NOT_RUN
VERSION_STATUS=NOT_RUN
VERIFIER_STATUS=NOT_RUN
FINAL_WORKTREE_CLEAN=NOT_RUN
FINAL_SOURCE_FINGERPRINT=NOT_RUN
SOURCE_RECHECK_STATUS=NOT_RUN
SOURCE_STABLE=NOT_RUN

if [ "$MODE" = diagnostic ]; then
  echo "Diagnostic packaging is enabled. Any artifact produced is UNGATED."
fi

write_status() {
  candidate_status=$1
  {
    echo "STATUS=$candidate_status"
    echo "COMMIT_SHA=$COMMIT_SHA"
    echo "MODE=$MODE"
    echo "WORKTREE_CLEAN=$WORKTREE_CLEAN"
    echo "RUN_ID=$RUN_ID"
    echo "SOURCE_FINGERPRINT=$SOURCE_FINGERPRINT"
    echo "SOURCE_FINGERPRINT_EXIT=$SOURCE_FINGERPRINT_STATUS"
    echo "FINAL_WORKTREE_CLEAN=$FINAL_WORKTREE_CLEAN"
    echo "FINAL_SOURCE_FINGERPRINT=$FINAL_SOURCE_FINGERPRINT"
    echo "SOURCE_RECHECK_EXIT=$SOURCE_RECHECK_STATUS"
    echo "SOURCE_STABLE=$SOURCE_STABLE"
    echo "AUTOMATIC_CHECK_EXIT=$AUTOMATIC_STATUS"
    echo "ARTIFACT_SEED_EXIT=$SEED_STATUS"
    echo "BUILD_EXIT=$BUILD_STATUS"
    echo "VERSION_RESOLUTION_EXIT=$VERSION_STATUS"
    echo "VERIFIER_EXIT=$VERIFIER_STATUS"
  } > "$STATUS_PATH"
}

valid_source_fingerprint() {
  candidate_fingerprint=$1
  case "$candidate_fingerprint" in
    ''|*[!a-fA-F0-9]*) return 1 ;;
  esac
  [ "${#candidate_fingerprint}" -eq 64 ]
}

recheck_source_state() {
  final_worktree_status=$(git status --porcelain)
  FINAL_WORKTREE_CLEAN=true
  if [ -n "$final_worktree_status" ]; then
    FINAL_WORKTREE_CLEAN=false
  fi

  set +e
  FINAL_SOURCE_FINGERPRINT=$(node "$FRONTEND_DIR/scripts/source-fingerprint.mjs" \
    --repository "$REPOSITORY_ROOT" 2> "$SOURCE_RECHECK_LOG")
  SOURCE_RECHECK_STATUS=$?
  set -e

  SOURCE_STABLE=false
  if [ "$SOURCE_RECHECK_STATUS" -eq 0 ] \
    && valid_source_fingerprint "$FINAL_SOURCE_FINGERPRINT" \
    && [ "$FINAL_SOURCE_FINGERPRINT" = "$SOURCE_FINGERPRINT" ]; then
    if [ "$MODE" = diagnostic ] || [ "$FINAL_WORKTREE_CLEAN" = true ]; then
      SOURCE_STABLE=true
    fi
  fi
}

block_source_change() {
  candidate_status=BLOCKED_SOURCE_CHANGED
  if [ "$MODE" = diagnostic ]; then
    candidate_status=UNGATED_SOURCE_CHANGED
  fi
  rm -f "$REPORT_PATH" "$PENDING_REPORT_PATH"
  write_status "$candidate_status"
  if [ "$SOURCE_RECHECK_STATUS" -ne 0 ]; then
    sed -n '1,$p' "$SOURCE_RECHECK_LOG" >&2
  fi
  echo "Release source changed or became unverifiable during packaging." >&2
  echo "Discard this artifact and restart from a stable worktree." >&2
  echo "Evidence: $EVIDENCE_DIR" >&2
}

set +e
SOURCE_FINGERPRINT=$(node "$FRONTEND_DIR/scripts/source-fingerprint.mjs" \
  --repository "$REPOSITORY_ROOT" 2> "$SOURCE_FINGERPRINT_LOG")
SOURCE_FINGERPRINT_STATUS=$?
set -e
if [ "$SOURCE_FINGERPRINT_STATUS" -ne 0 ] \
  || ! valid_source_fingerprint "$SOURCE_FINGERPRINT"; then
  SOURCE_FINGERPRINT=UNAVAILABLE
  candidate_status="SOURCE_FINGERPRINT_FAILED"
  if [ "$MODE" = diagnostic ]; then candidate_status="UNGATED_SOURCE_FINGERPRINT_FAILED"; fi
  write_status "$candidate_status"
  sed -n '1,$p' "$SOURCE_FINGERPRINT_LOG" >&2
  echo "Source fingerprint failed. Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

if [ "$WORKTREE_CLEAN" = false ] && [ "$MODE" = normal ]; then
  write_status "BLOCKED_DIRTY_WORKTREE"
  echo "Release candidate build refused: worktree changes are present." >&2
  echo "Commit, remove, or revert all tracked and untracked changes, then run the command again." >&2
  echo "Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

echo "Running the complete automatic release check for $COMMIT_SHA..."
set +e
"$SCRIPT_DIR/check.sh" --full > "$AUTOMATIC_LOG" 2>&1
AUTOMATIC_STATUS=$?
set -e
sed -n '1,$p' "$AUTOMATIC_LOG"

if [ "$AUTOMATIC_STATUS" -ne 0 ] && [ "$MODE" = "normal" ]; then
  write_status "BLOCKED_AUTOMATIC_CHECK"
  echo "Release candidate build blocked by the automatic check." >&2
  echo "Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

echo "Seeding reviewed local Universal sidecar artifacts..."
set +e
pnpm --dir "$FRONTEND_DIR" run artifacts:seed:universal > "$SEED_LOG" 2>&1
SEED_STATUS=$?
set -e
sed -n '1,$p' "$SEED_LOG"
if [ "$SEED_STATUS" -ne 0 ]; then
  candidate_status="ARTIFACT_SEED_FAILED"
  if [ "$MODE" = diagnostic ]; then candidate_status="UNGATED_ARTIFACT_SEED_FAILED"; fi
  write_status "$candidate_status"
  echo "Universal sidecar artifact seed failed. Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

echo "Building Universal Meetily app and DMG..."
set +e
pnpm --dir "$FRONTEND_DIR" run tauri:build:universal > "$BUILD_LOG" 2>&1
BUILD_STATUS=$?
set -e
sed -n '1,$p' "$BUILD_LOG"
if [ "$BUILD_STATUS" -ne 0 ]; then
  candidate_status="BUILD_FAILED"
  if [ "$MODE" = "diagnostic" ]; then candidate_status="UNGATED_BUILD_FAILED"; fi
  write_status "$candidate_status"
  echo "Universal candidate build failed. Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

set +e
VERSION=$(node -p "require('./frontend/package.json').version" 2> "$VERSION_LOG")
VERSION_STATUS=$?
set -e
if [ "$VERSION_STATUS" -eq 0 ] && [ -z "$VERSION" ]; then
  VERSION_STATUS=1
  echo "Package version command returned no value." > "$VERSION_LOG"
fi
if [ "$VERSION_STATUS" -ne 0 ]; then
  candidate_status="VERSION_RESOLUTION_FAILED"
  if [ "$MODE" = diagnostic ]; then candidate_status="UNGATED_VERSION_RESOLUTION_FAILED"; fi
  write_status "$candidate_status"
  sed -n '1,$p' "$VERSION_LOG" >&2
  echo "Package version resolution failed. Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

APP_PATH="$REPOSITORY_ROOT/target/universal-apple-darwin/release/bundle/macos/meetily.app"
DMG_PATH="$REPOSITORY_ROOT/target/universal-apple-darwin/release/bundle/dmg/meetily_${VERSION}_universal.dmg"

recheck_source_state
if [ "$SOURCE_STABLE" != true ]; then
  block_source_change
  exit 1
fi

echo "Verifying packaged app and DMG..."
set +e
node "$FRONTEND_DIR/scripts/verify-release-candidate.mjs" \
  --app "$APP_PATH" \
  --dmg "$DMG_PATH" \
  --commit "$COMMIT_SHA" \
  --mode "$MODE" \
  --worktree-clean "$WORKTREE_CLEAN" \
  --run-id "$RUN_ID" \
  --source-fingerprint "$SOURCE_FINGERPRINT" \
  --output "$PENDING_REPORT_PATH" > "$VERIFIER_LOG" 2>&1
VERIFIER_STATUS=$?
set -e
sed -n '1,$p' "$VERIFIER_LOG"

recheck_source_state
if [ "$SOURCE_STABLE" != true ]; then
  block_source_change
  exit 1
fi

REPORT_STATUS=
REPORT_VALID=false
if [ -s "$PENDING_REPORT_PATH" ]; then
  set +e
  REPORT_STATUS=$(node -e '
    const fs = require("node:fs");
    const report = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    if (!report || typeof report !== "object" || Array.isArray(report)
        || typeof report.status !== "string") {
      process.exit(1);
    }
    process.stdout.write(report.status);
  ' "$PENDING_REPORT_PATH" 2>> "$VERIFIER_LOG")
  report_validation_status=$?
  set -e
  if [ "$report_validation_status" -eq 0 ]; then
    REPORT_VALID=true
  fi
fi

if [ "$REPORT_VALID" != true ]; then
  rm -f "$REPORT_PATH" "$PENDING_REPORT_PATH"
  if [ "$MODE" = "diagnostic" ]; then
    write_status "UNGATED"
    echo "UNGATED diagnostic artifact. Verifier report is missing or invalid." >&2
  else
    write_status "BLOCKED_CANDIDATE_VERIFICATION"
    echo "Candidate verification failed: verifier report is missing or invalid." >&2
  fi
  echo "Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

if [ "$MODE" = "diagnostic" ]; then
  mv "$PENDING_REPORT_PATH" "$REPORT_PATH"
  write_status "UNGATED"
  echo "UNGATED diagnostic artifact. It must not be released." >&2
  echo "Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

set +e
node -e '
  const fs = require("node:fs");
  const crypto = require("node:crypto");

  const [reportPath, commitSha, mode, worktreeClean, runId, sourceFingerprint,
    dmgPath] = process.argv.slice(1);
  const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
  const hex64 = /^[a-f0-9]{64}$/i;
  const expectedWorktreeClean = worktreeClean === "true";
  const dmgSha256 = crypto.createHash("sha256")
    .update(fs.readFileSync(dmgPath))
    .digest("hex");
  const checksPass = Array.isArray(report.checks)
    && report.checks.length > 0
    && report.checks.every((check) => check?.status === "PASS");
  const bundleFingerprint = report.bundle?.fingerprint;
  const embeddedBundleFingerprint = report.artifact?.embeddedBundleFingerprint;

  const valid = report.schemaVersion === 2
    && report.status === "PASS"
    && report.commitSha === commitSha
    && report.release?.eligible === true
    && report.provenance?.mode === mode
    && report.provenance?.worktreeClean === expectedWorktreeClean
    && report.provenance?.runId === runId
    && report.provenance?.sourceFingerprint === sourceFingerprint
    && report.artifact?.path === dmgPath
    && report.artifact?.sha256 === dmgSha256
    && hex64.test(bundleFingerprint)
    && hex64.test(embeddedBundleFingerprint)
    && bundleFingerprint === embeddedBundleFingerprint
    && checksPass;

  if (!valid) process.exit(1);
' "$PENDING_REPORT_PATH" "$COMMIT_SHA" "$MODE" "$WORKTREE_CLEAN" \
  "$RUN_ID" "$SOURCE_FINGERPRINT" "$DMG_PATH" 2>> "$VERIFIER_LOG"
report_contract_status=$?
set -e

if [ "$report_contract_status" -ne 0 ]; then
  rm -f "$REPORT_PATH" "$PENDING_REPORT_PATH"
  write_status "BLOCKED_CANDIDATE_VERIFICATION"
  echo "Candidate verification failed: verifier report contract is invalid." >&2
  echo "Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

if [ "$VERIFIER_STATUS" -ne 0 ]; then
  rm -f "$REPORT_PATH" "$PENDING_REPORT_PATH"
  write_status "BLOCKED_CANDIDATE_VERIFICATION"
  echo "Candidate verification failed. Evidence: $EVIDENCE_DIR" >&2
  exit 1
fi

mv "$PENDING_REPORT_PATH" "$REPORT_PATH"
write_status "CANDIDATE_VERIFIED"
echo "Candidate verified. Manual smoke and release gate are still required."
echo "Evidence: $EVIDENCE_DIR"
