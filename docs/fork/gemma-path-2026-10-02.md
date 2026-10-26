# Gemma source and downloader verification — 2026-10-02

Scope: FORK-133 on `main@53f4415058fb01e38b8892a47ac036bfffa394ba`.
This is source and isolated-fixture evidence. It does not accept FORK-040,
FORK-134, FORK-167, or an installed application release.

## Source applicability

The local Gemma selection/helper fix
`44d54273264b98ff6cdd5354191bcd5b9264c2d8` and macOS recommendation change
`ee8ae69e02c41c532ee7b41791ee31c608822428` are ancestors of this main commit
(`git merge-base --is-ancestor`: exit 0 for each). No additional transfer is
needed. This does not claim fresh GitHub PR metadata or upstream merge status.
The [census](evidence/gemma-path-2026-10-02.json) binds inspected files to main
by SHA-256 and byte comparison with `git show`.

## Existing path

1. The backend catalogue declares `gemma4:e2b` and `gemma4:e4b`; both use the
   Gemma 4 turn template and pinned model URLs.
2. `BuiltInModelManager` obtains the backend catalogue, selects its model name,
   and invokes `builtin_ai_download_model`. `ModelSettingsModal` retains that
   name; `SummaryModelSettings` saves it through `api_save_model_config` and
   the native settings repository. No Gemma-specific downloader is required.
3. `ModelManager::download_model_detailed` resolves `summary_manifest`, calls
   the shared FEAT-070 downloader, and exposes `Available` only after verified
   installation and integrity-cache registration. Cancellation and a competing
   request retain their existing status rules.
4. The generation client verifies the selected file against the manifest again,
   formats the model-specific prompt, and starts the bundled helper with that
   model path. Source inspection establishes routing, not successful inference.

The local manifests specify:

| Model | Repository revision | Bytes | SHA-256 |
|---|---|---:|---|
| E2B | `unsloth/gemma-4-E2B-it-GGUF@90f9618340396838ee7ff5b0ba2da27da62953d3` | 3,106,736,256 | `9378bc471710229ef165709b62e34bfb62231420ddaf6d729e727305b5b8672d` |
| E4B | `ggml-org/gemma-4-E4B-it-GGUF@2714b5519c6c3516b1000e7c5e1eba998dfe1fe8` | 5,335,289,824 | `90ce98129eb3e8cc57e62433d500c97c624b1e3af1fcc85dd3b55ad7e0313e9f` |

These are checked-in values. Actual remote weight bytes and current remote
availability have not been verified in this audit.

## Focused verification

Tests run in the preserved integration candidate with matching source files,
its exact installed frontend dependencies, and existing offline Rust caches.
Logs and actual counts are recorded in the census. The shared downloader's
15 passing tests cover size/hash rejection, preservation of an old file when
replacement fails, atomic bundle publication, interrupted transfer/resume,
malformed ranges, cancellation, and per-model serialization. They use small
fixtures, not multi-gigabyte Gemma weights.

The selection/configuration Node run passed 12 tests, including selectable
undownloaded Gemma models, preference for an available model, rejected error
states, model-size labels, and effect cleanup. These do not prove persistence
in an installed WebView profile. The Rust Gemma filter passed 7 tests: pinned catalogue metadata, Gemma 4
turn formatting and escaping, model priority, RAM recommendation branches,
and retained Gemma 3 behavior. The existing onboarding browser fixture passed
1 test: the recommended E2B entry is visible and selectable. Its first launch
failed before the scenario because the default browser cache was absent; the
retry used the already installed Chromium cache. No browser was installed.

## Remaining acceptance

- FORK-134 must prove real Intel generation, saved selection after reopen,
  offline operation, the helper/model pairing, and actual RAM behavior.
- FORK-167 must prove the same runtime on physical Apple Silicon.
- The 24 GiB recommendation boundary is a code heuristic, not a benchmark.
- Existing opt-in model smoke requires explicitly supplied local model paths
  and fixtures. It was not run here; no user recording or profile was accessed.
- FORK-040 remains open until its native children and original dependencies
  meet their acceptance criteria. Fixture success is not release readiness.
