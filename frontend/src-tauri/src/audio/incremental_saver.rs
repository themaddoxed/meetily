use super::encode::encode_single_audio;
use super::recording_state::AudioChunk;
use anyhow::{anyhow, Result};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::capture::backend::TimestampQuality;
#[cfg(test)]
use super::capture::backend::TrackBlockKind;
use super::capture::backend::{TrackChunk, TrackKind};
use super::ffmpeg::find_ffmpeg_path;

pub const MIXED_CHECKPOINT_QUEUE_MAX_MESSAGES: usize = 64;
#[cfg(test)]
pub const MAX_TIMELINE_EVENT_BYTES: usize = 4 * 1024;
#[cfg(test)]
pub const MAX_TIMELINE_EVENTS: usize = 4 * 1024;
#[cfg(test)]
pub const MAX_TIMELINE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub struct FinalizedTrack {
    pub kind: TrackKind,
    pub sample_rate: u32,
    pub channels: u16,
    pub path: PathBuf,
    pub byte_length: u64,
    pub sample_count: u64,
    pub sha256: String,
}

#[derive(Debug)]
pub enum SourceTrackRecoveryError {
    Integrity(String),
    Persistence(String),
}

pub type SourceTrackFinalizationError = SourceTrackRecoveryError;

impl std::fmt::Display for SourceTrackRecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integrity(details) | Self::Persistence(details) => formatter.write_str(details),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CheckpointEvidence {
    index: u32,
    file_name: String,
    #[serde(default)]
    input_sample_count: u64,
    sha256: String,
}

#[cfg(test)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimelineEvent {
    seq: u64,
    known_frame_prefix: u64,
    start_frame: u64,
    end_frame: u64,
    epoch: u64,
    kind: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SourceTrackManifest {
    sample_rate: u32,
    channels: u16,
    checkpoint_count: u32,
    sample_count: u64,
    #[serde(default)]
    next_frame: u64,
    checkpoints: Vec<CheckpointEvidence>,
    #[serde(default)]
    final_sha256: Option<String>,
    #[serde(default)]
    timeline_journal: Option<String>,
    #[serde(default)]
    timeline_event_count: u64,
    #[serde(default)]
    timeline_bytes: u64,
    #[serde(default)]
    known_frame_prefix: u64,
    #[serde(default)]
    incomplete: bool,
}

pub struct SourceTrackSaver {
    kind: TrackKind,
    meeting_folder: PathBuf,
    checkpoints_dir: PathBuf,
    samples: Vec<f32>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    checkpoint_count: u32,
    sample_count: u64,
    next_frame: u64,
    checkpoints: Vec<CheckpointEvidence>,
    final_sha256: Option<String>,
    manifest_path: PathBuf,
    #[cfg(test)]
    journal_path: PathBuf,
    #[cfg(test)]
    timeline_events: Vec<TimelineEvent>,
    #[cfg(test)]
    timeline_bytes: usize,
    #[cfg(test)]
    known_frame_prefix: u64,
    #[cfg(test)]
    timeline_seq: u64,
    incomplete: bool,
    first_write_error: Option<String>,
}

impl SourceTrackSaver {
    pub fn new(meeting_folder: PathBuf, kind: TrackKind) -> Result<Self> {
        let source_namespace = match kind {
            TrackKind::Microphone => Path::new(".tracks/microphone"),
            TrackKind::System => Path::new(".tracks/system"),
        };
        let checkpoints_dir = meeting_folder.join(source_namespace).join(".checkpoints");
        let manifest_path = meeting_folder.join(source_namespace).join("track.json");
        #[cfg(test)]
        let journal_path = meeting_folder.join(source_namespace).join("timeline.jsonl");
        std::fs::create_dir_all(&checkpoints_dir)?;
        Ok(Self {
            kind,
            meeting_folder,
            checkpoints_dir,
            samples: Vec::new(),
            sample_rate: None,
            channels: None,
            checkpoint_count: 0,
            sample_count: 0,
            next_frame: 0,
            checkpoints: Vec::new(),
            final_sha256: None,
            manifest_path,
            #[cfg(test)]
            journal_path,
            #[cfg(test)]
            timeline_events: Vec::new(),
            #[cfg(test)]
            timeline_bytes: 0,
            #[cfg(test)]
            known_frame_prefix: 0,
            #[cfg(test)]
            timeline_seq: 0,
            incomplete: false,
            first_write_error: None,
        })
    }

    #[cfg(test)]
    fn append_timeline_event(&mut self, chunk: &TrackChunk) -> Result<()> {
        let frame_count = if chunk.channels == 0 {
            0
        } else {
            (chunk.samples.len() / chunk.channels as usize) as u64
        };
        let start_frame = chunk.span.start_frame;
        let end_frame = chunk
            .span
            .end_frame()
            .max(start_frame.saturating_add(frame_count));
        let kind = match chunk.block_kind {
            TrackBlockKind::Samples => "samples",
            TrackBlockKind::Mute => "mute",
            TrackBlockKind::Gap => "gap",
        }
        .to_owned();
        if let Some(last) = self.timeline_events.last_mut() {
            if last.kind == kind && last.epoch == chunk.span.epoch && last.end_frame == start_frame
            {
                last.end_frame = end_frame;
            } else {
                self.timeline_seq = self.timeline_seq.saturating_add(1);
                self.timeline_events.push(TimelineEvent {
                    seq: self.timeline_seq,
                    known_frame_prefix: self.known_frame_prefix,
                    start_frame,
                    end_frame,
                    epoch: chunk.span.epoch,
                    kind,
                });
            }
        } else {
            self.timeline_seq = self.timeline_seq.saturating_add(1);
            self.timeline_events.push(TimelineEvent {
                seq: self.timeline_seq,
                known_frame_prefix: self.known_frame_prefix,
                start_frame,
                end_frame,
                epoch: chunk.span.epoch,
                kind,
            });
        }
        if self.timeline_events.len() > MAX_TIMELINE_EVENTS {
            self.incomplete = true;
            return Err(anyhow!("Timeline journal event limit exceeded"));
        }
        let mut encoded = Vec::new();
        for event in &self.timeline_events {
            let bytes = serde_json::to_vec(event)?;
            if bytes.len() > MAX_TIMELINE_EVENT_BYTES {
                self.incomplete = true;
                return Err(anyhow!("Timeline journal event is oversized"));
            }
            encoded.extend_from_slice(&bytes);
            encoded.push(b'\n');
        }
        if encoded.len() > MAX_TIMELINE_BYTES {
            self.incomplete = true;
            return Err(anyhow!("Timeline journal byte limit exceeded"));
        }
        let temporary = self.journal_path.with_extension("jsonl.tmp");
        let mut file = File::create(&temporary)?;
        file.write_all(&encoded)?;
        file.sync_all()?;
        std::fs::rename(&temporary, &self.journal_path)?;
        if let Some(namespace) = self.journal_path.parent() {
            File::open(namespace)?.sync_all()?;
        }
        self.timeline_bytes = encoded.len();
        self.known_frame_prefix = self.known_frame_prefix.max(end_frame);
        Ok(())
    }

    pub fn add_chunk(&mut self, chunk: TrackChunk) -> Result<()> {
        if let Some(error) = &self.first_write_error {
            return Err(anyhow!(error.clone()));
        }
        if chunk.kind != self.kind || chunk.sample_rate == 0 || chunk.channels == 0 {
            return Err(anyhow!("Invalid {:?} Track chunk", self.kind));
        }
        match (self.sample_rate, self.channels) {
            (None, None) => {
                self.sample_rate = Some(chunk.sample_rate);
                self.channels = Some(chunk.channels);
            }
            (Some(sample_rate), Some(channels))
                if sample_rate == chunk.sample_rate && channels == chunk.channels => {}
            _ => return Err(anyhow!("Inconsistent {:?} Track chunks", self.kind)),
        }
        let channels = usize::from(chunk.channels);
        let incoming_frames = chunk.samples.len() / channels;
        let start_frame = if chunk.span.quality == TimestampQuality::Contiguous {
            self.next_frame
        } else {
            chunk.span.start_frame
        };
        if start_frame < self.next_frame {
            return Err(anyhow!(
                "Overlapping {:?} Track span: start={start_frame}, next={}",
                self.kind,
                self.next_frame
            ));
        }
        let gap_frames = start_frame.saturating_sub(self.next_frame);
        if gap_frames > 0 {
            let gap_samples = gap_frames
                .checked_mul(u64::from(chunk.channels))
                .and_then(|count| usize::try_from(count).ok())
                .ok_or_else(|| anyhow!("{:?} Track timeline gap is too large", self.kind))?;
            self.samples.try_reserve(gap_samples).map_err(|error| {
                anyhow!(
                    "{:?} Track timeline gap allocation failed: {error}",
                    self.kind
                )
            })?;
            self.samples
                .extend(std::iter::repeat(0.0).take(gap_samples));
            self.sample_count = self.sample_count.saturating_add(gap_samples as u64);
            self.next_frame = start_frame;
        }
        self.sample_count = self.sample_count.saturating_add(chunk.samples.len() as u64);
        self.samples.extend(chunk.samples);
        self.next_frame = self.next_frame.saturating_add(incoming_frames as u64);
        let checkpoint_samples = chunk.sample_rate as usize * chunk.channels as usize * 30;
        if self.samples.len() >= checkpoint_samples {
            self.save_checkpoint()?;
        }
        Ok(())
    }

    pub(crate) fn mark_incomplete(&mut self) {
        self.incomplete = true;
    }

    pub(crate) fn record_write_error(&mut self, error: String) {
        self.incomplete = true;
        self.first_write_error.get_or_insert(error);
    }

