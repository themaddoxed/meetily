#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

run_fresh_clone_regression() {
  source_root=$repo_root
  source_checker=$source_root/scripts/check-diarization-plan-migration.sh
  candidate_ref=$(git symbolic-ref -q HEAD) || {
    echo "FAIL: fresh-clone regression requires a named candidate branch" >&2
    exit 1
  }
  tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/diarization-plan-check.XXXXXX")
  remote_dir=$tmp_root/remote.git
  clone_dir=$tmp_root/clone
  trap 'rm -rf "$tmp_root"' EXIT HUP INT TERM

  git init --bare "$remote_dir" >/dev/null
  git --git-dir="$remote_dir" fetch --no-tags "$source_root" \
    "$candidate_ref:refs/heads/candidate" \
    "refs/heads/codex-port-v042:refs/heads/codex-port-v042" >/dev/null
  git clone --no-local --no-tags --branch candidate "$remote_dir" "$clone_dir" >/dev/null

  ! git -C "$clone_dir" show-ref --verify --quiet refs/heads/feature/speaker-diarization || {
    echo "FAIL: fresh clone unexpectedly has a local feature source ref" >&2
    exit 1
  }
  ! git -C "$clone_dir" show-ref --verify --quiet refs/remotes/origin/feature/speaker-diarization || {
    echo "FAIL: fresh clone unexpectedly has a remote feature source ref" >&2
    exit 1
  }
  git -C "$clone_dir" show-ref --verify --quiet refs/remotes/origin/codex-port-v042 || {
    echo "FAIL: fresh clone is missing remote v042 provenance ref" >&2
    exit 1
  }

  (cd "$clone_dir" && "$source_checker")
  echo "PASS: checker works with only candidate and remote v042 refs"

  git -C "$clone_dir" config user.name "Diarization Checker Test"
  git -C "$clone_dir" config user.email "diarization-checker-test@example.invalid"
  git -C "$clone_dir" config diff.renames true
  git -C "$clone_dir" rm scripts/check-diarization-plan-migration.sh >/dev/null
  git -C "$clone_dir" mv LICENSE.md scripts/check-diarization-plan-migration.sh
  git -C "$clone_dir" commit -m "test: hide baseline deletion behind expected-path rename" >/dev/null

  if rename_output=$(cd "$clone_dir" && "$source_checker" 2>&1); then
    echo "$rename_output"
    echo "FAIL: checker accepted an out-of-scope baseline deletion hidden by rename detection" >&2
    exit 1
  fi
  echo "$rename_output" | grep -Fq 'planning wave must contain exactly four paths' || {
    echo "$rename_output" >&2
    echo "FAIL: checker rejected rename fixture for an unexpected reason" >&2
    exit 1
  }
  echo "PASS: checker rejects an out-of-scope deletion renamed into an expected path"
}

if [ "${1:-}" = "--test-fresh-clone" ]; then
  run_fresh_clone_regression
  exit 0
fi

BASE=4c0915fef8ffab1ea4a325a824f9f359c57ecf9c
V042_TIP=8dceceb0ec36f07ef1b0742cc102f9d4c0b286b9
DESIGN_SOURCE=1cd1eee84d9c8fc3d2786b337cca7741da0c9f48
DESIGN_DUPLICATE=158399268ec80153ec13b3f3ed736d79dc66aad0
PLAN_SOURCE=5e865c08e6e661fd11773d111e0b6a77966eb837
PLAN_DUPLICATE=35a0784e6059e5abf087f562644132d775403659
RETARGET_ADDENDUM=78bddd26d2fceb6f3d4130ccab357c65e26a60db
DESIGN_DESTINATION=dc17c15af8bacf7f08128df2b20a864c94521308
PLAN_DESTINATION=23a1cf42b36625157090a27883adef99939490c9
STATUS_DESTINATION=10e71318d8c51053e3d0532f078eb69cc7d424f8

EXPECTED_DESIGN_PATCH=22091788bdfab97d085fa0a4d859c8561df46e5a
EXPECTED_PLAN_PATCH=dfd51f69b3793dd2c85cb248787ec1e68b69ebf0
EXPECTED_RETARGET_PATCH=ee9752646e73973174255769a26db0a4b18ca1c7
EXPECTED_DESIGN_BLOB=c2a9092a6fa2071ddc9d18b4056e205577cca98d
EXPECTED_PLAN_BLOB=f99cea1e2d7c6bd24c957dd200b358b19bc43e4d
EXPECTED_RETARGET_BLOB=932c7ca629511d154e4c55d45f7843e10595761e

DESIGN_PATH=docs/superpowers/specs/2026-06-26-speaker-diarization-design.md
PLAN_PATH=docs/superpowers/plans/2026-06-26-speaker-diarization-foundation.md
REGISTRY_PATH=docs/migration/legacy-feature-registry.md

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  expected=$1
  actual=$2
  label=$3
  [ "$expected" = "$actual" ] || fail "$label (expected '$expected', got '$actual')"
}

stable_patch_id() {
  git show --format=email --no-ext-diff "$1" | git patch-id --stable | awk '{print $1}'
}

