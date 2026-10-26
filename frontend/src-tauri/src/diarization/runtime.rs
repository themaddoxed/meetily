use super::audio::DiarizationAudio;
use super::types::{DiarizationStatus, SpeakerSegment};
use sha2::{Digest, Sha256};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const SHERPA_CLUSTER_THRESHOLD: f32 = 0.9;
pub const DIARIZATION_METHOD: &str = "sherpa_onnx_1_13_2";

const SEGMENTATION_MODEL_SIZE: u64 = 5_992_778;
const SEGMENTATION_MODEL_SHA256: &str =
    "fed22097bca974bad329a930b60865703766ff89f05fa09060bf6fd44e92e319";
const EMBEDDING_MODEL_SIZE: u64 = 39_593_761;
const EMBEDDING_MODEL_SHA256: &str =
    "1a331345f04805badbb495c775a6ddffcdd1a732567d5ec8b3d5749e3c7a5e4b";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {
    #[error("diarization model is unavailable")]
    ModelUnavailable,
    #[error("diarization model failed integrity verification")]
    ModelInvalid,
    #[error("audio is invalid for diarization")]
    InvalidAudio,
    #[error("diarization engine initialization failed")]
    InitializationFailed,
    #[error("diarization inference failed")]
    InferenceFailed,
    #[error("no speech was detected")]
    NoSpeech,
    #[error("diarization engine returned invalid output")]
    InvalidOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub path: PathBuf,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiarizationModelArtifacts {
    pub segmentation: ModelArtifact,
    pub embedding: ModelArtifact,
}

impl DiarizationModelArtifacts {
    pub fn from_resource_root(resource_root: &Path) -> Self {
        let model_root = resource_root.join("diarization/models");
        Self {
            segmentation: ModelArtifact {
                path: model_root.join("pyannote-segmentation-3.0.onnx"),
                size: SEGMENTATION_MODEL_SIZE,
                sha256: SEGMENTATION_MODEL_SHA256.to_string(),
            },
            embedding: ModelArtifact {
                path: model_root.join("3dspeaker-eres2net-base-16k.onnx"),
                size: EMBEDDING_MODEL_SIZE,
                sha256: EMBEDDING_MODEL_SHA256.to_string(),
            },
        }
    }

    pub fn verify(&self) -> Result<(), RuntimeError> {
        verify_model_artifact(&self.segmentation)?;
        verify_model_artifact(&self.embedding)
    }
}

pub fn sha256_file(path: &Path) -> Result<String, RuntimeError> {
    let mut file = File::open(path).map_err(|_| RuntimeError::ModelUnavailable)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|_| RuntimeError::ModelInvalid)?;
        if bytes_read == 0 {
            break;
        }
        digest.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", digest.finalize()))
}

pub fn verify_model_artifact(artifact: &ModelArtifact) -> Result<(), RuntimeError> {
    let metadata = std::fs::metadata(&artifact.path).map_err(|_| RuntimeError::ModelUnavailable)?;
    if !metadata.is_file() {
        return Err(RuntimeError::ModelUnavailable);
    }
    if metadata.len() != artifact.size {
        return Err(RuntimeError::ModelInvalid);
    }

    let actual_sha256 = sha256_file(&artifact.path)?;
    if !actual_sha256.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(RuntimeError::ModelInvalid);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineSegment {
    pub start: f64,
    pub end: f64,
    pub speaker: i32,
    pub confidence: Option<f64>,
}

pub trait DiarizationRuntime: Send + Sync {
    fn diarize(&self, audio: &DiarizationAudio) -> Result<Vec<EngineSegment>, RuntimeError>;
}

pub struct SherpaDiarizationRuntime {
    diarizer: OfflineSpeakerDiarization,
}

impl SherpaDiarizationRuntime {
    pub fn from_resource_root(
        resource_root: &Path,
        num_threads: i32,
    ) -> Result<Self, RuntimeError> {
        let artifacts = DiarizationModelArtifacts::from_resource_root(resource_root);
        artifacts.verify()?;
        let config = build_sherpa_config(
            artifacts.segmentation.path,
            artifacts.embedding.path,
            num_threads,
        );
        let diarizer =
            OfflineSpeakerDiarization::create(&config).ok_or(RuntimeError::InitializationFailed)?;

        Ok(Self { diarizer })
    }
}

impl DiarizationRuntime for SherpaDiarizationRuntime {
    fn diarize(&self, audio: &DiarizationAudio) -> Result<Vec<EngineSegment>, RuntimeError> {
        if audio.samples.is_empty()
            || audio.samples.iter().any(|sample| !sample.is_finite())
            || audio.sample_rate != self.diarizer.sample_rate() as u32
        {
            return Err(RuntimeError::InvalidAudio);
        }

        let result = self
            .diarizer
            .process(&audio.samples)
            .ok_or(RuntimeError::InferenceFailed)?;
        let segments = result
            .sort_by_start_time()
            .into_iter()
            .map(|segment| EngineSegment {
                start: f64::from(segment.start),
                end: f64::from(segment.end),
                speaker: segment.speaker,
                confidence: None,
            })
            .collect::<Vec<_>>();

        if segments.is_empty() {
            Err(RuntimeError::NoSpeech)
        } else {
            Ok(segments)
        }
    }
}

pub fn build_sherpa_config(
    segmentation_model: PathBuf,
    embedding_model: PathBuf,
    num_threads: i32,
) -> OfflineSpeakerDiarizationConfig {
    let num_threads = num_threads.max(1);
    OfflineSpeakerDiarizationConfig {
        segmentation: OfflineSpeakerSegmentationModelConfig {
            pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                model: Some(segmentation_model.to_string_lossy().into_owned()),
            },
            num_threads,
            debug: false,
            provider: Some("cpu".to_string()),
        },
        embedding: SpeakerEmbeddingExtractorConfig {
            model: Some(embedding_model.to_string_lossy().into_owned()),
            num_threads,
            debug: false,
            provider: Some("cpu".to_string()),
        },
        clustering: FastClusteringConfig {
            num_clusters: -1,
            threshold: SHERPA_CLUSTER_THRESHOLD,
        },
        ..Default::default()
    }
}