    fn save_checkpoint(&mut self) -> Result<()> {
        if self.samples.is_empty() {
            return Ok(());
        }
        #[cfg(test)]
        if std::fs::remove_file(self.meeting_folder.join(".force_checkpoint_write_error")).is_ok() {
            return Err(anyhow!("forced checkpoint write failure"));
        }
        let sample_rate = self
            .sample_rate
            .ok_or_else(|| anyhow!("Missing sample rate"))?;
        let channels = self
            .channels
            .ok_or_else(|| anyhow!("Missing channel count"))?;
        let source_name = match self.kind {
            TrackKind::Microphone => "microphone",
            TrackKind::System => "system",
        };
        let checkpoint = self.checkpoints_dir.join(format!(
            "{}_chunk_{:03}.mp4",
            source_name, self.checkpoint_count
        ));
        let input_sample_count = self.samples.len() as u64;
        encode_single_audio(
            bytemuck::cast_slice(&self.samples),
            sample_rate,
            channels,
            &checkpoint,
        )?;
        File::open(&checkpoint)?.sync_all()?;
        File::open(&self.checkpoints_dir)?.sync_all()?;
        self.checkpoints.push(CheckpointEvidence {
            index: self.checkpoint_count,
            file_name: checkpoint
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("Invalid checkpoint filename"))?
                .to_owned(),
            input_sample_count,
            sha256: sha256_file(&checkpoint)?,
        });
        self.samples.clear();
        self.checkpoint_count += 1;
        self.persist_manifest()?;
        Ok(())
    }

    fn persist_manifest(&self) -> Result<()> {
        let manifest = SourceTrackManifest {
            sample_rate: self
                .sample_rate
                .ok_or_else(|| anyhow!("Missing sample rate"))?,
            channels: self
                .channels
                .ok_or_else(|| anyhow!("Missing channel count"))?,
            checkpoint_count: self.checkpoint_count,
            sample_count: self.sample_count,
            next_frame: self.next_frame,
            checkpoints: self.checkpoints.clone(),
            final_sha256: self.final_sha256.clone(),
            // Timeline recovery is not validated yet; publish only checkpoint evidence.
            timeline_journal: None,
            timeline_event_count: 0,
            timeline_bytes: 0,
            known_frame_prefix: 0,
            incomplete: self.incomplete,
        };
        persist_source_manifest(&self.manifest_path, &manifest)
    }

    pub fn recover(
        meeting_folder: &Path,
        kind: TrackKind,
    ) -> std::result::Result<Option<FinalizedTrack>, SourceTrackRecoveryError> {
        let source_name = match kind {
            TrackKind::Microphone => "microphone",
            TrackKind::System => "system",
        };
        let namespace = meeting_folder.join(".tracks").join(source_name);
        let checkpoints_dir = namespace.join(".checkpoints");
        if !checkpoints_dir.exists() {
            return Ok(None);
        }
        let manifest_bytes = std::fs::read(namespace.join("track.json"))
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        let mut manifest: SourceTrackManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| SourceTrackRecoveryError::Integrity(error.to_string()))?;
        if manifest.sample_rate == 0
            || manifest.channels == 0
            || manifest.checkpoint_count == 0
            || manifest.sample_count == 0
            || manifest.checkpoints.len() != manifest.checkpoint_count as usize
        {
            return Err(SourceTrackRecoveryError::Integrity(format!(
                "Invalid recovered {source_name} Track manifest"
            )));
        }
        if manifest.incomplete {
            return Err(SourceTrackRecoveryError::Integrity(format!(
                "Recovered {source_name} Track is marked incomplete"
            )));
        }
        validate_checkpoint_evidence(&checkpoints_dir, kind, &manifest)?;
        let final_path = meeting_folder.join(format!("{source_name}.mp4"));
        if final_path.exists()
            && manifest.final_sha256.as_deref() == sha256_file(&final_path).ok().as_deref()
        {
            if let Ok(track) = verify_final_track(&final_path, kind, &manifest) {
                File::open(&final_path)
                    .and_then(|file| file.sync_all())
                    .and_then(|()| File::open(&checkpoints_dir)?.sync_all())
                    .and_then(|()| File::open(meeting_folder)?.sync_all())
                    .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
                return Ok(Some(track));
            }
        }
        let recovering_path = meeting_folder.join(format!(".{source_name}.recovering.mp4"));
        let _ = std::fs::remove_file(&recovering_path);
        if let Err(error) =
            merge_numbered_checkpoints(&checkpoints_dir, kind, &manifest, &recovering_path)
        {
            let _ = std::fs::remove_file(&recovering_path);
            return Err(SourceTrackRecoveryError::Persistence(error.to_string()));
        }
        if let Err(error) = verify_final_track(&recovering_path, kind, &manifest) {
            let _ = std::fs::remove_file(&recovering_path);
            return Err(error);
        }
        std::fs::rename(&recovering_path, &final_path)
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        File::open(&final_path)
            .and_then(|file| file.sync_all())
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        File::open(&checkpoints_dir)
            .and_then(|file| file.sync_all())
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        File::open(meeting_folder)
            .and_then(|file| file.sync_all())
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        let track = verify_final_track(&final_path, kind, &manifest)?;
        manifest.final_sha256 = Some(track.sha256.clone());
        persist_source_manifest(&namespace.join("track.json"), &manifest)
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        Ok(Some(track))
    }

    pub fn finalize(mut self) -> std::result::Result<FinalizedTrack, SourceTrackFinalizationError> {
        if let Err(error) = self.save_checkpoint() {
            return Err(SourceTrackFinalizationError::Persistence(
                self.first_write_error
                    .clone()
                    .unwrap_or_else(|| error.to_string()),
            ));
        }
        if let Some(error) = self.first_write_error.clone() {
            if self.checkpoint_count == 0 || self.sample_count == 0 {
                return Err(SourceTrackFinalizationError::Persistence(error));
            }
            self.persist_manifest().map_err(|persist_error| {
                SourceTrackFinalizationError::Persistence(format!(
                    "{error}; incomplete Track manifest persistence failed: {persist_error}"
                ))
            })?;
            return Err(SourceTrackFinalizationError::Persistence(error));
        }
        if self.incomplete {
            if self.checkpoint_count == 0 || self.sample_count == 0 {
                return Err(SourceTrackFinalizationError::Integrity(format!(
                    "No {:?} Track data was captured",
                    self.kind
                )));
            }
            self.persist_manifest()
                .map_err(|error| SourceTrackFinalizationError::Persistence(error.to_string()))?;
            return Err(SourceTrackFinalizationError::Integrity(format!(
                "Captured {:?} Track is incomplete",
                self.kind
            )));
        }
        if self.checkpoint_count == 0 || self.sample_count == 0 {
            return Err(SourceTrackFinalizationError::Integrity(format!(
                "No {:?} Track data was captured",
                self.kind
            )));
        }
        let final_path = match self.kind {
            TrackKind::Microphone => self.meeting_folder.join("microphone.mp4"),
            TrackKind::System => self.meeting_folder.join("system.mp4"),
        };
        let manifest = SourceTrackManifest {
            sample_rate: self.sample_rate.expect("validated Track sample rate"),
            channels: self.channels.expect("validated Track channel count"),
            checkpoint_count: self.checkpoint_count,
            sample_count: self.sample_count,
            next_frame: self.next_frame,
            checkpoints: self.checkpoints.clone(),
            final_sha256: None,
            // Timeline recovery is not validated yet; publish only checkpoint evidence.
            timeline_journal: None,
            timeline_event_count: 0,
            timeline_bytes: 0,
            known_frame_prefix: 0,
            incomplete: self.incomplete,
        };
        validate_checkpoint_evidence(&self.checkpoints_dir, self.kind, &manifest)?;
        merge_numbered_checkpoints(&self.checkpoints_dir, self.kind, &manifest, &final_path)
            .map_err(|error| SourceTrackFinalizationError::Persistence(error.to_string()))?;
        File::open(&final_path)
            .and_then(|file| file.sync_all())
            .and_then(|()| File::open(&self.checkpoints_dir)?.sync_all())
            .and_then(|()| File::open(&self.meeting_folder)?.sync_all())
            .map_err(|error| SourceTrackFinalizationError::Persistence(error.to_string()))?;
        let track = verify_final_track(&final_path, self.kind, &manifest)?;
        self.final_sha256 = Some(track.sha256.clone());
        let mut persisted_manifest = manifest;
        persisted_manifest.final_sha256 = self.final_sha256;
        persist_source_manifest(&self.manifest_path, &persisted_manifest)
            .map_err(|error| SourceTrackFinalizationError::Persistence(error.to_string()))?;
        Ok(track)
    }
}

fn persist_source_manifest(path: &Path, manifest: &SourceTrackManifest) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec(manifest)?)?;
    File::open(&temporary)?.sync_all()?;
    std::fs::rename(&temporary, path)?;
    if let Some(namespace) = path.parent() {
        File::open(namespace)?.sync_all()?;
    }
    Ok(())
}

fn source_name(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Microphone => "microphone",
        TrackKind::System => "system",
    }
}

fn cleanup_source_checkpoints(
    checkpoints_dir: &Path,
    namespace: &Path,
    meeting_folder: &Path,
) -> std::result::Result<(), SourceTrackRecoveryError> {
    if checkpoints_dir.exists() {
        std::fs::remove_dir_all(checkpoints_dir)
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
    }
    File::open(namespace)
        .and_then(|file| file.sync_all())
        .and_then(|()| File::open(meeting_folder)?.sync_all())
        .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
    Ok(())
}

pub(crate) fn cleanup_source_track_checkpoints(
    meeting_folder: &Path,
    kind: TrackKind,
) -> std::result::Result<(), SourceTrackRecoveryError> {
    let namespace = meeting_folder.join(".tracks").join(source_name(kind));
    if !namespace.exists() {
        return Ok(());
    }
    cleanup_source_checkpoints(&namespace.join(".checkpoints"), &namespace, meeting_folder)
}

fn has_checkpoint_files(checkpoints_dir: &Path) -> std::io::Result<bool> {
    if !checkpoints_dir.is_dir() {
        return Ok(false);
    }
    Ok(std::fs::read_dir(checkpoints_dir)?
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("mp4")
        }))
}

fn validate_checkpoint_sequence(
    checkpoints_dir: &Path,
    kind: TrackKind,
    checkpoint_count: u32,
) -> std::result::Result<(), SourceTrackRecoveryError> {
    let source_name = source_name(kind);
    let entries = std::fs::read_dir(checkpoints_dir)
        .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
    let mut actual = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("mp4") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                SourceTrackRecoveryError::Integrity(format!(
                    "Invalid {source_name} Track checkpoint name"
                ))
            })?;
        actual.push(name.to_owned());
    }
    actual.sort_by_key(|name| checkpoint_index(name, source_name).unwrap_or(u32::MAX));
    let expected: Vec<_> = (0..checkpoint_count)
        .map(|index| format!("{source_name}_chunk_{index:03}.mp4"))
        .collect();
    if actual != expected {
        return Err(SourceTrackRecoveryError::Integrity(format!(
            "Non-contiguous {source_name} Track checkpoint sequence"
        )));
    }
    for name in expected {
        let metadata = std::fs::metadata(checkpoints_dir.join(name))
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        if metadata.len() == 0 {
            return Err(SourceTrackRecoveryError::Integrity(format!(
                "Empty {source_name} Track checkpoint"
            )));
        }
    }
    Ok(())
}

