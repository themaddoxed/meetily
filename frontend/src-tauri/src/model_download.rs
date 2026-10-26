//! Shared, integrity checked model downloads.
//!
//! Model managers own the user visible status and events.  This module owns
//! the filesystem and HTTP boundary so every model uses the same rules:
//! partial files are kept out of the published path, retries are serialized
//! per model, and a file is published only after its exact size and SHA-256
//! match the trusted manifest.

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::redirect::{Attempt, Policy};
use reqwest::{Client, StatusCode, Url};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

pub const CANCELLATION_PREFIX: &str = "CANCELLED:";
const ALREADY_ACTIVE_PREFIX: &str = "DOWNLOAD_ALREADY_ACTIVE:";

/// Detailed progress shared by Whisper, Parakeet and built-in summary models.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub speed_mbps: f64,
    pub percent: u8,
}

impl DownloadProgress {
    pub fn new(downloaded: u64, total: u64, speed_mbps: f64) -> Self {
        let percent = if total == 0 {
            0
        } else {
            ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u8
        };
        Self {
            downloaded_bytes: downloaded,
            total_bytes: total,
            downloaded_mb: downloaded as f64 / 1_048_576.0,
            total_mb: total as f64 / 1_048_576.0,
            speed_mbps,
            percent,
        }
    }
}

pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// One immutable file in a model manifest.
#[derive(Debug, Clone)]
pub struct DownloadArtifact {
    pub filename: String,
    pub url: Url,
    pub size: u64,
    pub sha256: String,
}

impl DownloadArtifact {
    pub fn new(
        filename: impl Into<String>,
        url: &str,
        size: u64,
        sha256: impl Into<String>,
    ) -> Result<Self> {
        let filename = filename.into();
        validate_filename(&filename)?;
        let url = Url::parse(url).with_context(|| format!("invalid model URL: {url}"))?;
        let sha256 = sha256.into().to_ascii_lowercase();
        if size == 0 {
            return Err(anyhow!("model artifact {filename} has an empty size"));
        }
        if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(anyhow!("model artifact {filename} has an invalid SHA-256"));
        }
        Ok(Self {
            filename,
            url,
            size,
            sha256,
        })
    }
}

/// The complete set of files that must be installed for one model.
#[derive(Debug, Clone)]
pub struct DownloadManifest {
    pub model_id: String,
    pub revision: String,
    pub artifacts: Vec<DownloadArtifact>,
}

impl DownloadManifest {
    pub fn new(
        model_id: impl Into<String>,
        revision: impl Into<String>,
        artifacts: Vec<DownloadArtifact>,
    ) -> Result<Self> {
        let model_id = model_id.into();
        validate_model_id(&model_id)?;
        let revision = revision.into();
        if revision.is_empty() {
            return Err(anyhow!("model {model_id} has no pinned revision"));
        }
        if artifacts.is_empty() {
            return Err(anyhow!("model {model_id} has no artifacts"));
        }
        let mut filenames = std::collections::HashSet::new();
        for artifact in &artifacts {
            if !filenames.insert(artifact.filename.clone()) {
                return Err(anyhow!(
                    "model {model_id} contains duplicate artifact {}",
                    artifact.filename
                ));
            }
        }
        Ok(Self {
            model_id,
            revision,
            artifacts,
        })
    }

    pub fn total_size(&self) -> u64 {
        self.artifacts.iter().map(|artifact| artifact.size).sum()
    }
}

/// Network and timeout policy.  Production uses HTTPS and the fixed trusted
/// Hugging Face origin family.  Loopback HTTP is available only for local
/// downloader tests and is never used by product manifests.
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub allow_loopback_http: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(60 * 60),
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(30),
            allow_loopback_http: false,
        }
    }
}

impl DownloadConfig {
    /// Configuration used by the in-process local HTTP test server.
    pub fn local_http_for_tests() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(2),
            idle_timeout: Duration::from_millis(250),
            allow_loopback_http: true,
        }
    }
}

struct ActiveDownload {
    cancellation: CancellationToken,
    completed: Arc<Notify>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IntegrityStamp {
    size: u64,
    modified: Option<SystemTime>,
    // A file can be atomically replaced while preserving its byte count and
    // modification time.  These metadata values identify that replacement
    // without hashing the model again on every discovery poll.
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    ctime: i64,
    #[cfg(unix)]
    ctime_nsec: i64,
}

#[derive(Debug, Clone)]
struct CachedIntegrity {
    stamp: IntegrityStamp,
    sha256: String,
}

/// Cache successful integrity checks until the file's identity or metadata
/// changes.  Model scans can therefore remain responsive without treating a
/// size-only match as an installed model.
#[derive(Clone, Default)]
pub struct IntegrityCache {
    entries: Arc<tokio::sync::RwLock<HashMap<PathBuf, CachedIntegrity>>>,
}

impl IntegrityCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn verify_file_cached(&self, path: &Path, artifact: &DownloadArtifact) -> Result<()> {
        let stamp = match integrity_stamp(path).await {
            Ok(stamp) => stamp,
            Err(error) => {
                self.invalidate(path).await;
                return Err(error);
            }
        };
        {
            let entries = self.entries.read().await;
            if let Some(cached) = entries.get(path) {
                if stamp.modified.is_some()
                    && cached.stamp == stamp
                    && cached.sha256 == artifact.sha256
                {
                    return Ok(());
                }
            }
        }

        verify_file(path, artifact).await?;
        let verified_stamp = integrity_stamp(path).await?;
        if verified_stamp != stamp {
            self.invalidate(path).await;
            return Err(anyhow!(
                "model file changed while it was being verified: {}",
                path.display()
            ));
        }
        if verified_stamp.modified.is_some() {
            self.entries.write().await.insert(
                path.to_path_buf(),
                CachedIntegrity {
                    stamp: verified_stamp,
                    sha256: artifact.sha256.clone(),
                },
            );
        }
        Ok(())
    }

    /// Record a check performed by the downloader at its publish boundary.
    /// The downloader has already checked the complete bytes, so this only
    /// captures the file identity for subsequent scans.
    pub async fn remember_verified(&self, path: &Path, artifact: &DownloadArtifact) -> Result<()> {
        let stamp = match integrity_stamp(path).await {
            Ok(stamp) => stamp,
            Err(error) => {
                self.invalidate(path).await;
                return Err(error);
            }
        };
        if stamp.size != artifact.size {
            self.invalidate(path).await;
            return Err(anyhow!(
                "verified model file changed size before caching: {}",
                path.display()
            ));
        }
        if stamp.modified.is_some() {
            self.entries.write().await.insert(
                path.to_path_buf(),
                CachedIntegrity {
                    stamp,
                    sha256: artifact.sha256.clone(),
                },
            );
        }
        Ok(())
    }

    pub async fn verify_bundle_cached(
        &self,
        path: &Path,
        manifest: &DownloadManifest,
    ) -> Result<()> {
        if is_symlink(path)? || !path.is_dir() {
            return Err(anyhow!(
                "model bundle directory is missing: {}",
                path.display()
            ));
        }
        for artifact in &manifest.artifacts {
            self.verify_file_cached(&path.join(&artifact.filename), artifact)
                .await?;
        }
        Ok(())
    }

    pub async fn remember_bundle_verified(
        &self,
        path: &Path,
        manifest: &DownloadManifest,
    ) -> Result<()> {
        for artifact in &manifest.artifacts {
            self.remember_verified(&path.join(&artifact.filename), artifact)
                .await?;
        }
        Ok(())
    }

    pub async fn invalidate_bundle(&self, path: &Path, manifest: &DownloadManifest) {
        self.invalidate(path).await;
        for artifact in &manifest.artifacts {
            self.invalidate(&path.join(&artifact.filename)).await;
        }
    }

    pub async fn invalidate(&self, path: &Path) {
        self.entries.write().await.remove(path);
    }
}

#[derive(Clone)]
struct DownloadLease {
    cancellation: CancellationToken,
}

/// Shared downloader.  A key has at most one active operation, while
/// different keys may download concurrently.
#[derive(Clone)]
pub struct ResumableDownloader {
    client: Client,
    config: DownloadConfig,
    active: Arc<Mutex<HashMap<String, ActiveDownload>>>,
}

