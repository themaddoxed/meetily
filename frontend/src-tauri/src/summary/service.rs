use crate::database::repositories::{
    meeting::MeetingsRepository,
    setting::SettingsRepository,
    summary::{SummaryProcessesRepository, SummaryProcessingMetrics},
    transcript_chunk::TranscriptChunksRepository,
};
use crate::ollama::metadata::ModelMetadataCache;
use crate::summary::language_detection::detect_summary_language;
use crate::summary::llm_client::LLMProvider;
use crate::summary::metadata::read_detected_summary_language_from_metadata;
use crate::summary::persistence::save_summary_markdown;
use crate::summary::processor::{
    extract_meeting_name_from_markdown, generate_meeting_summary, language_name_from_code,
};
use crate::summary::templates::{self, Template};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

// Global cache for model metadata (5 minute TTL)
static METADATA_CACHE: Lazy<ModelMetadataCache> =
    Lazy::new(|| ModelMetadataCache::new(Duration::from_secs(300)));

// Global registry for cancellation tokens (thread-safe)
type CancellationRegistry = Arc<Mutex<HashMap<(String, String), CancellationToken>>>;
static CANCELLATION_REGISTRY: Lazy<CancellationRegistry> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Short critical sections serialize DB/file publication for a meeting without
// holding a lock during provider execution.
static MEETING_OPERATION_LOCKS: Lazy<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct CancellationRegistration {
    meeting_id: String,
    lifecycle_run_id: String,
}

impl Drop for CancellationRegistration {
    fn drop(&mut self) {
        SummaryService::cleanup_cancellation_token(&self.meeting_id, &self.lifecycle_run_id);
    }
}

/// Strips the first `#` heading line; returns "" if no `#` is found.
fn strip_leading_title(markdown: &str) -> String {
    if let Some(hash_pos) = markdown.find('#') {
        let body_start = markdown[hash_pos..]
            .find('\n')
            .map_or(markdown.len(), |line_end| hash_pos + line_end);
        markdown[body_start..].trim_start().to_string()
    } else {
        String::new()
    }
}

/// Strips the leading H1 (`# Title\n...`) only when the markdown starts with one.
/// No-op on already-stripped values, values starting with `## Subheading`, or values
/// without any heading. Avoids the silent-empty-return case where `strip_leading_title`
/// returns "" for input lacking a leading `#`.
fn strip_title_if_present(markdown: &str) -> String {
    if markdown.trim_start().starts_with("# ") {
        strip_leading_title(markdown)
    } else {
        markdown.to_string()
    }
}

const ENGLISH_CACHE_FIELD: &str = "english_cache";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SummaryCacheSource {
    transcript_fingerprint: String,
    custom_prompt_fingerprint: String,
    system_prompt_fingerprint: String,
    template_id: String,
    template_fingerprint: String,
    token_threshold: usize,
    model_provider: String,
    model_name: String,
    ollama_endpoint: Option<String>,
    custom_openai_endpoint: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct EnglishSummaryCache {
    markdown: String,
    source: SummaryCacheSource,
    output_language: Option<String>,
}

fn stable_text_fingerprint(text: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{:016x}:{}", hash, text.len())
}

#[allow(clippy::too_many_arguments)]
fn build_summary_cache_source(
    text: &str,
    custom_prompt: &str,
    system_prompt: &str,
    template_id: &str,
    template_fingerprint: &str,
    token_threshold: usize,
    model_provider: &str,
    model_name: &str,
    ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
) -> SummaryCacheSource {
    SummaryCacheSource {
        transcript_fingerprint: stable_text_fingerprint(text),
        custom_prompt_fingerprint: stable_text_fingerprint(custom_prompt),
        system_prompt_fingerprint: stable_text_fingerprint(system_prompt),
        template_id: template_id.to_string(),
        template_fingerprint: template_fingerprint.to_string(),
        token_threshold,
        model_provider: model_provider.to_string(),
        model_name: model_name.to_string(),
        ollama_endpoint: ollama_endpoint.map(str::to_string),
        custom_openai_endpoint: custom_openai_endpoint.map(str::to_string),
        max_tokens,
        temperature,
        top_p,
    }
}

fn template_cache_fingerprint(template: &Template) -> String {
    let rendered_template = format!(
        "{}\n---SECTION-INSTRUCTIONS---\n{}",
        template.to_markdown_structure(),
        template.to_section_instructions()
    );
    stable_text_fingerprint(&rendered_template)
}

fn normalise_summary_language_for_cache(summary_language: Option<&str>) -> Option<String> {
    language_name_from_code(summary_language?.trim()).map(str::to_string)
}

fn build_summary_result_json(
    final_markdown: &str,
    english_markdown: &str,
    source: SummaryCacheSource,
    output_language: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "markdown": strip_title_if_present(final_markdown),
        "export_markdown": final_markdown,
        ENGLISH_CACHE_FIELD: EnglishSummaryCache {
            markdown: english_markdown.to_string(),
            source,
            output_language: normalise_summary_language_for_cache(output_language),
        },
    })
}