fn validate_checkpoint_evidence(
    checkpoints_dir: &Path,
    kind: TrackKind,
    manifest: &SourceTrackManifest,
) -> std::result::Result<(), SourceTrackRecoveryError> {
    validate_checkpoint_sequence(checkpoints_dir, kind, manifest.checkpoint_count)?;
    let source = source_name(kind);
    let counts_are_known = manifest
        .checkpoints
        .iter()
        .all(|evidence| evidence.input_sample_count > 0);
    let mut input_sample_count = 0_u64;
    for (position, evidence) in manifest.checkpoints.iter().enumerate() {
        let index = position as u32;
        let expected_name = format!("{source}_chunk_{index:03}.mp4");
        if evidence.index != index
            || evidence.file_name != expected_name
            || (counts_are_known && evidence.input_sample_count == 0)
        {
            return Err(SourceTrackRecoveryError::Integrity(format!(
                "Invalid {source} checkpoint evidence at {index}"
            )));
        }
        if evidence.input_sample_count > 0
            && evidence.input_sample_count % u64::from(manifest.channels) != 0
        {
            return Err(SourceTrackRecoveryError::Integrity(format!(
                "Invalid {source} checkpoint sample alignment at {index}"
            )));
        }
        let path = checkpoints_dir.join(&evidence.file_name);
        let sha256 = sha256_file(&path)
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
        if sha256 != evidence.sha256 {
            return Err(SourceTrackRecoveryError::Integrity(format!(
                "{source} checkpoint hash mismatch at {index}"
            )));
        }
        if evidence.input_sample_count > 0 {
            input_sample_count = input_sample_count
                .checked_add(evidence.input_sample_count)
                .ok_or_else(|| {
                    SourceTrackRecoveryError::Integrity(format!(
                        "{source} checkpoint sample count overflow"
                    ))
                })?;
        }
    }
    if counts_are_known && input_sample_count != manifest.sample_count {
        return Err(SourceTrackRecoveryError::Integrity(format!(
            "{source} checkpoint sample contributions differ from manifest"
        )));
    }
    Ok(())
}

fn checkpoint_index(name: &str, source_name: &str) -> Option<u32> {
    name.strip_prefix(&format!("{source_name}_chunk_"))?
        .strip_suffix(".mp4")?
        .parse()
        .ok()
}

fn legacy_checkpoint_index(path: &Path) -> u32 {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| {
            name.strip_prefix("audio_chunk_")?
                .strip_suffix(".mp4")?
                .parse()
                .ok()
        })
        .unwrap_or(u32::MAX)
}

fn verify_final_track(
    path: &Path,
    kind: TrackKind,
    manifest: &SourceTrackManifest,
) -> std::result::Result<FinalizedTrack, SourceTrackRecoveryError> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?;
    if metadata.len() == 0 || manifest.sample_count == 0 {
        return Err(SourceTrackRecoveryError::Integrity(format!(
            "Final {kind:?} Track is empty"
        )));
    }
    let decoded_samples = decoded_sample_count_bounded(path)?;
    let checkpoint_padding = manifest
        .checkpoint_count
        .saturating_mul(4)
        .saturating_mul(1024) as u64
        * u64::from(manifest.channels);
    if decoded_samples == 0 || decoded_samples.abs_diff(manifest.sample_count) > checkpoint_padding
    {
        return Err(SourceTrackRecoveryError::Integrity(format!(
            "Final {kind:?} Track metadata is invalid"
        )));
    }
    Ok(FinalizedTrack {
        kind,
        sample_rate: manifest.sample_rate,
        channels: manifest.channels,
        path: path.to_path_buf(),
        byte_length: metadata.len(),
        sample_count: manifest.sample_count,
        sha256: sha256_file(path)
            .map_err(|error| SourceTrackRecoveryError::Persistence(error.to_string()))?,
    })
}

const DECODE_VERIFY_BUFFER_BYTES: usize = 64 * 1024;

fn decoded_sample_count_bounded(path: &Path) -> std::result::Result<u64, SourceTrackRecoveryError> {
    let persistence =
        |error: &dyn std::fmt::Display| SourceTrackRecoveryError::Persistence(error.to_string());
    let ffmpeg = find_ffmpeg_path()
        .ok_or_else(|| SourceTrackRecoveryError::Persistence("FFmpeg not found".into()))?;
    let mut child = std::process::Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-f", "f32le", "-acodec", "pcm_f32le", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| persistence(&error))?;
    let mut stdout = child.stdout.take().ok_or_else(|| {
        SourceTrackRecoveryError::Persistence("FFmpeg decoder stdout is unavailable".into())
    })?;
    let mut buffer = [0_u8; DECODE_VERIFY_BUFFER_BYTES];
    let mut byte_count = 0_u64;
    loop {
        let read = stdout
            .read(&mut buffer)
            .map_err(|error| persistence(&error))?;
        if read == 0 {
            break;
        }
        byte_count = byte_count.checked_add(read as u64).ok_or_else(|| {
            SourceTrackRecoveryError::Integrity("Decoded sample count overflow".into())
        })?;
    }
    let status = child.wait().map_err(|error| persistence(&error))?;
    if !status.success() || byte_count % std::mem::size_of::<f32>() as u64 != 0 {
        return Err(SourceTrackRecoveryError::Integrity(
            "FFmpeg bounded decode failed".into(),
        ));
    }
    Ok(byte_count / std::mem::size_of::<f32>() as u64)
}

fn merge_numbered_checkpoints(
    checkpoints_dir: &Path,
    kind: TrackKind,
    manifest: &SourceTrackManifest,
    output: &Path,
) -> Result<()> {
    let source_name = source_name(kind);
    if output.exists() {
        return Err(anyhow!(
            "Refusing to overwrite existing normalized Track output: {}",
            output.display()
        ));
    }
    if manifest.checkpoint_count == 1 {
        std::fs::copy(
            checkpoints_dir.join(format!("{source_name}_chunk_000.mp4")),
            output,
        )?;
        return Ok(());
    }

    // Independently encoded AAC files carry their own priming and packet
    // timestamps. Stream-copying them with concat makes those timestamps
    // overlap at every checkpoint boundary. Decode each checkpoint into one
    // encoder instead, trimming the exact recorded sample contribution when
    // that evidence is available.
    let temporary = normalized_checkpoint_path(output);
    if temporary.exists() {
        return Err(anyhow!(
            "Temporary normalized Track output already exists: {}",
            temporary.display()
        ));
    }
    let result = encode_checkpoint_stream(checkpoints_dir, kind, manifest, &temporary);
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&temporary, output) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn normalized_checkpoint_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("track.mp4");
    output.with_file_name(format!(
        ".{file_name}.aac-normalizing-{}.tmp",
        std::process::id()
    ))
}

fn encode_checkpoint_stream(
    checkpoints_dir: &Path,
    kind: TrackKind,
    manifest: &SourceTrackManifest,
    output: &Path,
) -> Result<()> {
    let ffmpeg = find_ffmpeg_path().ok_or_else(|| anyhow!("FFmpeg not found"))?;
    let sample_rate = manifest.sample_rate.to_string();
    let channels = manifest.channels.to_string();
    let mut encoder = std::process::Command::new(&ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "f32le",
            "-ar",
            &sample_rate,
            "-ac",
            &channels,
            "-i",
            "pipe:0",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-profile:a",
            "aac_low",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
            "-n",
        ])
        .arg(output)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut encoder_stdin = match encoder.stdin.take() {
        Some(stdin) => stdin,
        None => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            return Err(anyhow!("FFmpeg encoder stdin is unavailable"));
        }
    };

    for evidence in &manifest.checkpoints {
        let checkpoint = checkpoints_dir.join(&evidence.file_name);
        let input_sample_count = if evidence.input_sample_count == 0 {
            None
        } else {
            Some(evidence.input_sample_count)
        };
        if let Err(error) = decode_checkpoint_into_encoder(
            &checkpoint,
            manifest.sample_rate,
            manifest.channels,
            input_sample_count,
            &mut encoder_stdin,
        ) {
            drop(encoder_stdin);
            let _ = encoder.wait();
            return Err(error);
        }
    }

    drop(encoder_stdin);
    let status = encoder.wait()?;
    if !status.success() {
        return Err(anyhow!("FFmpeg {kind:?} Track AAC normalization failed"));
    }
    Ok(())
}

