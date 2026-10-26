// Model manager for built-in AI models - handles downloads and lifecycle
// Follows the same pattern as whisper_engine/whisper_engine.rs for consistency

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::RwLock;

use super::models::{get_available_models, get_model_by_name};
use crate::model_download::{
    is_already_active, is_cancelled, partial_file_path, summary_manifest, IntegrityCache,
    ResumableDownloader,
};

pub use crate::model_download::DownloadProgress;

// ============================================================================
// Model Status Types
// ============================================================================

/// Model status in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelStatus {
    /// Model is not yet downloaded
    NotDownloaded,

    /// Model is currently being downloaded (progress 0-100)
    Downloading { progress: u8 },

    /// Model is downloaded and ready to use
    Available,

    /// Model file is corrupted and needs redownload
    Corrupted {
        file_size: u64,
        expected_min_size: u64,
    },

    /// Error occurred with the model
    Error(String),
}

/// Model information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name (e.g., "gemma3:1b")
    pub name: String,

    /// Display name for UI
    pub display_name: String,

    /// Current status
    pub status: ModelStatus,

    /// File path (if available)
    pub path: PathBuf,

    /// Size in MB
    pub size_mb: u64,

    /// Context window size in tokens
    pub context_size: u32,

    /// Description
    pub description: String,

    /// GGUF filename on disk
    pub gguf_file: String,
}

// ============================================================================
// Model Manager
// ============================================================================

pub struct ModelManager {
    /// Directory where models are stored
    models_dir: PathBuf,

    /// Currently available models with their status
    available_models: Arc<RwLock<HashMap<String, ModelInfo>>>,

    /// Shared download primitive with one cancellation owner per model.
    downloader: ResumableDownloader,

    /// Successful exact checks reused by repeated model scans.
    integrity_cache: IntegrityCache,
}

impl ModelManager {
    /// Create a new model manager with default models directory
    pub fn new() -> Result<Self> {
        Self::new_with_models_dir(None)
    }

    /// Create a new model manager with custom models directory
    pub fn new_with_models_dir(models_dir: Option<PathBuf>) -> Result<Self> {
        let models_dir = if let Some(dir) = models_dir {
            dir
        } else {
            // Fallback: Use current directory in development
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

            if cfg!(debug_assertions) {
                // Development mode
                current_dir.join("models").join("summary")
            } else {
                // Production mode fallback (caller should provide path)
                log::warn!("ModelManager: No models directory provided, using fallback path");
                dirs::data_dir()
                    .or_else(dirs::home_dir)
                    .ok_or_else(|| anyhow!("Could not find system data directory"))?
                    .join("Meetily")
                    .join("models")
                    .join("summary")
            }
        };

        log::info!(
            "Built-in AI ModelManager using directory: {}",
            models_dir.display()
        );

        Ok(Self {
            models_dir,
            available_models: Arc::new(RwLock::new(HashMap::new())),
            downloader: ResumableDownloader::new()?,
            integrity_cache: IntegrityCache::new(),
        })
    }

    /// Initialize and scan for existing models
    pub async fn init(&self) -> Result<()> {
        // Create models directory if it doesn't exist
        if !self.models_dir.exists() {
            fs::create_dir_all(&self.models_dir).await?;
            log::info!("Created models directory: {}", self.models_dir.display());
        }

        // Scan for existing models
        self.scan_models().await?;

        Ok(())
    }

    /// Scan models directory and update status
    pub async fn scan_models(&self) -> Result<()> {
        let start = std::time::Instant::now();

        log::info!(
            "Starting model scan in directory: {}",
            self.models_dir.display()
        );

        let model_defs = get_available_models();
        let mut models_map = HashMap::new();

        for model_def in model_defs {
            let model_path = self.models_dir.join(&model_def.gguf_file);
            log::debug!(
                "Checking model '{}' at path: {}",
                model_def.name,
                model_path.display()
            );

            let existing_info = {
                let models = self.available_models.read().await;
                models.get(&model_def.name).cloned()
            };
            let status = if self.downloader.is_active(&model_def.name).await {
                existing_info
                    .as_ref()
                    .map(|info| info.status.clone())
                    .filter(|status| matches!(status, ModelStatus::Downloading { .. }))
                    .unwrap_or(ModelStatus::Downloading { progress: 0 })
            } else {
                match summary_manifest(&model_def.name, &model_def.gguf_file) {
                    Some(manifest) if model_path.exists() => {
                        match self
                            .integrity_cache
                            .verify_file_cached(&model_path, &manifest.artifacts[0])
                            .await
                        {
                            Ok(()) => {
                                log::info!("Model '{}': AVAILABLE", model_def.name);
                                ModelStatus::Available
                            }
                            Err(error) => {
                                let file_size = fs::symlink_metadata(&model_path)
                                    .await
                                    .map(|metadata| metadata.len() / (1024 * 1024))
                                    .unwrap_or(0);
                                log::warn!(
                                    "Model '{}': CORRUPTED (integrity check failed: {})",
                                    model_def.name,
                                    error
                                );
                                ModelStatus::Corrupted {
                                    file_size,
                                    expected_min_size: manifest.artifacts[0].size / (1024 * 1024),
                                }
                            }
                        }
                    }
                    Some(_) => ModelStatus::NotDownloaded,
                    None => ModelStatus::Error(format!(
                        "No trusted manifest for model {}",
                        model_def.name
                    )),
                }
            };

            let model_info = ModelInfo {
                name: model_def.name.clone(),
                display_name: model_def.display_name.clone(),
                status,
                path: model_path,
                size_mb: model_def.size_mb,
                context_size: model_def.context_size,
                description: model_def.description.clone(),
                gguf_file: model_def.gguf_file.clone(),
            };

            models_map.insert(model_def.name.clone(), model_info);
        }

        let model_count = models_map.len();

        let mut models = self.available_models.write().await;
        *models = models_map;

        let elapsed = start.elapsed();
        log::info!(
            "Model scan complete: {} models checked in {:?}",
            model_count,
            elapsed
        );
        Ok(())
    }

