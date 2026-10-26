# Git workflow for the public source repository

`origin` is the independent public repository `themaddoxed/meetily`. Its history
starts at the anonymized root `d25a861`; it has no legacy branches or tags. Old
SHA/worktree examples in migration records refer to a private archive, not to
refs that a fresh public clone should restore.

## Branch roles

`main` currently holds the source bootstrap and verified source/CI/documentation
corrections. It is not an accepted or signed application release. Product
integration should use clean `dev`, with focused `feature/*`, `fix/*` or
`migrate/*` branches. Source/CI corrections to the bootstrap can start from the
current `main` when that is the affected base.

For a fresh clone with no local `dev`:

```bash
git fetch origin
test -z "$(git status --porcelain)"
git switch -c dev origin/main
git worktree add .worktrees/<name> -b feature/<name> dev
```

If `dev` already exists, inspect and update its actual clean worktree instead of
running this bootstrap command again. Inspect source/destination refs and dirty
work before a Git mutation; preserve unrelated work. Publication uses the exact
repository and branch authorized for the task.

## Legacy and upstream work

Preserve private archive checkouts, bundles and unfinished work. Transfer a
legacy behavior through a reviewed, anonymized source patch with fresh tests.
Merging the old graph or pushing archive refs would reintroduce private identity
metadata, paths and unreviewed history into the public repository.

`docs/migration/` records historical sources, decisions and dependency boundaries.
A registry row containing a legacy SHA is not proof of integration in this
public graph. Mark new destinations only after code, checks and evidence match.
Old source/patch-ID migration scripts need their private archive and cannot
validate a standalone public clone. Their historical results remain historical.

An upstream contribution, if explicitly requested, starts separately from the
current canonical project's target branch. Do not submit this product snapshot
or its private archive as an upstream contribution by default.

## Toolchain and completion

The active CI pins Node.js 22.17.0 and pnpm 10.17.1. Use the frozen lockfile for
normal installs. Dependency fixes require a focused manifest/lockfile diff and
relevant checks; rebuilding a lockfile is not part of an unrelated migration.

Before publishing a correction:

1. Review the final diff and applicable task/source contracts.
2. Pass focused tests and applicable checks from [testing.md](../testing.md).
3. Check whitespace, public source privacy and neutral commit metadata. Use a
   public alias or the existing `Meetily Contributors` identity with
   `noreply@example.invalid`; do not copy local Git identity into public commits.
4. Confirm a clean checkout after the commit, push the authorized ref, and read
   back its exact SHA. Publishing source is separate from accepting an artifact.

A product release additionally requires the clean exact candidate, verifier,
installed/native smoke, trust checks and release gate in `docs/testing.md`.
The initial source root, CI success and an ad-hoc app are not release approval.
