import type {
  Block,
  BlockNoteBlock,
  MeetingSummary,
  Section,
  Summary,
} from '@/types';

export interface MeetingSummaryApiResponse {
  status: string;
  meetingName?: string | null;
  meeting_id?: string;
  start?: string | null;
  end?: string | null;
  data?: unknown;
  error?: string | null;
  exportPending?: boolean;
}

const MAX_JSON_DECODE_PASSES = 2;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function decodeSummaryPayload(value: unknown): unknown {
  let decoded = value;

  for (let pass = 0; pass < MAX_JSON_DECODE_PASSES; pass += 1) {
    if (typeof decoded !== 'string') break;

    try {
      decoded = JSON.parse(decoded) as unknown;
    } catch {
      return null;
    }
  }

  return decoded;
}

function normalizeBlockNoteBlock(value: unknown): BlockNoteBlock | null {
  if (
    !isRecord(value) ||
    typeof value.id !== 'string' ||
    typeof value.type !== 'string' ||
    (value.props !== undefined && !isRecord(value.props)) ||
    (value.children !== undefined && !Array.isArray(value.children))
  ) {
    return null;
  }

  const children = value.children?.map(normalizeBlockNoteBlock);
  if (children?.some((child) => child === null)) return null;

  return {
    id: value.id,
    type: value.type,
    ...(value.props === undefined ? {} : { props: value.props }),
    ...(!('content' in value) ? {} : { content: value.content }),
    ...(children === undefined
      ? {}
      : { children: children.filter((child): child is BlockNoteBlock => child !== null) }),
  };
}

function normalizeLegacyBlock(value: unknown): Block | null {
  if (
    !isRecord(value) ||
    typeof value.id !== 'string' ||
    typeof value.type !== 'string' ||
    typeof value.content !== 'string'
  ) {
    return null;
  }

  return {
    id: value.id,
    type: value.type,
    content: value.content.trim(),
    color: 'default',
  };
}

function normalizeLegacySection(key: string, value: unknown): Section | null {
  if (
    !isRecord(value) ||
    (value.title !== undefined && typeof value.title !== 'string') ||
    !Array.isArray(value.blocks)
  ) {
    return null;
  }

  const blocks = value.blocks.map(normalizeLegacyBlock);
  if (blocks.some((block) => block === null)) return null;

  return {
    title: value.title ?? key,
    blocks: blocks.filter((block): block is Block => block !== null),
  };
}

function isMeetingNotesMetadataCandidate(value: unknown): boolean {
  return (
    isRecord(value) &&
    !('title' in value) &&
    !('blocks' in value) &&
    ('meeting_name' in value || 'sections' in value)
  );
}

function isValidMeetingNotesMetadata(value: unknown): boolean {
  if (
    !isRecord(value) ||
    !Object.keys(value).every(
      (key) => key === 'meeting_name' || key === 'sections',
    ) ||
    (value.meeting_name !== undefined && typeof value.meeting_name !== 'string') ||
    !Array.isArray(value.sections)
  ) {
    return false;
  }

  return value.sections.every(
    (section) =>
      isRecord(section) &&
      typeof section.title === 'string' &&
      Object.keys(section).every(
        (key) => key === 'title' || key === 'blocks',
      ) &&
      normalizeLegacySection(section.title, section) !== null,
  );
}

function hasDirectLegacySections(
  value: Record<string, unknown>,
  ignoreMeetingNotesMetadata: boolean,
): boolean {
  return Object.keys(value).some(
    (key) =>
      key !== 'MeetingName' &&
      key !== '_section_order' &&
      !(ignoreMeetingNotesMetadata && key === 'MeetingNotes'),
  );
}

function normalizeLegacySummary(
  value: Record<string, unknown>,
  ignoreMeetingNotesMetadata = false,
): Summary | null {
  if (
    (value.MeetingName !== undefined && typeof value.MeetingName !== 'string') ||
    (value._section_order !== undefined &&
      (!Array.isArray(value._section_order) ||
        !value._section_order.every((key) => typeof key === 'string')))
  ) {
    return null;
  }

  const summaryValues = Object.fromEntries(
    Object.entries(value).filter(
      ([key]) =>
        key !== 'MeetingName' &&
        key !== '_section_order' &&
        !(ignoreMeetingNotesMetadata && key === 'MeetingNotes'),
    ),
  );
  const sectionKeys = Array.isArray(value._section_order)
    ? value._section_order.filter(
        (key) => !(ignoreMeetingNotesMetadata && key === 'MeetingNotes'),
      )
    : Object.keys(summaryValues);

  if (sectionKeys.length === 0) return null;

  const summary: Summary = {};
  for (const key of sectionKeys) {
    const section = normalizeLegacySection(key, summaryValues[key]);
    if (!section) return null;
    summary[key] = section;
  }

  return Object.values(summary).some((section) => section.blocks.length > 0)
    ? summary
    : null;
}

