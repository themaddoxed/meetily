# macOS fork baseline

This record fixes the source baseline for the macOS-only fork. It does not qualify a release, choose the final processor support policy, or create the future integration branch.

## Source authority

| Field | Value |
| --- | --- |
| Authoritative commit | `1e742fa22030fe5720db6a791407c64eea03ede4` |
| Source tree | `1fbcc32d422e5d86c5266f4dfb8b7ce863c86845` |
| Commit date | `2026-07-27T22:24:39+03:00` |
| Mutable locator | `feature/release-ready-ui` |
| Existing worktree | `.worktrees/dev` |

The full commit ID is the authority. The branch name is only a convenient locator and may move. No tag, branch, worktree, remote, or GitHub repository was created or changed by `FORK-026`; the isolated integration line belongs to `FORK-031`.

## Ancestry snapshot

Captured on `2026-08-10T10:14:49+03:00` from the existing local tracking refs. No network fetch was performed, so the remote-tracking values are a local snapshot rather than a claim about the current GitHub heads.

| Comparison | Local ref commit | Merge base | Left/right count |
| --- | --- | --- | --- |
| `upstream/devtest...baseline` | `0bedd8b7d805fc924fb706380c829cb1b48d4caf` | `0bedd8b7d805fc924fb706380c829cb1b48d4caf` | `0 / 146` |
| `origin/main...baseline` | `8ffc5e6de85d494ecf25fd34afb40b7a67c432cf` | `0bedd8b7d805fc924fb706380c829cb1b48d4caf` | `40 / 146` |

For `git rev-list --left-right --count A...B`, the first number belongs only to `A` and the second only to `B`. At this snapshot the baseline therefore contains all commits reachable from the local `upstream/devtest` ref plus 146 baseline-only commits.

## Build composition at the baseline

- Product metadata is still Meetily `0.4.1` with bundle ID `com.meetily.ai`.
- The frontend uses Next.js `^14.2.25` and React `^18.2.0`; the desktop shell uses Tauri `2.6.2`; the Rust package declares Rust `1.77`.
- The macOS Whisper dependency enables Metal and Core ML. The same manifest still contains separate Windows and Linux dependency sections and GPU features.
- `frontend/scripts/build-universal-macos.js` is exposed as `pnpm --dir frontend run tauri:build:universal`. `scripts/release-candidate.sh` runs the full automatic check, seeds reviewed sidecars, then builds and verifies a Universal app and DMG.
- The existing diagnostic report records `x86_64` and `arm64` slices for the app, `llama-helper`, FFmpeg, and the diarization runtime.

This is not yet a macOS-only source tree. The baseline contains these measured cleanup surfaces:

- 21 paths whose names match `windows`, `linux`, `.bat`, `.ps1`, AppImage, DEB, MSI, or NSIS;
- 29 non-lock files containing selected Windows/Linux target, conditional, or CI markers;
- 13 files under `.github/workflows`;
- bundle targets `deb`, `appimage`, `msi`, `nsis`, `app`, and `dmg`;
- a configured updater pointing to the upstream Meetily release endpoint.

The two file counts are audit heuristics with their exact commands below, not a promise that every cross-platform reference is covered.

## Initial verification evidence

The source checks were repeated read-only for `FORK-026`:

- the commit and tree objects resolve;
- `.worktrees/dev` is clean and checked out at the authoritative commit;
- the local branch containing the commit is `feature/release-ready-ui`;
- no tag currently contains the baseline commit.

An existing diagnostic build provides historical build provenance. It was created on `2026-07-27` and was not rebuilt for `FORK-026`:

| Evidence | Result |
| --- | --- |
| Run | `artifacts/release-gate/diagnostic/1e742fa22030fe5720db6a791407c64eea03ede4/run-JYu6my` inside `.worktrees/dev` |
| Source state | Clean, stable fingerprint `085054eb193953a33b6d55c113def818ca7a14d05eec8348813636b86912ef99` |
| Automatic check / seed / build | Exit `0 / 0 / 0` |
| Candidate verifier | Exit `1`; status `UNGATED`; 13 checks passed and 8 failed |
| DMG | `meetily_0.4.1_universal.dmg`, 157391740 bytes, SHA-256 `4d7151a3510fb2da90c105a9de1a93b269b2c5b8e0c9b33c71fc1c8357c31cc6` |
| Fresh artifact recheck | SHA-256 still matches; `hdiutil verify` reports `VALID` |

The artifact is diagnostic only. The app uses an ad-hoc signature; the DMG is unsigned; Gatekeeper, trust, and notarization checks fail. These results prove that the recorded source produced a structurally valid Universal diagnostic artifact, not that it is distributable or release-ready.

## Read-only reproduction

Run from the repository root:

```bash
git rev-parse 'feature/release-ready-ui^{commit}'
git rev-parse 'feature/release-ready-ui^{tree}'
git -C .worktrees/dev rev-parse HEAD
git -C .worktrees/dev status --short --branch

git rev-parse 'upstream/devtest^{commit}'
git merge-base upstream/devtest feature/release-ready-ui
git rev-list --left-right --count upstream/devtest...feature/release-ready-ui

git rev-parse 'origin/main^{commit}'
git merge-base origin/main feature/release-ready-ui
git rev-list --left-right --count origin/main...feature/release-ready-ui

git show feature/release-ready-ui:frontend/src-tauri/tauri.conf.json |
  jq '{productName, version, identifier, bundle: .bundle.targets, updater: .plugins.updater.endpoints}'

git ls-tree -r --name-only feature/release-ready-ui |
  rg -i 'windows|linux|\.(bat|ps1)$|appimage|\.deb$|\.msi$|nsis' |
  sort -u |
  wc -l

git grep -Il \
  -e 'target_os = "windows"' \
  -e 'target_os = "linux"' \
  -e 'cfg(windows)' \
  -e 'cfg(target_os = "linux")' \
  -e 'windows-latest' \
  -e 'ubuntu-latest' \
  feature/release-ready-ui -- ':!*.lock' |
  wc -l
```

The artifact checks are optional because the ignored historical build directory may not exist in another clone. If it is present:

```bash
shasum -a 256 .worktrees/dev/target/universal-apple-darwin/release/bundle/dmg/meetily_0.4.1_universal.dmg
hdiutil verify .worktrees/dev/target/universal-apple-darwin/release/bundle/dmg/meetily_0.4.1_universal.dmg
```