impl ResumableDownloader {
    pub fn new() -> Result<Self> {
        Self::with_config(DownloadConfig::default())
    }

    pub fn with_config(config: DownloadConfig) -> Result<Self> {
        let redirect_config = config.clone();
        let redirect_policy = Policy::custom(move |attempt: Attempt| {
            let target = attempt.url().clone();
            if is_trusted_source(&target, &redirect_config) {
                attempt.follow()
            } else {
                attempt.error(format!("redirect to untrusted model source: {target}"))
            }
        });
        let client = Client::builder()
            .redirect(redirect_policy)
            .tcp_nodelay(true)
            .pool_max_idle_per_host(2)
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .build()
            .context("failed to create model download client")?;
        Ok(Self {
            client,
            config,
            active: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Download and atomically publish one file or one model directory.
    pub async fn download(
        &self,
        key: &str,
        manifest: &DownloadManifest,
        destination: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        validate_model_id(key)?;
        if key != manifest.model_id {
            return Err(anyhow!(
                "download key {key} does not match manifest {}",
                manifest.model_id
            ));
        }
        validate_manifest(manifest, &self.config)?;
        validate_destination(destination, manifest.artifacts.len() > 1)?;

        let lease = self.start(key).await?;
        let result = self
            .download_inner(
                manifest,
                destination,
                progress_callback.as_deref(),
                &lease.cancellation,
            )
            .await;
        self.finish(key).await;
        result
    }

    /// Request cancellation and wait until the prior operation releases its
    /// ownership.  Waiting closes the retry race where a new request starts
    /// while the old response body is still being dropped.
    pub async fn cancel(&self, key: &str) -> Result<()> {
        validate_model_id(key)?;
        let (cancellation, completed) = {
            let active = self.active.lock().await;
            match active.get(key) {
                Some(operation) => (
                    operation.cancellation.clone(),
                    Arc::clone(&operation.completed),
                ),
                None => return Ok(()),
            }
        };
        cancellation.cancel();

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if !self.active.lock().await.contains_key(key) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(anyhow!("timed out waiting for download {key} to cancel"));
            }
            let notified = completed.notified();
            tokio::select! {
                _ = notified => {},
                _ = tokio::time::sleep(Duration::from_millis(25)) => {},
            }
        }
    }

    pub async fn is_active(&self, key: &str) -> bool {
        self.active.lock().await.contains_key(key)
    }

    async fn start(&self, key: &str) -> Result<DownloadLease> {
        let mut active = self.active.lock().await;
        if active.contains_key(key) {
            return Err(anyhow!(
                "{ALREADY_ACTIVE_PREFIX} download already in progress for model {key}"
            ));
        }
        let operation = ActiveDownload {
            cancellation: CancellationToken::new(),
            completed: Arc::new(Notify::new()),
        };
        let lease = DownloadLease {
            cancellation: operation.cancellation.clone(),
        };
        active.insert(key.to_string(), operation);
        Ok(lease)
    }

    async fn finish(&self, key: &str) {
        if let Some(operation) = self.active.lock().await.remove(key) {
            operation.completed.notify_waiters();
        }
    }

    async fn download_inner(
        &self,
        manifest: &DownloadManifest,
        destination: &Path,
        progress_callback: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
        cancellation: &CancellationToken,
    ) -> Result<()> {
        if manifest.artifacts.len() == 1 {
            self.download_single(manifest, destination, progress_callback, cancellation)
                .await
        } else {
            self.download_bundle(manifest, destination, progress_callback, cancellation)
                .await
        }
    }

    async fn download_single(
        &self,
        manifest: &DownloadManifest,
        destination: &Path,
        progress_callback: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
        cancellation: &CancellationToken,
    ) -> Result<()> {
        let artifact = &manifest.artifacts[0];
        if verify_file_with_cancellation(destination, artifact, Some(cancellation))
            .await
            .is_ok()
        {
            emit_progress(progress_callback, artifact.size, artifact.size, 0.0);
            return Ok(());
        }
        if cancellation.is_cancelled() {
            return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
        }

        if is_symlink(destination)? {
            return Err(anyhow!(
                "refusing to use symlink as model destination {}",
                destination.display()
            ));
        }

        let partial = partial_file_path(destination);
        if verify_file_with_cancellation(&partial, artifact, Some(cancellation))
            .await
            .is_ok()
        {
            if cancellation.is_cancelled() {
                return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
            }
            atomic_replace_file(&partial, destination).await?;
            emit_progress(progress_callback, artifact.size, artifact.size, 0.0);
            return Ok(());
        }
        if cancellation.is_cancelled() {
            return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
        }
        if partial.exists() && fs::metadata(&partial).await?.len() > artifact.size {
            fs::remove_file(&partial).await.with_context(|| {
                format!("failed to remove oversized partial {}", partial.display())
            })?;
        }

        let initial = file_len(&partial).await?.min(artifact.size);
        emit_progress(progress_callback, initial, artifact.size, 0.0);
        self.download_artifact(
            artifact,
            &partial,
            initial,
            0,
            artifact.size,
            progress_callback,
            cancellation,
        )
        .await?;

        if let Err(error) =
            verify_file_with_cancellation(&partial, artifact, Some(cancellation)).await
        {
            if is_cancelled(&error) {
                return Err(error);
            }
            return Err(error)
                .with_context(|| format!("integrity check failed for {}", artifact.filename));
        }
        if cancellation.is_cancelled() {
            return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
        }
        atomic_replace_file(&partial, destination).await?;
        emit_progress(progress_callback, artifact.size, artifact.size, 0.0);
        Ok(())
    }

    async fn download_bundle(
        &self,
        manifest: &DownloadManifest,
        destination: &Path,
        progress_callback: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
        cancellation: &CancellationToken,
    ) -> Result<()> {
        if verify_bundle_with_cancellation(destination, manifest, cancellation)
            .await
            .is_ok()
        {
            let total = manifest.total_size();
            emit_progress(progress_callback, total, total, 0.0);
            return Ok(());
        }
        if cancellation.is_cancelled() {
            return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
        }

        if is_symlink(destination)? {
            return Err(anyhow!(
                "refusing to use symlink as model directory {}",
                destination.display()
            ));
        }

        let stage = partial_directory_path(destination)?;
        if !stage.exists() {
            // Keep the currently installed directory in place while a
            // replacement is assembled.  A failed retry must never make a
            // previously usable model disappear. New downloads use the
            // hidden staging directory below; an interrupted retry can
            // resume from it on the next attempt.
            fs::create_dir_all(&stage).await?;
        }
        if !stage.is_dir() || is_symlink(&stage)? {
            return Err(anyhow!(
                "invalid model staging directory {}",
                stage.display()
            ));
        }

        let total = manifest.total_size();
        let mut downloaded = 0u64;
        for artifact in &manifest.artifacts {
            downloaded += file_len(&stage.join(&artifact.filename))
                .await?
                .min(artifact.size);
        }
        emit_progress(progress_callback, downloaded, total, 0.0);

        for artifact in &manifest.artifacts {
            let path = stage.join(&artifact.filename);
            let existing = file_len(&path).await?.min(artifact.size);
            self.download_artifact(
                artifact,
                &path,
                existing,
                downloaded.saturating_sub(existing),
                total,
                progress_callback,
                cancellation,
            )
            .await?;
            downloaded = downloaded.saturating_sub(existing) + artifact.size;
        }

        // Re-check immediately before the directory swap.  The staging
        // directory is private, but this closes a race with external tools
        // touching files while the final artifact is being prepared.
        if let Err(error) = verify_bundle_with_cancellation(&stage, manifest, cancellation).await {
            if is_cancelled(&error) {
                return Err(error);
            }
            return Err(error).context("model bundle failed its final integrity check");
        }
        if cancellation.is_cancelled() {
            return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
        }
        atomic_replace_directory(&stage, destination).await?;
        emit_progress(progress_callback, total, total, 0.0);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_artifact(
        &self,
        artifact: &DownloadArtifact,
        path: &Path,
        mut offset: u64,
        completed_before: u64,
        total: u64,
        progress_callback: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
        cancellation: &CancellationToken,
    ) -> Result<()> {
        validate_filename(&artifact.filename)?;
        if is_symlink(path)? {
            return Err(anyhow!(
                "refusing to write through symlink {}",
                path.display()
            ));
        }
        let mut progress_reset = false;
        if offset > artifact.size {
            remove_file_if_exists(path).await?;
            progress_reset = offset > 0;
            offset = 0;
        }
        if offset == artifact.size {
            if verify_file_with_cancellation(path, artifact, Some(cancellation))
                .await
                .is_ok()
            {
                return Ok(());
            }
            if cancellation.is_cancelled() {
                return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
            }
            remove_file_if_exists(path).await?;
            progress_reset = true;
            offset = 0;
        }

        let mut response = self.send(artifact, offset, cancellation).await?;
        let mut resuming = false;
        match response.status() {
            StatusCode::PARTIAL_CONTENT if offset > 0 => {
                let content_range = response
                    .headers()
                    .get(reqwest::header::CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_content_range);
                let valid_range = content_range.filter(|(start, end, total_size)| {
                    *start == offset
                        && *end >= *start
                        && end.saturating_add(1) <= artifact.size
                        && *total_size == artifact.size
                });
                let valid_length = valid_range.is_some_and(|(_, end, _)| {
                    response
                        .content_length()
                        .map_or(true, |length| length == end.saturating_sub(offset) + 1)
                });
                if valid_range.is_some() && valid_length {
                    resuming = true;
                } else {
                    // A malformed 206 must never be appended to an existing
                    // candidate.  Retry once from byte zero, accepting only
                    // a normal 200 response.
                    remove_file_if_exists(path).await?;
                    progress_reset = offset > 0;
                    offset = 0;
                    response = self.send(artifact, 0, cancellation).await?;
                    if response.status() != StatusCode::OK {
                        return Err(anyhow!(
                            "malformed partial response for {} (status {})",
                            artifact.filename,
                            response.status()
                        ));
                    }
                }
            }
            StatusCode::PARTIAL_CONTENT => {
                // We did not ask for a range.  Treat an unsolicited or
                // malformed partial response as unsafe and retry fresh.
                remove_file_if_exists(path).await?;
                progress_reset = offset > 0;
                response = self.send(artifact, 0, cancellation).await?;
                if response.status() != StatusCode::OK {
                    return Err(anyhow!(
                        "unexpected partial response for {}",
                        artifact.filename
                    ));
                }
                offset = 0;
            }
            StatusCode::OK => {
                // A server that ignores Range is safe when we truncate and
                // restart from the beginning.
                resuming = false;
                if offset > 0 {
                    progress_reset = true;
                    offset = 0;
                }
            }
            StatusCode::RANGE_NOT_SATISFIABLE => {
                if offset == artifact.size
                    && verify_file_with_cancellation(path, artifact, Some(cancellation))
                        .await
                        .is_ok()
                {
                    return Ok(());
                }
                if cancellation.is_cancelled() {
                    return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
                }
                remove_file_if_exists(path).await?;
                progress_reset = offset > 0;
                offset = 0;
                response = self.send(artifact, 0, cancellation).await?;
                if response.status() != StatusCode::OK {
                    return Err(anyhow!(
                        "range retry for {} failed with status {}",
                        artifact.filename,
                        response.status()
                    ));
                }
            }
            status => {
                return Err(anyhow!(
                    "download for {} failed with status {}",
                    artifact.filename,
                    status
                ));
            }
        }

        if let Some(length) = response.content_length() {
            let remaining = artifact.size.saturating_sub(offset);
            if length > remaining {
                remove_file_if_exists(path).await?;
                return Err(anyhow!(
                    "download for {} is larger than its manifest",
                    artifact.filename
                ));
            }
        }

        // A 200 fallback discards the current artifact's partial bytes. Make
        // that restart visible before reporting newly received bytes, while
        // retaining progress for artifacts that were already complete.
        if progress_reset {
            emit_progress(progress_callback, completed_before, total, 0.0);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(!resuming)
            .append(resuming)
            .open(path)
            .await
            .with_context(|| format!("failed to open partial model {}", path.display()))?;
        let mut downloaded = if resuming { offset } else { 0 };
        let started = Instant::now();
        let mut stream = response.bytes_stream();

        loop {
            enum Next {
                Cancelled,
                Item(Option<std::result::Result<bytes::Bytes, reqwest::Error>>),
            }
            let next = tokio::time::timeout(self.config.idle_timeout, async {
                tokio::select! {
                    _ = cancellation.cancelled() => Next::Cancelled,
                    item = stream.next() => Next::Item(item),
                }
            })
            .await;
            let next = match next {
                Ok(next) => next,
                Err(_) => {
                    file.flush().await.ok();
                    return Err(anyhow!(
                        "download stalled for {}: no data received before timeout",
                        artifact.filename
                    ));
                }
            };
            let chunk = match next {
                Next::Cancelled => {
                    file.flush().await.ok();
                    return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
                }
                Next::Item(None) => break,
                Next::Item(Some(Ok(chunk))) => chunk,
                Next::Item(Some(Err(error))) => {
                    file.flush().await.ok();
                    return Err(anyhow!(
                        "download stream failed for {}: {error}",
                        artifact.filename
                    ));
                }
            };
            if cancellation.is_cancelled() {
                file.flush().await.ok();
                return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
            }
            if downloaded.saturating_add(chunk.len() as u64) > artifact.size {
                file.flush().await.ok();
                remove_file_if_exists(path).await?;
                return Err(anyhow!(
                    "download for {} exceeded its manifest size",
                    artifact.filename
                ));
            }
            file.write_all(&chunk)
                .await
                .with_context(|| format!("failed to write partial model {}", path.display()))?;
            downloaded += chunk.len() as u64;
            let speed = downloaded.saturating_sub(offset) as f64
                / 1_048_576.0
                / started.elapsed().as_secs_f64().max(0.001);
            emit_progress(
                progress_callback,
                completed_before.saturating_add(downloaded),
                total,
                speed,
            );
        }
        file.flush().await?;
        drop(file);

        if downloaded != artifact.size {
            return Err(anyhow!(
                "download for {} ended early at {} of {} bytes",
                artifact.filename,
                downloaded,
                artifact.size
            ));
        }
        if let Err(error) = verify_file_with_cancellation(path, artifact, Some(cancellation)).await
        {
            if !is_cancelled(&error) {
                remove_file_if_exists(path).await?;
            }
            return Err(error);
        }
        Ok(())
    }

    async fn send(
        &self,
        artifact: &DownloadArtifact,
        offset: u64,
        cancellation: &CancellationToken,
    ) -> Result<reqwest::Response> {
        if cancellation.is_cancelled() {
            return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
        }
        let mut request = self.client.get(artifact.url.clone());
        if offset > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        tokio::select! {
            _ = cancellation.cancelled() => Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user")),
            result = request.send() => result.map_err(|error| anyhow!("failed to request {}: {error}", artifact.filename)),
        }
    }
}

pub fn is_cancelled(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.to_string().starts_with(CANCELLATION_PREFIX))
}

/// Return true when a caller attempted to start a second download for the
/// same model. Managers use this marker to leave the first operation's
/// visible status untouched, even if it finishes before the second caller
/// inspects the active map.
pub fn is_already_active(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.to_string().starts_with(ALREADY_ACTIVE_PREFIX))
}

async fn integrity_stamp(path: &Path) -> Result<IntegrityStamp> {
    let metadata = fs::symlink_metadata(path)
        .await
        .with_context(|| format!("model file is missing: {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(anyhow!("model file is a symlink: {}", path.display()));
    }
    if !metadata.is_file() {
        return Err(anyhow!("model path is not a file: {}", path.display()));
    }
    Ok(IntegrityStamp {
        size: metadata.len(),
        modified: metadata.modified().ok(),
        #[cfg(unix)]
        device: metadata.dev(),
        #[cfg(unix)]
        inode: metadata.ino(),
        #[cfg(unix)]
        ctime: metadata.ctime(),
        #[cfg(unix)]
        ctime_nsec: metadata.ctime_nsec(),
    })
}

pub async fn verify_file(path: &Path, artifact: &DownloadArtifact) -> Result<()> {
    verify_file_with_cancellation(path, artifact, None).await
}

async fn verify_file_with_cancellation(
    path: &Path,
    artifact: &DownloadArtifact,
    cancellation: Option<&CancellationToken>,
) -> Result<()> {
    let before = integrity_stamp(path).await?;
    if before.size != artifact.size {
        return Err(anyhow!(
            "model file {} has {} bytes, expected {}",
            path.display(),
            before.size,
            artifact.size
        ));
    }
    let mut file = fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = if let Some(cancellation) = cancellation {
            tokio::select! {
                _ = cancellation.cancelled() => {
                    return Err(anyhow!("{CANCELLATION_PREFIX} download cancelled by user"));
                }
                result = file.read(&mut buffer) => result?,
            }
        } else {
            file.read(&mut buffer).await?
        };
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != artifact.sha256 {
        return Err(anyhow!(
            "model file {} has SHA-256 {}, expected {}",
            path.display(),
            actual,
            artifact.sha256
        ));
    }
    let after = integrity_stamp(path).await?;
    if after != before {
        return Err(anyhow!(
            "model file changed while it was being verified: {}",
            path.display()
        ));
    }
    Ok(())
}

pub async fn verify_bundle(path: &Path, manifest: &DownloadManifest) -> Result<()> {
    verify_bundle_with_cancellation(path, manifest, &CancellationToken::new()).await
}

async fn verify_bundle_with_cancellation(
    path: &Path,
    manifest: &DownloadManifest,
    cancellation: &CancellationToken,
) -> Result<()> {
    if is_symlink(path)? || !path.is_dir() {
        return Err(anyhow!(
            "model bundle directory is missing: {}",
            path.display()
        ));
    }
    for artifact in &manifest.artifacts {
        verify_file_with_cancellation(&path.join(&artifact.filename), artifact, Some(cancellation))
            .await?;
    }
    Ok(())
}

fn validate_manifest(manifest: &DownloadManifest, config: &DownloadConfig) -> Result<()> {
    validate_model_id(&manifest.model_id)?;
    if manifest.revision.is_empty() {
        return Err(anyhow!(
            "model {} has no pinned revision",
            manifest.model_id
        ));
    }
    if manifest.artifacts.is_empty() {
        return Err(anyhow!("model {} has no artifacts", manifest.model_id));
    }
    let mut filenames = std::collections::HashSet::new();
    for artifact in &manifest.artifacts {
        validate_filename(&artifact.filename)?;
        if artifact.size == 0 {
            return Err(anyhow!(
                "model artifact {} has an empty size",
                artifact.filename
            ));
        }
        if artifact.sha256.len() != 64
            || !artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(anyhow!(
                "model artifact {} has an invalid SHA-256",
                artifact.filename
            ));
        }
        if !filenames.insert(&artifact.filename) {
            return Err(anyhow!(
                "model {} contains duplicate artifact {}",
                manifest.model_id,
                artifact.filename
            ));
        }
        if !is_trusted_source(&artifact.url, config) {
            return Err(anyhow!(
                "model source is not a trusted HTTPS origin: {}",
                artifact.url
            ));
        }
    }
    Ok(())
}

fn is_trusted_source(url: &Url, config: &DownloadConfig) -> bool {
    if config.allow_loopback_http
        && url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| host == "localhost" || host == "127.0.0.1" || host == "::1")
    {
        return true;
    }
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    // Hugging Face redirects model payloads to regional hf.co and
    // cdn-lfs.huggingface.co hosts.  The dot boundary avoids accepting a
    // lookalike host such as evilhf.co.
    host == "huggingface.co"
        || host.ends_with(".huggingface.co")
        || host == "hf.co"
        || host.ends_with(".hf.co")
}

