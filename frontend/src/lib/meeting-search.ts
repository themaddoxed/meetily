export type MeetingSearchSource = 'title' | 'transcript' | 'summary';

export interface MeetingSearchResult {
  id: string;
  title: string;
  matchContext: string;
  timestamp: string;
  sourceType: MeetingSearchSource;
  sourceId?: string;
  audioStartTime?: number;
}

/**
 * Keep one useful result per meeting for the sidebar preview.
 *
 * A query can match a meeting title and one or more transcript rows at the
 * same time. Transcript rows carry the only exact source anchor, so they must
 * win over title/summary rows regardless of the backend insertion order.
 */
export function groupMeetingSearchResults(
  results: readonly MeetingSearchResult[],
): Map<string, MeetingSearchResult> {
  const grouped = new Map<string, MeetingSearchResult>();

  for (const result of results) {
    const current = grouped.get(result.id);
    if (!current || searchResultPriority(result) < searchResultPriority(current)) {
      grouped.set(result.id, result);
    }
  }

  return grouped;
}

function searchResultPriority(result: MeetingSearchResult): number {
  switch (result.sourceType) {
    case 'transcript':
      return 0;
    case 'summary':
      return 1;
    case 'title':
      return 2;
  }
}

/** Build a stable deep link from a search result to its visible source. */
export function buildMeetingSearchHref(result: MeetingSearchResult): string {
  const params = new URLSearchParams({
    id: result.id,
    source: 'search',
  });

  if (result.sourceType === 'transcript' && result.sourceId) {
    params.set('transcriptId', result.sourceId);
  }

  if (
    result.sourceType === 'transcript' &&
    typeof result.audioStartTime === 'number' &&
    Number.isFinite(result.audioStartTime)
  ) {
    params.set('at', String(result.audioStartTime));
  }

  return `/meeting-details?${params.toString()}`;
}

export function formatMeetingSearchTime(result: MeetingSearchResult): string {
  if (
    result.sourceType === 'transcript' &&
    typeof result.audioStartTime === 'number' &&
    Number.isFinite(result.audioStartTime)
  ) {
    const totalSeconds = Math.max(0, Math.floor(result.audioStartTime));
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `[${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}]`;
  }

  return result.timestamp;
}

export function meetingSearchSourceLabel(source: MeetingSearchSource): string {
  switch (source) {
    case 'title':
      return 'Meeting';
    case 'summary':
      return 'Summary';
    case 'transcript':
      return 'Transcript';
  }
}