/// Parses a `summary_processes.result` JSON blob and extracts a cached English
/// summary only when it was produced from exactly the same source inputs and
/// the user is switching to a different non-English target language.
fn extract_cached_english_markdown(
    raw: &str,
    expected_source: &SummaryCacheSource,
    requested_language: Option<&str>,
) -> Result<Option<String>, serde_json::Error> {
    let requested_language = match normalise_summary_language_for_cache(requested_language) {
        Some(language) if language != "English" => language,
        _ => return Ok(None),
    };

    let value: serde_json::Value = serde_json::from_str(raw)?;
    let Some(cache_value) = value.get(ENGLISH_CACHE_FIELD) else {
        return Ok(None);
    };

    let cache: EnglishSummaryCache = match serde_json::from_value(cache_value.clone()) {
        Ok(cache) => cache,
        Err(_) => return Ok(None),
    };

    if cache.source != *expected_source {
        return Ok(None);
    }

    if cache.output_language.as_deref() == Some(requested_language.as_str()) {
        return Ok(None);
    }

    let markdown = cache.markdown.trim();
    if markdown.is_empty() {
        Ok(None)
    } else {
        Ok(Some(cache.markdown))
    }
}

/// Summary service - handles all summary generation logic
pub struct SummaryService;

impl SummaryService {
    /// Lock state changes and publication for one meeting across command tasks.
    pub async fn lock_meeting_operation(meeting_id: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = MEETING_OPERATION_LOCKS
                .lock()
                .expect("meeting operation lock registry poisoned");
            locks
                .entry(meeting_id.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }

    /// Claims a generation attempt, registers its exact cancellation token, and
    /// persists transcript inputs before a worker can be spawned.
    pub async fn prepare_summary_attempt(
        pool: &SqlitePool,
        meeting_id: &str,
        transcript: &str,
        model: &str,
        model_name: &str,
        chunk_size: i32,
        overlap: i32,
    ) -> Result<(String, CancellationToken), String> {
        let _operation_guard = Self::lock_meeting_operation(meeting_id).await;
        let (lifecycle_run_id, replaced_run_id) =
            SummaryProcessesRepository::start_process(pool, meeting_id)
                .await
                .map_err(|error| format!("Failed to initialize process: {}", error))?;
        Self::cancel_replaced_run(meeting_id, replaced_run_id.as_deref());
        let cancellation_token = Self::register_cancellation_token(meeting_id, &lifecycle_run_id);

        if let Err(error) = TranscriptChunksRepository::save_transcript_data(
            pool, meeting_id, transcript, model, model_name, chunk_size, overlap,
        )
        .await
        {
            let message = format!("Failed to save transcript data: {}", error);
            if let Err(status_error) =
                Self::fail_summary_attempt_locked(pool, meeting_id, &lifecycle_run_id, &message)
                    .await
            {
                error!(
                    "{}; failed to mark attempt failed: {}",
                    message, status_error
                );
            }
            return Err(message);
        }

        Ok((lifecycle_run_id, cancellation_token))
    }

    /// Terminalizes an early provider/setup failure and removes only that run's
    /// cancellation token. Background setup errors and transcript persistence
    /// errors use this same owner-checked path.
    pub async fn fail_summary_attempt(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        error_message: &str,
    ) -> Result<(), sqlx::Error> {
        let _operation_guard = Self::lock_meeting_operation(meeting_id).await;
        Self::fail_summary_attempt_locked(pool, meeting_id, lifecycle_run_id, error_message).await
    }

    /// Cancels the currently owned summary run in SQLite and then signals only
    /// the token returned by that atomic owner check.
    pub async fn cancel_current_summary_attempt(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let _operation_guard = Self::lock_meeting_operation(meeting_id).await;
        let run_id = SummaryProcessesRepository::cancel_current_process(pool, meeting_id).await?;
        if let Some(run_id) = run_id.as_deref() {
            Self::cancel_summary(meeting_id, run_id);
        }
        Ok(run_id)
    }

    async fn fail_summary_attempt_locked(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        error_message: &str,
    ) -> Result<(), sqlx::Error> {
        let result = SummaryProcessesRepository::update_process_failed_for_run(
            pool,
            meeting_id,
            lifecycle_run_id,
            error_message,
        )
        .await;
        Self::cleanup_cancellation_token(meeting_id, lifecycle_run_id);
        result
    }

    /// Registers the cancellation token for one exact attempt.
    pub fn register_cancellation_token(
        meeting_id: &str,
        lifecycle_run_id: &str,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        if let Ok(mut registry) = CANCELLATION_REGISTRY.lock() {
            registry.insert(
                (meeting_id.to_string(), lifecycle_run_id.to_string()),
                token.clone(),
            );
            info!(
                "Registered cancellation token for meeting {} run {}",
                meeting_id, lifecycle_run_id
            );
        }
        token
    }

    /// Cancels only the provider call owned by the supplied attempt.
    pub fn cancel_summary(meeting_id: &str, lifecycle_run_id: &str) -> bool {
        if let Ok(registry) = CANCELLATION_REGISTRY.lock() {
            if let Some(token) =
                registry.get(&(meeting_id.to_string(), lifecycle_run_id.to_string()))
            {
                info!(
                    "Cancelling summary generation for meeting {} run {}",
                    meeting_id, lifecycle_run_id
                );
                token.cancel();
                return true;
            }
        }
        warn!(
            "No active summary generation found for meeting {} run {}",
            meeting_id, lifecycle_run_id
        );
        false
    }

    /// Cleans up only the token for the completed attempt.
    pub fn cleanup_cancellation_token(meeting_id: &str, lifecycle_run_id: &str) {
        if let Ok(mut registry) = CANCELLATION_REGISTRY.lock() {
            if registry
                .remove(&(meeting_id.to_string(), lifecycle_run_id.to_string()))
                .is_some()
            {
                info!(
                    "Cleaned up cancellation token for meeting {} run {}",
                    meeting_id, lifecycle_run_id
                );
            }
        }
    }

    pub fn cancel_replaced_run(meeting_id: &str, replaced_run_id: Option<&str>) {
        if let Some(run_id) = replaced_run_id {
            Self::cancel_summary(meeting_id, run_id);
        }
    }

    /// Atomically accept a generated result for its current owner, then publish
    /// the accepted Markdown view. A stale owner returns `Ok(false)` and never
    /// touches the file or title.
    pub async fn accept_generated_summary(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        result: serde_json::Value,
        markdown: &str,
        metrics: SummaryProcessingMetrics,
        suggested_title: Option<&str>,
    ) -> Result<bool, String> {
        let _operation_guard = Self::lock_meeting_operation(meeting_id).await;
        let export_markdown = result
            .get("export_markdown")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(markdown)
            .to_owned();
        let active =
            SummaryProcessesRepository::is_run_processing(pool, meeting_id, lifecycle_run_id)
                .await
                .map_err(|error| format!("Failed to verify summary owner: {}", error))?;
        if !active {
            return Ok(false);
        }

        let has_recording_folder =
            match MeetingsRepository::get_meeting_metadata(pool, meeting_id).await {
                Ok(Some(meeting)) => meeting
                    .folder_path
                    .is_some_and(|path| !path.trim().is_empty()),
                Ok(None) => false,
                Err(error) => {
                    let message = format!("Failed to load recording folder: {}", error);
                    let _ = SummaryProcessesRepository::update_process_failed_for_run(
                        pool,
                        meeting_id,
                        lifecycle_run_id,
                        &message,
                    )
                    .await;
                    return Err(message);
                }
            };

        match SummaryProcessesRepository::accept_process_completed_for_run(
            pool,
            meeting_id,
            lifecycle_run_id,
            result,
            metrics,
            suggested_title,
            has_recording_folder,
        )
        .await
        {
            Ok(()) => {}
            Err(sqlx::Error::RowNotFound) => return Ok(false),
            Err(error) => {
                let message = format!("Failed to accept completed summary: {}", error);
                let _ = SummaryProcessesRepository::update_process_failed_for_run(
                    pool,
                    meeting_id,
                    lifecycle_run_id,
                    &message,
                )
                .await;
                return Err(message);
            }
        }

        if let Err(error) =
            Self::publish_summary_for_run(pool, meeting_id, lifecycle_run_id, &export_markdown)
                .await
        {
            warn!(
                "Accepted summary for {} but export remains pending: {}",
                meeting_id, error
            );
        }
        Ok(true)
    }

    /// Publish an already accepted database result to the recoverable Markdown
    /// file. Callers hold the meeting operation lock across this method.
    pub async fn publish_summary_for_run(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        markdown: &str,
    ) -> Result<(), String> {
        let folder = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
            .await
            .map_err(|error| format!("Failed to load recording folder: {}", error))?
            .and_then(|meeting| meeting.folder_path)
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from);

        let Some(folder) = folder else {
            // A meeting without a recording folder stores its canonical summary
            // in SQLite only; no file export is pending.
            SummaryProcessesRepository::clear_export_pending_for_run(
                pool,
                meeting_id,
                lifecycle_run_id,
            )
            .await
            .map_err(|error| format!("Failed to clear summary export state: {}", error))?;
            return Ok(());
        };

        let destination = folder.join("summary.md");
        let already_current = match std::fs::read(&destination) {
            Ok(current) => current == markdown.as_bytes(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                let reason = format!("Failed to read summary.md: {}", error);
                let _ = SummaryProcessesRepository::mark_export_pending_for_run(
                    pool,
                    meeting_id,
                    lifecycle_run_id,
                    &reason,
                )
                .await;
                return Err(reason);
            }
        };
        if !already_current {
            if let Err(error) = save_summary_markdown(&folder, markdown) {
                let _ = SummaryProcessesRepository::mark_export_pending_for_run(
                    pool,
                    meeting_id,
                    lifecycle_run_id,
                    &error,
                )
                .await;
                return Err(error);
            }
        }

        SummaryProcessesRepository::clear_export_pending_for_run(
            pool,
            meeting_id,
            lifecycle_run_id,
        )
        .await
        .map_err(|error| format!("Failed to clear summary export state: {}", error))?;
        Ok(())
    }