fn validate_model_id(model_id: &str) -> Result<()> {
    if model_id.is_empty()
        || model_id.contains('/')
        || model_id.contains('\\')
        || model_id.contains("..")
        || model_id.chars().any(char::is_control)
    {
        return Err(anyhow!("invalid model id: {model_id}"));
    }
    Ok(())
}

fn validate_filename(filename: &str) -> Result<()> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename == "."
        || filename == ".."
        || filename.chars().any(char::is_control)
    {
        return Err(anyhow!("invalid model artifact filename: {filename}"));
    }
    let mut components = Path::new(filename).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(anyhow!(
            "model artifact filename must be a single path component"
        ));
    }
    Ok(())
}

fn validate_destination(destination: &Path, is_directory: bool) -> Result<()> {
    if destination.as_os_str().is_empty()
        || destination
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return Err(anyhow!(
            "invalid model destination: {}",
            destination.display()
        ));
    }
    if is_directory {
        if destination.file_name().is_none() {
            return Err(anyhow!("model directory has no final component"));
        }
    } else if destination.file_name().is_none() {
        return Err(anyhow!("model file has no final component"));
    }
    Ok(())
}

pub fn partial_file_path(destination: &Path) -> PathBuf {
    let mut value = destination.as_os_str().to_os_string();
    value.push(".part");
    PathBuf::from(value)
}

