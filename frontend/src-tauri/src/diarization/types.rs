use serde::{Deserialize, Serialize};

const MAX_TEXT_CHARS: usize = 128;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationStatus {
    #[default]
    None,
    Provisional,
    Final,
    FallbackToLive,
    Failed,
    NeedsReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiarizationInputError {
    MeetingMismatch,
    InvalidTimeRange,
    InvalidConfidence,
    InvalidText,
    InvalidOverlapIdentity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerSegment {
    pub meeting_id: String,
    pub source: String,
    pub start_time: f64,
    pub end_time: f64,
    pub speaker_id: Option<String>,
    pub speaker_label: Option<String>,
    pub confidence: Option<f64>,
    pub is_overlap: bool,
    pub diarization_status: DiarizationStatus,
    pub diarization_method: Option<String>,
}

impl SpeakerSegment {
    pub fn validate_for_meeting(&self, meeting_id: &str) -> Result<(), DiarizationInputError> {
        if !is_valid_text(meeting_id)
            || !is_valid_text(&self.meeting_id)
            || !is_valid_text(&self.source)
            || !is_valid_optional_text(self.speaker_id.as_deref())
            || !is_valid_optional_text(self.speaker_label.as_deref())
            || !is_valid_optional_text(self.diarization_method.as_deref())
        {
            return Err(DiarizationInputError::InvalidText);
        }

        if self.meeting_id != meeting_id {
            return Err(DiarizationInputError::MeetingMismatch);
        }

        if !self.start_time.is_finite()
            || !self.end_time.is_finite()
            || self.start_time < 0.0
            || self.end_time < 0.0
            || self.end_time <= self.start_time
        {
            return Err(DiarizationInputError::InvalidTimeRange);
        }

        if let Some(confidence) = self.confidence {
            if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
                return Err(DiarizationInputError::InvalidConfidence);
            }
        }

        if self.is_overlap && (self.speaker_id.is_some() || self.speaker_label.is_some()) {
            return Err(DiarizationInputError::InvalidOverlapIdentity);
        }

        Ok(())
    }
}

fn is_valid_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= MAX_TEXT_CHARS
        && !value.chars().any(char::is_control)
}