    async fn publish_terminal_summary_without_owner(
        pool: &SqlitePool,
        meeting_id: &str,
        markdown: &str,
    ) -> Result<(), String> {
        let folder = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
            .await
            .map_err(|error| format!("Failed to load recording folder: {}", error))?
            .and_then(|meeting| meeting.folder_path)
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from);

        if let Some(folder) = folder {
            let destination = folder.join("summary.md");
            let already_current = match std::fs::read(&destination) {
                Ok(current) => current == markdown.as_bytes(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(format!("Failed to read summary.md: {}", error)),
            };
            if !already_current {
                save_summary_markdown(&folder, markdown)?;
            }
        }

        SummaryProcessesRepository::clear_terminal_export_pending_without_owner(pool, meeting_id)
            .await
            .map_err(|error| format!("Failed to clear summary export state: {}", error))?;
        Ok(())
    }

    /// On summary reads, restore the file view from the accepted SQLite value.
    /// This also retries exports that were pending when the app last closed.
    pub async fn reconcile_accepted_summary(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(), String> {
        let _operation_guard = Self::lock_meeting_operation(meeting_id).await;
        let Some(process) = SummaryProcessesRepository::get_summary_data(pool, meeting_id)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        if !matches!(
            process.status.as_str(),
            "completed" | "failed" | "cancelled"
        ) {
            return Ok(());
        }
        let lifecycle_run_id = MeetingsRepository::get_lifecycle_run_id(
            pool,
            meeting_id,
            crate::database::repositories::meeting::LIFECYCLE_OPERATION_SUMMARY,
        )
        .await
        .map_err(|error| error.to_string())?;
        if lifecycle_run_id.is_none()
            && !process
                .error
                .as_deref()
                .is_some_and(|error| error.starts_with("summary_export_pending:"))
        {
            return Ok(());
        }
        let Some((mut markdown, has_canonical_export)) = process
            .result
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|value| {
                value
                    .get("export_markdown")
                    .and_then(serde_json::Value::as_str)
                    .map(|markdown| (markdown.to_owned(), true))
                    .or_else(|| {
                        value
                            .get("markdown")
                            .and_then(serde_json::Value::as_str)
                            .map(|markdown| (markdown.to_owned(), false))
                    })
            })
        else {
            return Ok(());
        };
        if !has_canonical_export {
            // Older versions stored display Markdown without its leading H1,
            // while summary.md retained the full export. Preserve that title
            // only when the remaining file body still matches SQLite exactly.
            let folder = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
                .await
                .map_err(|error| error.to_string())?
                .and_then(|meeting| meeting.folder_path)
                .filter(|path| !path.trim().is_empty())
                .map(PathBuf::from);
            if let Some(folder) = folder {
                if let Ok(existing) = std::fs::read_to_string(folder.join("summary.md")) {
                    if strip_title_if_present(&existing) == markdown {
                        markdown = existing;
                    }
                }
            }
        }
        if let Some(lifecycle_run_id) = lifecycle_run_id {
            Self::publish_summary_for_run(pool, meeting_id, &lifecycle_run_id, &markdown).await
        } else {
            Self::publish_terminal_summary_without_owner(pool, meeting_id, &markdown).await
        }
    }

