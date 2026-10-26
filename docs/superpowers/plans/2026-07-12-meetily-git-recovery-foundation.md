> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Meetily Git Recovery Foundation Implementation Plan

> [!WARNING]
> **Current execution status — 2026-07-12:** This document is a historical execution reference, not a live runbook. Tasks 1–7 were executed with deviations and evidence recorded outside the repository in `/Users/developer/Desktop/meetily-recovery-2026-07-12/RECOVERY.md` and the evidence files it names. That external `RECOVERY.md` is authoritative for runtime state, completed actions, hashes, failures, resumptions, and verification results. Do not rerun the commands below blindly against the live repository. Re-audit current refs, remotes, worktrees, and evidence before adapting any command. Task 8 and the final foundation audit remain pending until foundation hardening is complete. The unchecked boxes below preserve the original plan and do not assert that an individual step was proved exactly as written.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve every existing Meetily feature and artifact, freeze the verified Universal build as the product baseline, and establish clean `main`, `dev`, migration, and upstream-contribution lanes without mass-merging legacy branches.

**Architecture:** This plan builds only the recovery foundation: redundant backups, immutable archive refs, a quarantined salvage snapshot, conventional remotes, a clean product `dev`, and a versioned migration registry. Feature transfer is intentionally excluded; after this plan, each registry row receives its own migration plan and only one `migrate/*` branch may be active.

**Tech Stack:** Git worktrees and refs, Git bundles and binary patches, GitHub remotes, POSIX shell on macOS, Markdown, pnpm, Rust/Cargo, Tauri.

---

## Historical Scope and Audited Baseline

The recovery execution targeted `/Users/developer/Desktop/meeting-minutes` and was designed to preserve all existing worktrees. The plan excluded branch deletion, worktree removal, remote-history rewriting, and wholesale merges of legacy branch heads.

Reference points captured when the plan was written:

- legacy local `main`: `0b5aef606090221ffb8c343a40d34ace9615dc34`;
- canonical upstream `devtest`: `0bedd8b7d805fc924fb706380c829cb1b48d4caf` at the 2026-07-12 audit;
- verified Universal baseline: `8ffc5e6de85d494ecf25fd34afb40b7a67c432cf`;
- root overlay: 167 tracked files changed plus the listed untracked design, test, configuration, and runtime files;
- six existing feature/build worktrees were clean at audit time;
- `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer cargo test -p meetily --no-run` on legacy `main` fails after sidecars are supplied, with 15 missing Tauri command symbols in `frontend/src-tauri/src/lib.rs`. This is a pre-existing red baseline, not a recovery regression.

The execution reserved `/Users/developer/Desktop/meetily-recovery-2026-07-12` for local recovery artifacts. That directory remains outside Git and must never be pushed. Consult its `RECOVERY.md` before relying on any planned expectation below.

### Planned files created or modified

- External create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/repository-before-recovery.bundle` — all committed refs before restructuring.
- External create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/root-tracked.patch` — binary-safe tracked root overlay.
- External create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked.tar.gz` — ignored-safe archive of current untracked files.
- External create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/*.txt` — refs, worktrees, remotes, hashes, and verification evidence.
- Create on new `dev`: `docs/migration/legacy-feature-registry.md` — authoritative inventory and migration status.
- Create on new `dev`: `docs/development/git-workflow.md` — daily branch rules and verification commands.
- Reuse on new `dev`: `docs/superpowers/specs/2026-07-12-meetily-dual-track-git-workflow-design.md`.
- Reuse on new `dev`: `docs/superpowers/plans/2026-07-12-meetily-git-recovery-foundation.md`.

## Task 1: Create a recovery pack before changing refs

