// audio/transcription/provider.rs
//
// Defines the unified TranscriptionProvider trait and common types for all
// transcription engines (Whisper, Parakeet, future providers).

use async_trait::async_trait;

// ============================================================================
// TRANSCRIPTION PROVIDER TRAIT & ERROR TYPES
// ============================================================================

/// Granular error types for transcription operations
#[derive(Debug, Clone)]
pub enum TranscriptionError {
    ModelNotLoaded,
    AudioTooShort { samples: usize, minimum: usize },
    EngineFailed(String),
    UnsupportedLanguage(String),
}

impl std::fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelNotLoaded => write!(f, "No transcription model is loaded"),
            Self::AudioTooShort { samples, minimum } => write!(
                f,
                "Audio too short: {} samples (minimum {})",
                samples, minimum
            ),
            Self::EngineFailed(msg) => write!(f, "Transcription engine failed: {}", msg),
            Self::UnsupportedLanguage(lang) => {
                write!(f, "Language '{}' is not supported by this provider", lang)
            }
        }
    }
}

impl std::error::Error for TranscriptionError {}

/// Unified transcription result across all providers
#[derive(Debug, Clone)]
pub struct TranscriptResult {
    pub text: String,
    pub confidence: Option<f32>, // None if provider doesn't support confidence scores
    pub is_partial: bool,
}

/// Trait for transcription providers (Whisper, Parakeet, future providers)
// async-trait 0.1.89 unconditionally adds #[must_use] to each boxed Future.
// Rust 1.99 already marks Future as must_use, so Clippy flags the macro's
// redundant attribute. Keep unused_must_use and all other warnings enabled.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    /// Transcribe audio samples to text
    ///
    /// # Arguments
    /// * `audio` - Audio samples (16kHz mono, f32 format)
    /// * `language` - Optional language hint (e.g., "en", "es", "fr")
    ///
    /// # Returns
    /// * `TranscriptResult` with text, optional confidence, and partial flag
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError>;

    /// Transcribe with an optional Whisper vocabulary. Providers that do not support
    /// vocabulary hints keep their existing behavior through this default method.
    async fn transcribe_with_vocabulary(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
        _vocabulary: Option<&str>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError> {
        self.transcribe(audio, language).await
    }

    /// Check if a model is currently loaded
    async fn is_model_loaded(&self) -> bool;

    /// Get the name of the currently loaded model
    async fn get_current_model(&self) -> Option<String>;

    /// Get the provider name (for logging/debugging)
    fn provider_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::{TranscriptResult, TranscriptionError, TranscriptionProvider};
    use async_trait::async_trait;

    struct UnsupportedVocabularyProvider;

    #[async_trait]
    impl TranscriptionProvider for UnsupportedVocabularyProvider {
        async fn transcribe(
            &self,
            _audio: Vec<f32>,
            _language: Option<String>,
        ) -> Result<TranscriptResult, TranscriptionError> {
            Ok(TranscriptResult {
                text: "provider output".to_string(),
                confidence: None,
                is_partial: false,
            })
        }

        async fn is_model_loaded(&self) -> bool {
            true
        }

        async fn get_current_model(&self) -> Option<String> {
            Some("unsupported-vocabulary-fixture".to_string())
        }

        fn provider_name(&self) -> &'static str {
            "fixture"
        }
    }

    #[tokio::test]
    async fn providers_without_vocabulary_support_keep_existing_behavior() {
        let result = UnsupportedVocabularyProvider
            .transcribe_with_vocabulary(vec![0.0; 1600], Some("ru".to_string()), Some("term"))
            .await
            .unwrap();

        assert_eq!(result.text, "provider output");
        assert_eq!(result.confidence, None);
    }
}
