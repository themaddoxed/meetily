use crate::database::models::{Setting, TranscriptSetting};
use crate::summary::CustomOpenAIConfig;
use sqlx::SqlitePool;
use std::collections::HashSet;

pub const MAX_VOCABULARY_CHARS: usize = 1000;

/// Normalize user-entered names and terms into the stable value stored in SQLite.
/// Commas and line breaks separate terms; the first spelling wins for duplicates.
pub fn normalize_vocabulary(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.contains('\0') {
        return Err("Vocabulary cannot contain a null character".to_string());
    }

    let mut seen = HashSet::new();
    let mut terms = Vec::new();
    for term in raw.split([',', '\n', '\r']) {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }
        let key = term.to_lowercase();
        if seen.insert(key) {
            terms.push(term.to_string());
        }
    }

    let normalized = terms.join(", ");
    if normalized.chars().count() > MAX_VOCABULARY_CHARS {
        return Err(format!(
            "Vocabulary is limited to {MAX_VOCABULARY_CHARS} characters"
        ));
    }
    if normalized.is_empty() {
        Ok(None)
    } else {
        Ok(Some(normalized))
    }
}

/// Merge a meeting override ahead of the global vocabulary, preserving order.
pub fn merge_vocabulary(global: Option<&str>, meeting: Option<&str>) -> Option<String> {
    let mut seen = HashSet::new();
    let mut terms = Vec::new();
    for value in [meeting, global].into_iter().flatten() {
        for term in value.split(',') {
            let term = term.trim();
            if !term.is_empty() && seen.insert(term.to_lowercase()) {
                terms.push(term.to_string());
            }
        }
    }
    (!terms.is_empty()).then(|| terms.join(", "))
}

#[derive(serde::Deserialize, Debug)]
pub struct SaveModelConfigRequest {
    pub provider: String,
    pub model: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "ollamaEndpoint")]
    pub ollama_endpoint: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SaveTranscriptConfigRequest {
    pub provider: String,
    pub model: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
}

pub struct SettingsRepository;

// Transcript providers: localWhisper, deepgram, elevenLabs, groq, openai
// Summary providers: openai, claude, ollama, groq, added openrouter
// NOTE: Handle data exclusion in the higher layer as this is database abstraction layer(using SELECT *)

