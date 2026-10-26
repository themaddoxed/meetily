> Public snapshot bootstrap: the historical refs/worktrees below are absent from this repository. The initial root commit is source publication, not a tested release. Start subsequent work from a clean `dev` created at the snapshot commit; retain the release gates. See [publication notes](public-source-snapshot.md).

# Meetily Git Workflow

This contract separates product development from contributions to the canonical Meetily project. It applies to every new branch and migration after the 2026-07-12 recovery foundation.

## Remote and permanent branch roles

- `origin` is the personal product fork, `themaddoxed/meetily`.
- `upstream` is the canonical project, `Zackriya-Solutions/meeting-minutes`.
- `main` contains tested product releases only. It is not a daily development or integration branch.
- `dev` is the sole product integration branch.
- `upstream/devtest` is the only base and pull-request target for canonical upstream contributions.

The validated Universal baseline is frozen at `8ffc5e6de85d494ecf25fd34afb40b7a67c432cf` on `build/universal-macos-stable` and `archive/universal-0.4.1`. Its worktree, `.worktrees/universal-macos-stable`, is a read-only recovery reference. Never pull, push, switch, reset, clean, install, build, commit, or otherwise mutate that branch or worktree. Read-only status, log, show, and comparison commands are allowed.

## Short-lived branch lanes

- Create `feature/<name>` from a clean, current `dev` for new product work.
- Create `migrate/<name>` from a clean, current `dev` for one legacy behavior at a time.
- Create `upstream/<name>` directly from the current `upstream/devtest` for upstream pull requests.
- Create `fix/<name>` from the permanent branch that ships the affected version.

Exactly one migration may be active at a time. An active migration is a registry row whose status is `migrating` paired with its corresponding `migrate/<name>` ref in a checked-out worktree, a local branch, or `origin`. Both the status and ref must exist; either one without the other is an inconsistent state that blocks new migration work. Retained local or remote `migrate/*` refs whose registry rows are not `migrating` are inactive recovery history and do not consume the work-in-progress slot. An urgent release fix is the only exception to this limit.

Never create a product feature or migration branch from another `feature/*`, `migrate/*`, `archive/*`, `salvage/*`, or legacy integration branch. Never merge an archive, salvage, or legacy branch wholesale into `dev`; transfer one registry item with the documented cherry-pick, partial patch, or behavior-preserving reimplementation method.

Product flow:

```text
dev -> feature/* or migrate/* -> verified integration into dev -> release candidate -> main
```

Upstream flow:

```text
upstream/devtest -> upstream/<name> -> pull request to upstream/devtest
```

After an upstream pull request merges, integrate the upstream result separately into product `dev`. Never use a product branch as the head of an upstream pull request.

## Supported recovery and migration toolchain

The currently verified local recovery and migration toolchain is:

- Node.js `v26.4.0`;
- pnpm `10.17.1`.

Use `pnpm install --frozen-lockfile` for every recovery, migration, and verification run. Lockfile regeneration is prohibited during feature migration. If a dependency change genuinely requires regeneration, make it on a separate isolated branch, review the lockfile-only diff and package-manager metadata independently, then run the full frontend verification gates before integration.

The current GitHub workflows still pin Node.js 20 and pnpm 8. That inconsistency is recorded in the migration registry as a separate toolchain-alignment follow-up; do not silently change CI or regenerate the lockfile as part of an unrelated migration.

The owner-controlled `.worktrees/whisper-streaming` worktree contains the committed review follow-up `3a8e9ee` on top of the streaming foundation. Do not switch, reset, clean, stash, commit, install, build, or edit in that worktree while executing other migrations. Its foundation and follow-up slices remain separate registry sources until they receive verified destinations.

## Start gates

Before product work, update the dedicated `dev` worktree, prove it is clean, and create a separate isolated feature worktree from its exact tip:

```bash
git fetch origin
git -C .worktrees/dev pull --ff-only origin dev
test -z "$(git -C .worktrees/dev status --porcelain)"
git worktree add .worktrees/<name> -b feature/<name> dev
git -C .worktrees/<name> merge-base --is-ancestor dev HEAD
```

For legacy transfer, substitute `migrate/<name>` and first use the registry as the authoritative active-work check. Ref listings are diagnostic evidence only:

```bash
MIGRATING_ROWS=$(rg -n '^\|.*\| `migrating` \|' docs/migration/legacy-feature-registry.md || true)
test -z "$MIGRATING_ROWS"
git branch --list 'migrate/*'
git branch -r --list 'origin/migrate/*'
git worktree add .worktrees/migrate-<name> -b migrate/<name> dev
git -C .worktrees/migrate-<name> merge-base --is-ancestor dev HEAD
```

Mark the selected registry row `migrating` as part of the migration start record. If the registry check or ref diagnostics reveal an unmatched active state, reconcile that evidence before creating another branch; do not delete retained refs merely to make the check pass.

Before upstream contribution work:

```bash
git fetch upstream
git worktree add .worktrees/upstream-<name> -b upstream/<name> upstream/devtest
git -C .worktrees/upstream-<name> merge-base --is-ancestor upstream/devtest HEAD
test -z "$(git -C .worktrees/upstream-<name> status --porcelain)"
```

Stop if the base is unexpected, the worktree is dirty, or another migration is active. Do not move unrelated local changes to make a gate pass.

## Completion gates

A short-lived branch is complete only when all of the following are true:

1. Its intended behavior and exclusions match the relevant registry row and migration plan.
2. Focused automated checks pass.
3. Required manual acceptance checks pass.
4. `pnpm install --frozen-lockfile` and the applicable build gates pass without changing the lockfile.
5. `git diff --check` passes and the final diff contains only intended paths.
6. The worktree is clean after the completion commit.
7. The completion ledger matches the branch lane: `migrate/*` must record its verified destination commit in `docs/migration/legacy-feature-registry.md`; `feature/*` records its `dev` integration commit in its merge or handoff evidence; `upstream/*` records its upstream pull request and resulting merge commit or final disposition.
8. Deleting the short-lived branch would not lose code, verification evidence, or migration knowledge.

Integrate a completed product branch into `dev`; promote only a verified release candidate from `dev` to `main`. Keep archive, salvage, Universal, and active Whisper recovery sources intact until their registry rows have verified destinations or explicit deferred dispositions.
