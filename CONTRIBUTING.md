# Contributing to Meetily

This is the independent macOS source repository. Start with [INDEX.md](INDEX.md),
[Git workflow](docs/development/git-workflow.md) and [build guide](docs/BUILDING.md).
The initial `main` is a source bootstrap; use clean `dev` for product integration
and focused feature/fix branches. Legacy `devtest` and archive refs are absent
from this public history. Upstream contributions need a separate agreed target.

Use the CI-pinned Node/pnpm versions and frozen lockfile. Preserve existing
SQLite migrations byte-for-byte, add a migration for a schema change, and test
real temporary-storage preservation and upgrade/reopen behavior. Run relevant
automated checks before requesting final installed/native acceptance.

Before publishing, review the diff, run `git diff --check`, production policy
and the public privacy guard. Keep real recordings, databases, models, secrets,
personal paths and personal Git identities out of commits. Synthetic fixtures
must be clearly identified. [testing.md](docs/testing.md) defines the complete
release gates; an unrun smoke item stays NOT_RUN, and CI is not release approval.

When proposing a change, state the trigger, resulting behavior, checks and any
remaining acceptance gates. Use focused `fix`, `feat`, `docs`, `test` or `chore`
commits. Include a relevant task ID from `docs/prd/` when one exists. A PR is
created only when requested; confirm the actual target branch. GitHub Issues
may be disabled, so repository PRD/review records remain the durable backlog.

Contributions use the project's [MIT license](LICENSE.md); retain required
upstream notices and attribution.