impl SettingsRepository {
    pub async fn get_model_config(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<Setting>, sqlx::Error> {
        let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings LIMIT 1")
            .fetch_optional(pool)
            .await?;
        Ok(setting)
    }

    pub async fn save_model_config(
        pool: &SqlitePool,
        provider: &str,
        model: &str,
        whisper_model: &str,
        ollama_endpoint: Option<&str>,
    ) -> std::result::Result<(), sqlx::Error> {
        // Using id '1' for backward compatibility
        sqlx::query(
            r#"
            INSERT INTO settings (id, provider, model, whisperModel, ollamaEndpoint)
            VALUES ('1', $1, $2, $3, $4)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                model = excluded.model,
                whisperModel = excluded.whisperModel,
                ollamaEndpoint = excluded.ollamaEndpoint
            "#,
        )
        .bind(provider)
        .bind(model)
        .bind(whisper_model)
        .bind(ollama_endpoint)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn save_api_key(
        pool: &SqlitePool,
        provider: &str,
        api_key: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        // Custom OpenAI uses JSON config (customOpenAIConfig) instead of a separate API key column
        if provider == "custom-openai" {
            return Err(sqlx::Error::Protocol(
                "custom-openai provider should use save_custom_openai_config() instead of save_api_key()".into(),
            ));
        }

        let api_key_column = match provider {
            "openai" => "openaiApiKey",
            "claude" => "anthropicApiKey",
            "ollama" => "ollamaApiKey",
            "groq" => "groqApiKey",
            "openrouter" => "openRouterApiKey",
            "builtin-ai" => return Ok(()), // No API key needed
            _ => {
                return Err(sqlx::Error::Protocol(format!(
                    "Invalid provider: {}",
                    provider
                )))
            }
        };

        let query = format!(
            r#"
            INSERT INTO settings (id, provider, model, whisperModel, "{}")
            VALUES ('1', 'openai', 'gpt-4o-2024-11-20', 'large-v3', $1)
            ON CONFLICT(id) DO UPDATE SET
                "{}" = $1
            "#,
            api_key_column, api_key_column
        );
        sqlx::query(&query).bind(api_key).execute(pool).await?;

        Ok(())
    }

    pub async fn get_api_key(
        pool: &SqlitePool,
        provider: &str,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        // Custom OpenAI uses JSON config - extract API key from there
        if provider == "custom-openai" {
            let config = Self::get_custom_openai_config(pool).await?;
            return Ok(config.and_then(|c| c.api_key));
        }

        let api_key_column = match provider {
            "openai" => "openaiApiKey",
            "ollama" => "ollamaApiKey",
            "groq" => "groqApiKey",
            "claude" => "anthropicApiKey",
            "openrouter" => "openRouterApiKey",
            "builtin-ai" => return Ok(None), // No API key needed
            _ => {
                return Err(sqlx::Error::Protocol(format!(
                    "Invalid provider: {}",
                    provider
                )))
            }
        };

        let query = format!(
            "SELECT {} FROM settings WHERE id = '1' LIMIT 1",
            api_key_column
        );
        let api_key = sqlx::query_scalar(&query).fetch_optional(pool).await?;
        Ok(api_key)
    }

    pub async fn get_transcript_config(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<TranscriptSetting>, sqlx::Error> {
        let setting =
            sqlx::query_as::<_, TranscriptSetting>("SELECT * FROM transcript_settings LIMIT 1")
                .fetch_optional(pool)
                .await?;
        Ok(setting)
    }

    pub async fn get_global_vocabulary(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT whisperVocabulary FROM transcript_settings WHERE id = '1' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map(|value| value.flatten())
    }

    pub async fn save_global_vocabulary(
        pool: &SqlitePool,
        vocabulary: Option<&str>,
    ) -> std::result::Result<(), sqlx::Error> {
        let vocabulary = normalize_vocabulary(vocabulary).map_err(sqlx::Error::Protocol)?;
        sqlx::query(
            r#"
            INSERT INTO transcript_settings (id, provider, model, whisperVocabulary)
            VALUES ('1', 'parakeet', $1, $2)
            ON CONFLICT(id) DO UPDATE SET
                whisperVocabulary = excluded.whisperVocabulary
            "#,
        )
        .bind(crate::config::DEFAULT_PARAKEET_MODEL)
        .bind(vocabulary)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_meeting_vocabulary(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT vocabulary FROM meeting_transcript_vocabulary WHERE meeting_id = ? LIMIT 1",
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
        .map(|value| value.flatten())
    }

    pub async fn save_meeting_vocabulary(
        pool: &SqlitePool,
        meeting_id: &str,
        vocabulary: Option<&str>,
    ) -> std::result::Result<(), sqlx::Error> {
        let vocabulary = normalize_vocabulary(vocabulary).map_err(sqlx::Error::Protocol)?;
        match vocabulary {
            Some(vocabulary) => {
                sqlx::query(
                    r#"
                    INSERT INTO meeting_transcript_vocabulary (meeting_id, vocabulary, updated_at)
                    VALUES (?, ?, CURRENT_TIMESTAMP)
                    ON CONFLICT(meeting_id) DO UPDATE SET
                        vocabulary = excluded.vocabulary,
                        updated_at = CURRENT_TIMESTAMP
                    "#,
                )
                .bind(meeting_id)
                .bind(vocabulary)
                .execute(pool)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM meeting_transcript_vocabulary WHERE meeting_id = ?")
                    .bind(meeting_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_effective_vocabulary(
        pool: &SqlitePool,
        meeting_id: Option<&str>,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        let global = Self::get_global_vocabulary(pool).await?;
        let meeting = match meeting_id {
            Some(meeting_id) => Self::get_meeting_vocabulary(pool, meeting_id).await?,
            None => None,
        };
        Ok(merge_vocabulary(global.as_deref(), meeting.as_deref()))
    }

    pub async fn save_transcript_config(
        pool: &SqlitePool,
        provider: &str,
        model: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO transcript_settings (id, provider, model)
            VALUES ('1', $1, $2)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                model = excluded.model
            "#,
        )
        .bind(provider)
        .bind(model)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn save_transcript_api_key(
        pool: &SqlitePool,
        provider: &str,
        api_key: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        let api_key_column = match provider {
            "localWhisper" => "whisperApiKey",
            "parakeet" => return Ok(()), // Parakeet doesn't need an API key, return early
            "deepgram" => "deepgramApiKey",
            "elevenLabs" => "elevenLabsApiKey",
            "groq" => "groqApiKey",
            "openai" => "openaiApiKey",
            _ => {
                return Err(sqlx::Error::Protocol(format!(
                    "Invalid provider: {}",
                    provider
                )))
            }
        };

        let query = format!(
            r#"
            INSERT INTO transcript_settings (id, provider, model, "{}")
            VALUES ('1', 'parakeet', '{}', $1)
            ON CONFLICT(id) DO UPDATE SET
                "{}" = $1
            "#,
            api_key_column,
            crate::config::DEFAULT_PARAKEET_MODEL,
            api_key_column
        );
        sqlx::query(&query).bind(api_key).execute(pool).await?;

        Ok(())
    }

    pub async fn get_transcript_api_key(
        pool: &SqlitePool,
        provider: &str,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        let api_key_column = match provider {
            "localWhisper" => "whisperApiKey",
            "parakeet" => return Ok(None), // Parakeet doesn't need an API key
            "deepgram" => "deepgramApiKey",
            "elevenLabs" => "elevenLabsApiKey",
            "groq" => "groqApiKey",
            "openai" => "openaiApiKey",
            _ => {
                return Err(sqlx::Error::Protocol(format!(
                    "Invalid provider: {}",
                    provider
                )))
            }
        };

        let query = format!(
            "SELECT {} FROM transcript_settings WHERE id = '1' LIMIT 1",
            api_key_column
        );
        let api_key = sqlx::query_scalar(&query).fetch_optional(pool).await?;
        Ok(api_key)
    }

    pub async fn delete_api_key(
        pool: &SqlitePool,
        provider: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        // Custom OpenAI uses JSON config - clear the entire config
        if provider == "custom-openai" {
            sqlx::query("UPDATE settings SET customOpenAIConfig = NULL WHERE id = '1'")
                .execute(pool)
                .await?;
            return Ok(());
        }

        let api_key_column = match provider {
            "openai" => "openaiApiKey",
            "ollama" => "ollamaApiKey",
            "groq" => "groqApiKey",
            "claude" => "anthropicApiKey",
            "openrouter" => "openRouterApiKey",
            "builtin-ai" => return Ok(()), // No API key needed
            _ => {
                return Err(sqlx::Error::Protocol(format!(
                    "Invalid provider: {}",
                    provider
                )))
            }
        };

        let query = format!(
            "UPDATE settings SET {} = NULL WHERE id = '1'",
            api_key_column
        );
        sqlx::query(&query).execute(pool).await?;

        Ok(())
    }

    // ===== CUSTOM OPENAI CONFIG METHODS =====

    /// Gets the custom OpenAI configuration from JSON
    ///
    /// # Returns
    /// * `Ok(Some(CustomOpenAIConfig))` - Config exists and is valid JSON
    /// * `Ok(None)` - No config stored
    /// * `Err(sqlx::Error)` - Database error
    pub async fn get_custom_openai_config(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<CustomOpenAIConfig>, sqlx::Error> {
        use sqlx::Row;

        let row = sqlx::query(
            r#"
            SELECT customOpenAIConfig
            FROM settings
            WHERE id = '1'
            LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(record) => {
                let config_json: Option<String> = record.get("customOpenAIConfig");

                if let Some(json) = config_json {
                    // Parse JSON into CustomOpenAIConfig
                    let config: CustomOpenAIConfig = serde_json::from_str(&json).map_err(|e| {
                        sqlx::Error::Protocol(format!("Invalid JSON in customOpenAIConfig: {}", e))
                    })?;

                    Ok(Some(config))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Saves the custom OpenAI configuration as JSON
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `config` - CustomOpenAIConfig to save (includes endpoint, apiKey, model, maxTokens, temperature, topP)
    ///
    /// # Returns
    /// * `Ok(())` - Config saved successfully
    /// * `Err(sqlx::Error)` - Database or JSON serialization error
    pub async fn save_custom_openai_config(
        pool: &SqlitePool,
        config: &CustomOpenAIConfig,
    ) -> std::result::Result<(), sqlx::Error> {
        // Serialize config to JSON
        let config_json = serde_json::to_string(config).map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to serialize config to JSON: {}", e))
        })?;

        // Upsert into settings table
        sqlx::query(
            r#"
            INSERT INTO settings (id, provider, model, whisperModel, customOpenAIConfig)
            VALUES ('1', 'custom-openai', $1, 'large-v3', $2)
            ON CONFLICT(id) DO UPDATE SET
                customOpenAIConfig = excluded.customOpenAIConfig
            "#,
        )
        .bind(&config.model)
        .bind(config_json)
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod vocabulary_tests {
    use super::{merge_vocabulary, normalize_vocabulary};

    #[test]
    fn normalizes_comma_and_newline_terms_without_case_insensitive_duplicates() {
        assert_eq!(
            normalize_vocabulary(Some("  Acme, acme\nProject\r\n  Roadmap  ")).unwrap(),
            Some("Acme, Project, Roadmap".to_string())
        );
    }

    #[test]
    fn empty_vocabulary_preserves_default_engine_behavior() {
        assert_eq!(normalize_vocabulary(Some("  ,\n\r  ")).unwrap(), None);
        assert_eq!(normalize_vocabulary(None).unwrap(), None);
    }

    #[test]
    fn rejects_nul_and_oversized_vocabulary() {
        assert!(normalize_vocabulary(Some("Acme\0Project")).is_err());
        assert!(normalize_vocabulary(Some(&"x".repeat(1001))).is_err());
    }

    #[test]
    fn meeting_terms_are_first_and_global_duplicates_are_removed() {
        assert_eq!(
            merge_vocabulary(Some("Global, Shared"), Some("Meeting, shared")),
            Some("Meeting, shared, Global".to_string())
        );
    }
}
