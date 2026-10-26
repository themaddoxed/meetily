#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P) ||
  fail "unable to resolve checker directory"
repo_root=$(git -C "$script_dir/.." rev-parse --show-toplevel 2>/dev/null) ||
  fail "checker is not inside a Git worktree or clone"
cd "$repo_root"

BASE=a75082bbae99bf9220cd02e36c3108d9f8a108bb
CHECKPOINT=90bcbf867c299c72e5d9bca034eb4d8d9014186c
DECISION_PATH=docs/migration/diarization-backend-foundation.md
PLAN_PATH=docs/superpowers/plans/2026-07-13-hidden-diarization-backend-foundation.md
REGISTRY_PATH=docs/migration/legacy-feature-registry.md

REGRESSION_TEMP_PATHS=()
CREATED_TEMP_ROOT=

cleanup_regression_temp_paths() {
  local path

  for path in "${REGRESSION_TEMP_PATHS[@]}"; do
    if ! rm -rf -- "$path"; then
      printf 'WARN: unable to remove regression temporary path: %s\n' "$path" >&2
    fi
  done
}

install_cleanup_traps() {
  trap cleanup_regression_temp_paths EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT
  trap 'exit 143' TERM
}

create_regression_temp_root() {
  local label=$1

  CREATED_TEMP_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/$label.XXXXXX") ||
    fail "$label could not create a temporary directory"
  REGRESSION_TEMP_PATHS+=("$CREATED_TEMP_ROOT")
}

run_cleanup_probe() {
  install_cleanup_traps
  create_regression_temp_root diarization-backend-cleanup
  printf '%s\n' "$CREATED_TEMP_ROOT" || fail "cleanup probe could not report its temporary path"
}

