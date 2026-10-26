# Building the macOS application

The product target is macOS 14.2+ on Intel and Apple Silicon. Architecture and
minimum-version acceptance are separate from compilation; see
[fork/platform-support.md](fork/platform-support.md).

## Tools

Use full Xcode (including its SDK), CMake, Rust stable with rustfmt/Clippy,
Node.js 22.17.0 and pnpm 10.17.1. CI uses these frontend versions in
`quality-gate.yml`. Dependency updates change the lockfile deliberately;
normal installs use `--frozen-lockfile`.

From the repository root:

```bash
pnpm --dir frontend install --frozen-lockfile
cargo build --release --package llama-helper --features metal
HOST_TARGET=$(rustc -vV | sed -n 's/^host: //p')
mkdir -p frontend/src-tauri/binaries
cp target/release/llama-helper "frontend/src-tauri/binaries/llama-helper-${HOST_TARGET}"
```

The helper must match the host architecture for host tests/development. Universal
packaging also needs the opposite-architecture dependencies; follow the candidate
procedure in [testing.md](testing.md) rather than inferring a Universal app from
this single host helper.

## Develop and check

```bash
pnpm --dir frontend tauri:dev
./scripts/check.sh --quick
./scripts/check.sh --full
```

`pnpm --dir frontend dev` starts only the browser frontend; it has no native IPC,
audio or permissions. The Playwright harness supplies explicit synthetic Tauri
fixtures and does not prove native capture or installed offline behavior.

Whisper's macOS dependency enables Metal/Core ML; the local helper's `metal`
feature is selected above. The current manifests do not expose the old
CUDA/Vulkan script commands. See [GPU notes](GPU_ACCELERATION.md).

## Package a candidate

Follow [testing.md](testing.md) for the clean-worktree candidate, exact artifact
SHA-256, verifier, installed smoke report and final release gate. Without a
Developer ID identity, packaging diagnosis uses only `--diagnostic-build` and
produces an UNGATED artifact. An ad-hoc CI `.app` is inspection evidence, not an
approved installer. No upstream download link qualifies a build from this repo.

## Clean generated outputs

After tests stop, `target/`, `frontend/.next/` and `frontend/out/` can be rebuilt.
Inspect each exact path first: retain any required `.app`/DMG/evidence, source,
models, user data and unrelated worktrees. Historical migration scripts that
require old commit refs are archive tools; those refs are absent from the clean
public history.
