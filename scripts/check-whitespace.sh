#!/bin/sh
set -eu

# Check uncommitted and staged edits first. A clean checkout makes both commands
# cheap, while local users still get an immediate, precise failure.
git diff --check
git diff --cached --check

BASE_REF=${MEETILY_CHECK_BASE_REF:-}

# GitHub uses an all-zero before SHA for a repository's first push and for a
# newly created branch. There is no base commit in either case: inspect every
# committed file rather than skipping the gate or assuming HEAD has a parent.
check_whole_tree() {
  EMPTY_TREE=$(git hash-object -t tree --stdin </dev/null)
  echo "Checking whole committed tree against the empty tree"
  git diff --check "$EMPTY_TREE" HEAD
}

if printf '%s' "$BASE_REF" | grep -Eq '^0{40}(0{24})?$'; then
  check_whole_tree
  exit 0
fi

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
    if git rev-parse --verify 'HEAD^' >/dev/null 2>&1; then
      BASE_REF=HEAD^
    else
      check_whole_tree
      exit 0
    fi
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