pub fn partial_directory_path(destination: &Path) -> Result<PathBuf> {
    let name = destination
        .file_name()
        .ok_or_else(|| anyhow!("model directory has no final component"))?
        .to_string_lossy();
    Ok(destination.with_file_name(format!(".{name}.download")))
}

async fn atomic_replace_file(partial: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::rename(partial, destination)
        .await
        .with_context(|| format!("failed to publish verified model {}", destination.display()))
}

async fn atomic_replace_directory(stage: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("model destination has no parent"))?;
    fs::create_dir_all(parent).await?;
    let backup = destination.with_file_name(format!(
        ".{}.previous",
        destination
            .file_name()
            .ok_or_else(|| anyhow!("model destination has no final component"))?
            .to_string_lossy()
    ));
    if backup.exists() {
        remove_path_if_exists(&backup).await?;
    }
    if destination.exists() {
        fs::rename(destination, &backup)
            .await
            .with_context(|| format!("failed to stage previous model {}", destination.display()))?;
    }
    if let Err(error) = fs::rename(stage, destination).await {
        if backup.exists() {
            let _ = fs::rename(&backup, destination).await;
        }
        return Err(error).with_context(|| {
            format!(
                "failed to publish verified model directory {}",
                destination.display()
            )
        });
    }
    remove_path_if_exists(&backup).await?;
    Ok(())
}

