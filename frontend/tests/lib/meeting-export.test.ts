import { describe, expect, it } from 'vitest';

import { formatMeetingMarkdown, formatRecordingTime } from '@/lib/meeting-export';
import { summaryToMarkdown } from '@/lib/summary-markdown';

describe('formatRecordingTime', () => {
  it('uses recording-relative time when available', () => {
    expect(formatRecordingTime(65.9, '14:03:01')).toBe('[01:05]');
  });

  it('falls back to the persisted wall-clock timestamp', () => {
    expect(formatRecordingTime(undefined, '14:03:01')).toBe('14:03:01');
  });
});

describe('formatMeetingMarkdown', () => {
  it('exports ordered Unicode transcript rows, speakers, overlap, and summary', () => {
    const markdown = formatMeetingMarkdown({
      meeting: {
        id: 'meeting-1',
        title: 'План\nрелиза',
        created_at: '2026-07-26T08:00:00.000Z',
      },
      transcripts: [
        {
          id: 'later',
          text: 'Вторая реплика',
          timestamp: '11:00:05',
          audio_start_time: 5.2,
          speaker_id: 'speaker-2',
          speaker_label: 'Борис',
          speaker_color: '#22c55e',
          is_overlap: false,
          diarization_status: 'final',
        },
        {
          id: 'first',
          text: 'Первая реплика',
          timestamp: '11:00:01',
          audio_start_time: 1.8,
          speaker_id: 'speaker-1',
          speaker_label: 'Алиса',
          speaker_color: '#2563eb',
          is_overlap: false,
          diarization_status: 'final',
        },
        {
          id: 'overlap',
          text: 'Перебивают друг друга',
          timestamp: '11:00:09',
          audio_start_time: 9,
          speaker_id: null,
          speaker_label: null,
          speaker_color: null,
          is_overlap: true,
          diarization_status: 'final',
        },
      ],
      summaryMarkdown: '### Решение\n\nВыпустить после smoke.',
    });

    expect(markdown).toBe(
      '# Meeting: План релиза\n\n' +
        '**Meeting ID:** meeting-1\n' +
        '**Date:** 2026-07-26T08:00:00.000Z\n\n' +
        '## Transcript\n\n' +
        '[00:01] **Алиса:** Первая реплика\n\n' +
        '[00:05] **Борис:** Вторая реплика\n\n' +
        '[00:09] **Multiple speakers:** Перебивают друг друга\n\n' +
        '## Summary\n\n' +
        '### Решение\n\nВыпустить после smoke.\n',
    );
  });

  it('exports a valid transcript-only document when no summary exists', () => {
    const markdown = formatMeetingMarkdown({
      meeting: {
        id: 'meeting-2',
        title: 'No summary',
        created_at: '2026-07-26T09:00:00.000Z',
      },
      transcripts: [
        {
          id: 'segment-1',
          text: 'Plain text',
          timestamp: '12:00:00',
        },
      ],
      summaryMarkdown: '   ',
    });

    expect(markdown).toContain('## Transcript\n\n12:00:00 Plain text\n');
    expect(markdown).not.toContain('## Summary');
  });
});

describe('summaryToMarkdown', () => {
  it('preserves stored Markdown and legacy summary sections', () => {
    expect(summaryToMarkdown({ markdown: '## Stored\n\nDecision' })).toBe(
      '## Stored\n\nDecision',
    );
    expect(
      summaryToMarkdown({
        decisions: {
          title: 'Decisions',
          blocks: [
            { id: 'block-1', type: 'bullet', content: 'Ship it', color: 'default' },
          ],
        },
      }),
    ).toBe('## Decisions\n\n- Ship it');
  });
});