resolve_v042_ref() {
  for ref in refs/remotes/origin/codex-port-v042 refs/heads/codex-port-v042
  do
    if git show-ref --verify --quiet "$ref"; then
      echo "$ref"
      return 0
    fi
  done
  fail "missing v042 provenance ref (tried refs/remotes/origin/codex-port-v042 and refs/heads/codex-port-v042)"
}

V042_SOURCE_REF=$(resolve_v042_ref)

for commit in \
  "$BASE" "$V042_TIP" \
  "$DESIGN_DUPLICATE" "$PLAN_DUPLICATE" "$RETARGET_ADDENDUM" \
  "$DESIGN_DESTINATION" "$PLAN_DESTINATION" "$STATUS_DESTINATION"
do
  git cat-file -e "$commit^{commit}" || fail "missing provenance commit $commit"
done

assert_eq "$V042_TIP" "$(git rev-parse "$V042_SOURCE_REF^{commit}")" "v042 source ref changed"
assert_eq "$BASE" "$(git merge-base HEAD "$BASE")" "candidate is not descended from clean dev base"

design_duplicate_patch=$(stable_patch_id "$DESIGN_DUPLICATE")
plan_duplicate_patch=$(stable_patch_id "$PLAN_DUPLICATE")
retarget_patch=$(stable_patch_id "$RETARGET_ADDENDUM")

assert_eq "$EXPECTED_DESIGN_PATCH" "$design_duplicate_patch" "unexpected design duplicate patch id"
assert_eq "$EXPECTED_PLAN_PATCH" "$plan_duplicate_patch" "unexpected plan duplicate patch id"
assert_eq "$EXPECTED_RETARGET_PATCH" "$retarget_patch" "unexpected retarget addendum patch id"
[ "$retarget_patch" != "$plan_duplicate_patch" ] || fail "retarget addendum collapsed into duplicate plan patch"

for commit in "$DESIGN_DUPLICATE" "$PLAN_DUPLICATE" "$RETARGET_ADDENDUM"
do
  git merge-base --is-ancestor "$commit" "$V042_SOURCE_REF" || fail "v042 provenance commit is not reachable from $V042_SOURCE_REF: $commit"
done

canonical_design_proof=unavailable
if git cat-file -e "$DESIGN_SOURCE^{commit}" 2>/dev/null; then
  design_patch=$(stable_patch_id "$DESIGN_SOURCE")
  assert_eq "$EXPECTED_DESIGN_PATCH" "$design_patch" "unexpected optional canonical design patch id"
  assert_eq \
    "$(git rev-parse "$DESIGN_SOURCE:$DESIGN_PATH")" \
    "$(git rev-parse "$DESIGN_DUPLICATE:$DESIGN_PATH")" \
    "optional canonical and duplicate design blobs differ"
  canonical_design_proof=$DESIGN_SOURCE
fi

canonical_plan_proof=unavailable
if git cat-file -e "$PLAN_SOURCE^{commit}" 2>/dev/null; then
  plan_patch=$(stable_patch_id "$PLAN_SOURCE")
  assert_eq "$EXPECTED_PLAN_PATCH" "$plan_patch" "unexpected optional canonical plan patch id"
  assert_eq \
    "$(git rev-parse "$PLAN_SOURCE:$PLAN_PATH")" \
    "$(git rev-parse "$PLAN_DUPLICATE:$PLAN_PATH")" \
    "optional canonical and duplicate plan blobs differ"
  canonical_plan_proof=$PLAN_SOURCE
fi

assert_eq \
  "$EXPECTED_DESIGN_BLOB" \
  "$(git rev-parse "$DESIGN_DUPLICATE:$DESIGN_PATH")" \
  "unexpected design duplicate blob"
assert_eq \
  "$EXPECTED_DESIGN_BLOB" \
  "$(git rev-parse "$DESIGN_DESTINATION:$DESIGN_PATH")" \
  "canonical design blob was not preserved"
assert_eq \
  "$EXPECTED_PLAN_BLOB" \
  "$(git rev-parse "$PLAN_DUPLICATE:$PLAN_PATH")" \
  "unexpected plan duplicate blob"
assert_eq \
  "$EXPECTED_PLAN_BLOB" \
  "$(git rev-parse "$PLAN_DESTINATION:$PLAN_PATH")" \
  "canonical plan blob was not preserved"
assert_eq \
  "$EXPECTED_RETARGET_BLOB" \
  "$(git rev-parse "$RETARGET_ADDENDUM:$PLAN_PATH")" \
  "unexpected retarget addendum blob"

retarget_reproduction=$(
  git show "$PLAN_DUPLICATE:$PLAN_PATH" |
    sed 's#\.worktrees/speaker-diarization#.worktrees/codex-port-v042#; s#^feature/speaker-diarization$#codex-port-v042#' |
    git hash-object --stdin
)
assert_eq \
  "$(git rev-parse "$RETARGET_ADDENDUM:$PLAN_PATH")" \
  "$retarget_reproduction" \
  "v042 retarget disposition is not the historical two-line addendum"

