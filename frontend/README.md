# Meetily frontend

Next.js/React/TypeScript supplies the interface of the macOS Tauri/Rust app.
Packaged builds use the static export in `frontend/out`; `pnpm dev` is development
only. Browser fixtures provide synthetic IPC for automated UI tests and do not
prove native audio, TCC or installed offline behavior.

Use Node.js 22.17.0 and pnpm 10.17.1, as pinned in CI. From the repository root:

```bash
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend lint
pnpm --dir frontend typecheck
pnpm --dir frontend test:frontend
```

For the host helper, Xcode, native development and candidate packaging, follow
[BUILDING.md](../docs/BUILDING.md) and [testing.md](../docs/testing.md).
Local transcription stays on the device; selected remote summary/Q&A providers
receive meeting text. See [PRIVACY_POLICY.md](../PRIVACY_POLICY.md).

`src/` contains the UI; `src-tauri/` contains native commands, audio, storage and
model integration; `tests/` contains Node, Vitest and Playwright checks.
The app has no Electron or Python/FastAPI runtime.
