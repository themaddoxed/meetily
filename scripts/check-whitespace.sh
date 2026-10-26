#!/bin/sh
set -eu

# Check uncommitted and staged edits first. A clean checkout makes both commands
# cheap, while local users still get an immediate, precise failure.
git diff --check
git diff --cached --check

BASE_REF=${MEETILY_CHECK_BASE_REF:-}

if [ -z "$BASE_REF" ]; then
  CURRENT_BRANCH=$(git symbolic-ref --quiet --short HEAD 2>/dev/null || true)

  if [ "$CURRENT_BRANCH" != "dev" ] && git rev-parse --verify 'dev^{commit}' >/dev/null 2>&1; then
    BASE_REF=dev
  elif [ "$CURRENT_BRANCH" = "dev" ] && git rev-parse --verify 'origin/dev^{commit}' >/dev/null 2>&1; then
    BASE_REF=origin/dev
  elif [ "$CURRENT_BRANCH" != "main" ] && git rev-parse --verify 'main^{commit}' >/dev/null 2>&1; then
    BASE_REF=main
  elif [ "$CURRENT_BRANCH" = "main" ] && git rev-parse --verify 'origin/main^{commit}' >/dev/null 2>&1; then
    BASE_REF=origin/main
  elif git rev-parse --verify 'HEAD^{commit}' >/dev/null 2>&1; then
    BASE_REF=HEAD^
  fi
fi

if [ -z "$BASE_REF" ]; then
  echo "Unable to determine a base commit for committed whitespace checks." >&2
  echo "Set MEETILY_CHECK_BASE_REF to the target branch or base commit." >&2
  exit 2
fi

if ! git rev-parse --verify "$BASE_REF^{commit}" >/dev/null 2>&1; then
  echo "Whitespace-check base does not exist: $BASE_REF" >&2
  exit 2
fi

echo "Checking committed whitespace against $BASE_REF...HEAD"
git diff --check "$BASE_REF...HEAD"
