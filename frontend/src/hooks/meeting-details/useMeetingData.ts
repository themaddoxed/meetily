import { useState, useCallback, useRef, useEffect } from 'react';
import { Block, MeetingDetails, MeetingSummary, Section, Summary } from '@/types';
import { BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { CurrentMeeting, useSidebar } from '@/components/Sidebar/SidebarProvider';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { buildMeetingSummarySavePayload } from '@/lib/meeting-summary';
import { useI18n } from '@/contexts/I18nContext';

interface UseMeetingDataProps {
  meeting: Pick<MeetingDetails, 'id' | 'title' | 'transcripts'>;
  summaryData: MeetingSummary | null;
  onMeetingUpdated?: () => Promise<void>;
}

function isBlock(value: unknown): value is Block {
  return (
    value !== null &&
    typeof value === 'object' &&
    'id' in value &&
    typeof value.id === 'string' &&
    'type' in value &&
    typeof value.type === 'string' &&
    'content' in value &&
    typeof value.content === 'string' &&
    'color' in value &&
    typeof value.color === 'string'
  );
}

function isSection(value: unknown): value is Section {
  return (
    value !== null &&
    typeof value === 'object' &&
    'title' in value &&
    typeof value.title === 'string' &&
    'blocks' in value &&
    Array.isArray(value.blocks) &&
    value.blocks.every(isBlock)
  );
}

function isLegacySummary(value: MeetingSummary): value is Summary {
  return Object.values(value).every(isSection);
}

export function useMeetingData({ meeting, summaryData }: UseMeetingDataProps) {
  const { t } = useI18n();
  const tRef = useRef(t);
  // State
  // Use prop directly since summary generation fetches transcripts independently
  const transcripts = meeting.transcripts;
  const [meetingTitle, setMeetingTitle] = useState(meeting.title || '+ New Call');
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [isTitleDirty, setIsTitleDirty] = useState(false);
  const isTitleDirtyRef = useRef(false);
  const [aiSummary, setAiSummary] = useState<MeetingSummary | null>(summaryData);
  const [isSaving, setIsSaving] = useState(false);
  const [, setIsSummaryDirty] = useState(false);
  const [, setError] = useState<string>('');

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  // Ref for BlockNoteSummaryView
  const blockNoteSummaryRef = useRef<BlockNoteSummaryViewRef>(null);

  // Sidebar context
  const {
    setCurrentMeeting,
    setMeetings,
    meetings: sidebarMeetings,
    currentMeeting,
  } = useSidebar();
  const currentMeetingIdRef = useRef(currentMeeting?.id);
  const previousMeetingIdRef = useRef(meeting.id);

  useEffect(() => {
    currentMeetingIdRef.current = currentMeeting?.id;
  }, [currentMeeting?.id]);

  useEffect(() => {
    const meetingChanged = previousMeetingIdRef.current !== meeting.id;
    if (meetingChanged || !isTitleDirty) {
      setMeetingTitle(meeting.title || '+ New Call');
    }
    if (meetingChanged) {
      isTitleDirtyRef.current = false;
      setIsTitleDirty(false);
      previousMeetingIdRef.current = meeting.id;
    }
  }, [isTitleDirty, meeting.id, meeting.title]);

  // Sync aiSummary state when summaryData prop changes (fixes display of fetched summaries)
  useEffect(() => {
    console.log('[useMeetingData] Syncing summary data from prop:', summaryData ? 'present' : 'null');
    setAiSummary(summaryData);
  }, [summaryData]); // Only trigger when parent prop changes, not when aiSummary changes

  // Handlers
  const handleTitleChange = useCallback((newTitle: string) => {
    setMeetingTitle(newTitle);
    isTitleDirtyRef.current = true;
    setIsTitleDirty(true);
  }, []);

  const handleSummaryChange = useCallback((newSummary: Summary) => {
    setAiSummary(newSummary);
  }, []);

  const handleSaveMeetingTitle = useCallback(async () => {
    try {
      await invokeTauri('api_save_meeting_title', {
        meetingId: meeting.id,
        title: meetingTitle,
      });

      console.log('Save meeting title success');
      isTitleDirtyRef.current = false;
      setIsTitleDirty(false);

      if (currentMeetingIdRef.current !== meeting.id) {
        return true;
      }

      // Update meetings with new title
      const updatedMeetings = sidebarMeetings.map((m: CurrentMeeting) =>
        m.id === meeting.id ? { id: m.id, title: meetingTitle } : m
      );
      setMeetings(updatedMeetings);
      setCurrentMeeting({ id: meeting.id, title: meetingTitle });
      return true;
    } catch (error) {
      console.error('Failed to save meeting title:', error);
      if (error instanceof Error) {
        setError(error.message);
      } else {
        setError(tRef.current('toast.meetingTitleUpdateFailed'));
      }
      return false;
    }
  }, [meeting.id, meetingTitle, sidebarMeetings, setMeetings, setCurrentMeeting]);

  const handleSaveSummary = useCallback(async (summary: MeetingSummary) => {
    console.log('📄 handleSaveSummary called with:', {
      hasMarkdown: 'markdown' in summary,
      hasSummaryJson: 'summary_json' in summary,
      summaryKeys: Object.keys(summary)
    });

    try {
      // Check if it's the new BlockNote format
      const isBlockNoteFormat = !isLegacySummary(summary);
      console.log(
        isBlockNoteFormat
          ? '📄 Saving new format (markdown/blocknote)'
          : '📄 Saving legacy format'
      );

      const formattedSummary = buildMeetingSummarySavePayload(summary, meetingTitle);

      await invokeTauri('api_save_meeting_summary', {
        meetingId: meeting.id,
        summary: formattedSummary,
      });

      console.log('✅ Save meeting summary success');
    } catch (error) {
      console.error('❌ Failed to save meeting summary:', error);
      if (error instanceof Error) {
        setError(error.message);
      } else {
        setError(tRef.current('toast.saveChangesFailed'));
      }
    }
  }, [meeting.id, meetingTitle]);

  const saveAllChanges = useCallback(async () => {
    setIsSaving(true);
    try {
      // Save meeting title only if changed
      if (isTitleDirty) {
        await handleSaveMeetingTitle();
      }

      // Save BlockNote editor changes if dirty
      if (blockNoteSummaryRef.current?.isDirty) {
        console.log('💾 Saving BlockNote editor changes...');
        await blockNoteSummaryRef.current.saveSummary();
      } else if (aiSummary) {
        await handleSaveSummary(aiSummary);
      }

      toast.success(tRef.current('toast.changesSaved'));
    } catch (error) {
      console.error('Failed to save changes:', error);
      toast.error(tRef.current('toast.saveChangesFailed'), { description: String(error) });
    } finally {
      setIsSaving(false);
    }
  }, [isTitleDirty, handleSaveMeetingTitle, aiSummary, handleSaveSummary]);

  // Update meeting title from external source (e.g., AI summary)
  const updateMeetingTitle = useCallback((newTitle: string) => {
    // A summary can finish after the user starts editing the title. Keep the
    // in-progress manual value until the explicit save completes.
    if (isTitleDirtyRef.current) {
      return;
    }
    console.log('📝 Updating meeting title to:', newTitle);
    setMeetingTitle(newTitle);
    const updatedMeetings = sidebarMeetings.map((m: CurrentMeeting) =>
      m.id === meeting.id ? { id: m.id, title: newTitle } : m
    );
    setMeetings(updatedMeetings);
    setCurrentMeeting({ id: meeting.id, title: newTitle });
  }, [meeting.id, sidebarMeetings, setMeetings, setCurrentMeeting]);

  return {
    // State
    transcripts,
    meetingTitle,
    isEditingTitle,
    isTitleDirty,
    aiSummary,
    isSaving,
    blockNoteSummaryRef,

    // Setters
    setMeetingTitle,
    setIsEditingTitle,
    setAiSummary,
    setIsSummaryDirty,

    // Handlers
    handleTitleChange,
    handleSummaryChange,
    handleSaveSummary,
    handleSaveMeetingTitle,
    saveAllChanges,
    updateMeetingTitle,
  };
}