    async fn read_detected_summary_language(pool: &SqlitePool, meeting_id: &str) -> Option<String> {
        let meeting = match MeetingsRepository::get_meeting_metadata(pool, meeting_id).await {
            Ok(Some(meeting)) => meeting,
            Ok(None) => {
                warn!(
                    "Meeting not found while reading detected summary language: {}",
                    meeting_id
                );
                return None;
            }
            Err(e) => {
                warn!(
                    "Failed to read meeting metadata for detected summary language (meeting_id={}): {}",
                    meeting_id, e
                );
                return None;
            }
        };

        let folder_path = meeting.folder_path.filter(|p| !p.trim().is_empty())?;

        match read_detected_summary_language_from_metadata(Path::new(&folder_path)) {
            Ok(language) => language,
            Err(e) => {
                warn!(
                    "Failed to read detected summary language metadata for meeting_id={}: {}",
                    meeting_id, e
                );
                None
            }
        }
    }

    fn detect_summary_language_from_text(text: &str) -> Option<String> {
        let transcript_texts = [text.to_string()];
        let detection = detect_summary_language(&transcript_texts);
        match &detection.language {
            Some(language) => {
                info!(
                    "Detected transcript summary language for normalization: {}",
                    language
                );
            }
            None => {
                info!(
                    "Transcript summary language unknown for normalization: {:?}",
                    detection.reason
                );
            }
        }
        detection.language
    }

