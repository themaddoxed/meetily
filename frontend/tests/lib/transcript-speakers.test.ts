import { describe, expect, it } from 'vitest';

import { convertTranscriptsToSegments } from '@/hooks/usePaginatedTranscripts';
import type { Transcript } from '@/types';

describe('convertTranscriptsToSegments', () => {
  it('preserves every persisted speaker attribution field without synthesis', () => {
    const transcript: Transcript = {
      id: 'segment-1',
      text: 'Attributed text',
      timestamp: '10:00:00',
      audio_start_time: 4,
      audio_end_time: 6,
      confidence: 0.91,
      speaker_id: 'speaker-1',
      speaker_label: 'Alice',
      speaker_color: '#2563eb',
      is_overlap: false,
      diarization_status: 'final',
      diarization_method: 'sherpa_onnx_offline',
      diarization_confidence: 0.83,
    };

    expect(convertTranscriptsToSegments([transcript])).toEqual([
      {
        id: 'segment-1',
        timestamp: 4,
        endTime: 6,
        text: 'Attributed text',
        confidence: 0.91,
        speaker_id: 'speaker-1',
        speaker_label: 'Alice',
        speaker_color: '#2563eb',
        is_overlap: false,
        diarization_status: 'final',
        diarization_method: 'sherpa_onnx_offline',
        diarization_confidence: 0.83,
      },
    ]);
  });

  it('keeps unknown and overlap values distinct', () => {
    const transcripts: Transcript[] = [
      {
        id: 'unknown',
        text: 'Unknown',
        timestamp: '10:00:00',
        speaker_id: null,
        speaker_label: null,
        is_overlap: false,
      },
      {
        id: 'overlap',
        text: 'Overlap',
        timestamp: '10:00:01',
        speaker_id: null,
        speaker_label: null,
        is_overlap: true,
      },
    ];

    const segments = convertTranscriptsToSegments(transcripts);
    expect(segments[0].is_overlap).toBe(false);
    expect(segments[1].is_overlap).toBe(true);
    expect(segments[0].speaker_id).toBeNull();
    expect(segments[1].speaker_id).toBeNull();
  });
});
