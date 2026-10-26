use super::types::{DiarizationStatus, SpeakerAssignment, SpeakerSegment, TranscriptWindow};

pub fn assign_speaker_to_transcript(
    transcript: &TranscriptWindow,
    segments: &[SpeakerSegment],
    min_overlap_ratio: f64,
    method: &str,
) -> SpeakerAssignment {
    let Some(transcript_start) = transcript.audio_start_time else {
        return unknown_assignment(method);
    };
    let Some(transcript_end) = transcript.audio_end_time else {
        return unknown_assignment(method);
    };

    if !is_valid_window(transcript_start, transcript_end) {
        return unknown_assignment(method);
    }

    let transcript_duration = transcript_end - transcript_start;
    let min_overlap_ratio = normalize_min_overlap_ratio(min_overlap_ratio);
    let mut best_overlap_segment: Option<(&SpeakerSegment, f64)> = None;
    let mut best_speaker_segment: Option<(&SpeakerSegment, f64)> = None;

    for segment in segments {
        if !is_valid_window(segment.start_time, segment.end_time) {
            continue;
        }

        let overlap_start = transcript_start.max(segment.start_time);
        let overlap_end = transcript_end.min(segment.end_time);
        let overlap = (overlap_end - overlap_start).max(0.0);
        if overlap <= 0.0 || overlap / transcript_duration < min_overlap_ratio {
            continue;
        }

        let best_segment = if segment.is_overlap {
            &mut best_overlap_segment
        } else {
            &mut best_speaker_segment
        };

        match best_segment {
            Some((_, best_overlap)) if *best_overlap >= overlap => {}
            _ => *best_segment = Some((segment, overlap)),
        }
    }

    if let Some((segment, _)) = best_overlap_segment {
        return SpeakerAssignment::overlap(
            segment.confidence.unwrap_or(0.0),
            segment.diarization_status,
            method,
        );
    }

    let Some((segment, _)) = best_speaker_segment else {
        return unknown_assignment(method);
    };

    SpeakerAssignment {
        speaker_id: segment.speaker_id.clone(),
        speaker_label: segment
            .speaker_label
            .clone()
            .or_else(|| Some("Unknown speaker".to_string())),
        speaker_color: segment.speaker_id.as_deref().map(stable_speaker_color),
        is_overlap: false,
        diarization_status: segment.diarization_status,
        diarization_method: Some(method.to_string()),
        diarization_confidence: segment.confidence,
    }
}

fn stable_speaker_color(speaker_id: &str) -> String {
    const PALETTE: [&str; 8] = [
        "#2563eb", "#7c3aed", "#0f766e", "#b45309", "#be123c", "#0369a1", "#4d7c0f", "#9333ea",
    ];

    let hash = speaker_id
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    PALETTE[hash as usize % PALETTE.len()].to_string()
}

fn unknown_assignment(method: &str) -> SpeakerAssignment {
    SpeakerAssignment::unknown(DiarizationStatus::NeedsReview, Some(method))
}

fn is_valid_window(start_time: f64, end_time: f64) -> bool {
    start_time.is_finite()
        && end_time.is_finite()
        && start_time >= 0.0
        && end_time >= 0.0
        && end_time > start_time
}

