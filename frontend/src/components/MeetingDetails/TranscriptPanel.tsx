"use client";

import { Transcript, TranscriptSegmentData } from '@/types';
import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { TranscriptButtonGroup } from './TranscriptButtonGroup';
import { useMemo } from 'react';
import { convertTranscriptsToSegments } from '@/hooks/usePaginatedTranscripts';
import { DiarizationProgress } from '@/hooks/meeting-details/useDiarizationRun';
import { useI18n } from '@/contexts/I18nContext';

interface TranscriptPanelProps {
  transcripts: Transcript[];
  customPrompt: string;
  onPromptChange: (value: string) => void;
  onCopyTranscript: () => void;
  onExportMeeting: () => Promise<void>;
  isExporting: boolean;
  onOpenMeetingFolder: () => Promise<void>;
  isRecording: boolean;
  onRunDiarization: () => Promise<void>;
  isDiarizationRunning: boolean;
  diarizationProgress: DiarizationProgress | null;
  disableAutoScroll?: boolean;
  activeSegmentId?: string;

  // Optional pagination props (when using virtualization)
  usePagination?: boolean;
  segments?: TranscriptSegmentData[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;

  // Retranscription props
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
}

export function TranscriptPanel({
  transcripts,
  customPrompt,
  onPromptChange,
  onCopyTranscript,
  onExportMeeting,
  isExporting,
  onOpenMeetingFolder,
  isRecording,
  onRunDiarization,
  isDiarizationRunning,
  diarizationProgress,
  disableAutoScroll = false,
  activeSegmentId,
  usePagination = false,
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
}: TranscriptPanelProps) {
  const { t } = useI18n();
  // Convert transcripts to segments if pagination is not used but we want virtualization
  const convertedSegments = useMemo(() => {
    if (usePagination && segments) {
      return segments;
    }
    // Convert transcripts to segments for virtualization
    return convertTranscriptsToSegments(transcripts);
  }, [transcripts, usePagination, segments]);

  return (
    <div className="hidden md:flex md:w-1/4 lg:w-1/3 min-w-0 border-r border-border bg-card flex-col relative shrink-0">
      {/* Title area */}
      <div className="p-4 border-b border-border">
        <TranscriptButtonGroup
          transcriptCount={usePagination ? (totalCount ?? convertedSegments.length) : (transcripts?.length || 0)}
          onCopyTranscript={onCopyTranscript}
          onExportMeeting={onExportMeeting}
          isExporting={isExporting}
          onOpenMeetingFolder={onOpenMeetingFolder}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onRefetchTranscripts={onRefetchTranscripts}
          isRecording={isRecording}
          onRunDiarization={onRunDiarization}
          isDiarizationRunning={isDiarizationRunning}
          diarizationProgress={diarizationProgress}
        />
      </div>

      {/* Transcript content - use virtualized view for better performance */}
      <div className="flex-1 overflow-hidden pb-4">
        <VirtualizedTranscriptView
          segments={convertedSegments}
          isRecording={isRecording}
          isPaused={false}
          isProcessing={false}
          isStopping={false}
          enableStreaming={false}
          showConfidence={true}
          disableAutoScroll={disableAutoScroll}
          activeSegmentId={activeSegmentId}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
          meetingId={meetingId}
          onSpeakerRenamed={onRefetchTranscripts}
        />
      </div>

      {/* Custom prompt input at bottom of transcript section */}
      {!isRecording && convertedSegments.length > 0 && (
        <div className="p-1 border-t border-border">
          <textarea
            placeholder={t('transcript.summaryContext')}
            className="w-full px-3 py-2 border border-border rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-info focus:border-info bg-card shadow-sm min-h-[80px] resize-y"
            value={customPrompt}
            onChange={(e) => onPromptChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
}