run_regression_fixtures() {
  local source_root=$repo_root
  local source_checker=$source_root/scripts/check-diarization-backend-migration.sh
  local source_head
  local tmp_root
  local fixture_path
  local failures=0

  source_head=$(git rev-parse --verify 'HEAD^{commit}') ||
    fail "self-test could not resolve the current HEAD commit"
  install_cleanup_traps
  create_regression_temp_root diarization-backend-check
  tmp_root=$CREATED_TEMP_ROOT

  make_fixture() {
    local name=$1
    fixture_path=$tmp_root/$name

    git init -q "$fixture_path" || fail "fixture $name: git init failed"
    git -C "$fixture_path" fetch --no-tags "$source_root" "$source_head" >/dev/null 2>&1 ||
      fail "fixture $name: fetch of current HEAD $source_head failed"
    git -C "$fixture_path" checkout --detach FETCH_HEAD >/dev/null 2>&1 ||
      fail "fixture $name: checkout of current HEAD $source_head failed"
    cp "$source_checker" "$fixture_path/scripts/check-diarization-backend-migration.sh" ||
      fail "fixture $name: checker copy failed"
    cp "$source_root/$DECISION_PATH" "$fixture_path/$DECISION_PATH" ||
      fail "fixture $name: decision record copy failed"
    cp "$source_root/$REGISTRY_PATH" "$fixture_path/$REGISTRY_PATH" ||
      fail "fixture $name: registry copy failed"
    git -C "$fixture_path" config user.name "Diarization Scope Guard Test" ||
      fail "fixture $name: test user name setup failed"
    git -C "$fixture_path" config user.email "diarization-scope-guard@example.invalid" ||
      fail "fixture $name: test user email setup failed"
  }

  expect_rejected() {
    local fixture=$1
    local label=$2
    local expected=$3
    local output

    if output=$(cd "$fixture" && scripts/check-diarization-backend-migration.sh 2>&1); then
      printf 'FAIL: checker accepted %s\n' "$label" >&2
      failures=$((failures + 1))
    elif ! grep -Fq -- "$expected" <<<"$output"; then
      printf 'FAIL: checker rejected %s for an unexpected reason\n%s\n' "$label" "$output" >&2
      failures=$((failures + 1))
    else
      printf 'PASS: checker rejects %s\n' "$label"
    fi
  }

  append_visible_fixture() {
    local fixture=$1
    printf '\n// Hidden diarization scope regression fixture.\n' >> \
      "$fixture/frontend/src/components/BetaSettings.tsx" ||
      fail "visible-path fixture mutation failed"
  }

  insert_policy_contradiction() {
    local fixture=$1
    local path=$fixture/$DECISION_PATH

    awk '
      /^## Selected backend source inventory$/ {
        print "| Engine and visibility | A visible diarization engine is shipped. |"
        print ""
      }
      { print }
    ' "$path" >"$path.tmp" || fail "policy contradiction fixture generation failed"
    mv "$path.tmp" "$path" || fail "policy contradiction fixture replacement failed"
  }

  insert_selected_inventory_contradiction() {
    local fixture=$1
    local path=$fixture/$DECISION_PATH

    awk '
      /^## Excluded and deferred source inventory$/ {
        print "Selected scope also includes visible frontend settings."
        print ""
      }
      { print }
    ' "$path" >"$path.tmp" || fail "selected-inventory contradiction fixture generation failed"
    mv "$path.tmp" "$path" || fail "selected-inventory contradiction fixture replacement failed"
  }

  insert_excluded_inventory_contradiction() {
    local fixture=$1
    local path=$fixture/$DECISION_PATH

    awk '
      /^## Verified backend checkpoint$/ {
        print "Excluded scope also permits speaker rename in this backend wave."
        print ""
      }
      { print }
    ' "$path" >"$path.tmp" || fail "excluded-inventory contradiction fixture generation failed"
    mv "$path.tmp" "$path" || fail "excluded-inventory contradiction fixture replacement failed"
  }

  replace_checkpoint_tip() {
    local fixture=$1
    local path=$fixture/$DECISION_PATH

    sed 's/90bcbf867c299c72e5d9bca034eb4d8d9014186c/0000000000000000000000000000000000000000/' \
      "$path" >"$path.tmp" || fail "checkpoint tip fixture generation failed"
    mv "$path.tmp" "$path" || fail "checkpoint tip fixture replacement failed"
  }

  replace_registry_destination() {
    local fixture=$1
    local path=$fixture/$REGISTRY_PATH

    awk 'BEGIN { FS = OFS = "|" }
      /^\| Speaker diarization and speaker rename chain / {
        $9 = " backend shipped to users; contract: docs/migration/diarization-backend-foundation.md "
      }
      { print }
    ' "$path" >"$path.tmp" || fail "registry shipped-claim fixture generation failed"
    mv "$path.tmp" "$path" || fail "registry shipped-claim fixture replacement failed"
  }

  local fixture
  local cleanup_path
  local probe_cwd=$tmp_root/cleanup-probe-cwd
  local weird_tmp_parent="$tmp_root/tmp dir with ' ; touch $probe_cwd/CLEANUP_PATH_EXECUTED ; # [glob]*?"
  local probe_record=$tmp_root/cleanup-probe-path
  local probe_error=$tmp_root/cleanup-probe-error

  mkdir -p "$weird_tmp_parent" "$probe_cwd" || fail "hostile TMPDIR fixture setup failed"
  if (
    cd "$probe_cwd" &&
      TMPDIR="$weird_tmp_parent" "$source_checker" --cleanup-probe \
        >"$probe_record" 2>"$probe_error"
  ); then
    if IFS= read -r cleanup_path <"$probe_record"; then
      if [ -e "$probe_cwd/CLEANUP_PATH_EXECUTED" ]; then
        printf 'FAIL: cleanup trap executed temporary path content\n' >&2
        failures=$((failures + 1))
      elif [ -e "$cleanup_path" ]; then
        printf 'FAIL: cleanup trap left its hostile temporary path behind\n' >&2
        failures=$((failures + 1))
      else
        printf 'PASS: cleanup trap treats hostile temporary paths as inert data\n'
      fi
    else
      printf 'FAIL: cleanup probe did not report its temporary path\n' >&2
      failures=$((failures + 1))
    fi
  else
    printf 'FAIL: cleanup probe failed under a hostile TMPDIR\n' >&2
    sed 's/^/  /' "$probe_error" >&2 || fail "cleanup probe error rendering failed"
    failures=$((failures + 1))
  fi

  make_fixture ref-shadow
  fixture=$fixture_path
  append_visible_fixture "$fixture"
  git -C "$fixture" add frontend/src/components/BetaSettings.tsx ||
    fail "ref-shadow fixture: staging failed"
  git -C "$fixture" commit -m "test: committed visible path" >/dev/null ||
    fail "ref-shadow fixture: commit failed"
  git -C "$fixture" tag -f a75082b HEAD || fail "ref-shadow fixture: tag creation failed"
  expect_rejected "$fixture" "an abbreviated-base ref shadow" \
    "visible frontend path has committed changes relative to"

  make_fixture staged-visible
  fixture=$fixture_path
  append_visible_fixture "$fixture"
  git -C "$fixture" add frontend/src/components/BetaSettings.tsx ||
    fail "staged visible-path fixture: staging failed"
  expect_rejected "$fixture" "a staged visible-path change" \
    "visible frontend path has staged changes"

  make_fixture dirty-visible
  fixture=$fixture_path
  append_visible_fixture "$fixture"
  expect_rejected "$fixture" "an unstaged visible-path change" \
    "visible frontend path has working-tree changes"

  make_fixture policy-contradiction
  fixture=$fixture_path
  insert_policy_contradiction "$fixture"
  expect_rejected "$fixture" "a contradictory protected policy" \
    "fixed backend policy section must match the canonical table"

  make_fixture selected-inventory-contradiction
  fixture=$fixture_path
  insert_selected_inventory_contradiction "$fixture"
  expect_rejected "$fixture" "contradictory selected-inventory prose" \
    "selected backend source inventory section must match the canonical block"

  make_fixture excluded-inventory-contradiction
  fixture=$fixture_path
  insert_excluded_inventory_contradiction "$fixture"
  expect_rejected "$fixture" "contradictory excluded-inventory prose" \
    "excluded and deferred source inventory section must match the canonical block"

  make_fixture checkpoint-tip
  fixture=$fixture_path
  replace_checkpoint_tip "$fixture"
  expect_rejected "$fixture" "a falsified backend checkpoint tip" \
    "verified backend checkpoint section must match the canonical block"

  make_fixture shipped-claim
  fixture=$fixture_path
  replace_registry_destination "$fixture"
  expect_rejected "$fixture" "a shipped registry claim" \
    "diarization implementation registry destination must match the canonical hidden backend boundary"

  [ "$failures" -eq 0 ] || fail "$failures diarization backend regression fixture(s) were accepted"
  echo "PASS: diarization backend checker regression fixtures"
}