fn normalize_min_overlap_ratio(ratio: f64) -> f64 {
    if ratio.is_nan() {
        1.0
    } else {
        ratio.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::types::{DiarizationStatus, SpeakerSegment, TranscriptWindow};

    fn segment(label: &str, start_time: f64, end_time: f64) -> SpeakerSegment {
        SpeakerSegment {
            meeting_id: "meeting-1".to_string(),
            source: "system".to_string(),
            start_time,
            end_time,
            speaker_id: Some(label.to_lowercase().replace(' ', "-")),
            speaker_label: Some(label.to_string()),
            confidence: Some(0.8),
            is_overlap: false,
            diarization_status: DiarizationStatus::Provisional,
            diarization_method: Some("segment_method".to_string()),
        }
    }

    fn transcript(start_time: Option<f64>, end_time: Option<f64>) -> TranscriptWindow {
        TranscriptWindow {
            transcript_id: "transcript-1".to_string(),
            audio_start_time: start_time,
            audio_end_time: end_time,
        }
    }

    fn assert_unknown(assignment: &crate::diarization::types::SpeakerAssignment) {
        assert_eq!(assignment.speaker_id, None);
        assert_eq!(assignment.speaker_label.as_deref(), Some("Unknown speaker"));
        assert!(!assignment.is_overlap);
        assert_eq!(
            assignment.diarization_status,
            DiarizationStatus::NeedsReview
        );
        assert_eq!(assignment.diarization_method.as_deref(), Some("unit_test"));
    }

    #[test]
    fn assigns_segment_with_largest_temporal_overlap() {
        let segments = [
            segment("Speaker 1", 10.0, 12.0),
            segment("Speaker 2", 12.0, 16.0),
        ];

        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(16.0)),
            &segments,
            0.1,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id.as_deref(), Some("speaker-2"));
        assert_eq!(assignment.speaker_label.as_deref(), Some("Speaker 2"));
    }

    #[test]
    fn equal_overlap_keeps_first_input_segment() {
        let segments = [
            segment("Speaker 1", 10.0, 13.0),
            segment("Speaker 2", 13.0, 16.0),
        ];

        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(16.0)),
            &segments,
            0.1,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id.as_deref(), Some("speaker-1"));
    }

    #[test]
    fn exact_overlap_threshold_is_inclusive() {
        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[segment("Speaker 1", 10.0, 15.0)],
            0.5,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id.as_deref(), Some("speaker-1"));
    }

    #[test]
    fn segment_below_overlap_threshold_returns_unknown() {
        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[segment("Speaker 1", 10.0, 14.0)],
            0.5,
            "unit_test",
        );

        assert_unknown(&assignment);
    }

    #[test]
    fn missing_transcript_window_returns_unknown() {
        for window in [transcript(None, Some(1.0)), transcript(Some(0.0), None)] {
            let assignment = assign_speaker_to_transcript(
                &window,
                &[segment("Speaker 1", 0.0, 1.0)],
                0.1,
                "unit_test",
            );

            assert_unknown(&assignment);
        }
    }

    #[test]
    fn zero_reversed_negative_and_non_finite_transcript_windows_return_unknown() {
        for (start_time, end_time) in [
            (0.0, 0.0),
            (2.0, 1.0),
            (-1.0, 1.0),
            (0.0, -1.0),
            (f64::NAN, 1.0),
            (0.0, f64::NAN),
            (f64::INFINITY, 1.0),
            (0.0, f64::INFINITY),
            (f64::NEG_INFINITY, 1.0),
            (0.0, f64::NEG_INFINITY),
        ] {
            let assignment = assign_speaker_to_transcript(
                &transcript(Some(start_time), Some(end_time)),
                &[segment("Speaker 1", 0.0, 1.0)],
                0.1,
                "unit_test",
            );

            assert_unknown(&assignment);
        }
    }

    #[test]
    fn negative_threshold_clamps_to_zero() {
        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[segment("Speaker 1", 10.0, 11.0)],
            -0.5,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id.as_deref(), Some("speaker-1"));
    }

    #[test]
    fn threshold_above_one_clamps_to_one() {
        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[segment("Speaker 1", 10.0, 20.0)],
            1.5,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id.as_deref(), Some("speaker-1"));
    }

    #[test]
    fn nan_threshold_behaves_like_one() {
        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[segment("Speaker 1", 10.0, 15.0)],
            f64::NAN,
            "unit_test",
        );

        assert_unknown(&assignment);
    }

    #[test]
    fn invalid_segment_windows_are_ignored() {
        for (start_time, end_time) in [
            (f64::NAN, 10.0),
            (0.0, f64::NAN),
            (f64::INFINITY, 10.0),
            (0.0, f64::INFINITY),
            (f64::NEG_INFINITY, 10.0),
            (0.0, f64::NEG_INFINITY),
            (-1.0, 10.0),
            (5.0, -1.0),
            (5.0, 5.0),
            (8.0, 5.0),
        ] {
            let invalid = segment("Invalid", start_time, end_time);
            let valid = segment("Valid", 4.0, 6.0);
            let assignment = assign_speaker_to_transcript(
                &transcript(Some(0.0), Some(10.0)),
                &[invalid, valid],
                0.1,
                "unit_test",
            );

            assert_eq!(assignment.speaker_id.as_deref(), Some("valid"));
        }
    }

    #[test]
    fn non_overlapping_segments_return_unknown_even_at_zero_threshold() {
        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[segment("Speaker 1", 0.0, 5.0)],
            0.0,
            "unit_test",
        );

        assert_unknown(&assignment);
    }

    #[test]
    fn overlap_assignment_never_leaks_segment_identity() {
        let mut overlap = segment("Concrete Speaker", 20.0, 24.0);
        overlap.is_overlap = true;
        overlap.confidence = Some(0.93);

        let assignment = assign_speaker_to_transcript(
            &transcript(Some(20.0), Some(24.0)),
            &[overlap],
            0.1,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id, None);
        assert_eq!(
            assignment.speaker_label.as_deref(),
            Some("Multiple speakers")
        );
        assert!(assignment.is_overlap);
        assert_eq!(assignment.diarization_confidence, Some(0.93));
    }

    #[test]
    fn qualifying_overlap_precedes_longer_ordinary_segment() {
        let primary = segment("Primary Speaker", 10.0, 20.0);
        let mut overlap = segment("Concrete Overlap Speaker", 14.0, 15.0);
        overlap.is_overlap = true;
        overlap.confidence = Some(0.77);

        let assignment = assign_speaker_to_transcript(
            &transcript(Some(10.0), Some(20.0)),
            &[primary, overlap],
            0.1,
            "unit_test",
        );

        assert_eq!(assignment.speaker_id, None);
        assert_eq!(
            assignment.speaker_label.as_deref(),
            Some("Multiple speakers")
        );
        assert!(assignment.is_overlap);
        assert_eq!(assignment.diarization_confidence, Some(0.77));
    }
}
