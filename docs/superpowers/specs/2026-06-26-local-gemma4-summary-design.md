# Local Gemma 4 Summary Model Design

Date: 2026-06-26

Status: design draft for review
Scope: built-in downloadable Gemma 4 models for local meeting summaries

> **Historical status (2026-07-13):** This is the canonical pre-implementation design record. The Gemma 4 model work and its provider-lifecycle hardening are already migrated on `dev` through code tip `4d8b45a`; final verification record `6af217f` documents the `39/39` Playwright result and supersedes initial record `03ad5b6`. Do not treat the steps below as a current implementation brief. The macOS recommendation remains separately inventoried and not migrated in the [legacy feature registry](../../migration/legacy-feature-registry.md).

## Summary

Add local Gemma 4 summary models to the existing `builtin-ai` flow. Users should see Gemma 4 models in the current Built-in AI model manager, download them with the same button and progress UI used by other built-in models, select them for summary generation, and run summaries without Ollama, API keys, or a custom server.

The key product requirement is stable downloading. Model URLs must use pinned Hugging Face commit URLs instead of branch URLs such as `main`, so a repository update cannot silently change the file that the app downloads.

## Goals

- Add Gemma 4 as built-in downloadable summary models.
- Keep the existing `builtin-ai` provider and UI path.
- Use pinned, verified download URLs for every Gemma 4 GGUF file.
- Preserve current Ollama and custom OpenAI-compatible server behavior.
- Keep Gemma 3 models available as lower-resource fallback options.
- Make recommendation logic prefer Gemma 4 only on machines with enough memory.
- Verify model metadata, download URL reachability, and summary generation behavior.

## Non-goals

- Add a new provider such as `gemma-local`.
- Depend on Ollama for the built-in Gemma 4 path.
- Build a separate model download UI.
- Remove or rename existing Gemma 3 built-in models.
- Implement license-gated or authenticated Hugging Face downloads.
- Add streaming summary output or new prompt-editing features.

## Current project context

The app already has the pieces needed for this feature:

- `frontend/src-tauri/src/summary/summary_engine/models.rs` defines built-in summary model metadata.
- `frontend/src-tauri/src/summary/summary_engine/model_manager.rs` manages model status, download, validation, delete, and list behavior.
- `frontend/src-tauri/src/summary/summary_engine/commands.rs` exposes built-in model commands to the frontend.
- `frontend/src/components/BuiltInModelManager.tsx` renders model cards with `Download`, progress, selection, retry, and delete controls.
- `llama-helper` loads GGUF models through `llama_cpp_2` and receives model-specific prompt formatting, context size, sampling parameters, and stop tokens from the Rust summary engine.

Because this path already works for built-in models, Gemma 4 should be represented as data in the built-in model registry plus narrowly scoped compatibility checks.

## Selected approach

Use the existing `builtin-ai` provider.

The model registry should contain two Gemma 4 entries:

- `gemma4:e2b`: balanced default for capable machines.
- `gemma4:e4b`: higher-quality option for machines with more RAM.

Each entry must define:

- stable internal model name;
- display name;
- local GGUF filename;
- pinned download URL;
- size in MB from the verified `Content-Length`;
- context size;
- layer count;
- prompt template name;
- sampling parameters;
- short UI description.

The current `BuiltInModelManager` should not need a dedicated Gemma 4 UI change. It lists whatever the backend registry returns.

## Stable download URLs

Use pinned Hugging Face commit URLs:

| Model | Repository | Commit | File | Verified content length |
| --- | --- | --- | --- | --- |
| `gemma4:e2b` | `unsloth/gemma-4-E2B-it-GGUF` | `90f9618340396838ee7ff5b0ba2da27da62953d3` | `gemma-4-E2B-it-Q4_K_M.gguf` | `3106736256` bytes |
| `gemma4:e4b` | `ggml-org/gemma-4-E4B-it-GGUF` | `2714b5519c6c3516b1000e7c5e1eba998dfe1fe8` | `gemma-4-E4B-it-Q4_K_M.gguf` | `5335289824` bytes |

Pinned URLs:

```text
https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/90f9618340396838ee7ff5b0ba2da27da62953d3/gemma-4-E2B-it-Q4_K_M.gguf
https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/2714b5519c6c3516b1000e7c5e1eba998dfe1fe8/gemma-4-E4B-it-Q4_K_M.gguf
```

These URLs were checked with `curl -I -L` on 2026-06-26. Both returned `200 OK` after redirect, exposed the expected `X-Repo-Commit`, and exposed the listed `Content-Length`.

## Model recommendation

The recommendation command should prefer models by available RAM:

- macOS with at least 32 GB RAM: recommend `gemma4:e4b`.
- macOS with at least 16 GB RAM: recommend `gemma4:e2b`.
- macOS with more than 8 GB RAM: recommend `gemma3:4b`.
- Otherwise: recommend `gemma3:1b`.

This keeps the default practical on local hardware while still exposing the higher-quality Gemma 4 option.

## Prompting and generation

Gemma 4 should use a model-specific chat template in `models.rs`. If the Gemma 4 template is already present, keep it scoped to the built-in model registry and verify it through summary generation rather than adding another prompt abstraction.

Generation should continue through:

1. frontend summary action;
2. `api_process_transcript`;
3. `SummaryService`;
4. `generate_meeting_summary`;
5. `LLMProvider::BuiltInAI`;
6. `summary_engine::generate_with_builtin`;
7. `llama-helper`.

Long transcripts should continue to use the existing chunking path based on the registered model context size.

## Error handling

Use existing model-manager behavior:

- failed download: model card shows error and retry;
- corrupt GGUF validation: model card shows corrupted state and delete/retry controls;
- missing local file: summary generation returns a clear "download the model first" style error;
- unavailable network: download command surfaces the request error through the existing progress/error channel.

No new error channel is needed for this feature.

## Testing and verification

Required verification before implementation is considered complete:

- Unit-level check that the built-in registry returns both Gemma 4 models with the expected names, filenames, URLs, and sizes.
- URL verification for both pinned URLs with `curl -I -L`.
- Rust check/test for `frontend/src-tauri` after registry changes.
- Frontend type/lint check if UI types are touched.
- Manual or smoke generation check with at least one small transcript using a downloaded Gemma 4 model, if model download time and disk space are acceptable in the environment.

If full model download is not practical during development, the implementation must still verify URL reachability and registry behavior, and document that generation was not run locally.

## Acceptance criteria

- `Built-in AI Models` lists Gemma 4 model cards.
- Each Gemma 4 card has a working `Download` button using a pinned URL.
- Download progress, retry, delete, and ready states behave like existing built-in models.
- Selecting a downloaded Gemma 4 model saves the summary model config.
- Summary generation routes through `builtin-ai`, not Ollama or `custom-openai`.
- Recommendation logic selects Gemma 4 only on machines with sufficient RAM.
- Existing Gemma 3, Ollama, and cloud provider behavior remains unchanged.

## Risks

- Hugging Face public unauthenticated downloads may be rate-limited. Pinned URLs provide stable content identity, not guaranteed bandwidth.
- GGUF files are large enough that development verification may need to stop at URL and registry checks unless a full download is acceptable.
- If `llama_cpp_2` lacks support for a specific Gemma 4 GGUF encoding detail, the registry can be correct while runtime loading still fails. This must be caught by a smoke generation test when possible.