    /// Get list of all models with their status
    pub async fn list_models(&self) -> Vec<ModelInfo> {
        self.available_models
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Get info for a specific model
    pub async fn get_model_info(&self, model_name: &str) -> Option<ModelInfo> {
        self.available_models.read().await.get(model_name).cloned()
    }

    /// Check if a model is ready to use
    /// If refresh=true, scans filesystem before checking (slower but accurate)
    pub async fn is_model_ready(&self, model_name: &str, refresh: bool) -> bool {
        if refresh {
            if let Err(e) = self.scan_models().await {
                log::error!("Failed to scan models: {}", e);
                return false;
            }
        }

        if let Some(info) = self.get_model_info(model_name).await {
            info.status == ModelStatus::Available
        } else {
            false
        }
    }

    /// Download a model with simple percentage callback (backward compatible)
    pub async fn download_model(
        &self,
        model_name: &str,
        progress_callback: Option<Box<dyn Fn(u8) + Send + Sync>>,
    ) -> Result<()> {
        let detailed_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>> =
            progress_callback.map(|callback| {
                Box::new(move |progress: DownloadProgress| callback(progress.percent))
                    as Box<dyn Fn(DownloadProgress) + Send + Sync>
            });
        self.download_model_detailed(model_name, detailed_callback)
            .await
    }

    /// Download a model with detailed progress (MB, speed, etc.).
    pub async fn download_model_detailed(
        &self,
        model_name: &str,
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> Result<()> {
        let model_def = get_model_by_name(model_name)
            .ok_or_else(|| anyhow!("Unknown model: {}", model_name))?;
        let manifest = summary_manifest(model_name, &model_def.gguf_file)
            .ok_or_else(|| anyhow!("No trusted manifest for model: {}", model_name))?;
        let file_path = self.models_dir.join(&model_def.gguf_file);

        fs::create_dir_all(&self.models_dir).await?;
        {
            let mut models = self.available_models.write().await;
            if let Some(model_info) = models.get_mut(model_name) {
                model_info.status = ModelStatus::Downloading { progress: 0 };
            }
        }
        self.integrity_cache.invalidate(&file_path).await;

        log::info!(
            "Downloading model '{}' from pinned revision {} to {}",
            model_name,
            manifest.revision,
            file_path.display()
        );
        let result = self
            .downloader
            .download(model_name, &manifest, &file_path, progress_callback)
            .await;

        match result {
            Ok(()) => {
                if let Err(error) = self
                    .integrity_cache
                    .remember_verified(&file_path, &manifest.artifacts[0])
                    .await
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model_info) = models.get_mut(model_name) {
                        model_info.status = ModelStatus::Error(error.to_string());
                    }
                    return Err(error);
                }
                let mut models = self.available_models.write().await;
                if let Some(model_info) = models.get_mut(model_name) {
                    model_info.status = ModelStatus::Available;
                    model_info.path = file_path;
                }
                Ok(())
            }
            Err(error) => {
                // A second request is rejected by the shared per-model lease;
                // it must not overwrite the first request's Downloading state.
                if !is_already_active(&error) && !self.downloader.is_active(model_name).await {
                    let mut models = self.available_models.write().await;
                    if let Some(model_info) = models.get_mut(model_name) {
                        model_info.status = if is_cancelled(&error) {
                            ModelStatus::NotDownloaded
                        } else {
                            ModelStatus::Error(error.to_string())
                        };
                    }
                }
                Err(error)
            }
        }
    }

    /// Cancel an ongoing download while keeping its deterministic partial file.
    pub async fn cancel_download(&self, model_name: &str) -> Result<()> {
        log::info!("Cancelling download for model: {}", model_name);
        let was_active = self.downloader.is_active(model_name).await;
        self.downloader.cancel(model_name).await?;
        if was_active {
            let mut models = self.available_models.write().await;
            if let Some(model_info) = models.get_mut(model_name) {
                model_info.status = ModelStatus::NotDownloaded;
            }
        }
        Ok(())
    }

    /// Delete a corrupted or available model file and any resumable candidate.
    pub async fn delete_model(&self, model_name: &str) -> Result<()> {
        log::info!("Deleting model: {}", model_name);
        let model_def = get_model_by_name(model_name)
            .ok_or_else(|| anyhow!("Unknown model: {}", model_name))?;

        self.cancel_download(model_name).await?;
        let file_path = self.models_dir.join(&model_def.gguf_file);
        for path in [&file_path, &partial_file_path(&file_path)] {
            match fs::symlink_metadata(path).await {
                Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
                    fs::remove_file(path).await?;
                }
                Ok(_) => {
                    return Err(anyhow!("model path is not a file: {}", path.display()));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            self.integrity_cache.invalidate(path).await;
        }

        let mut models = self.available_models.write().await;
        if let Some(model_info) = models.get_mut(model_name) {
            model_info.status = ModelStatus::NotDownloaded;
        }
        Ok(())
    }

    /// Get models directory path
    pub fn get_models_directory(&self) -> PathBuf {
        self.models_dir.clone()
    }
}