async fn file_len(path: &Path) -> Result<u64> {
    match fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(anyhow!("model partial is a symlink: {}", path.display()))
        }
        Ok(metadata) if metadata.is_file() => Ok(metadata.len()),
        Ok(_) => Err(anyhow!("model partial is not a file: {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

fn is_symlink(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

async fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn remove_path_if_exists(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            fs::remove_file(path).await?;
        }
        Ok(metadata) if metadata.is_dir() => {
            fs::remove_dir_all(path).await?;
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn parse_content_range(value: &str) -> Option<(u64, u64, u64)> {
    let value = value.strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let total = total.parse::<u64>().ok()?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?, total))
}

fn emit_progress(
    callback: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
    downloaded: u64,
    total: u64,
    speed_mbps: f64,
) {
    if let Some(callback) = callback {
        callback(DownloadProgress::new(
            downloaded.min(total),
            total,
            speed_mbps,
        ));
    }
}

/// Build a pinned Hugging Face URL for a repository revision and file.
fn hf_url(repo: &str, revision: &str, filename: &str) -> String {
    format!("https://huggingface.co/{repo}/resolve/{revision}/{filename}")
}

fn whisper_artifact(filename: &str, size: u64, sha256: &str) -> Result<DownloadArtifact> {
    DownloadArtifact::new(
        filename,
        &hf_url(
            "ggerganov/whisper.cpp",
            "5359861c739e955e79d9a303bcbc70fb988958b1",
            filename,
        ),
        size,
        sha256,
    )
}

pub fn whisper_manifest(model_name: &str) -> Option<DownloadManifest> {
    let entries = [
        (
            "tiny",
            "ggml-tiny.bin",
            77_691_713,
            "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        ),
        (
            "base",
            "ggml-base.bin",
            147_951_465,
            "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        ),
        (
            "small",
            "ggml-small.bin",
            487_601_967,
            "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        ),
        (
            "medium",
            "ggml-medium.bin",
            1_533_763_059,
            "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        ),
        (
            "large-v3-turbo",
            "ggml-large-v3-turbo.bin",
            1_624_555_275,
            "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
        ),
        (
            "large-v3",
            "ggml-large-v3.bin",
            3_095_033_483,
            "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        ),
        (
            "tiny-q5_1",
            "ggml-tiny-q5_1.bin",
            32_152_673,
            "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7",
        ),
        (
            "base-q5_1",
            "ggml-base-q5_1.bin",
            59_707_625,
            "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
        ),
        (
            "small-q5_1",
            "ggml-small-q5_1.bin",
            190_085_487,
            "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
        ),
        (
            "medium-q5_0",
            "ggml-medium-q5_0.bin",
            539_212_467,
            "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f",
        ),
        (
            "large-v3-turbo-q5_0",
            "ggml-large-v3-turbo-q5_0.bin",
            574_041_195,
            "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        ),
        (
            "large-v3-q5_0",
            "ggml-large-v3-q5_0.bin",
            1_081_140_203,
            "d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1",
        ),
    ];
    let (_, filename, size, sha256) = entries.iter().find(|(name, ..)| *name == model_name)?;
    let artifact = whisper_artifact(filename, *size, sha256).ok()?;
    DownloadManifest::new(
        model_name,
        "5359861c739e955e79d9a303bcbc70fb988958b1",
        vec![artifact],
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
fn parakeet_manifest_for(
    model_name: &str,
    revision: &str,
    repo: &str,
    decoder_size: u64,
    decoder_sha256: &str,
    encoder_size: u64,
    encoder_sha256: &str,
    vocab_size: u64,
    vocab_sha256: &str,
) -> Option<DownloadManifest> {
    let artifacts = vec![
        DownloadArtifact::new(
            "encoder-model.int8.onnx",
            &hf_url(repo, revision, "encoder-model.int8.onnx"),
            encoder_size,
            encoder_sha256,
        )
        .ok()?,
        DownloadArtifact::new(
            "decoder_joint-model.int8.onnx",
            &hf_url(repo, revision, "decoder_joint-model.int8.onnx"),
            decoder_size,
            decoder_sha256,
        )
        .ok()?,
        DownloadArtifact::new(
            "nemo128.onnx",
            &hf_url(repo, revision, "nemo128.onnx"),
            139_764,
            "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
        )
        .ok()?,
        DownloadArtifact::new(
            "vocab.txt",
            &hf_url(repo, revision, "vocab.txt"),
            vocab_size,
            vocab_sha256,
        )
        .ok()?,
    ];
    DownloadManifest::new(model_name, revision, artifacts).ok()
}

pub fn parakeet_manifest(model_name: &str) -> Option<DownloadManifest> {
    match model_name {
        "parakeet-tdt-0.6b-v2-int8" => parakeet_manifest_for(
            model_name,
            "0bbb45a3365852604aef28b538a8f066f4ccaa85",
            "istupakov/parakeet-tdt-0.6b-v2-onnx",
            8_998_286,
            "a449f49acd68979d418651dd2dcb737cc0f1bf0225e009e29ee326354edbf7d3",
            652_184_014,
            "3e0581fda6ab843888b51e56d7ee78b6d5bc3237ec113af1f732d1d5286aa155",
            9_384,
            "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
        ),
        "parakeet-tdt-0.6b-v3-int8" => parakeet_manifest_for(
            model_name,
            "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce",
            "istupakov/parakeet-tdt-0.6b-v3-onnx",
            18_202_004,
            "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
            652_183_999,
            "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
            93_939,
            "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
        ),
        _ => None,
    }
}

pub fn summary_manifest(model_name: &str, filename: &str) -> Option<DownloadManifest> {
    let (repo, revision, size, sha256) = match model_name {
        "qwen3.5:2b" => (
            "unsloth/Qwen3.5-2B-GGUF",
            "f6d5376be1edb4d416d56da11e5397a961aca8ae",
            1_280_835_840,
            "aaf42c8b7c3cab2bf3d69c355048d4a0ee9973d48f16c731c0520ee914699223",
        ),
        "qwen3.5:4b" => (
            "unsloth/Qwen3.5-4B-GGUF",
            "e87f176479d0855a907a41277aca2f8ee7a09523",
            2_740_937_888,
            "00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4",
        ),
        "gemma4:e2b" => (
            "unsloth/gemma-4-E2B-it-GGUF",
            "90f9618340396838ee7ff5b0ba2da27da62953d3",
            3_106_736_256,
            "9378bc471710229ef165709b62e34bfb62231420ddaf6d729e727305b5b8672d",
        ),
        "gemma4:e4b" => (
            "ggml-org/gemma-4-E4B-it-GGUF",
            "2714b5519c6c3516b1000e7c5e1eba998dfe1fe8",
            5_335_289_824,
            "90ce98129eb3e8cc57e62433d500c97c624b1e3af1fcc85dd3b55ad7e0313e9f",
        ),
        "gemma3:4b" => (
            "bartowski/google_gemma-3-4b-it-GGUF",
            "71506238f970075ca85125cd749c28b1b0eee84e",
            2_489_758_112,
            "4996030242583a40aa151ff93f49ed787ac8c25e4120c3ae4588b2e2a7d1ae94",
        ),
        "gemma3:1b" => (
            "bartowski/google_gemma-3-1b-it-GGUF",
            "116f76234503685a98f572982177b11d44ec8ff1",
            1_069_306_624,
            "375e12a4a18929a641f9744b060d4a7cf4e279530750555828ec0c117870bc96",
        ),
        _ => return None,
    };
    let remote_filename = match model_name {
        "gemma3:4b" => "google_gemma-3-4b-it-Q4_K_M.gguf",
        "gemma3:1b" => "google_gemma-3-1b-it-Q8_0.gguf",
        _ => filename,
    };
    let artifact = DownloadArtifact::new(
        filename,
        &hf_url(repo, revision, remote_filename),
        size,
        sha256,
    )
    .ok()?;
    DownloadManifest::new(model_name, revision, vec![artifact]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;

    fn test_artifact(url: String, bytes: &[u8]) -> DownloadArtifact {
        let digest = format!("{:x}", Sha256::digest(bytes));
        DownloadArtifact::new("model.bin", &url, bytes.len() as u64, digest).unwrap()
    }

    fn manifest(key: &str, artifact: DownloadArtifact) -> DownloadManifest {
        DownloadManifest::new(key, "test-revision", vec![artifact]).unwrap()
    }

    struct TestServer {
        address: String,
        stopped: Arc<AtomicBool>,
        requests: Arc<AtomicUsize>,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn new<F>(handler: F) -> Self
        where
            F: Fn(TcpStream, String) + Send + Sync + 'static,
        {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = format!("http://{}", listener.local_addr().unwrap());
            let stopped = Arc::new(AtomicBool::new(false));
            let requests = Arc::new(AtomicUsize::new(0));
            let stopped_thread = Arc::clone(&stopped);
            let requests_thread = Arc::clone(&requests);
            let handler = Arc::new(handler);
            let thread = thread::spawn(move || {
                while !stopped_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            requests_thread.fetch_add(1, Ordering::Relaxed);
                            // The listener is nonblocking so the accept loop can poll it,
                            // but request parsing must wait for the client's headers.
                            stream.set_nonblocking(false).unwrap();
                            let mut request = Vec::new();
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut buffer = [0u8; 1024];
                            loop {
                                match stream.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(read) => {
                                        request.extend_from_slice(&buffer[..read]);
                                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                            let request = String::from_utf8_lossy(&request).to_string();
                            handler(stream, request);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                address,
                stopped,
                requests,
                thread: Some(thread),
            }
        }

        fn url(&self) -> String {
            format!("{}/model.bin", self.address)
        }

        fn stop(mut self) -> usize {
            self.stopped.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
            self.requests.load(Ordering::Relaxed)
        }
    }

    fn response(stream: &mut TcpStream, status: &str, headers: &[(&str, String)], body: &[u8]) {
        let mut head = format!("HTTP/1.1 {status}\r\n");
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        ));
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    }

    fn requested_range(request: &str) -> Option<u64> {
        request.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if !name.eq_ignore_ascii_case("Range") {
                return None;
            }
            let value = value.trim().strip_prefix("bytes=")?;
            value.strip_suffix('-')?.parse().ok()
        })
    }

    #[tokio::test]
    async fn interrupted_response_keeps_partial_and_resumes_with_valid_content_range() {
        let payload = b"0123456789abcdef".to_vec();
        let first = Arc::new(AtomicBool::new(true));
        let first_thread = Arc::clone(&first);
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, request| {
            if first_thread.swap(false, Ordering::SeqCst) {
                response(
                    &mut stream,
                    "200 OK",
                    &[],
                    &payload_thread[..payload_thread.len() / 2],
                );
            } else if let Some(offset) = requested_range(&request) {
                let body = &payload_thread[offset as usize..];
                response(
                    &mut stream,
                    "206 Partial Content",
                    &[(
                        "Content-Range",
                        format!(
                            "bytes {}-{}/{}",
                            offset,
                            payload_thread.len() - 1,
                            payload_thread.len()
                        ),
                    )],
                    body,
                );
            } else {
                response(&mut stream, "200 OK", &[], &payload_thread);
            }
        });
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("resume", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        let first_result = downloader
            .download("resume", &manifest, &destination, None)
            .await;
        assert!(first_result.is_err());
        assert!(!destination.exists());
        assert!(partial_file_path(&destination).exists());
        downloader
            .download("resume", &manifest, &destination, None)
            .await
            .unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), payload);
        assert!(!partial_file_path(&destination).exists());
        assert!(server.stop() >= 2);
    }

    #[tokio::test]
    async fn resumed_single_progress_includes_existing_partial_bytes() {
        let payload = vec![42u8; 100];
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, request| {
            let offset = requested_range(&request).expect("resume request should include Range");
            assert_eq!(offset, 80);
            response(
                &mut stream,
                "206 Partial Content",
                &[(
                    "Content-Range",
                    format!("bytes 80-99/{}", payload_thread.len()),
                )],
                &payload_thread[80..],
            );
        });
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("single-progress", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        fs::write(partial_file_path(&destination), &payload[..80])
            .await
            .unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_thread = Arc::clone(&progress);

        downloader
            .download(
                "single-progress",
                &manifest,
                &destination,
                Some(Box::new(move |value| {
                    progress_thread.lock().unwrap().push(value.downloaded_bytes);
                })),
            )
            .await
            .unwrap();

        let values = progress.lock().unwrap().clone();
        assert_eq!(values.first(), Some(&80));
        assert_eq!(values.last(), Some(&100));
        assert!(values.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(values.iter().all(|value| *value <= 100));
        assert_eq!(fs::read(&destination).await.unwrap(), payload);
        server.stop();
    }

    #[tokio::test]
    async fn resumed_bundle_progress_includes_each_artifact_partial() {
        let first_payload = vec![1u8; 100];
        let second_payload = vec![2u8; 100];
        let second_thread = second_payload.clone();
        let server = TestServer::new(move |mut stream, request| {
            assert!(
                request.starts_with("GET /second.bin"),
                "unexpected local request: {request:?}"
            );
            let offset = requested_range(&request).expect("bundle resume should include Range");
            assert_eq!(offset, 80);
            response(
                &mut stream,
                "206 Partial Content",
                &[(
                    "Content-Range",
                    format!("bytes 80-99/{}", second_thread.len()),
                )],
                &second_thread[80..],
            );
        });
        let first_artifact = DownloadArtifact::new(
            "first.bin",
            &format!("{}/first.bin", server.address),
            first_payload.len() as u64,
            format!("{:x}", Sha256::digest(&first_payload)),
        )
        .unwrap();
        let second_artifact = DownloadArtifact::new(
            "second.bin",
            &format!("{}/second.bin", server.address),
            second_payload.len() as u64,
            format!("{:x}", Sha256::digest(&second_payload)),
        )
        .unwrap();
        let manifest = DownloadManifest::new(
            "bundle-progress",
            "test-revision",
            vec![first_artifact, second_artifact],
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("bundle");
        let stage = partial_directory_path(&destination).unwrap();
        fs::create_dir_all(&stage).await.unwrap();
        fs::write(stage.join("first.bin"), &first_payload)
            .await
            .unwrap();
        fs::write(stage.join("second.bin"), &second_payload[..80])
            .await
            .unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_thread = Arc::clone(&progress);

        downloader
            .download(
                "bundle-progress",
                &manifest,
                &destination,
                Some(Box::new(move |value| {
                    progress_thread.lock().unwrap().push(value.downloaded_bytes);
                })),
            )
            .await
            .unwrap();

        let values = progress.lock().unwrap().clone();
        assert_eq!(values.first(), Some(&180));
        assert_eq!(values.last(), Some(&200));
        assert!(values.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(values.iter().all(|value| *value <= 200));
        assert_eq!(
            fs::read(destination.join("first.bin")).await.unwrap(),
            first_payload
        );
        assert_eq!(
            fs::read(destination.join("second.bin")).await.unwrap(),
            second_payload
        );
        server.stop();
    }

    #[tokio::test]
    async fn ignored_range_200_reports_restart_without_exceeding_total() {
        let payload = vec![42u8; 100];
        let payload_thread = payload.clone();
        let saw_range = Arc::new(AtomicBool::new(false));
        let saw_range_thread = Arc::clone(&saw_range);
        let server = TestServer::new(move |mut stream, request| {
            if requested_range(&request).is_some() {
                saw_range_thread.store(true, Ordering::SeqCst);
            }
            response(&mut stream, "200 OK", &[], &payload_thread);
        });
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("restart-progress", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        fs::write(partial_file_path(&destination), &payload[..80])
            .await
            .unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_thread = Arc::clone(&progress);

        downloader
            .download(
                "restart-progress",
                &manifest,
                &destination,
                Some(Box::new(move |value| {
                    progress_thread.lock().unwrap().push(value.downloaded_bytes);
                })),
            )
            .await
            .unwrap();

        let values = progress.lock().unwrap().clone();
        assert!(saw_range.load(Ordering::SeqCst));
        assert_eq!(values.first(), Some(&80));
        assert!(values.contains(&0), "fallback should report its restart");
        assert_eq!(values.last(), Some(&100));
        assert!(values.iter().all(|value| *value <= 100));
        assert_eq!(fs::read(destination).await.unwrap(), payload);
        server.stop();
    }

    #[tokio::test]
    async fn ignored_range_200_restarts_without_duplicate_bytes() {
        let payload = b"abcdefghij".to_vec();
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, _request| {
            response(&mut stream, "200 OK", &[], &payload_thread);
        });
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("restart", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        fs::write(partial_file_path(&destination), b"stale")
            .await
            .unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        downloader
            .download("restart", &manifest, &destination, None)
            .await
            .unwrap();
        assert_eq!(fs::read(destination).await.unwrap(), payload);
        server.stop();
    }

    #[tokio::test]
    async fn malformed_range_and_416_never_publish_corrupt_content() {
        let payload = b"abcdefghij".to_vec();
        let mode = Arc::new(AtomicUsize::new(0));
        let mode_thread = Arc::clone(&mode);
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, request| {
            if requested_range(&request).is_some() {
                if mode_thread.fetch_add(1, Ordering::SeqCst) == 0 {
                    response(
                        &mut stream,
                        "206 Partial Content",
                        &[("Content-Range", "bytes 0-4/10".to_string())],
                        b"fghij",
                    );
                } else {
                    response(&mut stream, "416 Range Not Satisfiable", &[], b"");
                }
            } else {
                response(&mut stream, "200 OK", &[], &payload_thread);
            }
        });
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("malformed", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        fs::write(partial_file_path(&destination), b"abcde")
            .await
            .unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        downloader
            .download("malformed", &manifest, &destination, None)
            .await
            .unwrap();
        assert_eq!(fs::read(destination).await.unwrap(), payload);
        server.stop();
    }

    #[tokio::test]
    async fn hash_or_size_mismatch_is_removed_and_final_is_absent() {
        let expected = b"good-content".to_vec();
        let wrong = b"bad-content!".to_vec();
        let wrong_thread = wrong.clone();
        let server = TestServer::new(move |mut stream, _request| {
            response(&mut stream, "200 OK", &[], &wrong_thread);
        });
        let artifact = test_artifact(server.url(), &expected);
        let manifest = manifest("corrupt", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        assert!(downloader
            .download("corrupt", &manifest, &destination, None)
            .await
            .is_err());
        assert!(!destination.exists());
        assert!(!partial_file_path(&destination).exists());
        server.stop();
    }

    #[tokio::test]
    async fn failed_replacement_keeps_existing_file_until_new_bytes_verify() {
        let installed = b"installed-model".to_vec();
        let replacement = b"replacement-model".to_vec();
        let wrong = b"wrong-replacement".to_vec();
        let wrong_thread = wrong.clone();
        let server = TestServer::new(move |mut stream, _request| {
            response(&mut stream, "200 OK", &[], &wrong_thread);
        });
        let artifact = test_artifact(server.url(), &replacement);
        let manifest = manifest("preserve", artifact);
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        fs::write(&destination, &installed).await.unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();

        assert!(downloader
            .download("preserve", &manifest, &destination, None)
            .await
            .is_err());
        assert_eq!(fs::read(&destination).await.unwrap(), installed);
        assert!(!partial_file_path(&destination).exists());
        server.stop();
    }

    #[tokio::test]
    async fn cancellation_isolated_per_model_and_retry_can_start_after_cleanup() {
        let payload = b"0123456789abcdef".to_vec();
        let release = Arc::new(AtomicBool::new(false));
        let release_thread = Arc::clone(&release);
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, request| {
            if request.contains("/model.bin") {
                while !release_thread.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                response(&mut stream, "200 OK", &[], &payload_thread);
            }
        });
        let mut config = DownloadConfig::local_http_for_tests();
        config.idle_timeout = Duration::from_secs(2);
        let downloader = ResumableDownloader::with_config(config).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("cancel", artifact);
        let task_downloader = downloader.clone();
        let task_manifest = manifest.clone();
        let task_destination = destination.clone();
        let task = tokio::spawn(async move {
            task_downloader
                .download("cancel", &task_manifest, &task_destination, None)
                .await
        });
        while !downloader.is_active("cancel").await {
            tokio::task::yield_now().await;
        }
        downloader.cancel("cancel").await.unwrap();
        let result = task.await.unwrap();
        assert!(result
            .unwrap_err()
            .to_string()
            .starts_with(CANCELLATION_PREFIX));
        assert!(!destination.exists());
        release.store(true, Ordering::SeqCst);
        server.stop();
    }

    #[tokio::test]
    async fn cancellation_of_stalled_body_keeps_partial_for_retry() {
        let payload = b"0123456789abcdef".to_vec();
        let release = Arc::new(AtomicBool::new(false));
        let body_started = Arc::new(AtomicBool::new(false));
        let release_thread = Arc::clone(&release);
        let body_started_thread = Arc::clone(&body_started);
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, _request| {
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload_thread.len()
            );
            stream.write_all(headers.as_bytes()).unwrap();
            stream.write_all(&payload_thread[..4]).unwrap();
            stream.flush().unwrap();
            body_started_thread.store(true, Ordering::SeqCst);
            while !release_thread.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(5));
            }
            let _ = stream.write_all(&payload_thread[4..]);
        });
        let mut config = DownloadConfig::local_http_for_tests();
        config.idle_timeout = Duration::from_secs(2);
        let downloader = ResumableDownloader::with_config(config).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("model.bin");
        let artifact = test_artifact(server.url(), &payload);
        let manifest = manifest("stalled-body", artifact);
        let task_downloader = downloader.clone();
        let task_manifest = manifest.clone();
        let task_destination = destination.clone();
        let task = tokio::spawn(async move {
            task_downloader
                .download("stalled-body", &task_manifest, &task_destination, None)
                .await
        });
        while !body_started.load(Ordering::SeqCst) || !downloader.is_active("stalled-body").await {
            tokio::task::yield_now().await;
        }
        let partial = partial_file_path(&destination);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if file_len(&partial).await.unwrap_or(0) >= 4 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("stalled response should write its first body chunk");

        downloader.cancel("stalled-body").await.unwrap();
        let result = task.await.unwrap();
        let error = result.unwrap_err();
        assert!(is_cancelled(&error));
        assert!(!destination.exists());
        assert_eq!(fs::read(&partial).await.unwrap(), &payload[..4]);

        release.store(true, Ordering::SeqCst);
        downloader
            .download("stalled-body", &manifest, &destination, None)
            .await
            .unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), payload);
        assert!(!partial.exists());
        server.stop();
    }

    #[tokio::test]
    async fn same_model_is_serialized_but_two_models_are_independent() {
        let payload = b"payload".to_vec();
        let payload_thread = payload.clone();
        let server = TestServer::new(move |mut stream, _request| {
            response(&mut stream, "200 OK", &[], &payload_thread);
        });
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.bin");
        let second = directory.path().join("second.bin");
        let first_artifact = DownloadArtifact::new(
            "first.bin",
            &server.url(),
            payload.len() as u64,
            format!("{:x}", Sha256::digest(&payload)),
        )
        .unwrap();
        let second_artifact = DownloadArtifact::new(
            "second.bin",
            &server.url(),
            payload.len() as u64,
            format!("{:x}", Sha256::digest(&payload)),
        )
        .unwrap();
        let first_manifest = DownloadManifest::new("first", "r", vec![first_artifact]).unwrap();
        let second_manifest = DownloadManifest::new("second", "r", vec![second_artifact]).unwrap();
        let one = downloader.download("first", &first_manifest, &first, None);
        let two = downloader.download("first", &first_manifest, &first, None);
        let (one_result, two_result) = tokio::join!(one, two);
        assert!(
            one_result.is_ok() || two_result.is_ok(),
            "both same-model downloads failed: first={one_result:?}, second={two_result:?}"
        );
        assert!(one_result.is_err() || two_result.is_err());
        let duplicate_error = one_result
            .as_ref()
            .err()
            .or_else(|| two_result.as_ref().err())
            .expect("one same-model request must be rejected");
        assert!(is_already_active(duplicate_error));
        downloader
            .download("second", &second_manifest, &second, None)
            .await
            .unwrap();
        assert_eq!(fs::read(second).await.unwrap(), payload);
        server.stop();
    }

    #[tokio::test]
    async fn bundle_is_hidden_until_every_artifact_verifies_and_then_publishes_atomically() {
        let first_payload = b"first-artifact".to_vec();
        let second_payload = b"second-artifact".to_vec();
        let first_response = Arc::new(AtomicBool::new(true));
        let first_response_thread = Arc::clone(&first_response);
        let first_thread = first_payload.clone();
        let second_thread = second_payload.clone();
        let server = TestServer::new(move |mut stream, request| {
            if request.starts_with("GET /first.bin")
                && first_response_thread.swap(false, Ordering::SeqCst)
            {
                response(
                    &mut stream,
                    "200 OK",
                    &[],
                    &first_thread[..first_thread.len() / 2],
                );
            } else if request.starts_with("GET /second.bin") {
                response(&mut stream, "200 OK", &[], &second_thread);
            } else {
                response(&mut stream, "200 OK", &[], &first_thread);
            }
        });
        let first_artifact = DownloadArtifact::new(
            "first.bin",
            &format!("{}/first.bin", server.address),
            first_payload.len() as u64,
            format!("{:x}", Sha256::digest(&first_payload)),
        )
        .unwrap();
        let second_artifact = DownloadArtifact::new(
            "second.bin",
            &format!("{}/second.bin", server.address),
            second_payload.len() as u64,
            format!("{:x}", Sha256::digest(&second_payload)),
        )
        .unwrap();
        let manifest = DownloadManifest::new(
            "bundle",
            "test-revision",
            vec![first_artifact, second_artifact],
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("bundle");
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();

        assert!(downloader
            .download("bundle", &manifest, &destination, None)
            .await
            .is_err());
        assert!(!destination.exists());
        let stage = partial_directory_path(&destination).unwrap();
        assert!(stage.exists());
        assert!(stage.join("first.bin").exists());

        downloader
            .download("bundle", &manifest, &destination, None)
            .await
            .unwrap();
        assert_eq!(
            fs::read(destination.join("first.bin")).await.unwrap(),
            first_payload
        );
        assert_eq!(
            fs::read(destination.join("second.bin")).await.unwrap(),
            second_payload
        );
        assert!(!stage.exists());
        server.stop();
    }

    #[tokio::test]
    async fn failed_bundle_replacement_keeps_existing_directory() {
        let installed_first = b"installed-first".to_vec();
        let installed_second = b"installed-second".to_vec();
        let replacement_first = b"replacement-first".to_vec();
        let replacement_second = b"replacement-second".to_vec();
        let wrong = b"wrong-replacement".to_vec();
        let wrong_thread = wrong.clone();
        let server = TestServer::new(move |mut stream, _request| {
            response(&mut stream, "200 OK", &[], &wrong_thread);
        });
        let first_artifact = DownloadArtifact::new(
            "first.bin",
            &format!("{}/first.bin", server.address),
            replacement_first.len() as u64,
            format!("{:x}", Sha256::digest(&replacement_first)),
        )
        .unwrap();
        let second_artifact = DownloadArtifact::new(
            "second.bin",
            &format!("{}/second.bin", server.address),
            replacement_second.len() as u64,
            format!("{:x}", Sha256::digest(&replacement_second)),
        )
        .unwrap();
        let manifest = DownloadManifest::new(
            "preserve-bundle",
            "test-revision",
            vec![first_artifact, second_artifact],
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("bundle");
        fs::create_dir_all(&destination).await.unwrap();
        fs::write(destination.join("first.bin"), &installed_first)
            .await
            .unwrap();
        fs::write(destination.join("second.bin"), &installed_second)
            .await
            .unwrap();
        let downloader =
            ResumableDownloader::with_config(DownloadConfig::local_http_for_tests()).unwrap();

        assert!(downloader
            .download("preserve-bundle", &manifest, &destination, None)
            .await
            .is_err());
        assert_eq!(
            fs::read(destination.join("first.bin")).await.unwrap(),
            installed_first
        );
        assert_eq!(
            fs::read(destination.join("second.bin")).await.unwrap(),
            installed_second
        );
        assert!(partial_directory_path(&destination).unwrap().exists());
        server.stop();
    }

    #[tokio::test]
    async fn integrity_cache_rechecks_replaced_file_and_bundle_artifact() {
        let directory = tempfile::tempdir().unwrap();
        let file_path = directory.path().join("model.bin");
        let good_file = b"GOOD";
        let bad_file = b"BAAD";
        let file_artifact = DownloadArtifact::new(
            "model.bin",
            "https://huggingface.co/example/model.bin",
            good_file.len() as u64,
            format!("{:x}", Sha256::digest(good_file)),
        )
        .unwrap();
        fs::write(&file_path, good_file).await.unwrap();

        let cache = IntegrityCache::new();
        cache
            .verify_file_cached(&file_path, &file_artifact)
            .await
            .unwrap();

        // Keep both the byte count and modification time unchanged.  An
        // atomic replacement must still invalidate the cached verification.
        let original_modified = std::fs::metadata(&file_path).unwrap().modified().unwrap();
        let replacement_path = directory.path().join("replacement.bin");
        fs::write(&replacement_path, bad_file).await.unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&replacement_path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(original_modified))
            .unwrap();
        std::fs::rename(&replacement_path, &file_path).unwrap();
        assert!(cache
            .verify_file_cached(&file_path, &file_artifact)
            .await
            .is_err());

        let first = b"FIRST";
        let second = b"SECOND";
        let first_artifact = DownloadArtifact::new(
            "first.bin",
            "https://huggingface.co/example/first.bin",
            first.len() as u64,
            format!("{:x}", Sha256::digest(first)),
        )
        .unwrap();
        let second_artifact = DownloadArtifact::new(
            "second.bin",
            "https://huggingface.co/example/second.bin",
            second.len() as u64,
            format!("{:x}", Sha256::digest(second)),
        )
        .unwrap();
        let bundle_manifest = DownloadManifest::new(
            "cache-bundle",
            "test-revision",
            vec![first_artifact, second_artifact],
        )
        .unwrap();
        let bundle_path = directory.path().join("bundle");
        fs::create_dir_all(&bundle_path).await.unwrap();
        fs::write(bundle_path.join("first.bin"), first)
            .await
            .unwrap();
        fs::write(bundle_path.join("second.bin"), second)
            .await
            .unwrap();
        cache
            .verify_bundle_cached(&bundle_path, &bundle_manifest)
            .await
            .unwrap();

        // Replace one bundle member while preserving its size and mtime.  A
        // bundle scan must reject the whole model after the cached good scan.
        let second_path = bundle_path.join("second.bin");
        let second_modified = std::fs::metadata(&second_path).unwrap().modified().unwrap();
        let second_replacement = directory.path().join("second-replacement.bin");
        fs::write(&second_replacement, b"SXXOND").await.unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&second_replacement)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(second_modified))
            .unwrap();
        std::fs::rename(&second_replacement, &second_path).unwrap();
        assert!(cache
            .verify_bundle_cached(&bundle_path, &bundle_manifest)
            .await
            .is_err());
    }

    #[test]
    fn production_manifests_are_pinned_and_only_expose_supported_models() {
        assert!(whisper_manifest("large-v3-turbo").unwrap().artifacts[0]
            .url
            .path()
            .contains("5359861c739e955e79d9a303bcbc70fb988958b1"));
        assert!(parakeet_manifest("parakeet-tdt-0.6b-v3-int8").is_some());
        assert!(parakeet_manifest("parakeet-tdt-0.6b-v3-fp32").is_none());
        assert!(
            summary_manifest("gemma3:1b", "google_gemma-3-1b-it-Q8_0.gguf")
                .unwrap()
                .artifacts[0]
                .url
                .path()
                .contains("116f76234503685a98f572982177b11d44ec8ff1")
        );
        assert_eq!(
            summary_manifest("gemma3:1b", "gemma-3-1b-it-Q8_0.gguf")
                .unwrap()
                .artifacts[0]
                .url
                .path(),
            "/bartowski/google_gemma-3-1b-it-GGUF/resolve/116f76234503685a98f572982177b11d44ec8ff1/google_gemma-3-1b-it-Q8_0.gguf"
        );
        assert_eq!(
            (0..12)
                .filter_map(|index| {
                    [
                        "tiny",
                        "base",
                        "small",
                        "medium",
                        "large-v3-turbo",
                        "large-v3",
                        "tiny-q5_1",
                        "base-q5_1",
                        "small-q5_1",
                        "medium-q5_0",
                        "large-v3-turbo-q5_0",
                        "large-v3-q5_0",
                    ]
                    .get(index)
                    .and_then(|name| whisper_manifest(name))
                })
                .count(),
            12
        );
        assert_eq!(
            ["parakeet-tdt-0.6b-v2-int8", "parakeet-tdt-0.6b-v3-int8",]
                .iter()
                .filter_map(|name| parakeet_manifest(name))
                .map(|manifest| manifest.artifacts.len())
                .sum::<usize>(),
            8
        );
        assert_eq!(
            [
                ("qwen3.5:2b", "Qwen3.5-2B-Q4_K_M.gguf"),
                ("qwen3.5:4b", "Qwen3.5-4B-Q4_K_M.gguf"),
                ("gemma4:e2b", "gemma-4-E2B-it-Q4_K_M.gguf"),
                ("gemma4:e4b", "gemma-4-E4B-it-Q4_K_M.gguf"),
                ("gemma3:4b", "gemma-3-4b-it-Q4_K_M.gguf"),
                ("gemma3:1b", "gemma-3-1b-it-Q8_0.gguf"),
            ]
            .iter()
            .filter_map(|(name, filename)| summary_manifest(name, filename))
            .map(|manifest| manifest.artifacts.len())
            .sum::<usize>(),
            6
        );
    }
}