**Files:**
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/repository-before-recovery.bundle`
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/root-tracked.patch`
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked.tar.gz`
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/pre-recovery-*.txt`
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/SHA256SUMS`

- [ ] **Step 1: Create the external recovery directory with private permissions**

```bash
test ! -e /Users/developer/Desktop/meetily-recovery-2026-07-12
install -d -m 700 /Users/developer/Desktop/meetily-recovery-2026-07-12
```

Expected: the directory exists and `stat -f '%Sp'` reports `drwx------`.

- [ ] **Step 2: Record the current topology without abbreviating hashes**

```bash
git -C /Users/developer/Desktop/meeting-minutes status --porcelain=v2 --branch > /Users/developer/Desktop/meetily-recovery-2026-07-12/pre-recovery-status.txt
git -C /Users/developer/Desktop/meeting-minutes worktree list --porcelain > /Users/developer/Desktop/meetily-recovery-2026-07-12/pre-recovery-worktrees.txt
git -C /Users/developer/Desktop/meeting-minutes for-each-ref --format='%(refname)|%(objectname)|%(upstream)|%(upstream:track)' refs/heads refs/remotes refs/tags > /Users/developer/Desktop/meetily-recovery-2026-07-12/pre-recovery-refs.txt
git -C /Users/developer/Desktop/meeting-minutes remote -v > /Users/developer/Desktop/meetily-recovery-2026-07-12/pre-recovery-remotes.txt
```

Expected: all four files are non-empty. `pre-recovery-remotes.txt` contains both `themaddoxed/meetily` and `Zackriya-Solutions/meeting-minutes`.

- [ ] **Step 3: Save all tracked root changes as a binary-safe patch**

```bash
git -C /Users/developer/Desktop/meeting-minutes diff --binary HEAD > /Users/developer/Desktop/meetily-recovery-2026-07-12/root-tracked.patch
test -s /Users/developer/Desktop/meetily-recovery-2026-07-12/root-tracked.patch
```

Expected: `root-tracked.patch` is non-empty and starts with `diff --git`.

- [ ] **Step 4: Save the exact untracked file list and archive those files**

```bash
git -C /Users/developer/Desktop/meeting-minutes ls-files --others --exclude-standard -z > /Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked-files.zlist
git -C /Users/developer/Desktop/meeting-minutes ls-files --others --exclude-standard > /Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked-files.txt
tar -C /Users/developer/Desktop/meeting-minutes --null -T /Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked-files.zlist -czf /Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked.tar.gz
tar -tzf /Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked.tar.gz > /Users/developer/Desktop/meetily-recovery-2026-07-12/root-untracked-archive-contents.txt
```

Expected: the plain-text list and archive contents contain `AGENTS.md`, `frontend/tests/`, and `frontend/design/` entries.

- [ ] **Step 5: Bundle every committed ref**

```bash
git -C /Users/developer/Desktop/meeting-minutes bundle create /Users/developer/Desktop/meetily-recovery-2026-07-12/repository-before-recovery.bundle --all
git -C /Users/developer/Desktop/meeting-minutes bundle verify /Users/developer/Desktop/meetily-recovery-2026-07-12/repository-before-recovery.bundle > /Users/developer/Desktop/meetily-recovery-2026-07-12/bundle-verification.txt
```

Expected: verification exits 0 and reports that the bundle is complete.

- [ ] **Step 6: Generate and verify recovery checksums**

```bash
cd /Users/developer/Desktop/meetily-recovery-2026-07-12
shasum -a 256 repository-before-recovery.bundle root-tracked.patch root-untracked.tar.gz > SHA256SUMS
shasum -a 256 -c SHA256SUMS
```

Expected: three `OK` lines.

## Task 2: Freeze the Universal baseline and legacy planning history

**Files:**
- Modify: Git refs only
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/archive-refs.txt`

- [ ] **Step 1: Reconfirm that the Universal worktree is clean and unchanged**

```bash
test -z "$(git -C /Users/developer/Desktop/meeting-minutes/.worktrees/universal-macos-stable status --porcelain)"
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse build/universal-macos-stable)" = "8ffc5e6de85d494ecf25fd34afb40b7a67c432cf"
```

Expected: both commands exit 0 with no output. If either fails, stop before creating refs.

- [ ] **Step 2: Create archive branches without moving their source refs**

