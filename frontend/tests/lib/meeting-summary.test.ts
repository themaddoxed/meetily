import { describe, expect, test } from 'vitest';

import {
  buildMeetingSummarySavePayload,
  normalizeMeetingSummary,
} from '@/lib/meeting-summary';

const blockNoteBlock = {
  id: 'block-1',
  type: 'paragraph',
  props: {
    backgroundColor: 'default',
    textAlignment: 'left',
    textColor: 'default',
  },
  content: [{ type: 'text', text: 'Decision', styles: {} }],
  children: [],
};

describe('normalizeMeetingSummary', () => {
  test('normalizes a valid legacy summary without mutating its source', () => {
    const input = {
      MeetingName: 'Planning',
      _section_order: ['Decisions'],
      Decisions: {
        title: 'Decisions',
        blocks: [
          {
            id: 'decision-1',
            type: 'bullet',
            content: '  Ship the release  ',
            color: 'accent',
          },
        ],
      },
    };
    const original = structuredClone(input);

    expect(normalizeMeetingSummary(input)).toEqual({
      Decisions: {
        title: 'Decisions',
        blocks: [
          {
            id: 'decision-1',
            type: 'bullet',
            content: 'Ship the release',
            color: 'default',
          },
        ],
      },
    });
    expect(input).toEqual(original);
  });

  test.each([
    [{ markdown: '# Release\nReady.' }, { markdown: '# Release\nReady.' }],
    [
      { markdown: '# Release', summary_json: [blockNoteBlock] },
      { markdown: '# Release', summary_json: [blockNoteBlock] },
    ],
    [
      { summary_json: [blockNoteBlock] },
      { summary_json: [blockNoteBlock] },
    ],
  ])('normalizes markdown and BlockNote payload %#', (input, expected) => {
    expect(normalizeMeetingSummary(input)).toEqual(expected);
  });

  test('decodes single- and double-encoded JSON payloads', () => {
    const summary = { markdown: '# Encoded summary' };

    expect(normalizeMeetingSummary(JSON.stringify(summary))).toEqual(summary);
    expect(normalizeMeetingSummary(JSON.stringify(JSON.stringify(summary)))).toEqual(summary);
  });

  test('preserves the opaque English summary cache for an unchanged save', () => {
    const input = {
      markdown: '# Release summary',
      english_cache: {
        markdown: '# Release summary',
        source: {
          transcript_hash: 'sha256:abc123',
          template_id: 'default',
        },
        output_language: 'en',
      },
    };

    const normalized = normalizeMeetingSummary(input);

    expect(normalized).toEqual(input);
    expect(normalized).not.toBeNull();
    if (!normalized) throw new Error('Expected a normalized summary');

    const savePayload = buildMeetingSummarySavePayload(normalized, 'Release Meeting');
    expect(JSON.stringify(savePayload)).toBe(JSON.stringify(input));
  });

  test('round-trips the persisted legacy MeetingNotes sections shape', () => {
    const summary = {
      Overview: {
        title: 'Overview',
        blocks: [
          {
            id: 'overview-1',
            type: 'paragraph',
            content: 'Release scope is approved.',
            color: 'default',
          },
        ],
      },
      Decisions: {
        title: 'Decisions',
        blocks: [
          {
            id: 'decision-1',
            type: 'bullet',
            content: 'Ship on Friday.',
            color: 'default',
          },
        ],
      },
    };

    const persisted = buildMeetingSummarySavePayload(summary, 'Release Meeting');
    const reloaded = normalizeMeetingSummary(persisted);

    expect(reloaded).not.toBeNull();
    if (!reloaded || 'markdown' in reloaded || 'summary_json' in reloaded) {
      throw new Error('Expected a reloaded legacy summary');
    }

    expect(Object.values(reloaded).map((section) => ({
      title: section.title,
      blocks: section.blocks.map(({ id, type, content }) => ({ id, type, content })),
    }))).toEqual(Object.values(summary).map((section) => ({
      title: section.title,
      blocks: section.blocks.map(({ id, type, content }) => ({ id, type, content })),
    })));
  });

  test('rejects a malformed persisted legacy MeetingNotes payload', () => {
    expect(normalizeMeetingSummary({
      MeetingName: 'Broken Meeting',
      MeetingNotes: {
        sections: [
          {
            title: 'Missing blocks',
          },
        ],
      },
    })).toBeNull();
  });

  test('preserves direct historical sections and ignores MeetingNotes metadata', () => {
    const input = {
      MeetingName: 'Historical Meeting',
      People: {
        title: 'People',
        blocks: [
          {
            id: 'person-1',
            type: 'bullet',
            content: 'Ada owns the release.',
            color: 'default',
          },
        ],
      },
      SessionSummary: {
        title: 'Session Summary',
        blocks: [
          {
            id: 'summary-1',
            type: 'paragraph',
            content: 'The release is ready.',
            color: 'default',
          },
        ],
      },
      MeetingNotes: {
        meeting_name: 'Historical Meeting',
        sections: [
          {
            title: 'People',
            blocks: [
              {
                id: 'duplicate-person-1',
                type: 'bullet',
                content: 'Duplicated metadata section.',
                color: 'default',
              },
            ],
          },
        ],
      },
    };

    expect(normalizeMeetingSummary(input)).toEqual({
      People: input.People,
      SessionSummary: input.SessionSummary,
    });
  });

  test('does not ignore a genuine top-level MeetingNotes section', () => {
    const meetingNotes = {
      title: 'Meeting Notes',
      blocks: [
        {
          id: 'note-1',
          type: 'paragraph',
          content: 'Keep this direct section.',
          color: 'default',
        },
      ],
    };

    expect(normalizeMeetingSummary({ MeetingNotes: meetingNotes })).toEqual({
      MeetingNotes: meetingNotes,
    });
  });

  test('rejects malformed MeetingNotes metadata beside valid direct sections', () => {
    expect(normalizeMeetingSummary({
      People: {
        title: 'People',
        blocks: [
          {
            id: 'person-1',
            type: 'bullet',
            content: 'Ada',
            color: 'default',
          },
        ],
      },
      MeetingNotes: {
        meeting_name: 'Broken metadata',
        sections: [{ title: 'Missing blocks' }],
      },
    })).toBeNull();
  });

  test.each([
    null,
    undefined,
    '',
    'not JSON',
    {},
    { markdown: 42 },
    { summary_json: [{ id: 'missing-type' }] },
    {
      Decisions: {
        title: 'Decisions',
        blocks: [{ id: 'missing-content', type: 'bullet' }],
      },
    },
  ])('rejects malformed input without changing it: %#', (input) => {
    const original = typeof input === 'object' && input !== null
      ? structuredClone(input)
      : input;

    expect(normalizeMeetingSummary(input)).toBeNull();
    expect(input).toEqual(original);
  });
});