function hasPersistedLegacySections(value: Record<string, unknown>): boolean {
  return isRecord(value.MeetingNotes) && 'sections' in value.MeetingNotes;
}

function normalizePersistedLegacySummary(
  value: Record<string, unknown>,
): Summary | null {
  if (
    typeof value.MeetingName !== 'string' ||
    !Object.keys(value).every(
      (key) => key === 'MeetingName' || key === 'MeetingNotes',
    ) ||
    !isRecord(value.MeetingNotes) ||
    !Object.keys(value.MeetingNotes).every((key) => key === 'sections') ||
    !Array.isArray(value.MeetingNotes.sections) ||
    value.MeetingNotes.sections.length === 0
  ) {
    return null;
  }

  const summary: Summary = {};
  for (const [index, rawSection] of value.MeetingNotes.sections.entries()) {
    if (
      !isRecord(rawSection) ||
      typeof rawSection.title !== 'string' ||
      !Object.keys(rawSection).every(
        (key) => key === 'title' || key === 'blocks',
      )
    ) {
      return null;
    }

    const section = normalizeLegacySection(`section-${index}`, rawSection);
    if (!section) return null;
    summary[`section-${index}`] = section;
  }

  return Object.values(summary).some((section) => section.blocks.length > 0)
    ? summary
    : null;
}

export function normalizeMeetingSummary(value: unknown): MeetingSummary | null {
  const decoded = decodeSummaryPayload(value);
  if (!isRecord(decoded)) return null;

  if (Array.isArray(decoded.summary_json)) {
    const blocks = decoded.summary_json.map(normalizeBlockNoteBlock);
    if (blocks.length === 0 || blocks.some((block) => block === null)) return null;
    if (decoded.markdown !== undefined && typeof decoded.markdown !== 'string') return null;

    return {
      summary_json: blocks.filter((block): block is BlockNoteBlock => block !== null),
      ...(typeof decoded.markdown === 'string' ? { markdown: decoded.markdown } : {}),
    };
  }

  if (typeof decoded.markdown === 'string' && decoded.markdown.trim().length > 0) {
    return {
      markdown: decoded.markdown,
      ...('english_cache' in decoded
        ? { english_cache: decoded.english_cache }
        : {}),
    };
  }

  const meetingNotesMetadata = isMeetingNotesMetadataCandidate(decoded.MeetingNotes);
  if (meetingNotesMetadata) {
    if (!isValidMeetingNotesMetadata(decoded.MeetingNotes)) return null;

    if (hasDirectLegacySections(decoded, true)) {
      return normalizeLegacySummary(decoded, true);
    }

    if (hasPersistedLegacySections(decoded)) {
      return normalizePersistedLegacySummary(decoded);
    }
  }

  return normalizeLegacySummary(decoded);
}

export function readMeetingNameFromSummaryPayload(value: unknown): string | null {
  const decoded = decodeSummaryPayload(value);
  return isRecord(decoded) && typeof decoded.MeetingName === 'string'
    ? decoded.MeetingName
    : null;
}

export function isBlockNoteMeetingSummary(
  value: MeetingSummary,
): value is { summary_json: BlockNoteBlock[]; markdown?: string } {
  return 'summary_json' in value && Array.isArray(value.summary_json);
}

export function isMarkdownMeetingSummary(
  value: MeetingSummary,
): value is { markdown: string; summary_json?: never; english_cache?: unknown } {
  return 'markdown' in value && typeof value.markdown === 'string';
}

export function isLegacyMeetingSummary(value: MeetingSummary): value is Summary {
  return !isBlockNoteMeetingSummary(value) && !isMarkdownMeetingSummary(value);
}

export function buildMeetingSummarySavePayload(
  summary: MeetingSummary,
  meetingTitle: string,
) {
  if (!isLegacyMeetingSummary(summary)) return summary;

  return {
    MeetingName: meetingTitle,
    MeetingNotes: {
      sections: Object.values(summary).map((section) => ({
        title: section.title,
        blocks: section.blocks,
      })),
    },
  };
}