fn is_valid_optional_text(value: Option<&str>) -> bool {
    value.map_or(true, is_valid_text)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptWindow {
    pub transcript_id: String,
    pub audio_start_time: Option<f64>,
    pub audio_end_time: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerAssignment {
    pub speaker_id: Option<String>,
    pub speaker_label: Option<String>,
    pub speaker_color: Option<String>,
    pub is_overlap: bool,
    pub diarization_status: DiarizationStatus,
    pub diarization_method: Option<String>,
    pub diarization_confidence: Option<f64>,
}

impl SpeakerAssignment {
    pub fn unknown(status: DiarizationStatus, method: Option<impl Into<String>>) -> Self {
        Self {
            speaker_id: None,
            speaker_label: Some("Unknown speaker".to_string()),
            speaker_color: None,
            is_overlap: false,
            diarization_status: status,
            diarization_method: method.map(Into::into),
            diarization_confidence: None,
        }
    }

    pub fn overlap(confidence: f64, status: DiarizationStatus, method: impl Into<String>) -> Self {
        Self {
            speaker_id: None,
            speaker_label: Some("Multiple speakers".to_string()),
            speaker_color: Some("#f97316".to_string()),
            is_overlap: true,
            diarization_status: status,
            diarization_method: Some(method.into()),
            diarization_confidence: Some(confidence),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment() -> SpeakerSegment {
        SpeakerSegment {
            meeting_id: "meeting-1".to_string(),
            source: "system".to_string(),
            start_time: 1.0,
            end_time: 2.0,
            speaker_id: Some("speaker-1".to_string()),
            speaker_label: Some("Speaker 1".to_string()),
            confidence: Some(0.8),
            is_overlap: false,
            diarization_status: DiarizationStatus::Provisional,
            diarization_method: Some("unit_test".to_string()),
        }
    }

    #[test]
    fn diarization_status_round_trips_every_snake_case_variant() {
        let cases = [
            (DiarizationStatus::None, "none"),
            (DiarizationStatus::Provisional, "provisional"),
            (DiarizationStatus::Final, "final"),
            (DiarizationStatus::FallbackToLive, "fallback_to_live"),
            (DiarizationStatus::Failed, "failed"),
            (DiarizationStatus::NeedsReview, "needs_review"),
        ];

        for (status, serialized) in cases {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{serialized}\""));
            assert_eq!(
                serde_json::from_str::<DiarizationStatus>(&json).unwrap(),
                status
            );
        }
    }

    #[test]
    fn unknown_assignment_has_only_generic_identity() {
        let assignment =
            SpeakerAssignment::unknown(DiarizationStatus::NeedsReview, Some("unit_test"));

        assert_eq!(assignment.speaker_id, None);
        assert_eq!(assignment.speaker_label.as_deref(), Some("Unknown speaker"));
        assert_eq!(assignment.speaker_color, None);
        assert!(!assignment.is_overlap);
        assert_eq!(
            assignment.diarization_status,
            DiarizationStatus::NeedsReview
        );
        assert_eq!(assignment.diarization_method.as_deref(), Some("unit_test"));
        assert_eq!(assignment.diarization_confidence, None);
    }

    #[test]
    fn unknown_assignment_accepts_no_method() {
        let assignment = SpeakerAssignment::unknown(DiarizationStatus::NeedsReview, None::<String>);

        assert_eq!(assignment.diarization_method, None);
    }

    #[test]
    fn overlap_assignment_is_generic_and_preserves_metadata() {
        let assignment =
            SpeakerAssignment::overlap(0.91, DiarizationStatus::Provisional, "unit_test");

        assert_eq!(assignment.speaker_id, None);
        assert_eq!(
            assignment.speaker_label.as_deref(),
            Some("Multiple speakers")
        );
        assert_eq!(assignment.speaker_color.as_deref(), Some("#f97316"));
        assert!(assignment.is_overlap);
        assert_eq!(
            assignment.diarization_status,
            DiarizationStatus::Provisional
        );
        assert_eq!(assignment.diarization_method.as_deref(), Some("unit_test"));
        assert_eq!(assignment.diarization_confidence, Some(0.91));
    }

    #[test]
    fn valid_segment_and_confidence_boundaries_pass_validation() {
        for confidence in [None, Some(0.0), Some(1.0)] {
            let mut candidate = segment();
            candidate.confidence = confidence;
            assert_eq!(candidate.validate_for_meeting("meeting-1"), Ok(()));
        }
    }

    #[test]
    fn blank_required_strings_are_invalid_text() {
        let mut blank_meeting = segment();
        blank_meeting.meeting_id = " \t".to_string();
        assert_eq!(
            blank_meeting.validate_for_meeting("meeting-1"),
            Err(DiarizationInputError::InvalidText)
        );

        assert_eq!(
            segment().validate_for_meeting("  "),
            Err(DiarizationInputError::InvalidText)
        );

        let mut blank_source = segment();
        blank_source.source = "  ".to_string();
        assert_eq!(
            blank_source.validate_for_meeting("meeting-1"),
            Err(DiarizationInputError::InvalidText)
        );
    }

    #[test]
    fn cross_meeting_segment_is_rejected() {
        assert_eq!(
            segment().validate_for_meeting("meeting-2"),
            Err(DiarizationInputError::MeetingMismatch)
        );
    }

    #[test]
    fn blank_optional_strings_are_invalid_text() {
        let mut candidates = Vec::new();

        let mut blank_speaker_id = segment();
        blank_speaker_id.speaker_id = Some("  ".to_string());
        candidates.push(blank_speaker_id);

        let mut blank_speaker_label = segment();
        blank_speaker_label.speaker_label = Some("\t".to_string());
        candidates.push(blank_speaker_label);

        let mut blank_method = segment();
        blank_method.diarization_method = Some("\n".to_string());
        candidates.push(blank_method);

        for candidate in candidates {
            assert_eq!(
                candidate.validate_for_meeting("meeting-1"),
                Err(DiarizationInputError::InvalidText)
            );
        }
    }

    #[test]
    fn control_characters_are_rejected_in_every_segment_string_field() {
        let mut candidates = Vec::new();

        let mut meeting_id = segment();
        meeting_id.meeting_id = "meeting\n1".to_string();
        candidates.push((meeting_id, "meeting\n1"));

        let mut source = segment();
        source.source = "sys\0tem".to_string();
        candidates.push((source, "meeting-1"));

        let mut speaker_id = segment();
        speaker_id.speaker_id = Some("speaker\u{7f}1".to_string());
        candidates.push((speaker_id, "meeting-1"));

        let mut speaker_label = segment();
        speaker_label.speaker_label = Some("Speaker\r1".to_string());
        candidates.push((speaker_label, "meeting-1"));

        let mut method = segment();
        method.diarization_method = Some("unit\ttest".to_string());
        candidates.push((method, "meeting-1"));

        for (candidate, meeting_id) in candidates {
            assert_eq!(
                candidate.validate_for_meeting(meeting_id),
                Err(DiarizationInputError::InvalidText)
            );
        }
    }

    #[test]
    fn strings_over_128_characters_are_rejected_in_every_segment_string_field() {
        let too_long = "x".repeat(129);
        let mut candidates = Vec::new();

        let mut meeting_id = segment();
        meeting_id.meeting_id = too_long.clone();
        candidates.push((meeting_id, too_long.as_str()));

        let mut source = segment();
        source.source = too_long.clone();
        candidates.push((source, "meeting-1"));

        let mut speaker_id = segment();
        speaker_id.speaker_id = Some(too_long.clone());
        candidates.push((speaker_id, "meeting-1"));

        let mut speaker_label = segment();
        speaker_label.speaker_label = Some(too_long.clone());
        candidates.push((speaker_label, "meeting-1"));

        let mut method = segment();
        method.diarization_method = Some(too_long.clone());
        candidates.push((method, "meeting-1"));

        for (candidate, meeting_id) in candidates {
            assert_eq!(
                candidate.validate_for_meeting(meeting_id),
                Err(DiarizationInputError::InvalidText)
            );
        }
    }

    #[test]
    fn invalid_time_ranges_are_rejected() {
        for (start_time, end_time) in [
            (f64::NAN, 2.0),
            (1.0, f64::NAN),
            (f64::INFINITY, 2.0),
            (1.0, f64::INFINITY),
            (f64::NEG_INFINITY, 2.0),
            (1.0, f64::NEG_INFINITY),
            (-0.1, 2.0),
            (1.0, -0.1),
            (1.0, 1.0),
            (2.0, 1.0),
        ] {
            let mut candidate = segment();
            candidate.start_time = start_time;
            candidate.end_time = end_time;
            assert_eq!(
                candidate.validate_for_meeting("meeting-1"),
                Err(DiarizationInputError::InvalidTimeRange)
            );
        }
    }

    #[test]
    fn invalid_confidence_values_are_rejected() {
        for confidence in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, 1.01] {
            let mut candidate = segment();
            candidate.confidence = Some(confidence);
            assert_eq!(
                candidate.validate_for_meeting("meeting-1"),
                Err(DiarizationInputError::InvalidConfidence)
            );
        }
    }

    #[test]
    fn overlap_segment_cannot_carry_speaker_id() {
        let mut candidate = segment();
        candidate.is_overlap = true;
        candidate.speaker_label = None;

        assert_eq!(
            candidate.validate_for_meeting("meeting-1"),
            Err(DiarizationInputError::InvalidOverlapIdentity)
        );
    }

    #[test]
    fn overlap_segment_cannot_carry_speaker_label() {
        let mut candidate = segment();
        candidate.is_overlap = true;
        candidate.speaker_id = None;

        assert_eq!(
            candidate.validate_for_meeting("meeting-1"),
            Err(DiarizationInputError::InvalidOverlapIdentity)
        );
    }

    #[test]
    fn generic_overlap_segment_passes_validation() {
        let mut candidate = segment();
        candidate.is_overlap = true;
        candidate.speaker_id = None;
        candidate.speaker_label = None;

        assert_eq!(candidate.validate_for_meeting("meeting-1"), Ok(()));
    }
}
