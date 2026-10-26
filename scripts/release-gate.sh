#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

usage() {
  cat <<'EOF'
Usage: ./scripts/release-gate.sh --smoke-report <markdown> [--candidate-report <json>]

The candidate report defaults to:
  artifacts/release-gate/<current-commit>/candidate-report.json

The gate requires a PASS candidate-report.json, recomputes the artifact SHA-256
and current source fingerprint, and requires PASS plus concrete evidence for
every manual smoke item.
EOF
}

SMOKE_REPORT=""
CANDIDATE_REPORT=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --smoke-report)
      [ "$#" -ge 2 ] || { echo "Missing value for --smoke-report" >&2; exit 2; }
      SMOKE_REPORT=$2
      shift 2
      ;;
    --candidate-report)
      [ "$#" -ge 2 ] || { echo "Missing value for --candidate-report" >&2; exit 2; }
      CANDIDATE_REPORT=$2
      shift 2
      ;;
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
done

if [ -z "$SMOKE_REPORT" ]; then
  echo "--smoke-report is required." >&2
  usage >&2
  exit 2
fi

cd "$REPOSITORY_ROOT"
COMMIT_SHA=$(git rev-parse HEAD)
SOURCE_FINGERPRINT=$(node "$REPOSITORY_ROOT/frontend/scripts/source-fingerprint.mjs" \
  --repository "$REPOSITORY_ROOT")
if [ -z "$CANDIDATE_REPORT" ]; then
  CANDIDATE_REPORT="$REPOSITORY_ROOT/artifacts/release-gate/$COMMIT_SHA/candidate-report.json"
fi

exec node "$REPOSITORY_ROOT/frontend/scripts/verify-smoke-report.mjs" \
  --smoke "$SMOKE_REPORT" \
  --candidate "$CANDIDATE_REPORT" \
  --commit "$COMMIT_SHA" \
  --source-fingerprint "$SOURCE_FINGERPRINT"