fn decode_checkpoint_into_encoder(
    checkpoint: &Path,
    sample_rate: u32,
    channels: u16,
    input_sample_count: Option<u64>,
    encoder: &mut impl Write,
) -> Result<()> {
    if let Some(sample_count) = input_sample_count {
        if sample_count % u64::from(channels) != 0 {
            return Err(anyhow!(
                "Checkpoint sample count is not aligned to its channel count"
            ));
        }
    }
    let expected_bytes = input_sample_count
        .map(|sample_count| {
            sample_count
                .checked_mul(std::mem::size_of::<f32>() as u64)
                .ok_or_else(|| anyhow!("Checkpoint sample byte count overflow"))
        })
        .transpose()?;
    let ffmpeg = find_ffmpeg_path().ok_or_else(|| anyhow!("FFmpeg not found"))?;
    let sample_rate = sample_rate.to_string();
    let channels = channels.to_string();
    let mut decoder = std::process::Command::new(ffmpeg)
        .args([
            "-v",
            "error",
            "-i",
            checkpoint.to_str().unwrap_or_default(),
            "-map",
            "0:a:0",
            "-ar",
            &sample_rate,
            "-ac",
            &channels,
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdout = match decoder.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = decoder.kill();
            let _ = decoder.wait();
            return Err(anyhow!("FFmpeg decoder stdout is unavailable"));
        }
    };
    let mut buffer = [0_u8; DECODE_VERIFY_BUFFER_BYTES];
    let mut decoded_bytes = 0_u64;
    let mut read_error = None;
    let mut write_error = None;
    let mut remaining = expected_bytes;
    loop {
        let read = match stdout.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => {
                read_error = Some(anyhow!(error));
                break;
            }
        };
        if read == 0 {
            break;
        }
        decoded_bytes = match decoded_bytes.checked_add(read as u64) {
            Some(total) => total,
            None => {
                read_error = Some(anyhow!("Decoded checkpoint byte count overflow"));
                break;
            }
        };
        if write_error.is_none() {
            let write_len = remaining.map_or(read, |left| left.min(read as u64) as usize);
            if write_len > 0 {
                if let Err(error) = encoder.write_all(&buffer[..write_len]) {
                    write_error = Some(anyhow!("FFmpeg encoder input failed: {error}"));
                }
                if let Some(left) = remaining.as_mut() {
                    *left -= write_len as u64;
                }
            }
        }
    }
    drop(stdout);
    let status = decoder.wait()?;
    if let Some(error) = read_error {
        return Err(error);
    }
    if let Some(error) = write_error {
        return Err(error);
    }
    if !status.success() {
        return Err(anyhow!(
            "FFmpeg checkpoint decode failed for {}",
            checkpoint.display()
        ));
    }
    if decoded_bytes % std::mem::size_of::<f32>() as u64 != 0 {
        return Err(anyhow!("FFmpeg checkpoint decode returned partial samples"));
    }
    if let Some(expected_bytes) = expected_bytes {
        if decoded_bytes < expected_bytes {
            return Err(anyhow!(
                "FFmpeg checkpoint decode was shorter than recorded input: decoded={} expected={expected_bytes}",
                decoded_bytes
            ));
        }
    } else if decoded_bytes == 0 {
        return Err(anyhow!("FFmpeg checkpoint decode returned no samples"));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Finalize one source Track without changing the legacy mixed-audio recovery layout.
pub fn finalize_source_track(
    meeting_folder: &Path,
    kind: TrackKind,
    chunks: Vec<TrackChunk>,
) -> Result<FinalizedTrack> {
    let mut saver = SourceTrackSaver::new(meeting_folder.to_path_buf(), kind)?;
    for chunk in chunks {
        saver.add_chunk(chunk)?;
    }
    saver.finalize().map_err(|error| anyhow!(error.to_string()))
}

/// Validate the mixed legacy filename after the mixed saver has finalized it.
pub fn materialize_compatibility_audio(meeting_folder: &Path) -> Result<PathBuf> {
    let compatibility_path = meeting_folder.join("audio.mp4");
    if !compatibility_path.exists() {
        return Err(anyhow!("Finalized mixed audio.mp4 is missing"));
    }
    File::open(&compatibility_path)?.sync_all()?;
    if std::fs::metadata(&compatibility_path)?.len() == 0 {
        return Err(anyhow!("Compatibility audio.mp4 is empty"));
    }
    Ok(compatibility_path)
}

/// Audio data without device type (we only store mixed audio)
#[derive(Clone)]
struct AudioData {
    data: Vec<f32>,
    // sample_rate: u32,
}

/// Incremental audio saver that writes checkpoints every 30 seconds
/// to minimize memory usage and enable crash recovery
pub struct IncrementalAudioSaver {
    checkpoint_buffer: Vec<AudioData>,
    checkpoint_interval_samples: usize, // 30s at 48kHz = 1,440,000 samples
    checkpoint_count: u32,
    checkpoints_dir: PathBuf,
    meeting_folder: PathBuf,
    sample_rate: u32,
}

impl IncrementalAudioSaver {
    pub fn start_mixed_writer(
        meeting_folder: PathBuf,
        sample_rate: u32,
    ) -> Result<(
        tokio::sync::mpsc::Sender<AudioChunk>,
        tokio::task::JoinHandle<Result<Self>>,
    )> {
        let saver = Self::new(meeting_folder, sample_rate)?;
        let (sender, mut receiver) =
            tokio::sync::mpsc::channel(MIXED_CHECKPOINT_QUEUE_MAX_MESSAGES);
        let join = tokio::spawn(async move {
            let mut saver = saver;
            while let Some(chunk) = receiver.recv().await {
                saver.add_chunk(chunk)?;
            }
            Ok(saver)
        });
        Ok((sender, join))
    }

    /// Create a new incremental saver
    ///
    /// # Arguments
    /// * `meeting_folder` - Path to the meeting folder (contains .checkpoints/)
    /// * `sample_rate` - Sample rate of audio (typically 48000)
    pub fn new(meeting_folder: PathBuf, sample_rate: u32) -> Result<Self> {
        let checkpoints_dir = meeting_folder.join(".checkpoints");

        // Verify checkpoints directory exists
        if !checkpoints_dir.exists() {
            return Err(anyhow!(
                "Checkpoints directory does not exist: {}",
                checkpoints_dir.display()
            ));
        }

        Ok(Self {
            checkpoint_buffer: Vec::new(),
            checkpoint_interval_samples: sample_rate as usize * 30, // 30 seconds
            checkpoint_count: 0,
            checkpoints_dir,
            meeting_folder,
            sample_rate,
        })
    }

    /// Add an audio chunk to the buffer
    /// Automatically saves a checkpoint when buffer reaches 30 seconds
    pub fn add_chunk(&mut self, chunk: AudioChunk) -> Result<()> {
        let audio_data = AudioData {
            data: chunk.data,
            // sample_rate: chunk.sample_rate,
        };

        self.checkpoint_buffer.push(audio_data);

        // Calculate total samples in buffer
        let total_samples: usize = self.checkpoint_buffer.iter().map(|c| c.data.len()).sum();

        // Save checkpoint when buffer reaches threshold (30 seconds)
        if total_samples >= self.checkpoint_interval_samples {
            self.save_checkpoint()?;
            self.checkpoint_buffer.clear();
        }

        Ok(())
    }

    /// Save current buffer as a checkpoint file
    fn save_checkpoint(&mut self) -> Result<()> {
        // Concatenate all chunks in buffer
        let audio_data: Vec<f32> = self
            .checkpoint_buffer
            .iter()
            .flat_map(|c| &c.data)
            .cloned()
            .collect();

        if audio_data.is_empty() {
            warn!("Attempted to save empty checkpoint, skipping");
            return Ok(());
        }

        // Generate checkpoint filename
        let checkpoint_path = self
            .checkpoints_dir
            .join(format!("audio_chunk_{:03}.mp4", self.checkpoint_count));

        // Encode and save checkpoint
        encode_single_audio(
            bytemuck::cast_slice(&audio_data),
            self.sample_rate,
            1, // mono
            &checkpoint_path,
        )?;

        let duration_seconds = audio_data.len() as f32 / self.sample_rate as f32;
        self.checkpoint_count += 1;

        info!(
            "Saved checkpoint {}: {:.2}s of audio ({} samples)",
            self.checkpoint_count,
            duration_seconds,
            audio_data.len()
        );

        Ok(())
    }

    /// Finalize the recording: save final checkpoint, merge all checkpoints, cleanup
    ///
    /// Returns the path to the final merged audio.mp4 file
    pub async fn finalize(&mut self) -> Result<PathBuf> {
        info!("Finalizing incremental recording...");

        // Save final buffer if not empty
        if !self.checkpoint_buffer.is_empty() {
            info!(
                "Saving final checkpoint with remaining {} chunks",
                self.checkpoint_buffer.len()
            );
            self.save_checkpoint()?;
            self.checkpoint_buffer.clear();
        }

        if self.checkpoint_count == 0 {
            return Err(anyhow!(
                "No audio checkpoints to merge - recording may have failed"
            ));
        }

        // Merge all checkpoints using FFmpeg concat
        let final_audio_path = self.meeting_folder.join("audio.mp4");
        self.merge_checkpoints(&final_audio_path).await?;

        // Clean up checkpoints directory
        info!("Cleaning up {} checkpoint files", self.checkpoint_count);
        if let Err(e) = std::fs::remove_dir_all(&self.checkpoints_dir) {
            warn!("Failed to clean up checkpoints directory: {}", e);
            // Non-fatal - user can manually delete
        }

        info!("Finalized recording: {}", final_audio_path.display());

        Ok(final_audio_path)
    }

    /// Merge all checkpoint files into final audio.mp4 using FFmpeg concat
    /// Uses concat demuxer for fast merging without re-encoding
    async fn merge_checkpoints(&self, output: &Path) -> Result<()> {
        info!(
            "Merging {} checkpoints into final audio file...",
            self.checkpoint_count
        );

        // Create concat list file for FFmpeg
        let list_file = self.checkpoints_dir.join("concat_list.txt");
        let mut list_content = String::new();

        for i in 0..self.checkpoint_count {
            let checkpoint_path = self
                .checkpoints_dir
                .join(format!("audio_chunk_{:03}.mp4", i));

            // Verify checkpoint exists
            if !checkpoint_path.exists() {
                return Err(anyhow!(
                    "Checkpoint file missing: {}",
                    checkpoint_path.display()
                ));
            }

            // Use absolute path for FFmpeg (required for safe mode)
            let abs_path = checkpoint_path.canonicalize()?;
            list_content.push_str(&format!("file '{}'\n", abs_path.display()));
        }

        std::fs::write(&list_file, list_content)?;

        let ffmpeg_path = find_ffmpeg_path().ok_or_else(|| {
            anyhow!("FFmpeg not found. Please install FFmpeg to finalize recordings.")
        })?;
        info!("Using FFmpeg at: {:?}", ffmpeg_path);

        // Run FFmpeg concat command
        // Using concat demuxer with copy codec for fast merging (no re-encoding)

        let mut command = std::process::Command::new(ffmpeg_path);

        command.args([
            "-f",
            "concat", // Use concat demuxer
            "-safe",
            "0", // Allow absolute paths
            "-i",
            list_file.to_str().unwrap(),
            "-c",
            "copy", // Copy codec - no re-encoding!
            "-y",   // Overwrite output file
            output.to_str().unwrap(),
        ]);

        // Hide console window on Windows to prevent CMD popup during finalization
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let ffmpeg_output = command.output()?;

        if !ffmpeg_output.status.success() {
            let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr);
            error!("FFmpeg merge failed: {}", stderr);
            return Err(anyhow!("FFmpeg concat failed: {}", stderr));
        }

        // Verify output file was created
        if !output.exists() {
            return Err(anyhow!(
                "Merged audio file was not created: {}",
                output.display()
            ));
        }

        info!(
            "Successfully merged {} checkpoints → {}",
            self.checkpoint_count,
            output.display()
        );

        Ok(())
    }

    /// Get the meeting folder path
    pub fn get_meeting_folder(&self) -> &PathBuf {
        &self.meeting_folder
    }

    /// Get current checkpoint count
    pub fn get_checkpoint_count(&self) -> u32 {
        self.checkpoint_count
    }
}

/// Audio recovery status for transcript recovery feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRecoveryStatus {
    pub status: String, // "success" | "partial" | "failed" | "none"
    pub chunk_count: u32,
    pub estimated_duration_seconds: f64,
    pub audio_file_path: Option<String>,
    pub message: String,
}

fn checkpoint_file_count(checkpoints: &Path) -> Result<u32, String> {
    let entries = match std::fs::read_dir(checkpoints) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(format!(
                "Cannot inspect remaining checkpoints at {}: {error}",
                checkpoints.display()
            ));
        }
    };
    let mut chunk_count = 0_u32;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("mp4")
        {
            chunk_count = chunk_count.saturating_add(1);
        }
    }
    Ok(chunk_count)
}

fn source_checkpoint_file_count(folder: &Path) -> Result<u32, String> {
    let mut chunk_count = 0_u32;
    for checkpoints in [
        folder
            .join(".tracks")
            .join("microphone")
            .join(".checkpoints"),
        folder.join(".tracks").join("system").join(".checkpoints"),
    ] {
        chunk_count = chunk_count.saturating_add(checkpoint_file_count(&checkpoints)?);
    }
    Ok(chunk_count)
}

fn keep_recovery_partial_for_source_checkpoints(
    mut recovery: AudioRecoveryStatus,
    source_chunk_count: u32,
) -> AudioRecoveryStatus {
    if recovery.status == "success" && source_chunk_count > 0 {
        recovery.status = "partial".into();
        recovery.chunk_count = recovery.chunk_count.saturating_add(source_chunk_count);
        recovery.message = format!(
            "{}; retained {source_chunk_count} source checkpoints for reprocessing, so recovery remains pending",
            recovery.message
        );
    }
    recovery
}