if [ "${1:-}" = "--self-test" ]; then
  run_regression_fixtures
  exit 0
fi

if [ "${1:-}" = "--cleanup-probe" ]; then
  run_cleanup_probe
  exit 0
fi

[ "$#" -eq 0 ] || fail "usage: $0 [--self-test|--cleanup-probe]"

base_type=$(git cat-file -t "$BASE" 2>/dev/null) || fail "missing clean diarization planning base $BASE"
[ "$base_type" = commit ] || fail "clean diarization planning base is not a commit: $BASE"
resolved_base=$(git rev-parse --verify "$BASE^{commit}")
[ "$resolved_base" = "$BASE" ] || fail "clean diarization planning base resolved unexpectedly: $resolved_base"
git merge-base --is-ancestor "$BASE" HEAD || fail "HEAD is not descended from $BASE"

checkpoint_type=$(git cat-file -t "$CHECKPOINT" 2>/dev/null) ||
  fail "missing verified diarization backend checkpoint $CHECKPOINT"
[ "$checkpoint_type" = commit ] ||
  fail "verified diarization backend checkpoint is not a commit: $CHECKPOINT"
resolved_checkpoint=$(git rev-parse --verify "$CHECKPOINT^{commit}")
[ "$resolved_checkpoint" = "$CHECKPOINT" ] ||
  fail "verified diarization backend checkpoint resolved unexpectedly: $resolved_checkpoint"
