# macOS Fork Git Policy

This policy separates the current Meetily checkout from the future macOS product
workflow. It records the target rules; it does not authorize moving refs,
changing remotes, or publishing branches.

For the macOS fork, this document takes precedence over the older
`docs/development/git-workflow.md` contained in the immutable baseline. That
document remains historical evidence: its older baseline and its
`upstream/*` contribution lane are not the macOS product workflow. In this
policy, `upstream` means the remote; existing local `upstream/*` refs, if any,
are retained evidence and are not bases for new product work.

## Integration context 2026-10-02

The owner authorized local integration of the current macOS product features into
`main` on 2026-10-02. The isolated candidate is
`integration/main-features-2026-10-02`, based on `main@952e3614`.
The current feature source is `themaddoxed/feat071-audio-controls@61e890f4`
plus frozen audio/privacy, summary, and visible-search working-tree snapshots.
All original source worktrees and refs are preserved. Temporary diagnostic
patches referring to `/private/tmp/fork051-tao-patched` and the experimental
panic hook are not part of the product candidate.

The candidate passed the automatic integration stages, including repaired Rust
stages rechecked separately. The owner explicitly authorized its local merge
commit, preservation of the old root overlay via stash, and a local main
fast-forward on 2026-10-02. The exact result is recorded in
`.worktrees/integration-backups-2026-10-02/integration-report.json`.
No push, PR, native application installation, or release is included.
Native acceptance and distribution gates remain separate from automated checks.
The older execution contexts below are retained as provenance.

## Execution context 2026-09-22

The current verified clean product reference for the next scoped task is
`themaddoxed/feat071-audio-controls` at
`61e890f444f81718d8136beae6afc6f61a5f0e44` in `themaddoxed/meetily-macos`.
The plan lives in the existing `docs/prd` files on
`themaddoxed/task-decomposition-plan`; that documentation branch and preservation
`main` are not code bases. Use the [execution plan](execution-plan-2026-09-22.md)
as the single current entry point; earlier topology below is historical.

This is an execution-base clarification, not a canonical integration handoff.
Recheck HEAD and dirty files before implementation. A new isolated task checkout
starts from the verified product reference and uses `themaddoxed/<task>-<slug>`.
Dirty `fork-051-permissions-lifecycle` work remains separate and requires targeted
review before any reuse. Do not copy it or merge donors as part of planning.
No existing ref, integration destination, remote or publication policy is changed.
Commit, push, PR, integration and release each remain subject to the existing
explicit authorization boundary; FORK-054 stays open.

## Current topology

Local snapshot on 2026-08-11:

- `origin` points to `themaddoxed/meetily`; it may remain the current fork while
  the migration is audited. The independent repository target is not chosen.
- `upstream` points to `Zackriya-Solutions/meeting-minutes`. Its fetch and push
  URLs are currently the same, so Git does not enforce the intended read-only
  role locally.
- Local `main` tracks `origin/main` and is 2 commits ahead. Local `dev` tracks
  `origin/dev` and is 120 commits ahead. These existing branches are historical
  state, not permission to repoint them to the new product line.
- The task branch has no upstream. Multiple legacy, archive, feature, and build
  branches remain attached to separate worktrees and must stay in place until
  the migration registry and final handoff account for them.
- No executable local `pre-push` hook or explicit push default was found.
  Server-side branch protection cannot be proven from this checkout. Existing
  entry workflows use manual `workflow_dispatch`; `build.yml` is reusable
  through `workflow_call`. Neither proves an automatic required-check gate.

FORK-029 changes none of this topology.

## Branch and remote contract

Two stages keep product work unambiguous.

Before the FORK-054 canonical handoff:

- The provisional integration ref created by FORK-031 is the only base and
  integration destination for product tasks FORK-032 through FORK-053 and
  scoped architecture tasks added before handoff. FORK-031 chooses and records
  its exact name; this policy does not preselect it.
