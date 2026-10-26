# Frontend checks on a non-product host

This file preserves an old documentation URL. The current desktop product target
is macOS; it does not provide an accepted desktop build on this host.

CI uses an Ubuntu runner for source integrity, TypeScript/frontend contracts and
static export. These checks do not exercise native audio or Tauri packaging.
See [current build instructions](BUILDING.md), [testing](testing.md), and
[workflow definitions](../.github/workflows/quality-gate.yml).

The former CUDA/Vulkan/AppImage and Python-server instructions do not describe
this source snapshot and have been removed from the active operator guide.