```bash
git -C /Users/developer/Desktop/meeting-minutes show-ref --verify --quiet refs/heads/archive/universal-0.4.1 && exit 1 || true
git -C /Users/developer/Desktop/meeting-minutes show-ref --verify --quiet refs/heads/archive/pre-recovery-main-2026-07-12 && exit 1 || true
git -C /Users/developer/Desktop/meeting-minutes branch archive/universal-0.4.1 8ffc5e6de85d494ecf25fd34afb40b7a67c432cf
git -C /Users/developer/Desktop/meeting-minutes branch archive/pre-recovery-main-2026-07-12 0b5aef606090221ffb8c343a40d34ace9615dc34
```

Expected: both archive branches are created exactly at the audited commits.

- [ ] **Step 3: Add an annotated immutable baseline tag**

```bash
git -C /Users/developer/Desktop/meeting-minutes show-ref --verify --quiet refs/tags/legacy-v0.4.1-universal-2026-07-11 && exit 1 || true
git -C /Users/developer/Desktop/meeting-minutes tag -a legacy-v0.4.1-universal-2026-07-11 8ffc5e6de85d494ecf25fd34afb40b7a67c432cf -m "Verified Universal macOS baseline before Git recovery"
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse legacy-v0.4.1-universal-2026-07-11^{})" = "8ffc5e6de85d494ecf25fd34afb40b7a67c432cf"
```

Expected: the dereferenced annotated tag equals the Universal commit.

- [ ] **Step 4: Record the archive refs and add them to a second verified bundle**

```bash
git -C /Users/developer/Desktop/meeting-minutes for-each-ref --format='%(refname)|%(objectname)' refs/heads/archive refs/tags/legacy-v0.4.1-universal-2026-07-11 > /Users/developer/Desktop/meetily-recovery-2026-07-12/archive-refs.txt
git -C /Users/developer/Desktop/meeting-minutes bundle create /Users/developer/Desktop/meetily-recovery-2026-07-12/repository-with-archive-refs.bundle --all
git -C /Users/developer/Desktop/meeting-minutes bundle verify /Users/developer/Desktop/meetily-recovery-2026-07-12/repository-with-archive-refs.bundle
cd /Users/developer/Desktop/meetily-recovery-2026-07-12
shasum -a 256 repository-with-archive-refs.bundle >> SHA256SUMS
shasum -a 256 -c SHA256SUMS
```

Expected: verification exits 0, `archive-refs.txt` contains both archive branches and the annotated tag, and four checksum lines report `OK`.

## Task 3: Quarantine the dirty root overlay

**Files:**
- Create branch: `salvage/root-2026-07-12`
- Preserve externally: `/Users/developer/Desktop/meetily-recovery-2026-07-12/generated-superpowers-brainstorm/`
- Commit only the existing root overlay and selected untracked project artifacts

- [ ] **Step 1: Move generated brainstorming runtime state out of the repository**

```bash
mv /Users/developer/Desktop/meeting-minutes/.superpowers/brainstorm /Users/developer/Desktop/meetily-recovery-2026-07-12/generated-superpowers-brainstorm
```

Expected: `.superpowers/brainstorm` is absent from the worktree and preserved in the recovery directory.

- [ ] **Step 2: Create the salvage branch in place so the dirty overlay follows it**

```bash
git -C /Users/developer/Desktop/meeting-minutes show-ref --verify --quiet refs/heads/salvage/root-2026-07-12 && exit 1 || true
git -C /Users/developer/Desktop/meeting-minutes switch -c salvage/root-2026-07-12
```

Expected: the current branch becomes `salvage/root-2026-07-12`; the tracked and selected untracked project files remain present.

- [ ] **Step 3: Stage tracked changes and the explicit project artifact set**

```bash
git -C /Users/developer/Desktop/meeting-minutes add -u
git -C /Users/developer/Desktop/meeting-minutes add -- AGENTS.md docs/superpowers/plans/2026-07-10-meetily-design-system.md docs/superpowers/specs/2026-07-10-meetily-design-system-extraction-design.md frontend/design frontend/playwright.config.ts frontend/pnpm-workspace.yaml frontend/scripts/next-dev.js frontend/src/components/ThemeScript.tsx frontend/src/lib/theme.ts frontend/tests
```

Expected: `.superpowers/` is not staged. All other paths from `root-untracked-files.txt` are either staged or preserved in the external archive.