    /// Processes transcript in the background and generates summary
    ///
    /// This function is designed to be spawned as an async task and does not block
    /// the main thread. It updates the database with progress and results.
    ///
    /// # Arguments
    /// * `_app` - Tauri app handle (for future use)
    /// * `pool` - SQLx connection pool
    /// * `meeting_id` - Unique identifier for the meeting
    /// * `text` - Full transcript text
    /// * `model_provider` - LLM provider name (e.g., "ollama", "openai")
    /// * `model_name` - Specific model (e.g., "gpt-4", "llama3.2:latest")
    /// * `custom_prompt` - Optional user-provided context
    /// * `system_prompt` - Persistent system-level summary instructions
    /// * `template_id` - Template identifier (e.g., "daily_standup", "standard_meeting")
    // CLIPPY-JUSTIFICATION: Keep provider and template fields explicit in the stable background request schema.
    #[allow(clippy::too_many_arguments)]
    pub async fn process_transcript_background<R: tauri::Runtime>(
        _app: AppHandle<R>,
        pool: SqlitePool,
        meeting_id: String,
        text: String,
        model_provider: String,
        model_name: String,
        custom_prompt: String,
        system_prompt: String,
        template_id: String,
        summary_language: Option<String>,
        lifecycle_run_id: String,
        cancellation_token: CancellationToken,
    ) {
        let _registration = CancellationRegistration {
            meeting_id: meeting_id.clone(),
            lifecycle_run_id: lifecycle_run_id.clone(),
        };
        let start_time = Instant::now();
        info!(
            "Starting background processing for meeting_id: {}",
            meeting_id
        );

        // Cancellation may have won immediately after attempt creation. Check
        // durable ownership before performing any provider setup or work.
        {
            let _operation_guard = Self::lock_meeting_operation(&meeting_id).await;
            match SummaryProcessesRepository::is_run_processing(
                &pool,
                &meeting_id,
                &lifecycle_run_id,
            )
            .await
            {
                Ok(true) => {}
                Ok(false) => return,
                Err(error) => {
                    error!(
                        "Failed to verify summary run owner for {}: {}",
                        meeting_id, error
                    );
                    return;
                }
            }
        }

        // Parse provider
        let provider = match model_provider.parse::<LLMProvider>() {
            Ok(p) => p,
            Err(e) => {
                Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, &e).await;
                return;
            }
        };

