import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

import type { BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { formatMeetingMarkdown } from '@/lib/meeting-export';
import { fetchAllMeetingTranscripts } from '@/lib/meeting-transcripts';
import { summaryToMarkdown } from '@/lib/summary-markdown';
import type { MeetingDetails, MeetingSummary } from '@/types';
import { useI18n } from '@/contexts/I18nContext';

interface UseMeetingExportProps {
  meeting: Pick<MeetingDetails, 'id' | 'title' | 'created_at'>;
  meetingTitle: string;
  aiSummary: MeetingSummary | null;
  blockNoteSummaryRef: RefObject<BlockNoteSummaryViewRef>;
}

interface ExportMeetingOutcome {
  status: 'saved' | 'cancelled';
}

export function useMeetingExport({
  meeting,
  meetingTitle,
  aiSummary,
  blockNoteSummaryRef,
}: UseMeetingExportProps) {
  const { t } = useI18n();
  const tRef = useRef(t);
  const [exportingMeetingId, setExportingMeetingId] = useState<string | null>(null);
  const currentMeetingIdRef = useRef(meeting.id);
  const activeExportMeetingIdRef = useRef<string | null>(null);
  currentMeetingIdRef.current = meeting.id;

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  const handleExportMeeting = useCallback(async () => {
    const requestedMeetingId = meeting.id;
    if (activeExportMeetingIdRef.current === requestedMeetingId) {
      return;
    }

    activeExportMeetingIdRef.current = requestedMeetingId;
    setExportingMeetingId(requestedMeetingId);

    try {
      const transcripts = await fetchAllMeetingTranscripts(requestedMeetingId);
      if (transcripts.length === 0) {
        if (currentMeetingIdRef.current === requestedMeetingId) {
          toast.error(tRef.current('toast.exportNoTranscript'));
        }
        return;
      }

      let summaryMarkdown = '';
      if (blockNoteSummaryRef.current?.getMarkdown) {
        summaryMarkdown = await blockNoteSummaryRef.current.getMarkdown();
      }
      if (!summaryMarkdown.trim()) {
        summaryMarkdown = summaryToMarkdown(aiSummary);
      }

      const title = meetingTitle.trim() || meeting.title;
      const content = formatMeetingMarkdown({
        meeting: { ...meeting, title },
        transcripts,
        summaryMarkdown,
      });
      const outcome = await invoke<ExportMeetingOutcome>('export_meeting_markdown', {
        suggestedName: title,
        content,
      });

      if (outcome.status === 'cancelled') {
        return;
      }
      if (currentMeetingIdRef.current !== requestedMeetingId) {
        return;
      }

      toast.success(tRef.current('toast.exported'));
    } catch (error) {
      console.error('Failed to export meeting:', error);
      if (currentMeetingIdRef.current === requestedMeetingId) {
        toast.error(tRef.current('toast.exportFailed'));
      }
    } finally {
      if (activeExportMeetingIdRef.current === requestedMeetingId) {
        activeExportMeetingIdRef.current = null;
      }
      setExportingMeetingId((activeMeetingId) =>
        activeMeetingId === requestedMeetingId ? null : activeMeetingId,
      );
    }
  }, [aiSummary, blockNoteSummaryRef, meeting, meetingTitle]);

  return {
    handleExportMeeting,
    isExporting: exportingMeetingId === meeting.id,
  };
}