- [ ] **Step 4: Verify whitespace, path scope, and absence of credential-like additions**

```bash
git -C /Users/developer/Desktop/meeting-minutes diff --cached --check
git -C /Users/developer/Desktop/meeting-minutes diff --cached --name-status > /Users/developer/Desktop/meetily-recovery-2026-07-12/salvage-staged-paths.txt
if git -C /Users/developer/Desktop/meeting-minutes diff --cached | rg -n 'BEGIN [A-Z ]*PRIVATE KEY|API[_-]?KEY\s*[=:]|PASSWORD\s*[=:]|SECRET\s*[=:]|TOKEN\s*[=:]|sk-[A-Za-z0-9]{20,}'; then
  echo "credential-like content found in staged salvage diff" >&2
  exit 1
fi
```

Expected: `diff --check` exits 0. The credential scan exits 1 with no matches. If it prints a match, unstage and inspect that file before proceeding.

- [ ] **Step 5: Commit the quarantine snapshot**

```bash
git -C /Users/developer/Desktop/meeting-minutes commit -m "chore: snapshot legacy root overlay"
git -C /Users/developer/Desktop/meeting-minutes rev-parse HEAD > /Users/developer/Desktop/meetily-recovery-2026-07-12/salvage-commit.txt
```

Expected: one salvage commit is created; no other branch moves.

- [ ] **Step 6: Verify the salvage worktree is clean and recoverable**

```bash
test -z "$(git -C /Users/developer/Desktop/meeting-minutes status --porcelain)"
git -C /Users/developer/Desktop/meeting-minutes apply --reverse --check /Users/developer/Desktop/meetily-recovery-2026-07-12/root-tracked.patch
```

Expected: the worktree is clean and the reverse patch check exits 0, proving that the tracked salvage content matches the saved pre-commit patch.

## Task 4: Normalize remote names without pushing