git merge-base --is-ancestor "$CHECKPOINT" HEAD ||
  fail "HEAD is not descended from verified diarization backend checkpoint $CHECKPOINT"

[ -f "$DECISION_PATH" ] || fail "missing hidden backend decision record"
[ -f "$PLAN_PATH" ] || fail "missing hidden diarization backend implementation plan"
[ -f "$REGISTRY_PATH" ] || fail "missing legacy feature registry"

expected_policy_section=$(cat <<'EOF'
## Fixed policies

| Policy | Fixed decision |
|---|---|
| Engine and visibility | This foundation adds no diarization engine or producer. The backend remains hidden, with no visible UI. |
| Settings authority | SQLite database settings are the runtime authority. |
| Apply gate | Apply requires `enabled=true` and a non-`off` persisted mode compatible with the explicit run kind. |
| Pre-mutation rejection | Reject empty requests, cross-meeting segments, malformed segments, nonexistent meetings, and requests with no alignable transcript before any mutation. |
| No implicit clear | An empty request never clears prior diarization data; there is no implicit clear command. |
| Generic overlap | Overlap is represented as `Multiple speakers` with `is_overlap=true` and no participating speaker identities. |
| Explicit cleanup | Meeting deletion and retranscription explicitly remove diarization rows and transcript diarization columns, independently of SQLite foreign-key enforcement. |
| Speaker cardinality | There is no speaker-count cap. |
| Command errors | Webview command errors are stable and sanitized; internal SQL details are logged locally and are never returned. |
| Frontend deferral | Frontend settings, transcript presentation, labels, copy, and rename are deferred to `2026-07-13-hidden-diarization-frontend-handoff.md`. |
EOF
)
policy_headings=$(grep -c '^## Fixed policies$' "$DECISION_PATH" || true)
[ "$policy_headings" -eq 1 ] || fail "decision record must contain exactly one fixed backend policy section"
policy_section=$(
  awk '
    /^## Fixed policies$/ { inside = 1 }
    inside && /^## / && $0 != "## Fixed policies" { exit }
    inside { print }
  ' "$DECISION_PATH"
)
[ "$policy_section" = "$expected_policy_section" ] ||
  fail "fixed backend policy section must match the canonical table"

expected_selected_section=$(cat <<'EOF'
## Selected backend source inventory

Only these bounded source slices are selected for the hidden backend migration, in this order.

| Source slice | Disposition | Backend scope |
|---|---|---|
| `aa5e006` | selected | diarization domain types |
| `d7b0c1c` code only | selected | domain constructor API |
| `49c3ab5` | selected | deterministic transcript alignment |
| `c67308e` code only | selected | alignment threshold hardening |
| `a5a699b` semantic slice | selected | persistence models, DTOs, and repository fields |
| `b971a58` test only | selected | import fixture coverage |
| `7222852` | selected | database-authoritative settings repository behavior |
| `4c02489` | selected | validated atomic apply |
| `6fe2a10` backend slice | selected | backend command and application handoff |

No whole source commit is assumed safe merely because one of its slices is selected. Code-only, semantic, test-only, backend-only, and frontend-only boundaries are part of the inventory.
EOF
)
selected_headings=$(grep -c '^## Selected backend source inventory$' "$DECISION_PATH" || true)
[ "$selected_headings" -eq 1 ] ||
  fail "decision record must contain exactly one selected backend source inventory section"