- Each task branch starts from that exact ref and uses the unambiguous namespace
  `fork/<task-id>-<slug>`. It returns only to the provisional integration ref
  through review.
- Every pre-existing `feature/*`, `build/*`, `archive/*`, `legacy/*`, and
  `salvage/*` branch is migration evidence, not a product base or integration
  candidate, unless FORK-030 explicitly admits it for a named migration.
- Current `main`, `dev`, their tracking configuration, and both remotes remain
  unchanged.

After the FORK-054 canonical handoff:

- `origin` is the writable macOS product repository. Keeping the current fork
  during audit is allowed; changing to an independent repository requires a
  separately approved target.
- `upstream` is the canonical Meetily repository and is read-only.
- `dev` is the sole integration branch for product work.
- `main` contains verified releases only.
- `feature/<task>` branches start from `dev` and return to `dev` through review.
- Superseded evidence branches are archived after their migration is verified.
  They are never merged wholesale into `dev` or `main`.

Branch ownership is explicit:

- A task owner controls one short-lived `fork/*` branch before canonical handoff
  or one `feature/*` branch after it, plus the branch's worktree.
- The integration owner accepts reviewed, verified changes into the active
  integration ref: provisional before FORK-054, then `dev`.
- The release owner promotes an exact verified `dev` commit to `main` after an
  explicit release decision. Feature work never goes directly to `main`.

## Taking changes from upstream

Allowed upstream intake is selective:

1. Fetch the required upstream refs without pruning or pushing.
2. Review the full source commit and diff manually against the product base.
3. Create the product branch from the active integration ref. For a small atomic
   change, cherry-pick the full upstream SHA with `-x`; for a tangled change,
   reimplement only the required behavior.
4. In the FORK-030 registry, record the source ref and full SHA, transfer method,
   destination commit, review decision, and relevant verification before the
   change enters the active integration ref.

Do not run `git pull upstream` on a product branch, merge an upstream branch
wholesale, base product features on `upstream/*`, or push any ref to `upstream`.

## Push and history safeguards

At the final handoff, before any publication:

- Give `upstream` a deliberately failing push URL and verify that a dry-run push
  fails; keep its fetch URL unchanged.
- Set `origin` as the explicit default push remote and make product branches
  track only `origin`. Normal work still names `origin` in push commands.
- Protect `main` and `dev` on the server against deletion and force pushes,
  require review and the relevant checks, and restrict direct writes to their
  owners. Confirm these settings on the server; local Git cannot prove them.
- Never use `--force`, `--force-with-lease`, or a leading `+` refspec on `main`,
  `dev`, a release branch, or any shared review branch. Never rebase or reset a
  published `main` or `dev`. Rebase is limited to unpublished local work owned
  by the current task.

No remote write, branch deletion, or shared-history rewrite is part of FORK-029.

## Handoff gates

### FORK-031: isolated integration line

FORK-031 may create only a new local branch and worktree from the immutable
baseline `1e742fa22030fe5720db6a791407c64eea03ede4` after FORK-030 records all
migration sources. Its handoff must name the exact provisional integration ref,
worktree path, verification, and rollback. That ref becomes the only base and
integration destination for product work before FORK-054. Current `main`,
`dev`, their tracking configuration, and both remotes remain unchanged.

### FORK-054: canonical switch

The final switch requires completed release evidence, the resolved product and
repository identity, exact destination SHAs for future `dev` and `main`, remote
URLs, branch-protection evidence, a rollback procedure, and explicit owner
approval. Only then may the current canonical branches or remotes change and
product refs be published. This policy does not preselect the canonical
ref-mapping or new-repository strategy. FORK-054 must archive every superseded
branch after FORK-030 records its verified migration and destination; only then
may related PRs be commented on and closed. A non-fast-forward mapping must use
new refs or another separately approved path, never a forced rewrite of shared
`main` or `dev`.
