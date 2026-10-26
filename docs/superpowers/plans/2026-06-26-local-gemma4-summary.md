# Local Gemma 4 Summary Implementation Plan

> **Historical status (2026-07-13):** This plan is preserved as the canonical pre-migration execution record. Its implementation and provider-lifecycle hardening are already present on `dev` through code tip `4d8b45a`; final verification record `6af217f` documents the `39/39` Playwright result and supersedes initial record `03ad5b6`. Do not re-run these tasks as a current plan. The macOS recommendation remains separately inventoried and not migrated in the [legacy feature registry](../../migration/legacy-feature-registry.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Gemma 4 built-in summary model downloads stable by pinning E2B and E4B GGUF URLs to verified Hugging Face commits and proving the registry metadata with tests.

**Architecture:** The existing `builtin-ai` provider already drives built-in model listing, download, selection, and summary generation. This implementation keeps that architecture intact and changes only the model registry data plus focused Rust tests for the Gemma 4 metadata and prompt template.

**Tech Stack:** Rust, Tauri, `llama_cpp_2` via `llama-helper`, existing built-in summary model manager, Hugging Face pinned `resolve/<commit>/...` downloads.

---

## File Structure

- Modify: `frontend/src-tauri/src/summary/summary_engine/models.rs`
  - Owns built-in summary model definitions and prompt templates.
  - Add tests for Gemma 4 model registry metadata.
  - Replace `resolve/main` Gemma 4 download URLs with pinned commit URLs.
- No frontend files should change.
  - `frontend/src/components/BuiltInModelManager.tsx` already lists backend registry entries.
  - Keeping UI untouched preserves existing download/progress/retry/delete behavior.
- No database migrations should change.
  - Summary model config already stores provider/model strings.

## Task 1: Add Failing Registry Tests

**Files:**
- Modify: `frontend/src-tauri/src/summary/summary_engine/models.rs`

- [ ] **Step 1: Add tests at the end of `models.rs`**

Append this module after the `GENERATION_TIMEOUT_SECS` constant:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemma4_models_use_pinned_download_urls_and_verified_metadata() {
        let e2b = get_model_by_name("gemma4:e2b").expect("gemma4:e2b should be registered");
        assert_eq!(e2b.display_name, "Gemma 4 E2B (Enhanced)");
        assert_eq!(e2b.gguf_file, "gemma-4-E2B-it-Q4_K_M.gguf");
        assert_eq!(e2b.template, "gemma4");
        assert_eq!(e2b.size_mb, 2963);
        assert_eq!(e2b.context_size, 32768);
        assert_eq!(e2b.layer_count, 35);
        assert_eq!(
            e2b.download_url,
            "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/90f9618340396838ee7ff5b0ba2da27da62953d3/gemma-4-E2B-it-Q4_K_M.gguf"
        );
        assert!(!e2b.download_url.contains("/resolve/main/"));

        let e4b = get_model_by_name("gemma4:e4b").expect("gemma4:e4b should be registered");
        assert_eq!(e4b.display_name, "Gemma 4 E4B (High Quality)");
        assert_eq!(e4b.gguf_file, "gemma-4-E4B-it-Q4_K_M.gguf");
        assert_eq!(e4b.template, "gemma4");
        assert_eq!(e4b.size_mb, 5088);
        assert_eq!(e4b.context_size, 32768);
        assert_eq!(e4b.layer_count, 42);
        assert_eq!(
            e4b.download_url,
            "https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/2714b5519c6c3516b1000e7c5e1eba998dfe1fe8/gemma-4-E4B-it-Q4_K_M.gguf"
        );
        assert!(!e4b.download_url.contains("/resolve/main/"));
    }

    #[test]
    fn gemma4_prompt_template_uses_system_and_user_turns() {
        let prompt = format_prompt("gemma4", "Summarize carefully.", "Transcript text").unwrap();

        assert!(prompt.starts_with("<start_of_turn>system\nSummarize carefully.<end_of_turn>"));
        assert!(prompt.contains("<start_of_turn>user\nTranscript text<end_of_turn>"));
        assert!(prompt.ends_with("<start_of_turn>model\n"));
    }

    #[test]
    fn built_in_registry_keeps_gemma3_fallbacks_with_gemma4_models() {
        let names: Vec<String> = get_available_models()
            .into_iter()
            .map(|model| model.name)
            .collect();

        assert_eq!(
            names,
            vec![
                "gemma3:1b".to_string(),
                "gemma3:4b".to_string(),
                "gemma4:e2b".to_string(),
                "gemma4:e4b".to_string(),
            ]
        );
    }
}
```

- [ ] **Step 2: Run the focused failing test**

Run:

```bash
cd frontend/src-tauri
cargo test summary::summary_engine::models::tests::gemma4_models_use_pinned_download_urls_and_verified_metadata --lib
```

Expected: FAIL. The failure should show that `gemma4:e2b` and/or `gemma4:e4b` still use a URL containing `/resolve/main/`.

## Task 2: Pin Gemma 4 Download URLs

**Files:**
- Modify: `frontend/src-tauri/src/summary/summary_engine/models.rs`

- [ ] **Step 1: Replace the Gemma 4 E2B URL**

In the `ModelDef` for `name: "gemma4:e2b"`, replace:

```rust
download_url: "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf".to_string(),
```

with:

```rust
download_url: "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/90f9618340396838ee7ff5b0ba2da27da62953d3/gemma-4-E2B-it-Q4_K_M.gguf".to_string(),
```

- [ ] **Step 2: Replace the Gemma 4 E4B URL**

In the `ModelDef` for `name: "gemma4:e4b"`, replace:

```rust
download_url: "https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
```

with:

```rust
download_url: "https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/2714b5519c6c3516b1000e7c5e1eba998dfe1fe8/gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
```

- [ ] **Step 3: Run the model registry tests**

Run:

```bash
cd frontend/src-tauri
cargo test summary::summary_engine::models::tests --lib
```

Expected: PASS. The Gemma 4 metadata test should pass, and the prompt template test should pass without additional implementation changes.

- [ ] **Step 4: Commit the tests and URL fix**

Run:

```bash
git add frontend/src-tauri/src/summary/summary_engine/models.rs
git commit -m "fix: pin Gemma 4 built-in model downloads"
```

## Task 3: Verify Pinned URL Reachability

**Files:**
- No file changes.

- [ ] **Step 1: Verify the E2B pinned URL**

Run:

```bash
curl -I -L --max-time 30 https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/90f9618340396838ee7ff5b0ba2da27da62953d3/gemma-4-E2B-it-Q4_K_M.gguf
```

Expected:

```text
HTTP/2 200
x-repo-commit: 90f9618340396838ee7ff5b0ba2da27da62953d3
content-length: 3106736256
```

- [ ] **Step 2: Verify the E4B pinned URL**

Run:

```bash
curl -I -L --max-time 30 https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/2714b5519c6c3516b1000e7c5e1eba998dfe1fe8/gemma-4-E4B-it-Q4_K_M.gguf
```

Expected:

```text
HTTP/2 200
x-repo-commit: 2714b5519c6c3516b1000e7c5e1eba998dfe1fe8
content-length: 5335289824
```

## Task 4: Run Final Rust Verification

**Files:**
- No file changes.

- [ ] **Step 1: Run all summary engine library tests**

Run:

```bash
cd frontend/src-tauri
cargo test summary::summary_engine --lib
```

Expected: PASS for the summary engine tests.

- [ ] **Step 2: Run a library check**

Run:

```bash
cd frontend/src-tauri
cargo check --lib
```

Expected: PASS with no compile errors.

- [ ] **Step 3: Confirm no frontend files changed in the implementation commit**

Run:

```bash
git show --name-only --format= HEAD
```

Expected:

```text
frontend/src-tauri/src/summary/summary_engine/models.rs
```

No React or TypeScript files should appear, so no frontend lint/typecheck is needed for this change.

## Task 5: Document Verification Limits

**Files:**
- No file changes unless final execution notes are being added to a PR or release checklist.

- [ ] **Step 1: Record whether full GGUF download was run**

Use this exact completion note in the implementation summary:

```text
Verified pinned Gemma 4 URLs with curl HEAD requests and Rust registry tests. Full GGUF download and local generation smoke test were not run in this pass because the files are 2.9 GiB and 5.0 GiB; release validation should download at least gemma4:e2b and generate a short summary before packaging.
```

If a full E2B download and smoke generation are run during execution, replace the note with:

```text
Verified pinned Gemma 4 URLs with curl HEAD requests, Rust registry tests, and a local gemma4:e2b summary smoke test.
```

## Self-Review

- Spec coverage: The plan pins both Gemma 4 download URLs, preserves `builtin-ai`, keeps Gemma 3 fallbacks, verifies metadata, verifies URL reachability, and avoids frontend/provider changes.
- Placeholder scan: The plan contains exact file paths, commands, expected results, and code snippets. It does not require unspecified implementation work.
- Type consistency: Test code uses existing `ModelDef`, `get_model_by_name`, `get_available_models`, and `format_prompt` APIs from `models.rs`.
