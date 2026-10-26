use crate::model_download::{
    is_already_active, is_cancelled, parakeet_manifest, partial_directory_path, IntegrityCache,
    ResumableDownloader,
};
use crate::parakeet_engine::model::ParakeetModel;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

pub use crate::model_download::DownloadProgress;

/// Quantization type for Parakeet models
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum QuantizationType {
    FP32, // Full precision
    #[default]
    Int8, // 8-bit integer quantization (faster)
}

/// Model status for Parakeet models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelStatus {
    Available,
    Missing,
    Downloading {
        progress: u8,
    },
    Error(String),
    Corrupted {
        file_size: u64,
        expected_min_size: u64,
    },
}

/// Information about a Parakeet model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: u32,
    pub quantization: QuantizationType,
    pub speed: String, // Performance description
    pub status: ModelStatus,
    pub description: String,
}

#[derive(Debug)]
pub enum ParakeetEngineError {
    ModelNotLoaded,
    ModelNotFound(String),
    TranscriptionFailed(String),
    DownloadFailed(String),
    IoError(std::io::Error),
    Other(String),
}

impl std::fmt::Display for ParakeetEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParakeetEngineError::ModelNotLoaded => write!(f, "No Parakeet model loaded"),
            ParakeetEngineError::ModelNotFound(name) => write!(f, "Model '{}' not found", name),
            ParakeetEngineError::TranscriptionFailed(err) => {
                write!(f, "Transcription failed: {}", err)
            }
            ParakeetEngineError::DownloadFailed(err) => write!(f, "Download failed: {}", err),
            ParakeetEngineError::IoError(err) => write!(f, "IO error: {}", err),
            ParakeetEngineError::Other(err) => write!(f, "Error: {}", err),
        }
    }
}

impl std::error::Error for ParakeetEngineError {}

impl From<std::io::Error> for ParakeetEngineError {
    fn from(err: std::io::Error) -> Self {
        ParakeetEngineError::IoError(err)
    }
}

pub struct ParakeetEngine {
    models_dir: PathBuf,
    current_model: Arc<RwLock<Option<ParakeetModel>>>,
    current_model_name: Arc<RwLock<Option<String>>>,
    pub(crate) available_models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    downloader: ResumableDownloader,
    integrity_cache: IntegrityCache,
}