inventory=$(
  git ls-files |
    grep -E '(^|/)(2026-06-26-speaker-diarization-design\.md|2026-06-26-speaker-diarization-foundation\.md)$' |
    LC_ALL=C sort || true
)
expected_inventory=$(printf '%s\n%s\n' "$PLAN_PATH" "$DESIGN_PATH" | LC_ALL=C sort)
assert_eq "$expected_inventory" "$inventory" "canonical document inventory contains a duplicate or omission"

grep -Fq '**Current migration status (2026-07-13):**' "$DESIGN_PATH" || fail "design current-status banner missing"
grep -Fq '**Current migration status (2026-07-13):**' "$PLAN_PATH" || fail "plan current-status banner missing"
grep -Fq 'Candidate history contains no patch-equivalent copy of any of the 15 selected implementation commits from the `codex-port-v042` chain' "$PLAN_PATH" || fail "patch-equivalence status wording missing"
grep -Fq 'v0.4.2 retarget addendum: `78bddd2`' "$PLAN_PATH" || fail "v042 addendum provenance missing"
grep -Fq '/Users/developer/Desktop/meeting-minutes/.worktrees/codex-port-v042' "$PLAN_PATH" || fail "v042 worktree disposition missing"

implementation_commits='aa5e006e3c51d8b2ff1c4dce278c36dda4a9a5d6
d7b0c1c29a940069ba88cd449bf2423566eb6526
49c3ab5a880222b16ead5ecdecf88f3f4e0d1dc6
c67308e1d1181fec9007f74e3b2829520f57de99
a5a699b73ed3372463aaa88a1e2aadc9f149120f
b971a584e3586b8479f26a2c762e875a9daf33f1
7222852a31b0f6a83236f8d7f33d44a4c5f145e6
dea457848ee05e3cd7c5642aac8571303ca2c886
29b24fc4c61960a76996e28e3a49997490636804
28e1bb037ed4f5d6d37b89ca908e6f92a393f2c6
d2db258000d71a82b3a79c9096c2161023fa633b
2576ba81dc98c4aea77c49b311e7158c973f205b
4c0248982a42935b87ebda92a2ab8896c87c3c6f
6fe2a10628fa651fc73fe6a50f72bd4241f4f7da
0160d8470396eeeebd7ff97b0966d8d0227e4785'
cherry_inventory=$(git cherry HEAD "$V042_SOURCE_REF")
echo "$implementation_commits" | while IFS= read -r commit
do
  git cat-file -e "$commit^{commit}" || fail "missing implementation source commit $commit"
  git merge-base --is-ancestor "$commit" "$V042_SOURCE_REF" || fail "selected implementation commit is not reachable from $V042_SOURCE_REF: $commit"
  echo "$cherry_inventory" | grep -Fq "+ $commit" || fail "candidate HEAD contains an equivalent of selected v042 implementation patch: $commit"
done

planning_rows=$(grep -c '^| Speaker diarization design and migration plans |' "$REGISTRY_PATH")
implementation_rows=$(grep -c '^| Speaker diarization and speaker rename chain |' "$REGISTRY_PATH")
assert_eq 1 "$planning_rows" "planning registry row count"
assert_eq 1 "$implementation_rows" "implementation registry row count"

planning_status=$(awk -F '|' '/^\| Speaker diarization design and migration plans / {gsub(/^ +| +$/, "", $8); gsub(/`/, "", $8); print $8}' "$REGISTRY_PATH")
implementation_status=$(awk -F '|' '/^\| Speaker diarization and speaker rename chain / {gsub(/^ +| +$/, "", $8); gsub(/`/, "", $8); print $8}' "$REGISTRY_PATH")
assert_eq verified "$planning_status" "planning registry status"
assert_eq inventoried "$implementation_status" "implementation registry status"

changed_inventory=$(git diff --no-renames --name-only "$BASE"...HEAD | LC_ALL=C sort)
expected_changed_inventory=$(
  printf '%s\n%s\n%s\n%s\n' \
    "$REGISTRY_PATH" \
    "$PLAN_PATH" \
    "$DESIGN_PATH" \
    scripts/check-diarization-plan-migration.sh |
    LC_ALL=C sort
)
assert_eq "$expected_changed_inventory" "$changed_inventory" "planning wave must contain exactly four paths"

echo "PASS: diarization planning documents are canonical and provenance-verified"
echo "v042 source: $V042_SOURCE_REF at $V042_TIP"
echo "design patch: $design_duplicate_patch (duplicate $DESIGN_DUPLICATE; optional canonical $canonical_design_proof)"
echo "plan patch: $plan_duplicate_patch (duplicate $PLAN_DUPLICATE; optional canonical $canonical_plan_proof)"
echo "retarget patch: $retarget_patch ($RETARGET_ADDENDUM, distinct and incorporated)"
echo "documents: $DESIGN_PATH; $PLAN_PATH"
echo "implementation: candidate HEAD has no patch-equivalent copy of any of the 15 selected v042 implementation commits"
