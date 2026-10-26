# Meetily Dual-Track Git Workflow Design

Date: 2026-07-12
Status: approved by the user

## Goal

Keep every accumulated feature while replacing the current branch sprawl with two independent workflows:

1. a stable product fork that can ship releases;
2. clean contribution branches that can be submitted to upstream `devtest`.

The migration must preserve a working Universal macOS baseline and must not require a mass merge of legacy branch heads.

## Current-State Constraints

- The root worktree on local `main` contains a large uncommitted overlay and cannot be used as a clean integration base.
- Existing clean worktrees contain independently valuable feature and integration branches.
- `codex-port-v042` and the Universal build branch are integration-heavy snapshots, not safe parents for new work.
- The repository currently names the canonical upstream remote `origin` and the personal fork remote `fork`, which makes branch provenance easy to confuse.
- Upstream contributions must start from upstream `devtest` and target upstream `devtest`.

## Target Repository Model

After the recovery is complete, remotes have conventional roles:

- `origin`: `themaddoxed/meetily`, the product fork;
- `upstream`: `Zackriya-Solutions/meeting-minutes`, the canonical project.

Long-lived product branches:

- `main`: tested releases of the product fork only;
- `dev`: the next integrated product release;
- `archive/universal-0.4.1`: frozen reference to the validated Universal baseline.

Short-lived branches:

- `feature/<name>`: new product work, created from `dev`;
- `migrate/<name>`: one legacy feature being transferred to `dev`;
- `upstream/<name>`: contribution work created directly from `upstream/devtest`;
- `fix/<name>`: urgent fixes, based on the branch that actually ships the affected version.

The existing uncommitted root overlay is captured separately as `salvage/root-2026-07-12` after generated files and sensitive local data are excluded. It is evidence and recovery material, not an integration base.

## Migration Strategy

Legacy branch heads are never merged wholesale into the new `dev`.

For each feature, choose the least risky transfer method:

1. Cherry-pick clean, atomic commits when their dependencies and diff are well bounded.
2. Reapply a small patch when only part of a commit belongs to the feature.
3. Reimplement the observable behavior against the new base when the old branch is tangled with unrelated work.

Reimplementation does not mean dropping the feature. The legacy branch remains available as the behavioral reference until parity is verified.

## Migration Registry

The versioned registry lives at `docs/migration/legacy-feature-registry.md`. It records every legacy feature and prevents silent loss. Each row contains:

- feature name and user-visible purpose;
- source branches and commits;
- dependencies on other migrated features;
- affected subsystems;
- chosen transfer method;
- required automated and manual checks;
- status: `inventoried`, `ready`, `migrating`, `verified`, or `deferred`;
- destination commit in `dev`;
- upstream PR or issue, when applicable.

All legacy features remain in the registry, including deferred experiments. A feature is not considered migrated merely because its source branch was merged.

## Work-in-Progress Limit

Only one `migrate/*` branch may be active at a time. Urgent fixes are the only exception.

A migration is complete only when:

1. its intended behavior is present on the new base;
2. focused automated checks pass;
3. required manual acceptance checks pass;
4. it is integrated into `dev`;
5. the registry points to the destination commit;
6. deleting the short-lived migration branch would not lose knowledge.

This limit is the primary protection against cognitive overload. The frozen legacy baseline remains runnable throughout the migration.

## Product and Upstream Data Flow

Product work flows as follows:

```text
dev -> feature/* or migrate/* -> verified integration into dev -> release candidate -> main
```

Upstream contribution work remains independent:

```text
upstream/devtest -> upstream/<feature> -> PR to upstream/devtest
```

After an upstream PR merges, the upstream result is integrated into product `dev`. The product branch is never used as the head of an upstream PR.

## Recovery Sequence

1. Freeze and tag the clean Universal baseline.
2. Capture the dirty root worktree in a quarantined salvage branch after an audit.
3. Rename remotes to `origin` and `upstream` according to their roles.
4. Fetch upstream and create a fresh `dev` from the then-current `upstream/devtest`.
5. Build the full migration registry before moving code.
6. Determine dependency order and migrate one feature at a time.
7. Produce a verified release candidate from `dev` before promoting it to `main`.
8. Archive or delete redundant local worktrees only after registry evidence shows that their behavior is preserved.

## Non-Goals

- Flattening all history into one branch immediately.
- Deleting legacy branches before migration evidence exists.
- Making experimental audio engines coexist in one commit before each is independently validated.
- Using `main` as a scratchpad or daily integration branch.
- Requiring every product-only feature to be acceptable upstream.

## Success Criteria

The recovery is successful when:

- the validated legacy build remains reproducible;
- `main` contains releases only and has no uncommitted work;
- `dev` is the sole product integration branch;
- every retained legacy feature has a registry entry and a verified destination or explicit deferred status;
- every upstream PR branch is demonstrably based on `upstream/devtest`;
- no new feature branch uses another feature or legacy integration branch as an accidental parent;
- obsolete worktrees can be removed without losing code, decisions, or verification evidence.