impl ParakeetEngine {
    /// Create a new Parakeet engine with optional custom models directory
    pub fn new_with_models_dir(models_dir: Option<PathBuf>) -> Result<Self> {
        let models_dir = if let Some(dir) = models_dir {
            dir.join("parakeet") // Parakeet models in subdirectory
        } else {
            // Fallback to default location
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

            if cfg!(debug_assertions) {
                // Development mode
                current_dir.join("models").join("parakeet")
            } else {
                // Production mode
                dirs::data_dir()
                    .or_else(dirs::home_dir)
                    .ok_or_else(|| anyhow!("Could not find system data directory"))?
                    .join("Meetily")
                    .join("models")
                    .join("parakeet")
            }
        };

        log::info!(
            "ParakeetEngine using models directory: {}",
            models_dir.display()
        );

        // Create directory if it doesn't exist
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir)?;
        }

        Ok(Self {
            models_dir,
            current_model: Arc::new(RwLock::new(None)),
            current_model_name: Arc::new(RwLock::new(None)),
            available_models: Arc::new(RwLock::new(HashMap::new())),
            downloader: ResumableDownloader::new()?,
            integrity_cache: IntegrityCache::new(),
        })
    }

    /// Discover the two supported Parakeet INT8 models.
    pub async fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        let model_configs = [
            (
                "parakeet-tdt-0.6b-v3-int8",
                670,
                "Ultra Fast (v3)",
                "Real time on M4 Max, latest version with int8 quantization",
            ),
            (
                "parakeet-tdt-0.6b-v2-int8",
                661,
                "Fast (v2)",
                "Previous version with int8 quantization, good balance of speed and accuracy",
            ),
        ];
        let mut models = Vec::with_capacity(model_configs.len());

        for (name, size_mb, speed, description) in model_configs {
            let model_path = self.models_dir.join(name);
            let existing_info = {
                let available_models = self.available_models.read().await;
                available_models.get(name).cloned()
            };
            let status = if self.downloader.is_active(name).await {
                existing_info
                    .as_ref()
                    .map(|info| info.status.clone())
                    .filter(|status| matches!(status, ModelStatus::Downloading { .. }))
                    .unwrap_or(ModelStatus::Downloading { progress: 0 })
            } else {
                match parakeet_manifest(name) {
                    Some(manifest) if model_path.exists() => {
                        match self
                            .integrity_cache
                            .verify_bundle_cached(&model_path, &manifest)
                            .await
                        {
                            Ok(()) => ModelStatus::Available,
                            Err(error) => {
                                let file_size = manifest
                                    .artifacts
                                    .iter()
                                    .filter_map(|artifact| {
                                        std::fs::symlink_metadata(
                                            model_path.join(&artifact.filename),
                                        )
                                        .ok()
                                        .map(|metadata| metadata.len())
                                    })
                                    .sum();
                                log::warn!(
                                    "Parakeet model '{}': integrity check failed: {}",
                                    name,
                                    error
                                );
                                ModelStatus::Corrupted {
                                    file_size,
                                    expected_min_size: manifest.total_size(),
                                }
                            }
                        }
                    }
                    Some(_) => ModelStatus::Missing,
                    None => ModelStatus::Error(format!("No trusted manifest for model {name}")),
                }
            };

            models.push(ModelInfo {
                name: name.to_string(),
                path: model_path,
                size_mb,
                quantization: QuantizationType::Int8,
                speed: speed.to_string(),
                status,
                description: description.to_string(),
            });
        }

        let mut available_models = self.available_models.write().await;
        available_models.clear();
        for model in &models {
            available_models.insert(model.name.clone(), model.clone());
        }
        Ok(models)
    }

    /// Load a Parakeet model
    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        // Refresh the exact manifest check before loading.  A file changed
        // outside the app must not remain usable from an old in-memory state.
        self.discover_models().await?;
        let model_info = self
            .available_models
            .read()
            .await
            .get(model_name)
            .cloned()
            .ok_or_else(|| anyhow!("Model {} not found", model_name))?;

        match model_info.status {
            ModelStatus::Available => {
                // Check if this model is already loaded
                if let Some(current_model) = self.current_model_name.read().await.as_ref() {
                    if current_model == model_name {
                        log::info!(
                            "Parakeet model {} is already loaded, skipping reload",
                            model_name
                        );
                        return Ok(());
                    }

                    // Unload current model before loading new one
                    log::info!(
                        "Unloading current Parakeet model '{}' before loading '{}'",
                        current_model,
                        model_name
                    );
                    self.unload_model().await;
                }

                log::info!("Loading Parakeet model: {}", model_name);

                // Load model based on quantization type
                let quantized = model_info.quantization == QuantizationType::Int8;
                let model = ParakeetModel::new(&model_info.path, quantized)
                    .map_err(|e| anyhow!("Failed to load Parakeet model {}: {}", model_name, e))?;

                // Update current model and model name
                *self.current_model.write().await = Some(model);
                *self.current_model_name.write().await = Some(model_name.to_string());

                log::info!(
                    "Successfully loaded Parakeet model: {} ({})",
                    model_name,
                    if quantized { "Int8 quantized" } else { "FP32" }
                );
                Ok(())
            }
            ModelStatus::Missing => Err(anyhow!("Parakeet model {} is not downloaded", model_name)),
            ModelStatus::Downloading { .. } => Err(anyhow!(
                "Parakeet model {} is currently downloading",
                model_name
            )),
            ModelStatus::Error(ref err) => {
                Err(anyhow!("Parakeet model {} has error: {}", model_name, err))
            }
            ModelStatus::Corrupted { .. } => Err(anyhow!(
                "Parakeet model {} is corrupted and cannot be loaded",
                model_name
            )),
        }
    }

    /// Unload the current model
    pub async fn unload_model(&self) -> bool {
        let mut model_guard = self.current_model.write().await;
        let unloaded = model_guard.take().is_some();
        if unloaded {
            log::info!("Parakeet model unloaded");
        }

        let mut model_name_guard = self.current_model_name.write().await;
        model_name_guard.take();

        unloaded
    }

    /// Get the currently loaded model name
    pub async fn get_current_model(&self) -> Option<String> {
        self.current_model_name.read().await.clone()
    }

    /// Check if a model is loaded
    pub async fn is_model_loaded(&self) -> bool {
        self.current_model.read().await.is_some()
    }

    /// Transcribe audio samples using the loaded Parakeet model
    pub async fn transcribe_audio(&self, audio_data: Vec<f32>) -> Result<String> {
        let mut model_guard = self.current_model.write().await;
        let model = model_guard
            .as_mut()
            .ok_or_else(|| anyhow!("No Parakeet model loaded. Please load a model first."))?;

        let duration_seconds = audio_data.len() as f64 / 16000.0; // Assuming 16kHz
        log::debug!(
            "Parakeet transcribing {} samples ({:.1}s duration)",
            audio_data.len(),
            duration_seconds
        );

        // Transcribe using Parakeet model
        let result = model
            .transcribe_samples(audio_data)
            .map_err(|e| anyhow!("Parakeet transcription failed: {}", e))?;

        log::debug!("Parakeet transcription result: '{}'", result.text);

        Ok(result.text)
    }

    /// Get the models directory path
    pub async fn get_models_directory(&self) -> PathBuf {
        self.models_dir.clone()
    }

    /// Delete a model and any resumable staging directory.
    pub async fn delete_model(&self, model_name: &str) -> Result<String> {
        log::info!("Attempting to delete Parakeet model: {}", model_name);
        let model_info = {
            let models = self.available_models.read().await;
            models.get(model_name).cloned()
        }
        .ok_or_else(|| anyhow!("Parakeet model '{}' not found", model_name))?;

        self.cancel_download(model_name).await?;
        for path in [
            model_info.path.clone(),
            partial_directory_path(&model_info.path)?,
        ] {
            match fs::symlink_metadata(&path).await {
                Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
                    fs::remove_file(&path).await?;
                }
                Ok(metadata) if metadata.is_dir() => {
                    fs::remove_dir_all(&path).await?;
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            self.integrity_cache.invalidate(&path).await;
        }

        let mut models = self.available_models.write().await;
        if let Some(model) = models.get_mut(model_name) {
            model.status = ModelStatus::Missing;
        }
        Ok(format!(
            "Successfully deleted Parakeet model '{}'",
            model_name
        ))
    }

    /// Download a Parakeet model with the shared resumable downloader.
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

    /// Download and atomically publish all files in a Parakeet model bundle.
    pub async fn download_model_detailed(
        &self,
        model_name: &str,
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> Result<()> {
        let manifest = parakeet_manifest(model_name)
            .ok_or_else(|| anyhow!("Unknown or unsupported Parakeet model: {}", model_name))?;
        let model_dir = self.models_dir.join(model_name);
        fs::create_dir_all(&self.models_dir).await?;
        {
            let mut models = self.available_models.write().await;
            if let Some(model) = models.get_mut(model_name) {
                model.status = ModelStatus::Downloading { progress: 0 };
            }
        }
        self.integrity_cache
            .invalidate_bundle(&model_dir, &manifest)
            .await;

        log::info!(
            "Downloading Parakeet model '{}' from pinned revision {} to {}",
            model_name,
            manifest.revision,
            model_dir.display()
        );
        let result = self
            .downloader
            .download(model_name, &manifest, &model_dir, progress_callback)
            .await;

        match result {
            Ok(()) => {
                if let Err(error) = self
                    .integrity_cache
                    .remember_bundle_verified(&model_dir, &manifest)
                    .await
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Error(error.to_string());
                    }
                    return Err(error);
                }
                let mut models = self.available_models.write().await;
                if let Some(model) = models.get_mut(model_name) {
                    model.status = ModelStatus::Available;
                    model.path = model_dir;
                }
                Ok(())
            }
            Err(error) => {
                // A concurrent request is rejected by the lease and must not
                // replace the first request's visible Downloading state.
                if !is_already_active(&error) && !self.downloader.is_active(model_name).await {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = if is_cancelled(&error) {
                            ModelStatus::Missing
                        } else {
                            ModelStatus::Error(error.to_string())
                        };
                    }
                }
                Err(error)
            }
        }
    }

    /// Cancel an ongoing model download while retaining its staging files.
    pub async fn cancel_download(&self, model_name: &str) -> Result<()> {
        let was_active = self.downloader.is_active(model_name).await;
        self.downloader.cancel(model_name).await?;
        if was_active {
            let mut models = self.available_models.write().await;
            if let Some(model) = models.get_mut(model_name) {
                model.status = ModelStatus::Missing;
            }
        }
        Ok(())
    }
}