pub fn normalize_engine_segments(
    meeting_id: &str,
    engine_segments: Vec<EngineSegment>,
) -> Result<Vec<SpeakerSegment>, RuntimeError> {
    if engine_segments.is_empty() {
        return Err(RuntimeError::NoSpeech);
    }
    if engine_segments.iter().any(invalid_engine_segment) {
        return Err(RuntimeError::InvalidOutput);
    }

    let speaker_numbers = engine_segments
        .iter()
        .map(|segment| segment.speaker)
        .collect::<BTreeSet<_>>();
    let speaker_names = speaker_numbers
        .into_iter()
        .enumerate()
        .map(|(index, raw_speaker)| (raw_speaker, index + 1))
        .collect::<BTreeMap<_, _>>();

    let mut boundaries = engine_segments
        .iter()
        .flat_map(|segment| [segment.start, segment.end])
        .collect::<Vec<_>>();
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|left, right| left.total_cmp(right).is_eq());

    let mut normalized: Vec<SpeakerSegment> = Vec::new();
    for interval in boundaries.windows(2) {
        let start = interval[0];
        let end = interval[1];
        if end <= start {
            continue;
        }

        let active = engine_segments
            .iter()
            .filter(|segment| segment.start < end && segment.end > start)
            .collect::<Vec<_>>();
        let active_speakers = active
            .iter()
            .map(|segment| segment.speaker)
            .collect::<BTreeSet<_>>();
        if active_speakers.is_empty() {
            continue;
        }

        let (speaker_id, speaker_label, confidence, is_overlap) = if active_speakers.len() == 1 {
            let raw_speaker = *active_speakers.first().ok_or(RuntimeError::InvalidOutput)?;
            let speaker_number = speaker_names
                .get(&raw_speaker)
                .ok_or(RuntimeError::InvalidOutput)?;
            let confidence = active
                .iter()
                .filter_map(|segment| segment.confidence)
                .max_by(f64::total_cmp);
            (
                Some(format!("speaker-{speaker_number}")),
                Some(format!("Speaker {speaker_number}")),
                confidence,
                false,
            )
        } else {
            (None, None, None, true)
        };

        let candidate = SpeakerSegment {
            meeting_id: meeting_id.to_string(),
            source: "mixed".to_string(),
            start_time: start,
            end_time: end,
            speaker_id,
            speaker_label,
            confidence,
            is_overlap,
            diarization_status: DiarizationStatus::Final,
            diarization_method: Some(DIARIZATION_METHOD.to_string()),
        };

        if let Some(previous) = normalized.last_mut() {
            if previous.end_time == candidate.start_time
                && previous.speaker_id == candidate.speaker_id
                && previous.speaker_label == candidate.speaker_label
                && previous.confidence == candidate.confidence
                && previous.is_overlap == candidate.is_overlap
            {
                previous.end_time = candidate.end_time;
                continue;
            }
        }
        normalized.push(candidate);
    }

    if normalized.is_empty() {
        return Err(RuntimeError::NoSpeech);
    }
    if normalized
        .iter()
        .any(|segment| segment.validate_for_meeting(meeting_id).is_err())
    {
        return Err(RuntimeError::InvalidOutput);
    }

    Ok(normalized)
}

fn invalid_engine_segment(segment: &EngineSegment) -> bool {
    !segment.start.is_finite()
        || !segment.end.is_finite()
        || segment.start < 0.0
        || segment.end <= segment.start
        || segment.speaker < 0
        || segment
            .confidence
            .is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
}