selected_section=$(
  awk '
    /^## Selected backend source inventory$/ { inside = 1 }
    inside && /^## / && $0 != "## Selected backend source inventory" { exit }
    inside { print }
  ' "$DECISION_PATH"
)
[ "$selected_section" = "$expected_selected_section" ] ||
  fail "selected backend source inventory section must match the canonical block"

expected_excluded_section=$(cat <<'EOF'
## Excluded and deferred source inventory

These sources and slices are outside this backend wave. The frontend and rename slices remain assigned to `2026-07-13-hidden-diarization-frontend-handoff.md`.

| Source or slice | Disposition | Reason or later scope |
|---|---|---|
| docs-only staging commits | excluded | documentation staging is not backend implementation |
| duplicate theme/Gemma commits | excluded | already migrated through their own registry rows |
| `c0201eb` | excluded | duplicate audio-verification patch |
| `8dceceb` | excluded | merge commit |
| `dea4578` | deferred | frontend settings |
| `29b24fc` | deferred | frontend settings and presentation |
| `28e1bb0` | deferred | frontend transcript presentation |
| `d2db258` | deferred | frontend labels and copy |
| `2576ba8` | deferred | frontend presentation follow-up |
| `6fe2a10` frontend slice | deferred | frontend application handoff |
| `0160d84` | deferred | speaker rename |
EOF
)
excluded_headings=$(grep -c '^## Excluded and deferred source inventory$' "$DECISION_PATH" || true)
[ "$excluded_headings" -eq 1 ] ||
  fail "decision record must contain exactly one excluded and deferred source inventory section"
excluded_section=$(
  awk '
    /^## Excluded and deferred source inventory$/ { inside = 1 }
    inside && /^## / && $0 != "## Excluded and deferred source inventory" { exit }
    inside { print }
  ' "$DECISION_PATH"
)
[ "$excluded_section" = "$expected_excluded_section" ] ||
  fail "excluded and deferred source inventory section must match the canonical block"

expected_checkpoint_section=$(cat <<'EOF'
## Verified backend checkpoint

The hidden backend-only checkpoint is verified at code tip `90bcbf867c299c72e5d9bca034eb4d8d9014186c`. The combined implementation registry row remains `migrating`: no diarization engine or producer exists, no visible UI is enabled, and the deferred frontend and rename work is not claimed complete.

| Selected source slice | Verified destination |
|---|---|
| `aa5e006` | domain types in `f18ff4d` |
| `d7b0c1c` code only | constructor API in `f18ff4d` |
| `49c3ab5` | deterministic alignment in `f18ff4d`, overlap-precedence correction in `7db0d53` |
| `c67308e` code only | alignment threshold hardening in `f18ff4d` |
| `a5a699b` semantic slice | persistence in `12a1bea`, immutable migration compatibility in `4481511`, sanitized persistence errors in `17f6d9a` |
| `b971a58` test only | import fixture coverage in `12a1bea` |
| `7222852` | typed database settings in `146b64f`, strict persisted-boolean validation in `d1a6043` |
| `4c02489` | validated atomic apply in `27f8c3e`, transactional settings recheck in `90bcbf8` |
| `6fe2a10` backend slice | backend handoff in `27f8c3e`, transactional settings recheck in `90bcbf8` |

Verification evidence at the checkpoint:

- Focused `rustfmt --edition 2021 --check` passed for the changed repository, command, and integration-test files; `git diff --check` passed for the complete wave, including the one-line handler registration.
- `cargo check -p meetily --tests` passed offline with the local Tauri test configuration.
- `cargo test -p meetily` passed: 241 library tests, 11 atomic-apply integration tests, 9 migration/cleanup integration tests, and 2 doctests; 3 hardware-dependent tests were ignored and no test failed.
- `scripts/check-diarization-backend-migration.sh --self-test`, the normal scope guard, and the exact visible-path audit passed.
- Repo-wide `cargo fmt --all --check` is not claimed green: it fails identically on clean `dev` at `a75082b` because of pre-existing formatting drift in more than one hundred unrelated Rust files. This backend wave does not absorb that legacy cleanup.
- No GitHub, network publication, or user-visible feature activation occurred.

Deferred slices remain exactly `dea4578`, `29b24fc`, `28e1bb0`, `d2db258`, `2576ba8`, the frontend portion of `6fe2a10`, and `0160d84`, all assigned to `2026-07-13-hidden-diarization-frontend-handoff.md`.
EOF
)
checkpoint_headings=$(grep -c '^## Verified backend checkpoint$' "$DECISION_PATH" || true)
[ "$checkpoint_headings" -eq 1 ] ||
  fail "decision record must contain exactly one verified backend checkpoint section"
