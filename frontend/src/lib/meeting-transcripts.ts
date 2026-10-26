import { invoke } from '@tauri-apps/api/core';

import type { PaginatedTranscriptsResponse, Transcript } from '@/types';

export async function fetchAllMeetingTranscripts(meetingId: string): Promise<Transcript[]> {
  const firstPage = await invoke<PaginatedTranscriptsResponse>('api_get_meeting_transcripts', {
    meetingId,
    limit: 1,
    offset: 0,
  });

  if (firstPage.total_count === 0) {
    return [];
  }

  const completePage = await invoke<PaginatedTranscriptsResponse>(
    'api_get_meeting_transcripts',
    {
      meetingId,
      limit: firstPage.total_count,
      offset: 0,
    },
  );

  return completePage.transcripts;
}