        // Validate and setup api_key, Flexible for Ollama, BuiltInAI, and CustomOpenAI
        let api_key = if provider == LLMProvider::Ollama
            || provider == LLMProvider::BuiltInAI
            || provider == LLMProvider::CustomOpenAI
        {
            // These providers don't require API keys from the standard database column
            String::new()
        } else {
            match SettingsRepository::get_api_key(&pool, &model_provider).await {
                Ok(Some(key)) if !key.is_empty() => key,
                Ok(None) | Ok(Some(_)) => {
                    let err_msg = format!("API key not found for {}", model_provider);
                    Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, &err_msg)
                        .await;
                    return;
                }
                Err(e) => {
                    let err_msg =
                        format!("Failed to retrieve API key for {}: {}", model_provider, e);
                    Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, &err_msg)
                        .await;
                    return;
                }
            }
        };

        // Get Ollama endpoint if provider is Ollama
        let ollama_endpoint = if provider == LLMProvider::Ollama {
            match SettingsRepository::get_model_config(&pool).await {
                Ok(Some(config)) => config.ollama_endpoint,
                Ok(None) => None,
                Err(e) => {
                    info!("Failed to retrieve Ollama endpoint: {}, using default", e);
                    None
                }
            }
        } else {
            None
        };

        // Get CustomOpenAI config if provider is CustomOpenAI
        let (
            custom_openai_endpoint,
            custom_openai_api_key,
            custom_openai_max_tokens,
            custom_openai_temperature,
            custom_openai_top_p,
        ) = if provider == LLMProvider::CustomOpenAI {
            match SettingsRepository::get_custom_openai_config(&pool).await {
                Ok(Some(config)) => {
                    info!("✓ Using custom OpenAI endpoint: {}", config.endpoint);
                    (
                        Some(config.endpoint),
                        config.api_key,
                        config.max_tokens.map(|t| t as u32),
                        config.temperature,
                        config.top_p,
                    )
                }
                Ok(None) => {
                    let err_msg = "Custom OpenAI provider selected but no configuration found";
                    Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, err_msg)
                        .await;
                    return;
                }
                Err(e) => {
                    let err_msg = format!("Failed to retrieve custom OpenAI config: {}", e);
                    Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, &err_msg)
                        .await;
                    return;
                }
            }
        } else {
            (None, None, None, None, None)
        };

        // For CustomOpenAI, use its API key (if any) instead of the empty string
        let final_api_key = if provider == LLMProvider::CustomOpenAI {
            custom_openai_api_key.unwrap_or_default()
        } else {
            api_key
        };

        // Dynamically fetch context size based on provider and model
        let token_threshold = if provider == LLMProvider::Ollama {
            match METADATA_CACHE
                .get_or_fetch(&model_name, ollama_endpoint.as_deref())
                .await
            {
                Ok(metadata) => {
                    // Reserve 300 tokens for prompt overhead
                    let optimal = metadata.context_size.saturating_sub(300);
                    info!(
                        "✓ Using dynamic context for {}: {} tokens (chunk size: {})",
                        model_name, metadata.context_size, optimal
                    );
                    optimal
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch context for {}: {}. Using default 4000",
                        model_name, e
                    );
                    4000 // Fallback to safe default
                }
            }
        } else if provider == LLMProvider::BuiltInAI {
            // Get model's context size from registry
            use crate::summary::summary_engine::models;
            let model = models::get_model_by_name(&model_name)
                .ok_or_else(|| format!("Unknown model: {}", model_name));

            match model {
                Ok(model_def) => {
                    // Reserve 300 tokens for prompt overhead
                    let optimal = model_def.context_size.saturating_sub(300) as usize;
                    info!(
                        "✓ Using BuiltInAI context size: {} tokens (chunk size: {})",
                        model_def.context_size, optimal
                    );
                    optimal
                }
                Err(e) => {
                    warn!("{}, using default 2048", e);
                    1748 // 2048 - 300 for overhead
                }
            }
        } else {
            // Cloud providers (OpenAI, Claude, Groq, CustomOpenAI) handle large contexts automatically
            100000 // Effectively unlimited for single-pass processing
        };

        // Get app data directory for BuiltInAI provider
        let app_data_dir = _app.path().app_data_dir().ok();

        if let Some(code) = &summary_language {
            info!("📝 Summary language preference: {}", code);
        }

        let detected_summary_language = Self::read_detected_summary_language(&pool, &meeting_id)
            .await
            .or_else(|| Self::detect_summary_language_from_text(&text));

        if let Some(code) = &detected_summary_language {
            info!("📝 Detected transcript summary language: {}", code);
        }

        let template = match templates::get_template(&template_id) {
            Ok(template) => template,
            Err(e) => {
                let err_msg = format!("Failed to load template '{}': {}", template_id, e);
                Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, &err_msg).await;
                return;
            }
        };
        let template_fingerprint = template_cache_fingerprint(&template);

        let cache_source = build_summary_cache_source(
            &text,
            &custom_prompt,
            &system_prompt,
            &template_id,
            &template_fingerprint,
            token_threshold,
            &model_provider,
            &model_name,
            ollama_endpoint.as_deref(),
            custom_openai_endpoint.as_deref(),
            custom_openai_max_tokens,
            custom_openai_temperature,
            custom_openai_top_p,
        );

        let cached_english = match SummaryProcessesRepository::get_summary_data(&pool, &meeting_id).await {
            Err(e) => {
                warn!(
                    "Failed to load prior summary row for cache lookup (meeting_id={}): {}. Falling back to full pass-1 generation.",
                    meeting_id, e
                );
                None
            }
            Ok(None) => None,
            Ok(Some(process)) => process.result.and_then(|raw| {
                match extract_cached_english_markdown(
                    &raw,
                    &cache_source,
                    summary_language.as_deref(),
                ) {
                    Ok(opt) => opt,
                    Err(e) => {
                        warn!(
                            "Cached summary result for meeting_id={} is not valid JSON ({}); ignoring cache.",
                            meeting_id, e
                        );
                        None
                    }
                }
            }),
        };

        let client = reqwest::Client::new();
        let result = generate_meeting_summary(
            &client,
            &provider,
            &model_name,
            &final_api_key,
            &text,
            &custom_prompt,
            &system_prompt,
            &template_id,
            &template,
            token_threshold,
            ollama_endpoint.as_deref(),
            custom_openai_endpoint.as_deref(),
            custom_openai_max_tokens,
            custom_openai_temperature,
            custom_openai_top_p,
            app_data_dir.as_ref(),
            Some(&cancellation_token),
            summary_language.as_deref(),
            detected_summary_language.as_deref(),
            cached_english.as_deref(),
        )
        .await;

        let duration = start_time.elapsed().as_secs_f64();

        match result {
            Ok((final_markdown, english_markdown, num_chunks)) => {
                info!(
                    "✓ Successfully processed {} chunks for meeting_id: {}. Duration: {:.2}s",
                    num_chunks, meeting_id, duration
                );
                info!("Final markdown generated ({} chars)", final_markdown.len());

                let suggested_title =
                    extract_meeting_name_from_markdown(&final_markdown).filter(|n| !n.is_empty());
                if final_markdown.trim().is_empty() {
                    Self::update_process_failed(
                        &pool,
                        &meeting_id,
                        &lifecycle_run_id,
                        "Generated summary is empty; the previous summary was preserved",
                    )
                    .await;
                    return;
                }

                let result_json = build_summary_result_json(
                    &final_markdown,
                    &english_markdown,
                    cache_source,
                    summary_language.as_deref(),
                );

                match Self::accept_generated_summary(
                    &pool,
                    &meeting_id,
                    &lifecycle_run_id,
                    result_json,
                    &final_markdown,
                    SummaryProcessingMetrics {
                        chunk_count: num_chunks,
                        processing_time: duration,
                    },
                    suggested_title.as_deref(),
                )
                .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        info!(
                            "Discarding result from superseded summary run {} for {}",
                            lifecycle_run_id, meeting_id
                        );
                    }
                    Err(error) => {
                        error!(
                            "Failed to accept completed summary for {}: {}",
                            meeting_id, error
                        );
                    }
                }
            }
            Err(e) => {
                // Check if error is due to cancellation
                if e.contains("cancelled") {
                    info!(
                        "Summary generation was cancelled for meeting_id: {}",
                        meeting_id
                    );
                    let _operation_guard = Self::lock_meeting_operation(&meeting_id).await;
                    if let Err(db_err) =
                        SummaryProcessesRepository::update_process_cancelled_for_run(
                            &pool,
                            &meeting_id,
                            &lifecycle_run_id,
                        )
                        .await
                    {
                        error!(
                            "Failed to update DB status to cancelled for {}: {}",
                            meeting_id, db_err
                        );
                    }
                } else {
                    Self::update_process_failed(&pool, &meeting_id, &lifecycle_run_id, &e).await;
                }
            }
        }
    }

    /// Updates the summary process status to failed with error message
    ///
    /// # Arguments
    /// * `pool` - SQLx connection pool
    /// * `meeting_id` - Meeting identifier
    /// * `error_msg` - Error message to store
    async fn update_process_failed(
        pool: &SqlitePool,
        meeting_id: &str,
        lifecycle_run_id: &str,
        error_msg: &str,
    ) {
        error!(
            "Processing failed for meeting_id {}: {}",
            meeting_id, error_msg
        );
        if let Err(e) =
            Self::fail_summary_attempt(pool, meeting_id, lifecycle_run_id, error_msg).await
        {
            error!(
                "Failed to update DB status to failed for {}: {}",
                meeting_id, e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_leading_title_with_body() {
        let input = "# Meeting Title\nThis is the body.\nMore content.";
        let result = strip_leading_title(input);
        assert_eq!(result, "This is the body.\nMore content.");
    }

    #[test]
    fn test_strip_leading_title_only() {
        let input = "# Meeting Title";
        let result = strip_leading_title(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_leading_title_no_heading() {
        let input = "No heading here.\nJust body.";
        let result = strip_leading_title(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_leading_title_multiline_body() {
        let input = "# Title\n## Subheading\nParagraph 1\n\nParagraph 2";
        let result = strip_leading_title(input);
        assert_eq!(result, "## Subheading\nParagraph 1\n\nParagraph 2");
    }

    #[test]
    fn test_strip_leading_title_empty_after_heading() {
        let input = "# Title\n";
        let result = strip_leading_title(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_leading_title_whitespace_after_heading() {
        let input = "# Title\n   \n Body with leading spaces";
        let result = strip_leading_title(input);
        assert_eq!(result, "Body with leading spaces");
    }

    #[test]
    fn test_strip_title_if_present_preserves_already_stripped() {
        assert_eq!(
            strip_title_if_present("## Action Items\nfoo"),
            "## Action Items\nfoo"
        );
    }

    #[test]
    fn test_strip_title_if_present_strips_leading_h1() {
        assert_eq!(
            strip_title_if_present("# Meeting Title\n## Action Items\nfoo"),
            "## Action Items\nfoo"
        );
    }

    #[test]
    fn test_strip_title_if_present_no_heading_preserved() {
        // Distinct from strip_leading_title which returns "" — this preserves input.
        assert_eq!(strip_title_if_present("Just body text"), "Just body text");
    }

    #[test]
    fn test_strip_title_if_present_hash_no_space_preserved() {
        // `#NoSpace` is not a markdown H1 — preserve.
        assert_eq!(strip_title_if_present("#NoSpace\nbody"), "#NoSpace\nbody");
    }

    #[test]
    fn test_strip_title_if_present_mid_document_h1_preserved() {
        // H1 after body content must NOT be stripped — guards the asymmetry where
        // extract_meeting_name_from_markdown scans every line for "# ".
        let input = "Some paragraph\n\n# H1 on line 3\n## Section\nbody";
        assert_eq!(strip_title_if_present(input), input);
    }

    #[test]
    fn test_strip_title_if_present_leading_whitespace_h1_stripped() {
        assert_eq!(
            strip_title_if_present("  # Title\n## Section\nbody"),
            "## Section\nbody"
        );
    }

    fn sample_cache_source() -> SummaryCacheSource {
        let template_fingerprint = stable_text_fingerprint("standard template prompt");
        build_summary_cache_source(
            "transcript body",
            "custom prompt",
            "system prompt",
            "standard_meeting",
            &template_fingerprint,
            3700,
            "ollama",
            "gemma3:1b",
            Some("http://localhost:11434"),
            None,
            None,
            None,
            None,
        )
    }

    fn test_template(section_title: &str) -> Template {
        Template {
            name: "Test".to_string(),
            description: "Test template".to_string(),
            sections: vec![crate::summary::templates::TemplateSection {
                title: section_title.to_string(),
                instruction: "Summarize this section".to_string(),
                format: "paragraph".to_string(),
                item_format: None,
                example_item_format: None,
            }],
        }
    }

    #[test]
    fn test_template_cache_fingerprint_changes_with_rendered_template() {
        assert_ne!(
            template_cache_fingerprint(&test_template("Summary")),
            template_cache_fingerprint(&test_template("Decisions"))
        );
    }

    #[test]
    fn test_legacy_english_markdown_field_is_cache_miss() {
        let raw = serde_json::json!({
            "markdown": "translated",
            "english_markdown": "# Old English\nBody"
        })
        .to_string();

        assert_eq!(
            extract_cached_english_markdown(&raw, &sample_cache_source(), Some("de")).unwrap(),
            None
        );
    }

    #[test]
    fn test_matching_source_changed_translation_target_reuses_cache() {
        let source = sample_cache_source();
        let raw = build_summary_result_json(
            "# Reunion\n## Points\nBonjour",
            "# Meeting\n## Points\nHello",
            source.clone(),
            Some("fr"),
        )
        .to_string();

        assert_eq!(
            extract_cached_english_markdown(&raw, &source, Some("de")).unwrap(),
            Some("# Meeting\n## Points\nHello".to_string())
        );
    }

    #[test]
    fn test_same_language_regeneration_rejects_cache() {
        let source = sample_cache_source();
        let raw = build_summary_result_json(
            "# Reunion\n## Points\nBonjour",
            "# Meeting\n## Points\nHello",
            source.clone(),
            Some("fr"),
        )
        .to_string();

        assert_eq!(
            extract_cached_english_markdown(&raw, &source, Some("fr")).unwrap(),
            None
        );
    }

    #[test]
    fn test_changed_summary_inputs_reject_cache() {
        let source = sample_cache_source();
        let template_fingerprint = source.template_fingerprint.clone();
        let raw = build_summary_result_json(
            "# Reunion\n## Points\nBonjour",
            "# Meeting\n## Points\nHello",
            source,
            Some("fr"),
        )
        .to_string();

        let changed_sources = [
            build_summary_cache_source(
                "changed transcript",
                "custom prompt",
                "system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "ollama",
                "gemma3:1b",
                Some("http://localhost:11434"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "changed prompt",
                "system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "ollama",
                "gemma3:1b",
                Some("http://localhost:11434"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "custom prompt",
                "changed system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "ollama",
                "gemma3:1b",
                Some("http://localhost:11434"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "custom prompt",
                "system prompt",
                "daily_standup",
                &template_fingerprint,
                3700,
                "ollama",
                "gemma3:1b",
                Some("http://localhost:11434"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "custom prompt",
                "system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "openai",
                "gemma3:1b",
                Some("http://localhost:11434"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "custom prompt",
                "system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "ollama",
                "qwen2.5:3b",
                Some("http://localhost:11434"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "custom prompt",
                "system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "ollama",
                "gemma3:1b",
                Some("http://localhost:11500"),
                None,
                None,
                None,
                None,
            ),
            build_summary_cache_source(
                "transcript body",
                "custom prompt",
                "system prompt",
                "standard_meeting",
                &template_fingerprint,
                3700,
                "ollama",
                "gemma3:1b",
                Some("http://localhost:11434"),
                Some("https://custom.example/v1"),
                Some(2048),
                Some(0.2),
                Some(0.9),
            ),
        ];

        for changed_source in changed_sources {
            assert_eq!(
                extract_cached_english_markdown(&raw, &changed_source, Some("de")).unwrap(),
                None
            );
        }
    }

    #[test]
    fn test_changed_template_content_rejects_cache() {
        let source = sample_cache_source();
        let raw = build_summary_result_json(
            "# Reunion\n## Points\nBonjour",
            "# Meeting\n## Points\nHello",
            source.clone(),
            Some("fr"),
        )
        .to_string();

        let changed_template = SummaryCacheSource {
            template_fingerprint: stable_text_fingerprint("changed template prompt"),
            ..source
        };

        assert_eq!(
            extract_cached_english_markdown(&raw, &changed_template, Some("de")).unwrap(),
            None
        );
    }

    #[test]
    fn test_changed_token_threshold_rejects_cache() {
        let source = sample_cache_source();
        let raw = build_summary_result_json(
            "# Reunion\n## Points\nBonjour",
            "# Meeting\n## Points\nHello",
            source.clone(),
            Some("fr"),
        )
        .to_string();

        let changed_threshold = SummaryCacheSource {
            token_threshold: 8192,
            ..source
        };

        assert_eq!(
            extract_cached_english_markdown(&raw, &changed_threshold, Some("de")).unwrap(),
            None
        );
    }

    #[test]
    fn test_result_json_strips_display_markdown_but_keeps_cache_title() {
        let result = build_summary_result_json(
            "# Translated Title\n## Decisions\nDone",
            "# English Title\n## Decisions\nDone",
            sample_cache_source(),
            Some("fr"),
        );

        assert_eq!(result["markdown"], "## Decisions\nDone");
        assert_eq!(
            result["export_markdown"],
            "# Translated Title\n## Decisions\nDone"
        );
        assert_eq!(
            result["english_cache"]["markdown"],
            "# English Title\n## Decisions\nDone"
        );
    }

    #[test]
    fn test_extract_cached_english_from_malformed_json_errors() {
        let raw = r#"{ not valid json"#;
        assert!(extract_cached_english_markdown(raw, &sample_cache_source(), Some("de")).is_err());
    }

    #[test]
    fn cancellation_and_cleanup_are_scoped_to_one_attempt() {
        let meeting_id = format!("meeting-token-scope-{}", uuid::Uuid::new_v4());
        let attempt_a = "attempt-a";
        let attempt_b = "attempt-b";
        let token_a = SummaryService::register_cancellation_token(&meeting_id, attempt_a);
        let token_b = SummaryService::register_cancellation_token(&meeting_id, attempt_b);

        assert!(SummaryService::cancel_summary(&meeting_id, attempt_a));
        assert!(token_a.is_cancelled());
        assert!(!token_b.is_cancelled());

        drop(CancellationRegistration {
            meeting_id: meeting_id.clone(),
            lifecycle_run_id: attempt_a.to_string(),
        });
        assert!(!SummaryService::cancel_summary(&meeting_id, attempt_a));
        assert!(SummaryService::cancel_summary(&meeting_id, attempt_b));
        assert!(token_b.is_cancelled());
        SummaryService::cleanup_cancellation_token(&meeting_id, attempt_b);
    }
}
