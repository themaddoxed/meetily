use crate::audio::decode_audio_file;
use std::path::Path;

const DIARIZATION_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, PartialEq)]
pub struct DiarizationAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DiarizationAudioError {
    #[error("audio file is unavailable")]
    Unavailable,
    #[error("audio format is unsupported or unreadable")]
    Unsupported,
    #[error("audio file is empty")]
    Empty,
    #[error("decoded audio contains invalid samples")]
    InvalidSamples,
}

pub fn decode_audio_to_mono_16k(path: &Path) -> Result<DiarizationAudio, DiarizationAudioError> {
    let metadata = std::fs::metadata(path).map_err(|_| DiarizationAudioError::Unavailable)?;
    if !metadata.is_file() {
        return Err(DiarizationAudioError::Unavailable);
    }
    if metadata.len() == 0 {
        return Err(DiarizationAudioError::Empty);
    }

    let decoded = decode_audio_file(path).map_err(|_| DiarizationAudioError::Unsupported)?;
    let samples = decoded.to_whisper_format();
    if samples.is_empty() {
        return Err(DiarizationAudioError::Empty);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(DiarizationAudioError::InvalidSamples);
    }

    Ok(DiarizationAudio {
        samples,
        sample_rate: DIARIZATION_SAMPLE_RATE,
    })
}