// A completed mixed file can outlive its checkpoints when session shutdown is
// interrupted after the saver finishes. Reuse it without rewriting user audio.
// Remaining checkpoints make completeness ambiguous: `partial` keeps the hook
// from marking the meeting saved and deleting those checkpoints.
fn inspect_existing_mixed_audio(folder: &Path) -> Result<Option<AudioRecoveryStatus>, String> {
    let path = folder.join("audio.mp4");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot inspect saved audio: {error}")),
    };
    if !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err("Saved audio is empty or is not a regular file; it was preserved".into());
    }
    let before = sha256_file(&path).map_err(|error| error.to_string())?;
    let samples = decoded_sample_count_bounded(&path).map_err(|error| error.to_string())?;
    if samples == 0 || sha256_file(&path).map_err(|error| error.to_string())? != before {
        return Err("Saved audio could not be verified or changed during verification".into());
    }
    let chunk_count = checkpoint_file_count(&folder.join(".checkpoints"))?
        .saturating_add(source_checkpoint_file_count(folder)?);
    Ok(Some(AudioRecoveryStatus {
        status: if chunk_count == 0 {
            "success"
        } else {
            "partial"
        }
        .into(),
        chunk_count,
        // Legacy mixed files have no authenticated sample-rate manifest. Do not
        // invent a duration from the caller's rate or checkpoint count.
        estimated_duration_seconds: 0.0,
        audio_file_path: Some(path.to_string_lossy().into_owned()),
        message: if chunk_count == 0 {
            "Verified existing saved audio without changing it"
        } else {
            "Saved audio is available; remaining checkpoints were preserved for further recovery"
        }
        .into(),
    }))
}

async fn existing_mixed_audio(folder: PathBuf) -> Result<Option<AudioRecoveryStatus>, String> {
    tokio::task::spawn_blocking(move || inspect_existing_mixed_audio(&folder))
        .await
        .map_err(|error| format!("Saved audio verification failed: {error}"))?
}

fn publish_recovered_audio_no_clobber(temporary: &Path, output: &Path) -> std::io::Result<()> {
    File::open(temporary)?.sync_all()?;
    // Both paths are in the same directory. Unlike an existence check followed
    // by rename, hard_link atomically refuses to replace a concurrently created
    // final file. Never fall back to an overwriting rename.
    std::fs::hard_link(temporary, output)?;
    std::fs::remove_file(temporary)?;
    File::open(output.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Missing audio parent")
    })?)?
    .sync_all()
}