**Files:**
- Modify: local Git remote configuration
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/post-rename-remotes.txt`

- [ ] **Step 1: Assert the current reversed remote mapping**

```bash
test "$(git -C /Users/developer/Desktop/meeting-minutes remote get-url origin)" = "https://github.com/Zackriya-Solutions/meeting-minutes"
test "$(git -C /Users/developer/Desktop/meeting-minutes remote get-url fork)" = "https://github.com/themaddoxed/meetily.git"
```

Expected: both assertions exit 0. If URLs differ, stop instead of renaming by assumption.

- [ ] **Step 2: Rename upstream first, then the personal fork**

```bash
git -C /Users/developer/Desktop/meeting-minutes remote rename origin upstream
git -C /Users/developer/Desktop/meeting-minutes remote rename fork origin
```

Expected: exactly two remotes remain: `origin` and `upstream`.

- [ ] **Step 3: Verify URLs and refresh refs without pruning**

```bash
test "$(git -C /Users/developer/Desktop/meeting-minutes remote get-url origin)" = "https://github.com/themaddoxed/meetily.git"
test "$(git -C /Users/developer/Desktop/meeting-minutes remote get-url upstream)" = "https://github.com/Zackriya-Solutions/meeting-minutes"
git -C /Users/developer/Desktop/meeting-minutes fetch upstream
git -C /Users/developer/Desktop/meeting-minutes fetch origin
git -C /Users/developer/Desktop/meeting-minutes remote -v > /Users/developer/Desktop/meetily-recovery-2026-07-12/post-rename-remotes.txt
```

Expected: no fetch uses `--prune`; all existing remote refs remain available during migration.

- [ ] **Step 4: Confirm that the audited upstream base has not moved unexpectedly**

```bash
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse upstream/devtest)" = "0bedd8b7d805fc924fb706380c829cb1b48d4caf"
git -C /Users/developer/Desktop/meeting-minutes log -1 --oneline upstream/devtest
```

Expected: the hash assertion exits 0. If it fails, stop and refresh this plan against the newer upstream history before creating `dev`.

## Task 5: Make the verified Universal commit product `main`

**Files:**
- Modify: local `main` ref
- Modify: personal fork `origin/main` by fast-forward push only

- [ ] **Step 1: Confirm that the personal fork main is an ancestor of the Universal baseline**

```bash
git -C /Users/developer/Desktop/meeting-minutes merge-base --is-ancestor origin/main 8ffc5e6de85d494ecf25fd34afb40b7a67c432cf
```

Expected: exit 0. A non-zero result stops the plan; never substitute `--force`.

- [ ] **Step 2: Repoint local `main` after its prior tip has been archived**

```bash
git -C /Users/developer/Desktop/meeting-minutes branch -f main 8ffc5e6de85d494ecf25fd34afb40b7a67c432cf
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse main)" = "8ffc5e6de85d494ecf25fd34afb40b7a67c432cf"
```

Expected: local `main` equals the annotated Universal baseline; the root worktree remains on `salvage/root-2026-07-12`.

- [ ] **Step 3: Dry-run and perform a normal fast-forward push**

```bash
git -C /Users/developer/Desktop/meeting-minutes push --dry-run origin main
git -C /Users/developer/Desktop/meeting-minutes push origin main
```

Expected: the dry run shows a normal update and does not contain `forced update`. The actual push succeeds without `--force`.

- [ ] **Step 4: Push the immutable archive tag and archive branch**

```bash
git -C /Users/developer/Desktop/meeting-minutes push origin refs/tags/legacy-v0.4.1-universal-2026-07-11
git -C /Users/developer/Desktop/meeting-minutes push -u origin archive/universal-0.4.1
```

Expected: the personal fork has both a release-facing `main` and an explicitly named archive ref.

## Task 6: Create and validate a clean product `dev`

**Files:**
- Create branch: `dev` from then-current `upstream/devtest`
- Create worktree: `/Users/developer/Desktop/meeting-minutes/.worktrees/dev`
- Local ignored setup: `frontend/src-tauri/binaries/ffmpeg-x86_64-apple-darwin`
- Local ignored setup: `frontend/src-tauri/binaries/llama-helper-x86_64-apple-darwin`

- [ ] **Step 1: Assert that local `dev` and its worktree do not already exist**

```bash
git -C /Users/developer/Desktop/meeting-minutes show-ref --verify --quiet refs/heads/dev && exit 1 || true
test ! -e /Users/developer/Desktop/meeting-minutes/.worktrees/dev
```

Expected: both checks exit 0.

- [ ] **Step 2: Create an untracked product branch from upstream and attach a worktree**

```bash
git -C /Users/developer/Desktop/meeting-minutes branch --no-track dev upstream/devtest
git -C /Users/developer/Desktop/meeting-minutes worktree add /Users/developer/Desktop/meeting-minutes/.worktrees/dev dev
```

Expected: `.worktrees/dev` is clean and its branch is `dev`.

- [ ] **Step 3: Prove exact initial ancestry**

```bash
test "$(git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev rev-parse HEAD)" = "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse upstream/devtest)"
git -C /Users/developer/Desktop/meeting-minutes merge-base --is-ancestor upstream/devtest dev
```

Expected: both checks exit 0.

- [ ] **Step 4: Reuse verified ignored sidecars to avoid build-time network downloads**

```bash
mkdir -p /Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/binaries
cp /Users/developer/Desktop/meeting-minutes/.worktrees/universal-macos-stable/frontend/src-tauri/binaries/ffmpeg-x86_64-apple-darwin /Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/binaries/ffmpeg-x86_64-apple-darwin
cp /Users/developer/Desktop/meeting-minutes/.worktrees/universal-macos-stable/frontend/src-tauri/binaries/llama-helper-x86_64-apple-darwin /Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend/src-tauri/binaries/llama-helper-x86_64-apple-darwin
git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev status --short
```

Expected: Git status remains empty because `frontend/src-tauri/binaries/` is ignored.

- [ ] **Step 5: Compile the clean Rust test baseline**

```bash
cd /Users/developer/Desktop/meeting-minutes/.worktrees/dev
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer cargo test -p meetily --no-run
```

Expected: exit 0. If it fails, save the complete output to `/Users/developer/Desktop/meetily-recovery-2026-07-12/dev-rust-baseline-failure.txt` and stop before adding migration commits.

- [ ] **Step 6: Build the clean frontend baseline**

```bash
cd /Users/developer/Desktop/meeting-minutes/.worktrees/dev/frontend
pnpm install --frozen-lockfile
pnpm run build
```

Expected: both commands exit 0 and `git status --short` remains empty.

- [ ] **Step 7: Publish product `dev` only after both baselines pass**

```bash
git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev push -u origin dev
```

Expected: `dev` tracks `origin/dev`; it does not track `upstream/devtest` even though that was its creation point.

## Task 7: Add the workflow contract and initial migration registry

**Files:**
- Create: `docs/development/git-workflow.md`
- Create: `docs/migration/legacy-feature-registry.md`
- Cherry-pick: approved design and recovery plan documents into `dev`

- [ ] **Step 1: Bring the approved design and this plan onto `dev`**

```bash
git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev cherry-pick 0b5aef606090221ffb8c343a40d34ace9615dc34
PLAN_COMMIT=$(git -C /Users/developer/Desktop/meeting-minutes log -1 --format=%H docs/git-workflow-recovery-plan -- docs/superpowers/plans/2026-07-12-meetily-git-recovery-foundation.md)
git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev cherry-pick "$PLAN_COMMIT"
```

Expected: `dev` contains the approved spec and recovery plan, with no product code changes from legacy `main`.

- [ ] **Step 2: Create the daily workflow contract**

Create `docs/development/git-workflow.md` with exactly this content:

````markdown
# Meetily Git Workflow

## Permanent branches

- `main` contains tested product releases only.
- `dev` is the sole product integration branch.
- `upstream/devtest` is the base for contributions to the canonical project.

## Short-lived branches

- Create `feature/<name>` from `dev` for new product work.
- Create `migrate/<name>` from `dev` for one legacy feature at a time.
- Create `upstream/<name>` directly from `upstream/devtest` for upstream pull requests.
- Create `fix/<name>` from the branch that ships the affected version.

Never create a feature branch from another feature, migration, archive, or salvage branch. Never merge an archive or salvage branch wholesale into `dev`.

## Start checks

Before product work:

```bash
git fetch origin
git switch dev
git pull --ff-only origin dev
git switch -c feature/<name>
```

Before upstream contribution work:

```bash
git fetch upstream
git switch -c upstream/<name> upstream/devtest
git merge-base --is-ancestor upstream/devtest HEAD
```

## Completion checks

A branch is complete only when focused tests pass, the worktree is clean, its intended destination commit is recorded, and its short-lived branch can be deleted without losing knowledge.

Only one `migrate/*` branch may be active. Urgent release fixes are the only exception.
````

- [ ] **Step 3: Create the initial legacy migration registry**

Create `docs/migration/legacy-feature-registry.md` with exactly this initial inventory:

```markdown
# Meetily Legacy Feature Registry

Status values: `inventoried`, `ready`, `migrating`, `verified`, `deferred`.

| Feature or artifact | Source | Dependency | Transfer method | Verification gate | Status | Destination |
|---|---|---|---|---|---|---|
| Semantic theme and summary system prompt | `feature/meetily-0.4.0-dark-theme` (`7598290`..`9cd089a`) | clean `dev` | ordered cherry-pick of the 12-commit series | theme browser acceptance suite and frontend build | ready | none, not migrated |
| Updated neutral palette and native macOS theme sync | `build/universal-macos-stable` (`d949992`, `9daac46`, `e49ca33`, `c901298`) | semantic theme | selective cherry-pick | secondary-surface theme tests and manual native appearance check | inventoried | none, not migrated |
| Universal Tauri target and sidecar packaging | `build/universal-macos-stable` (`73b910e`, `1b67186`, `efa5dc1`) | clean build baseline | selective cherry-pick | `lipo -info`, `codesign --verify --deep --strict`, DMG verification | inventoried | none, not migrated |
| Interface polish and motion | `build/universal-macos-stable` (`ce5fabd`..`236fb0a`) | semantic theme | ordered selective cherry-pick | UI policy tests, frontend build, visual acceptance | inventoried | none, not migrated |
| Gemma 4 built-in summary models | `gemma4-summary` (`e4ace4f`..`35a6c9f`) | clean `dev` | ordered cherry-pick of four clean commits | model registry tests, frontend build, local helper smoke test | inventoried | none, not migrated |
| Gemma 4 macOS recommendation | `build/universal-macos-stable` (`0275b49`) | Gemma 4 models | cherry-pick after Gemma 4 | recommendation test on macOS | inventoried | none, not migrated |
| Speaker diarization and rename workflow | `codex-port-v042` (`aa5e006`..`0160d84`), excluding duplicated theme/Gemma commits and merge `8dceceb` | clean database and transcript baseline | selective ordered cherry-pick by diarization slice | Rust tests, frontend build, speaker rename acceptance | inventoried | none, not migrated |
| Native Whisper streaming | `feature/whisper-streaming` (`fdf03b3`, `6ac4831`) | stable transcription pipeline | two-commit cherry-pick followed by audio regression checks | focused Rust streaming tests and live transcription smoke test | inventoried | none, not migrated |
| Recording save-folder selection | `salvage/root-2026-07-12` tracked overlay and `frontend/tests/recording-folder-selection.spec.ts` | clean settings/Tauri baseline | behavior-preserving reimplementation | recording folder e2e and persisted preference check | inventoried | none, not migrated |
| Interrupted-meeting recovery on first relaunch | `build/universal-macos-stable` (`dfb7dd2`) | clean recovery baseline | cherry-pick | recovery dialog regression test | inventoried | none, not migrated |
| Browser user-agent analytics privacy fix | `build/universal-macos-stable` (`6dc6570`) | clean analytics baseline | cherry-pick | analytics payload unit test | inventoried | none, not migrated |
| Audio verification stabilization | `build/universal-macos-stable` (`5424bc7`) and `codex-port-v042` (`c0201eb`) | chosen audio implementation | patch-equivalence review, then keep one version | Rust audio verification target | inventoried | none, not migrated |
| Design-system extraction and Pencil artifacts | `salvage/root-2026-07-12`, `frontend/design/`, approved design docs | semantic theme | preserve artifacts and documentation without product merge | Pencil MCP layout and token audit | inventoried | none, not migrated |
| LFM2.5-Audio experiment | former `origin/feat/lfm-integration` at `f484a88b26b0c12078430bb3bd2c93848ef5aab5`, renamed to `upstream/feat/lfm-integration` | stable audio provider boundary | behavior-preserving port after dedicated architecture review | sidecar setup and transcription smoke test | deferred | none, not migrated |
| Earlier Whisper streaming improvements | former `origin/feat/whisper-streaming-improvements` at `29bcd12bb5d8fd4bfc533d430229af6b21dc8be3`, renamed to `upstream/feat/whisper-streaming-improvements` | native Whisper streaming comparison | patch-equivalence and behavior comparison | streaming latency and transcript stability checks | deferred | none, not migrated |
| Live-transcription disable mode | former `origin/feat/turn-off-live-transcription` at `6d50593ca0f3a2902d1b860589493a35cc7bb2da` | stable recording state | behavior-preserving port | recording without live transcript acceptance | deferred | none, not migrated |
| Audio import and retranscription | former `origin/feat/import-audio-and-retranscription` at `1d9d9178719527ea14dcdc8f3d2d7b1211eda1bd` | stable transcription pipeline | patch-equivalence review before port | import, retranscription, and persistence acceptance | deferred | none, not migrated |

## Migration order

1. Semantic theme and summary system prompt.
2. Updated palette and native theme sync.
3. Gemma 4 models and macOS recommendation.
4. Recording save-folder selection.
5. Universal packaging and sidecars.
6. Interface polish and motion.
7. Recovery and analytics privacy fixes.
8. Speaker diarization and rename workflow.
9. Native Whisper streaming.
10. Audio verification stabilization against the chosen pipeline.
11. Deferred audio experiments, each with its own approved design and migration plan.

The design-system artifacts remain preserved throughout and are updated only through Pencil MCP.
```

- [ ] **Step 4: Validate the two documents**

```bash
cd /Users/developer/Desktop/meeting-minutes/.worktrees/dev
git diff --check
rg -n 'T[B]D|T[O]DO|P[L]ACEHOLDER|f[i]ll in|i[m]plement later' docs/development/git-workflow.md docs/migration/legacy-feature-registry.md
```

Expected: `git diff --check` exits 0; `rg` exits 1 with no output.

- [ ] **Step 5: Commit and push the workflow contract**

```bash
git add docs/development/git-workflow.md docs/migration/legacy-feature-registry.md docs/superpowers/specs/2026-07-12-meetily-dual-track-git-workflow-design.md docs/superpowers/plans/2026-07-12-meetily-git-recovery-foundation.md
git commit -m "docs: establish dual-track migration workflow"
git push origin dev
```

Expected: only the four documentation files are included in the commit.

## Task 8: Verify the recovery foundation and stop before feature migration

**Files:**
- Create: `/Users/developer/Desktop/meetily-recovery-2026-07-12/final-*.txt`
- No branch or worktree deletion

- [ ] **Step 1: Capture final topology**

```bash
git -C /Users/developer/Desktop/meeting-minutes worktree list --porcelain > /Users/developer/Desktop/meetily-recovery-2026-07-12/final-worktrees.txt
git -C /Users/developer/Desktop/meeting-minutes for-each-ref --format='%(refname)|%(objectname)|%(upstream)|%(upstream:track)' refs/heads refs/remotes refs/tags > /Users/developer/Desktop/meetily-recovery-2026-07-12/final-refs.txt
git -C /Users/developer/Desktop/meeting-minutes remote -v > /Users/developer/Desktop/meetily-recovery-2026-07-12/final-remotes.txt
```

Expected: `main`, `dev`, the two archive branches, the salvage branch, all pre-existing feature branches, and all worktrees remain visible.

- [ ] **Step 2: Verify permanent branch roles**

```bash
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse main)" = "8ffc5e6de85d494ecf25fd34afb40b7a67c432cf"
git -C /Users/developer/Desktop/meeting-minutes merge-base --is-ancestor upstream/devtest dev
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse archive/universal-0.4.1)" = "8ffc5e6de85d494ecf25fd34afb40b7a67c432cf"
test "$(git -C /Users/developer/Desktop/meeting-minutes rev-parse archive/pre-recovery-main-2026-07-12)" = "0b5aef606090221ffb8c343a40d34ace9615dc34"
```

Expected: all four checks exit 0.

- [ ] **Step 3: Verify remote roles and tracking**

```bash
test "$(git -C /Users/developer/Desktop/meeting-minutes remote get-url origin)" = "https://github.com/themaddoxed/meetily.git"
test "$(git -C /Users/developer/Desktop/meeting-minutes remote get-url upstream)" = "https://github.com/Zackriya-Solutions/meeting-minutes"
test "$(git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev rev-parse --abbrev-ref --symbolic-full-name '@{upstream}')" = "origin/dev"
```

Expected: all three checks exit 0.

- [ ] **Step 4: Reverify both recovery bundles and checksums**

```bash
git -C /Users/developer/Desktop/meeting-minutes bundle verify /Users/developer/Desktop/meetily-recovery-2026-07-12/repository-before-recovery.bundle
git -C /Users/developer/Desktop/meeting-minutes bundle verify /Users/developer/Desktop/meetily-recovery-2026-07-12/repository-with-archive-refs.bundle
cd /Users/developer/Desktop/meetily-recovery-2026-07-12
shasum -a 256 -c SHA256SUMS
```

Expected: both bundles verify and all four checksums report `OK`.

- [ ] **Step 5: Confirm WIP remains zero and hand off the first migration**

```bash
git -C /Users/developer/Desktop/meeting-minutes branch --list 'migrate/*'
git -C /Users/developer/Desktop/meeting-minutes/.worktrees/dev status --short --branch
git -C /Users/developer/Desktop/meeting-minutes status --short --branch
```

Expected: no `migrate/*` branch exists yet; `dev` and the salvage worktree are clean. Stop here. The next document must be a dedicated implementation plan for `migrate/theme-foundation`, based on `dev` and limited to the first registry row.
