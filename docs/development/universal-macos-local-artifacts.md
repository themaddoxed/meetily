> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Local Universal macOS artifacts

The Universal macOS build is intentionally offline and requires the exact
arm64 and x86_64 FFmpeg and ONNX Runtime inputs recorded in committed
manifests. The binary files are not committed. They live in a repository-local
store shared by all worktrees:

```text
$(git rev-parse --git-common-dir)/local-artifacts/universal-macos/v1/
```

Initialize the store once from a directory that already contains all four
verified thin artifacts:

```bash
cd frontend
pnpm run artifacts:init:universal -- /absolute/path/to/universal-artifact-source
```

The initializer rejects missing files, wrong architectures, missing executable
bits on sidecars, and SHA-256 mismatches before copying anything. The required
inputs are:

- `ffmpeg-aarch64-apple-darwin` — `77d2c853f431318d55ec02676d9b2f185ebfdddb9f7677a251fbe453affe025a`
- `ffmpeg-x86_64-apple-darwin` — `430d60fbf419dab28daee9b679e7929a31ee9bae53f6e42e8ae26b725584290f`
- `onnxruntime-aarch64-apple-darwin.a` — `e5c83560aa9e88afa39d9dca9fb5f5a767e28adb5458d1c36fe0357131b6af8b`
- `onnxruntime-x86_64-apple-darwin.a` — `f41370c27dd380419c6cbdfe4f78238613a49506c8f491c0cb07084edac9d561`

In a clean worktree, provision those inputs explicitly and then build:

```bash
cd frontend
pnpm run artifacts:seed:universal
pnpm run tauri:build:universal
```

Provisioning copies through same-directory temporary files, verifies the copy,
and renames it atomically. The Universal build verifies both FFmpeg inputs and
both ONNX Runtime static archives before Cargo starts. It creates a verified
fat ONNX Runtime archive for `ort-sys`, builds both llama-helper thin binaries
offline, verifies their target architectures, and atomically creates the fat
helper. After Tauri bundles the app, the main executable and both embedded
sidecars must all be executable arm64+x86_64 Mach-O binaries before DMG
creation begins.

The local store is not committed or pushed. Back it up with the recovery pack
when preserving this machine's migration state. Normal non-Universal builds do
not use the store and retain their existing FFmpeg fallback behavior.

If a recovery pack contains all four artifacts, restore it into a recreated
repository before seeding a worktree. Older packs containing only FFmpeg are
incomplete and `artifacts:seed:universal` will reject them:

```bash
REC=/Users/developer/Desktop/meetily-recovery-2026-07-12
COMMON_DIR="$(git rev-parse --git-common-dir)"
mkdir -p "$COMMON_DIR/local-artifacts/universal-macos/v1"
cp "$REC"/local-artifacts/universal-macos/v1/* \
  "$COMMON_DIR/local-artifacts/universal-macos/v1/"
chmod 755 "$COMMON_DIR"/local-artifacts/universal-macos/v1/ffmpeg-*-apple-darwin
cd frontend
pnpm run artifacts:seed:universal
```