/// Recover audio from checkpoint files
/// This is called by the transcript recovery system to merge audio chunks after a crash
#[tauri::command]
pub async fn recover_audio_from_checkpoints(
    meeting_folder: String,
    _sample_rate: u32,
) -> Result<AudioRecoveryStatus, String> {
    info!("Starting audio recovery for folder: {}", meeting_folder);

    let folder_path = PathBuf::from(&meeting_folder);
    if let Some(saved_audio) = existing_mixed_audio(folder_path.clone()).await? {
        return Ok(saved_audio);
    }
    let checkpoints_dir = folder_path.join(".checkpoints");

    // New recordings keep source-specific checkpoints under .tracks/<source>.
    // The browser recovery contract still calls this legacy command, so bridge
    // both layouts here instead of reporting "none" for a valid source track.
    if !has_checkpoint_files(&checkpoints_dir)
        .map_err(|error| format!("Failed to inspect checkpoints directory: {error}"))?
    {
        let mut recovered_tracks = Vec::new();
        let mut source_chunk_count = 0_u32;
        for kind in [TrackKind::Microphone, TrackKind::System] {
            let source = source_name(kind);
            let source_checkpoints = folder_path
                .join(".tracks")
                .join(source)
                .join(".checkpoints");
            if !has_checkpoint_files(&source_checkpoints)
                .map_err(|error| format!("Failed to inspect {source} checkpoints: {error}"))?
            {
                continue;
            }
            source_chunk_count = source_chunk_count.saturating_add(
                std::fs::read_dir(&source_checkpoints)
                    .map_err(|error| format!("Failed to read {source} checkpoints: {error}"))?
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|extension| extension.to_str())
                            == Some("mp4")
                    })
                    .count() as u32,
            );
            match SourceTrackSaver::recover(&folder_path, kind) {
                Ok(Some(track)) => recovered_tracks.push(track),
                Ok(None) => {}
                Err(error) => {
                    error!("Source {source} recovery failed: {error}");
                    return Ok(AudioRecoveryStatus {
                        status: "failed".to_string(),
                        chunk_count: source_chunk_count,
                        estimated_duration_seconds: f64::from(source_chunk_count) * 30.0,
                        audio_file_path: None,
                        message: format!("Source {source} recovery failed: {error}"),
                    });
                }
            }
        }
        if let Some(track) = recovered_tracks.first() {
            let audio_file_path = track.path.to_string_lossy().into_owned();
            info!("Successfully recovered source audio: {}", audio_file_path);
            return Ok(AudioRecoveryStatus {
                status: "partial".to_string(),
                chunk_count: source_chunk_count,
                estimated_duration_seconds: f64::from(source_chunk_count) * 30.0,
                audio_file_path: Some(audio_file_path),
                message: format!(
                    "Recovered {source_chunk_count} source audio chunks, but no playback-compatible audio.mp4 was created; recovery remains pending"
                ),
            });
        }
    }

    // Check if checkpoints directory exists
    if !has_checkpoint_files(&checkpoints_dir)
        .map_err(|error| format!("Failed to inspect checkpoints directory: {error}"))?
    {
        info!(
            "No checkpoints directory found at: {}",
            checkpoints_dir.display()
        );
        return Ok(AudioRecoveryStatus {
            status: "none".to_string(),
            chunk_count: 0,
            estimated_duration_seconds: 0.0,
            audio_file_path: None,
            message: "No audio checkpoints found".to_string(),
        });
    }

    // Scan for checkpoint files
    let mut checkpoint_files: Vec<_> = std::fs::read_dir(&checkpoints_dir)
        .map_err(|e| format!("Failed to read checkpoints directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("mp4"))
        .collect();

    if checkpoint_files.is_empty() {
        info!(
            "No checkpoint files found in: {}",
            checkpoints_dir.display()
        );
        return Ok(AudioRecoveryStatus {
            status: "none".to_string(),
            chunk_count: 0,
            estimated_duration_seconds: 0.0,
            audio_file_path: None,
            message: "No audio checkpoint files found".to_string(),
        });
    }

    checkpoint_files.sort_by_key(|entry| legacy_checkpoint_index(&entry.path()));

    let chunk_count = checkpoint_files.len() as u32;
    let estimated_duration = (chunk_count as f64) * 30.0; // 30 seconds per chunk

    info!(
        "Found {} checkpoint files, estimated duration: {:.2}s",
        chunk_count, estimated_duration
    );

    // Create FFmpeg concat file
    let concat_file_path = checkpoints_dir.join("concat_list.txt");
    let mut concat_content = String::new();

    for entry in &checkpoint_files {
        let path = entry
            .path()
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize path: {}", e))?;
        concat_content.push_str(&format!("file '{}'\n", path.display()));
    }

    std::fs::write(&concat_file_path, concat_content)
        .map_err(|e| format!("Failed to write concat file: {}", e))?;

    // Run FFmpeg to merge chunks
    let output_path = folder_path.join("audio.mp4");
    let recovering_output_path = folder_path.join(".audio.recovering.mp4");
    let _ = std::fs::remove_file(&recovering_output_path);
    let output_path_str = output_path
        .to_str()
        .ok_or("Invalid output path")?
        .to_string();

    let ffmpeg_path = find_ffmpeg_path()
        .ok_or_else(|| "FFmpeg not found. Please install FFmpeg to recover audio.".to_string())?;
    info!("Using FFmpeg at: {:?}", ffmpeg_path);

    let mut command = std::process::Command::new(ffmpeg_path);

    command.args([
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        concat_file_path.to_str().unwrap(),
        "-c",
        "copy",
        "-y", // Overwrite if exists
    ]);
    command.arg(&recovering_output_path);

    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let ffmpeg_result = command.output();

    match ffmpeg_result {
        Ok(output) if output.status.success() => {
            if let Err(error) =
                publish_recovered_audio_no_clobber(&recovering_output_path, &output_path)
            {
                let _ = std::fs::remove_file(&recovering_output_path);
                return Err(format!("Failed to finalize recovered audio: {error}"));
            }
            // Clean up concat file
            let _ = std::fs::remove_file(concat_file_path);

            info!("Successfully recovered audio: {}", output_path_str);

            let recovery = AudioRecoveryStatus {
                status: "success".to_string(),
                chunk_count,
                estimated_duration_seconds: estimated_duration,
                audio_file_path: Some(output_path_str),
                message: format!("Successfully recovered {} audio chunks", chunk_count),
            };
            let source_chunk_count = source_checkpoint_file_count(&folder_path)?;
            Ok(keep_recovery_partial_for_source_checkpoints(
                recovery,
                source_chunk_count,
            ))
        }
        Ok(output) => {
            let _ = std::fs::remove_file(&recovering_output_path);
            let error = String::from_utf8_lossy(&output.stderr);
            error!("FFmpeg recovery failed: {}", error);
            Ok(AudioRecoveryStatus {
                status: "failed".to_string(),
                chunk_count,
                estimated_duration_seconds: estimated_duration,
                audio_file_path: None,
                message: format!("FFmpeg failed: {}", error),
            })
        }
        Err(e) => {
            let _ = std::fs::remove_file(&recovering_output_path);
            error!("Failed to run FFmpeg: {}", e);
            Ok(AudioRecoveryStatus {
                status: "failed".to_string(),
                chunk_count,
                estimated_duration_seconds: estimated_duration,
                audio_file_path: None,
                message: format!("Failed to run FFmpeg: {}", e),
            })
        }
    }
}

/// Clean up checkpoint files after successful recording or recovery
/// This command is called by the frontend after successful save to clean up checkpoint files
#[tauri::command]
pub async fn cleanup_checkpoints(meeting_folder: String) -> Result<(), String> {
    info!("Cleaning up checkpoints for folder: {}", meeting_folder);

    let folder_path = PathBuf::from(&meeting_folder);
    let checkpoint_dirs = [
        folder_path.join(".checkpoints"),
        folder_path.join(".tracks/microphone/.checkpoints"),
        folder_path.join(".tracks/system/.checkpoints"),
    ];
    let mut removed = 0;
    for checkpoints_dir in checkpoint_dirs {
        if checkpoints_dir.exists() {
            std::fs::remove_dir_all(&checkpoints_dir).map_err(|error| {
                format!(
                    "Failed to remove checkpoints directory {}: {error}",
                    checkpoints_dir.display()
                )
            })?;
            removed += 1;
        }
    }
    if removed > 0 {
        File::open(&folder_path)
            .and_then(|file| file.sync_all())
            .map_err(|error| {
                format!("Failed to sync meeting folder after checkpoint cleanup: {error}")
            })?;
        info!("Successfully cleaned up {removed} checkpoint directories");
    } else {
        info!("No checkpoint directories to clean up");
    }

    Ok(())
}

/// Check for reusable saved mixed audio or source-track checkpoints.
/// The command name is retained for the existing frontend contract.
#[tauri::command]
pub async fn has_audio_checkpoints(meeting_folder: String) -> Result<bool, String> {
    let folder_path = PathBuf::from(&meeting_folder);
    if existing_mixed_audio(folder_path.clone()).await?.is_some() {
        return Ok(true);
    }
    for checkpoints_dir in [
        folder_path.join(".checkpoints"),
        folder_path.join(".tracks/microphone/.checkpoints"),
        folder_path.join(".tracks/system/.checkpoints"),
    ] {
        if has_checkpoint_files(&checkpoints_dir)
            .map_err(|error| format!("Failed to read checkpoints directory: {error}"))?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::super::capture::backend::TrackSpan;
    use super::super::recording_state::DeviceType;
    use super::*;
    use tempfile::tempdir;

    fn write_recovery_fixture(folder: &Path, kind: TrackKind, sentinel: f32) {
        let source = match kind {
            TrackKind::Microphone => "microphone",
            TrackKind::System => "system",
        };
        let namespace = folder.join(".tracks").join(source);
        let checkpoints = namespace.join(".checkpoints");
        std::fs::create_dir_all(&checkpoints).unwrap();
        let samples = vec![sentinel; 4_800];
        let checkpoint = checkpoints.join(format!("{source}_chunk_000.mp4"));
        encode_single_audio(bytemuck::cast_slice(&samples), 48_000, 1, &checkpoint).unwrap();
        std::fs::write(
            namespace.join("track.json"),
            serde_json::to_vec(&SourceTrackManifest {
                sample_rate: 48_000,
                channels: 1,
                checkpoint_count: 1,
                sample_count: samples.len() as u64,
                next_frame: samples.len() as u64,
                checkpoints: vec![CheckpointEvidence {
                    index: 0,
                    file_name: format!("{source}_chunk_000.mp4"),
                    input_sample_count: samples.len() as u64,
                    sha256: sha256_file(&checkpoint).unwrap(),
                }],
                final_sha256: None,
                timeline_journal: None,
                timeline_event_count: 0,
                timeline_bytes: 0,
                known_frame_prefix: 0,
                incomplete: false,
            })
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn raw_recovery_reads_only_the_two_source_namespaces() {
        let temp = tempdir().unwrap();
        let folder = temp.path();
        std::fs::create_dir_all(folder.join(".checkpoints")).unwrap();
        std::fs::write(folder.join(".checkpoints/audio_chunk_000.mp4"), b"legacy").unwrap();
        write_recovery_fixture(folder, TrackKind::Microphone, 0.2);
        write_recovery_fixture(folder, TrackKind::System, -0.2);

        let microphone = SourceTrackSaver::recover(folder, TrackKind::Microphone)
            .unwrap()
            .unwrap();
        let system = SourceTrackSaver::recover(folder, TrackKind::System)
            .unwrap()
            .unwrap();

        assert_ne!(microphone.sha256, system.sha256);
        assert_eq!(
            std::fs::read(folder.join(".checkpoints/audio_chunk_000.mp4")).unwrap(),
            b"legacy"
        );
    }

    #[test]
    fn raw_recovery_rejects_noncontiguous_or_foreign_checkpoint_names() {
        let temp = tempdir().unwrap();
        let namespace = temp.path().join(".tracks/microphone");
        let checkpoints = namespace.join(".checkpoints");
        std::fs::create_dir_all(&checkpoints).unwrap();
        std::fs::write(checkpoints.join("microphone_chunk_001.mp4"), b"not-empty").unwrap();
        std::fs::write(
            namespace.join("track.json"),
            serde_json::to_vec(&SourceTrackManifest {
                sample_rate: 48_000,
                channels: 1,
                checkpoint_count: 1,
                sample_count: 1,
                next_frame: 1,
                checkpoints: vec![CheckpointEvidence {
                    index: 0,
                    file_name: "microphone_chunk_000.mp4".into(),
                    input_sample_count: 1,
                    sha256: sha256_file(&checkpoints.join("microphone_chunk_001.mp4")).unwrap(),
                }],
                final_sha256: None,
                timeline_journal: None,
                timeline_event_count: 0,
                timeline_bytes: 0,
                known_frame_prefix: 0,
                incomplete: false,
            })
            .unwrap(),
        )
        .unwrap();

        assert!(matches!(
            SourceTrackSaver::recover(temp.path(), TrackKind::Microphone),
            Err(SourceTrackRecoveryError::Integrity(_))
        ));
    }

    #[test]
    fn checkpoint_sequence_sorts_numeric_indices_past_999() {
        let mut names = vec![
            "microphone_chunk_1000.mp4".to_string(),
            "microphone_chunk_099.mp4".to_string(),
            "microphone_chunk_999.mp4".to_string(),
        ];
        names.sort_by_key(|name| checkpoint_index(name, "microphone").unwrap());
        assert_eq!(
            names,
            vec![
                "microphone_chunk_099.mp4",
                "microphone_chunk_999.mp4",
                "microphone_chunk_1000.mp4",
            ]
        );
    }

    #[test]
    fn legacy_checkpoint_sequence_sorts_numeric_indices_past_999() {
        let mut paths = vec![
            PathBuf::from("audio_chunk_1000.mp4"),
            PathBuf::from("audio_chunk_099.mp4"),
            PathBuf::from("audio_chunk_999.mp4"),
        ];
        paths.sort_by_key(|path| legacy_checkpoint_index(path));
        assert_eq!(
            paths,
            vec![
                PathBuf::from("audio_chunk_099.mp4"),
                PathBuf::from("audio_chunk_999.mp4"),
                PathBuf::from("audio_chunk_1000.mp4"),
            ]
        );
    }

    #[test]
    fn checkpoint_sample_aggregate_mismatch_is_rejected_as_integrity_failure() {
        let temp = tempdir().unwrap();
        write_recovery_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let manifest_path = temp.path().join(".tracks/microphone/track.json");
        let mut manifest: SourceTrackManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        manifest.sample_count += 48_000;
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

        assert!(matches!(
            SourceTrackSaver::recover(temp.path(), TrackKind::Microphone),
            Err(SourceTrackRecoveryError::Integrity(_))
        ));
    }

    #[test]
    fn retry_replaces_a_stale_partial_final_through_a_verified_temporary_file() {
        let temp = tempdir().unwrap();
        write_recovery_fixture(temp.path(), TrackKind::Microphone, 0.2);
        std::fs::write(temp.path().join("microphone.mp4"), b"partial prior attempt").unwrap();

        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();

        assert_eq!(recovered.kind, TrackKind::Microphone);
        assert_eq!(recovered.sample_count, 4_800);
        assert!(!temp.path().join(".microphone.recovering.mp4").exists());
    }

    #[test]
    fn packet_truncated_final_is_rebuilt_from_authenticated_checkpoints() {
        let temp = tempdir().unwrap();
        write_recovery_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let final_path = temp.path().join("microphone.mp4");
        let original = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        let truncated = temp.path().join("truncated.mp4");
        let status = std::process::Command::new(find_ffmpeg_path().unwrap())
            .args(["-y", "-i"])
            .arg(&final_path)
            .args(["-c", "copy", "-t", "0.04"])
            .arg(&truncated)
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::rename(truncated, &final_path).unwrap();
        assert_ne!(sha256_file(&final_path).unwrap(), original.sha256);

        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.sha256, original.sha256);
        assert_eq!(sha256_file(&final_path).unwrap(), original.sha256);
    }

    #[test]
    fn corrupted_checkpoint_is_rejected_without_replacing_a_healthy_final() {
        let temp = tempdir().unwrap();
        write_recovery_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        let healthy_hash = recovered.sha256;
        let checkpoint = temp
            .path()
            .join(".tracks/microphone/.checkpoints/microphone_chunk_000.mp4");
        let mut bytes = std::fs::read(&checkpoint).unwrap();
        let index = bytes.len() - 1;
        bytes[index] ^= 1;
        std::fs::write(checkpoint, bytes).unwrap();

        assert!(matches!(
            SourceTrackSaver::recover(temp.path(), TrackKind::Microphone),
            Err(SourceTrackRecoveryError::Integrity(_))
        ));
        assert_eq!(
            sha256_file(&temp.path().join("microphone.mp4")).unwrap(),
            healthy_hash
        );
    }

    #[test]
    fn checkpoint_directory_io_failure_is_retryable_persistence() {
        let temp = tempdir().unwrap();
        write_recovery_fixture(temp.path(), TrackKind::Microphone, 0.2);
        let checkpoints = temp.path().join(".tracks/microphone/.checkpoints");
        std::fs::remove_dir_all(&checkpoints).unwrap();
        std::fs::write(&checkpoints, b"not a directory").unwrap();

        assert!(matches!(
            SourceTrackSaver::recover(temp.path(), TrackKind::Microphone),
            Err(SourceTrackRecoveryError::Persistence(_))
        ));
    }

    #[test]
    fn bounded_decoder_uses_a_fixed_buffer_independent_of_recording_length() {
        assert_eq!(DECODE_VERIFY_BUFFER_BYTES, 64 * 1024);
        let buffer_size = std::hint::black_box(DECODE_VERIFY_BUFFER_BYTES);
        assert!(buffer_size < 48_000 * 4);
    }

    fn write_dts_boundary_fixture(folder: &Path) -> (u64, Vec<usize>) {
        let namespace = folder.join(".tracks/microphone");
        let checkpoints = namespace.join(".checkpoints");
        std::fs::create_dir_all(&checkpoints).unwrap();
        let frame_lengths = [48_013_usize, 48_029, 24_117];
        let marker_frames = [vec![43_500_usize], vec![1_000, 46_000], vec![1_000]];
        let mut checkpoint_evidence = Vec::new();
        let mut expected_markers = Vec::new();
        let mut frame_offset = 0_usize;

        for (index, (&frame_count, markers)) in frame_lengths.iter().zip(marker_frames).enumerate()
        {
            let mut samples = vec![0.0_f32; frame_count];
            for marker in markers {
                for sample in samples.iter_mut().skip(marker).take(480) {
                    *sample = 0.8;
                }
                expected_markers.push(frame_offset + marker);
            }
            let checkpoint = checkpoints.join(format!("microphone_chunk_{index:03}.mp4"));
            encode_single_audio(bytemuck::cast_slice(&samples), 48_000, 1, &checkpoint).unwrap();
            checkpoint_evidence.push(CheckpointEvidence {
                index: index as u32,
                file_name: checkpoint
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                input_sample_count: frame_count as u64,
                sha256: sha256_file(&checkpoint).unwrap(),
            });
            frame_offset += frame_count;
        }

        let sample_count = frame_offset as u64;
        std::fs::write(
            namespace.join("track.json"),
            serde_json::to_vec(&SourceTrackManifest {
                sample_rate: 48_000,
                channels: 1,
                checkpoint_count: frame_lengths.len() as u32,
                sample_count,
                next_frame: sample_count,
                checkpoints: checkpoint_evidence,
                final_sha256: None,
                timeline_journal: None,
                timeline_event_count: 0,
                timeline_bytes: 0,
                known_frame_prefix: 0,
                incomplete: false,
            })
            .unwrap(),
        )
        .unwrap();
        (sample_count, expected_markers)
    }

    fn decoded_test_samples(path: &Path) -> Vec<f32> {
        let ffmpeg = find_ffmpeg_path().unwrap();
        let mut child = std::process::Command::new(ffmpeg)
            .args([
                "-v",
                "error",
                "-i",
                path.to_str().unwrap(),
                "-map",
                "0:a:0",
                "-f",
                "f32le",
                "-acodec",
                "pcm_f32le",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; DECODE_VERIFY_BUFFER_BYTES];
        loop {
            let read = stdout.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        assert!(child.wait().unwrap().success());
        assert_eq!(bytes.len() % std::mem::size_of::<f32>(), 0);
        bytes
            .chunks_exact(std::mem::size_of::<f32>())
            .map(|sample| f32::from_le_bytes(sample.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn checkpoint_merge_reencodes_one_stream_without_dts_overlap_or_marker_shift() {
        let temp = tempdir().unwrap();
        let (expected_samples, expected_markers) = write_dts_boundary_fixture(temp.path());
        let output = temp.path().join("microphone.mp4");
        let manifest: SourceTrackManifest = serde_json::from_slice(
            &std::fs::read(temp.path().join(".tracks/microphone/track.json")).unwrap(),
        )
        .unwrap();
        merge_numbered_checkpoints(
            &temp.path().join(".tracks/microphone/.checkpoints"),
            TrackKind::Microphone,
            &manifest,
            &output,
        )
        .unwrap();

        let decoded = decoded_test_samples(&output);
        assert!(
            decoded.len() as u64 <= expected_samples + 1_024,
            "checkpoint merge retained too much AAC padding: decoded={} expected={expected_samples}",
            decoded.len()
        );
        let marker_starts: Vec<_> = expected_markers
            .iter()
            .map(|&expected| {
                let start = expected.saturating_sub(48_000 / 20);
                let end = (expected + 48_000 / 20).min(decoded.len());
                (start..end)
                    .find(|&index| decoded[index].abs() > 0.25)
                    .unwrap_or_else(|| panic!("marker near frame {expected} was lost"))
            })
            .collect();
        for (actual, expected) in marker_starts.iter().zip(expected_markers) {
            assert!(
                actual.abs_diff(expected) <= 48_000 / 100,
                "marker moved from frame {expected} to {actual}"
            );
        }
    }

    #[test]
    fn recovery_reencodes_legacy_checkpoints_without_input_sample_counts() {
        let temp = tempdir().unwrap();
        let (expected_samples, _) = write_dts_boundary_fixture(temp.path());
        let manifest_path = temp.path().join(".tracks/microphone/track.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        for checkpoint in manifest["checkpoints"].as_array_mut().unwrap() {
            checkpoint
                .as_object_mut()
                .unwrap()
                .remove("input_sample_count");
        }
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.sample_count, expected_samples);
        assert!(temp.path().join("microphone.mp4").is_file());
        assert!(
            !normalized_checkpoint_path(&temp.path().join(".microphone.recovering.mp4")).exists()
        );
    }

    #[test]
    fn recovery_cleans_normalizing_output_after_checkpoint_decode_failure() {
        let temp = tempdir().unwrap();
        write_dts_boundary_fixture(temp.path());
        let manifest_path = temp.path().join(".tracks/microphone/track.json");
        let checkpoint = temp
            .path()
            .join(".tracks/microphone/.checkpoints/microphone_chunk_001.mp4");
        std::fs::write(&checkpoint, b"not an AAC checkpoint").unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        manifest["checkpoints"][1]["sha256"] =
            serde_json::Value::String(sha256_file(&checkpoint).unwrap());
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let recovering = temp.path().join(".microphone.recovering.mp4");
        let temporary = normalized_checkpoint_path(&recovering);

        assert!(matches!(
            SourceTrackSaver::recover(temp.path(), TrackKind::Microphone),
            Err(SourceTrackRecoveryError::Persistence(_))
        ));
        assert!(!recovering.exists());
        assert!(!temporary.exists());
        assert!(!temp.path().join("microphone.mp4").exists());
    }

    #[test]
    fn timeline_journal_is_durable_before_checkpoint_manifest_progress() {
        let temp = tempdir().unwrap();
        let mut saver =
            SourceTrackSaver::new(temp.path().to_path_buf(), TrackKind::Microphone).unwrap();
        let chunk = TrackChunk::contiguous(TrackKind::Microphone, vec![0.1; 4], 48_000, 1);
        saver.append_timeline_event(&chunk).unwrap();

        let journal = std::fs::read_to_string(&saver.journal_path).unwrap();
        assert!(journal.contains("\"seq\":1"));
        assert_eq!(saver.timeline_events.len(), 1);
        assert!(!saver.manifest_path.exists());
    }

    #[test]
    fn positioned_track_saver_materializes_forward_gaps() {
        let temp = tempdir().unwrap();
        let mut saver =
            SourceTrackSaver::new(temp.path().to_path_buf(), TrackKind::Microphone).unwrap();
        let span = |start_frame, frame_count| TrackSpan {
            start_frame,
            frame_count,
            epoch: 0,
            quality: TimestampQuality::Relative,
        };

        saver
            .add_chunk(TrackChunk::new(
                TrackKind::Microphone,
                vec![1.0, 2.0],
                48_000,
                1,
                span(0, 2),
                TrackBlockKind::Samples,
            ))
            .unwrap();
        saver
            .add_chunk(TrackChunk::new(
                TrackKind::Microphone,
                vec![3.0, 4.0],
                48_000,
                1,
                span(4, 2),
                TrackBlockKind::Samples,
            ))
            .unwrap();

        assert_eq!(saver.samples, vec![1.0, 2.0, 0.0, 0.0, 3.0, 4.0]);
        assert_eq!(saver.sample_count, 6);
        assert_eq!(saver.next_frame, 6);
    }

    #[test]
    fn positioned_track_saver_rejects_overlapping_spans() {
        let temp = tempdir().unwrap();
        let mut saver =
            SourceTrackSaver::new(temp.path().to_path_buf(), TrackKind::Microphone).unwrap();
        let span = |start_frame, frame_count| TrackSpan {
            start_frame,
            frame_count,
            epoch: 0,
            quality: TimestampQuality::Relative,
        };
        saver
            .add_chunk(TrackChunk::new(
                TrackKind::Microphone,
                vec![1.0, 2.0],
                48_000,
                1,
                span(10, 2),
                TrackBlockKind::Samples,
            ))
            .unwrap();
        let error = saver
            .add_chunk(TrackChunk::new(
                TrackKind::Microphone,
                vec![3.0, 4.0],
                48_000,
                1,
                span(11, 2),
                TrackBlockKind::Samples,
            ))
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Overlapping Microphone Track span"));
        assert_eq!(saver.next_frame, 12);
    }

    #[test]
    fn final_track_io_is_retryable_but_empty_content_is_integrity_failure() {
        let temp = tempdir().unwrap();
        let manifest = SourceTrackManifest {
            sample_rate: 48_000,
            channels: 1,
            checkpoint_count: 1,
            sample_count: 4_800,
            next_frame: 4_800,
            checkpoints: Vec::new(),
            final_sha256: None,
            timeline_journal: None,
            timeline_event_count: 0,
            timeline_bytes: 0,
            known_frame_prefix: 0,
            incomplete: false,
        };
        let path = temp.path().join("microphone.mp4");
        assert!(matches!(
            verify_final_track(&path, TrackKind::Microphone, &manifest),
            Err(SourceTrackRecoveryError::Persistence(_))
        ));
        std::fs::write(&path, []).unwrap();
        assert!(matches!(
            verify_final_track(&path, TrackKind::Microphone, &manifest),
            Err(SourceTrackRecoveryError::Integrity(_))
        ));
    }

    #[test]
    fn two_checkpoint_track_finalizes_and_recovers_with_verified_metadata() {
        let temp = tempdir().unwrap();
        let mut saver =
            SourceTrackSaver::new(temp.path().to_path_buf(), TrackKind::Microphone).unwrap();
        for value in [0.2_f32, -0.2_f32] {
            saver
                .add_chunk(TrackChunk::contiguous(
                    TrackKind::Microphone,
                    vec![value; 48_000 * 30],
                    48_000,
                    1,
                ))
                .unwrap();
        }

        let namespace = temp.path().join(".tracks/microphone");
        let manifest_path = namespace.join("track.json");
        assert!(!namespace.join("timeline.jsonl").exists());
        let checkpoint_manifest: SourceTrackManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        assert!(checkpoint_manifest.timeline_journal.is_none());
        assert_eq!(checkpoint_manifest.timeline_event_count, 0);
        assert_eq!(checkpoint_manifest.timeline_bytes, 0);
        assert_eq!(checkpoint_manifest.known_frame_prefix, 0);

        let finalized = saver.finalize().unwrap();
        assert_eq!(finalized.sample_rate, 48_000);
        assert_eq!(finalized.channels, 1);
        assert_eq!(finalized.sample_count, 2_880_000);

        // Older manifests have no optional timeline or incomplete fields.
        let mut legacy: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        assert!(legacy["timeline_journal"].is_null());
        for field in [
            "timeline_journal",
            "timeline_event_count",
            "timeline_bytes",
            "known_frame_prefix",
            "incomplete",
        ] {
            legacy.as_object_mut().unwrap().remove(field);
        }
        std::fs::write(&manifest_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        let recovered = SourceTrackSaver::recover(temp.path(), TrackKind::Microphone)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.sample_count, finalized.sample_count);
        assert_eq!(recovered.sha256, finalized.sha256);
    }

    #[tokio::test]
    async fn checkpoint_commands_cover_source_track_layout() {
        let temp = tempdir().unwrap();
        let microphone_checkpoints = temp.path().join(".tracks/microphone/.checkpoints");
        let system_checkpoints = temp.path().join(".tracks/system/.checkpoints");
        std::fs::create_dir_all(&microphone_checkpoints).unwrap();
        std::fs::create_dir_all(&system_checkpoints).unwrap();
        std::fs::write(
            microphone_checkpoints.join("microphone_chunk_000.mp4"),
            b"mic",
        )
        .unwrap();
        std::fs::write(system_checkpoints.join("system_chunk_000.mp4"), b"system").unwrap();

        assert!(
            has_audio_checkpoints(temp.path().to_string_lossy().into_owned())
                .await
                .unwrap()
        );
        cleanup_checkpoints(temp.path().to_string_lossy().into_owned())
            .await
            .unwrap();
        assert!(!microphone_checkpoints.exists());
        assert!(!system_checkpoints.exists());
        assert!(
            !has_audio_checkpoints(temp.path().to_string_lossy().into_owned())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn source_only_recovery_remains_partial_and_preserves_checkpoints() {
        let temp = tempdir().unwrap();
        let folder = temp.path().to_string_lossy().into_owned();
        write_recovery_fixture(temp.path(), TrackKind::Microphone, 0.25);
        let checkpoints = temp.path().join(".tracks/microphone/.checkpoints");

        let result = recover_audio_from_checkpoints(folder, 48_000)
            .await
            .unwrap();

        assert_eq!(result.status, "partial");
        assert!(result
            .message
            .contains("no playback-compatible audio.mp4 was created"));
        let expected_path = temp.path().join("microphone.mp4");
        assert_eq!(result.audio_file_path.as_deref(), expected_path.to_str());
        assert!(checkpoints.join("microphone_chunk_000.mp4").exists());
    }

    #[test]
    fn legacy_recovery_stays_partial_when_source_checkpoints_are_also_pending() {
        let temp = tempdir().unwrap();
        for source in ["microphone", "system"] {
            let checkpoints = temp
                .path()
                .join(".tracks")
                .join(source)
                .join(".checkpoints");
            std::fs::create_dir_all(&checkpoints).unwrap();
            std::fs::write(
                checkpoints.join(format!("{source}_chunk_000.mp4")),
                b"chunk",
            )
            .unwrap();
        }

        let source_chunk_count = source_checkpoint_file_count(temp.path()).unwrap();
        let recovery = keep_recovery_partial_for_source_checkpoints(
            AudioRecoveryStatus {
                status: "success".into(),
                chunk_count: 3,
                estimated_duration_seconds: 90.0,
                audio_file_path: Some(temp.path().join("audio.mp4").to_string_lossy().into()),
                message: "Successfully recovered 3 audio chunks".into(),
            },
            source_chunk_count,
        );

        assert_eq!(source_chunk_count, 2);
        assert_eq!(recovery.status, "partial");
        assert_eq!(recovery.chunk_count, 5);
        assert!(recovery
            .message
            .contains("retained 2 source checkpoints for reprocessing"));
        assert!(recovery.audio_file_path.is_some());
    }

    #[tokio::test]
    async fn test_checkpoint_creation() {
        // Create temp meeting folder
        let temp_dir = tempdir().unwrap();
        let meeting_folder = temp_dir.path().join("Test_Meeting");
        std::fs::create_dir_all(&meeting_folder).unwrap();
        std::fs::create_dir_all(meeting_folder.join(".checkpoints")).unwrap();

        let mut saver = IncrementalAudioSaver::new(meeting_folder.clone(), 48000).unwrap();

        // Add 60 seconds worth of audio (should create 2 checkpoints)
        for i in 0..120 {
            // 120 chunks of 0.5s each
            let chunk = AudioChunk {
                data: vec![0.5f32; 24000], // 0.5s at 48kHz
                sample_rate: 48000,
                timestamp: i as f64 * 0.5, // timestamp in seconds
                chunk_id: i as u64,
                device_type: DeviceType::Microphone,
                span: None,
            };
            saver.add_chunk(chunk).unwrap();
        }

        // Verify 2 checkpoints created
        assert_eq!(saver.checkpoint_count, 2);

        // Finalize and verify merge
        let final_path = saver.finalize().await.unwrap();
        assert!(final_path.exists());

        // Verify checkpoints directory deleted
        assert!(!meeting_folder.join(".checkpoints").exists());
    }

    #[tokio::test]
    async fn recovery_reopens_finalized_audio_after_checkpoint_cleanup_without_rewriting_it() {
        let temp = tempdir().unwrap();
        let folder = temp.path().to_string_lossy().into_owned();
        std::fs::create_dir(temp.path().join(".checkpoints")).unwrap();
        let mut saver = IncrementalAudioSaver::new(temp.path().to_path_buf(), 48_000).unwrap();
        saver
            .add_chunk(AudioChunk {
                data: vec![0.2; 4_800],
                sample_rate: 48_000,
                timestamp: 0.0,
                chunk_id: 0,
                device_type: DeviceType::Microphone,
                span: None,
            })
            .unwrap();
        let final_path = saver.finalize().await.unwrap();
        let before = std::fs::read(&final_path).unwrap();
        assert!(!temp.path().join(".checkpoints").exists());

        assert!(has_audio_checkpoints(folder.clone()).await.unwrap());
        for _ in 0..2 {
            let recovered = recover_audio_from_checkpoints(folder.clone(), 48_000)
                .await
                .unwrap();
            assert_eq!(recovered.status, "success");
            assert_eq!(recovered.audio_file_path.as_deref(), final_path.to_str());
            assert_eq!(std::fs::read(&final_path).unwrap(), before);
            assert!(!temp.path().join(".checkpoints").exists());
        }
    }

    #[tokio::test]
    async fn recovery_preserves_existing_audio_and_remaining_checkpoints_as_partial() {
        let temp = tempdir().unwrap();
        let folder = temp.path().to_string_lossy().into_owned();
        let final_path = temp.path().join("audio.mp4");
        encode_single_audio(
            bytemuck::cast_slice(&vec![0.2_f32; 4_800]),
            48_000,
            1,
            &final_path,
        )
        .unwrap();
        let checkpoints = temp.path().join(".checkpoints");
        std::fs::create_dir(&checkpoints).unwrap();
        let checkpoint = checkpoints.join("audio_chunk_000.mp4");
        encode_single_audio(
            bytemuck::cast_slice(&vec![0.4_f32; 9_600]),
            48_000,
            1,
            &checkpoint,
        )
        .unwrap();
        let source_checkpoints = temp.path().join(".tracks/microphone/.checkpoints");
        std::fs::create_dir_all(&source_checkpoints).unwrap();
        let source_checkpoint = source_checkpoints.join("microphone_chunk_000.mp4");
        encode_single_audio(
            bytemuck::cast_slice(&vec![0.3_f32; 4_800]),
            48_000,
            1,
            &source_checkpoint,
        )
        .unwrap();
        let final_before = std::fs::read(&final_path).unwrap();
        let checkpoint_before = std::fs::read(&checkpoint).unwrap();
        let source_checkpoint_before = std::fs::read(&source_checkpoint).unwrap();

        assert!(has_audio_checkpoints(folder.clone()).await.unwrap());
        let result = recover_audio_from_checkpoints(folder, 48_000)
            .await
            .unwrap();
        assert_eq!(result.status, "partial");
        assert_eq!(result.audio_file_path.as_deref(), final_path.to_str());
        assert_eq!(result.chunk_count, 2);
        assert_eq!(std::fs::read(final_path).unwrap(), final_before);
        assert_eq!(std::fs::read(checkpoint).unwrap(), checkpoint_before);
        assert_eq!(
            std::fs::read(source_checkpoint).unwrap(),
            source_checkpoint_before
        );
    }

    #[tokio::test]
    async fn recovery_rejects_invalid_existing_audio_without_replacing_it() {
        for bytes in [vec![], b"not an audio file".to_vec()] {
            let temp = tempdir().unwrap();
            let folder = temp.path().to_string_lossy().into_owned();
            let final_path = temp.path().join("audio.mp4");
            std::fs::write(&final_path, &bytes).unwrap();
            let checkpoints = temp.path().join(".checkpoints");
            std::fs::create_dir(&checkpoints).unwrap();
            let checkpoint = checkpoints.join("audio_chunk_000.mp4");
            encode_single_audio(
                bytemuck::cast_slice(&vec![0.2_f32; 4_800]),
                48_000,
                1,
                &checkpoint,
            )
            .unwrap();
            let checkpoint_before = std::fs::read(&checkpoint).unwrap();

            assert!(has_audio_checkpoints(folder.clone()).await.is_err());
            assert!(recover_audio_from_checkpoints(folder, 48_000)
                .await
                .is_err());
            assert_eq!(std::fs::read(final_path).unwrap(), bytes);
            assert_eq!(std::fs::read(checkpoint).unwrap(), checkpoint_before);
        }
    }

    #[tokio::test]
    async fn recovery_merges_legacy_checkpoints_when_no_final_exists() {
        let temp = tempdir().unwrap();
        let folder = temp.path().to_string_lossy().into_owned();
        let checkpoints = temp.path().join(".checkpoints");
        std::fs::create_dir(&checkpoints).unwrap();
        let checkpoint = checkpoints.join("audio_chunk_000.mp4");
        encode_single_audio(
            bytemuck::cast_slice(&vec![0.2_f32; 4_800]),
            48_000,
            1,
            &checkpoint,
        )
        .unwrap();
        let before = std::fs::read(&checkpoint).unwrap();
        assert!(has_audio_checkpoints(folder.clone()).await.unwrap());
        let result = recover_audio_from_checkpoints(folder.clone(), 48_000)
            .await
            .unwrap();
        assert_eq!(result.status, "success");
        assert!(decoded_sample_count_bounded(&temp.path().join("audio.mp4")).unwrap() > 0);
        assert_eq!(std::fs::read(&checkpoint).unwrap(), before);
        // The hook performs cleanup only after the meeting has been saved.
        cleanup_checkpoints(folder.clone()).await.unwrap();
        assert!(!checkpoints.exists());
        assert_eq!(
            recover_audio_from_checkpoints(folder, 48_000)
                .await
                .unwrap()
                .status,
            "success"
        );
    }

    #[test]
    fn recovery_publication_never_replaces_a_concurrently_created_final() {
        let temp = tempdir().unwrap();
        let temporary = temp.path().join(".audio.recovering.mp4");
        let output = temp.path().join("audio.mp4");
        std::fs::write(&temporary, b"new recovered audio").unwrap();
        std::fs::write(&output, b"existing saved audio").unwrap();
        assert!(publish_recovered_audio_no_clobber(&temporary, &output).is_err());
        assert_eq!(std::fs::read(&output).unwrap(), b"existing saved audio");
        assert_eq!(std::fs::read(&temporary).unwrap(), b"new recovered audio");
    }

    #[tokio::test]
    async fn test_empty_recording() {
        let temp_dir = tempdir().unwrap();
        let meeting_folder = temp_dir.path().join("Empty_Test");
        std::fs::create_dir_all(&meeting_folder).unwrap();
        std::fs::create_dir_all(meeting_folder.join(".checkpoints")).unwrap();

        let mut saver = IncrementalAudioSaver::new(meeting_folder.clone(), 48000).unwrap();

        // Try to finalize without adding any chunks
        let result = saver.finalize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No audio checkpoints"));
    }
}