checkpoint_section=$(
  awk '
    /^## Verified backend checkpoint$/ { inside = 1 }
    inside && /^## / && $0 != "## Verified backend checkpoint" { exit }
    inside { print }
  ' "$DECISION_PATH"
)
[ "$checkpoint_section" = "$expected_checkpoint_section" ] ||
  fail "verified backend checkpoint section must match the canonical block"

planning_rows=$(grep -c '^| Speaker diarization design and migration plans |' "$REGISTRY_PATH")
implementation_rows=$(grep -c '^| Speaker diarization and speaker rename chain |' "$REGISTRY_PATH")
[ "$planning_rows" -eq 1 ] || fail "expected exactly one diarization planning registry row"
[ "$implementation_rows" -eq 1 ] || fail "expected exactly one diarization implementation registry row"

planning_status=$(
  awk -F '|' '/^\| Speaker diarization design and migration plans / {
    gsub(/^ +| +$/, "", $8); gsub(/`/, "", $8); print $8
  }' "$REGISTRY_PATH"
)
implementation_status=$(
  awk -F '|' '/^\| Speaker diarization and speaker rename chain / {
    gsub(/^ +| +$/, "", $8); gsub(/`/, "", $8); print $8
  }' "$REGISTRY_PATH"
)
implementation_destination=$(
  awk -F '|' '/^\| Speaker diarization and speaker rename chain / {
    gsub(/^ +| +$/, "", $9); print $9
  }' "$REGISTRY_PATH"
)
expected_implementation_destination='backend-only checkpoint verified at `90bcbf867c299c72e5d9bca034eb4d8d9014186c`; ledger and evidence: `docs/migration/diarization-backend-foundation.md`; included wave: domain, alignment, persistence, settings, and atomic apply; boundary: no engine, producer, or visible UI, so the combined chain remains migrating; deferred handoff: `dea4578`, `29b24fc`, `28e1bb0`, `d2db258`, `2576ba8`, frontend `6fe2a10`, and `0160d84` in `2026-07-13-hidden-diarization-frontend-handoff.md`'

[ "$planning_status" = verified ] || fail "diarization planning registry row must stay verified"
[ "$implementation_status" = migrating ] || fail "diarization implementation registry row must be migrating"
[ "$implementation_destination" = "$expected_implementation_destination" ] ||
  fail "diarization implementation registry destination must match the canonical hidden backend boundary"

VISIBLE_PATHS=(
  'frontend/src/components/BetaSettings.tsx'
  'frontend/src/components/TranscriptSettings.tsx'
  'frontend/src/contexts/ConfigContext.tsx'
  'frontend/src/types/betaFeatures.ts'
)

for path in "${VISIBLE_PATHS[@]}"; do
  git diff --quiet --no-renames "$BASE"...HEAD -- "$path" ||
    fail "visible frontend path has committed changes relative to $BASE: $path"
  git diff --quiet --no-renames --cached -- "$path" ||
    fail "visible frontend path has staged changes: $path"
  git diff --quiet --no-renames -- "$path" ||
    fail "visible frontend path has working-tree changes: $path"
done

echo "PASS: hidden diarization backend scope is preserved"
