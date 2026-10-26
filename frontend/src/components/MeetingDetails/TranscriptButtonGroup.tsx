"use client";

import { useState, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, FileDown, FolderOpen, RefreshCw, UsersRound } from 'lucide-react';
import { RetranscribeDialog } from './RetranscribeDialog';
import { useConfig } from '@/contexts/ConfigContext';
import { DiarizationProgress } from '@/hooks/meeting-details/useDiarizationRun';
import { useI18n } from '@/contexts/I18nContext';


interface TranscriptButtonGroupProps {
  transcriptCount: number;
  onCopyTranscript: () => void;
  onExportMeeting: () => Promise<void>;
  isExporting: boolean;
  onOpenMeetingFolder: () => Promise<void>;
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
  isRecording: boolean;
  onRunDiarization: () => Promise<void>;
  isDiarizationRunning: boolean;
  diarizationProgress: DiarizationProgress | null;
}


export function TranscriptButtonGroup({
  transcriptCount,
  onCopyTranscript,
  onExportMeeting,
  isExporting,
  onOpenMeetingFolder,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
  isRecording,
  onRunDiarization,
  isDiarizationRunning,
  diarizationProgress,
}: TranscriptButtonGroupProps) {
  const { t } = useI18n();
  const { betaFeatures } = useConfig();
  const detectSpeakersLabel = t('transcript.detectSpeakers') || 'Detect speakers';
  const [showRetranscribeDialog, setShowRetranscribeDialog] = useState(false);

  const handleRetranscribeComplete = useCallback(async () => {
    // Refetch transcripts to show the updated data
    if (onRefetchTranscripts) {
      await onRefetchTranscripts();
    }
  }, [onRefetchTranscripts]);

  return (
    <div className="flex min-w-0 max-w-full flex-wrap items-center justify-center w-full gap-2">
      <ButtonGroup className="min-w-0 max-w-full flex-wrap justify-center">
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            onCopyTranscript();
          }}
          disabled={transcriptCount === 0}
          title={transcriptCount === 0 ? t('common.noTranscript') : t('transcript.copyTitle')}
        >
          <Copy />
          <span className="hidden lg:inline">{t('common.copy')}</span>
        </Button>

        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            void onRunDiarization();
          }}
          disabled={transcriptCount === 0 || isRecording || isDiarizationRunning || !meetingId}
          aria-busy={isDiarizationRunning}
          title={
            transcriptCount === 0
              ? t('common.noTranscript')
              : isRecording
                ? t('transcript.stopBeforeSpeakers')
                : detectSpeakersLabel
          }
        >
          <UsersRound className={isDiarizationRunning ? 'animate-pulse' : undefined} />
          <span className="hidden xl:inline">
            {isDiarizationRunning
              ? `${diarizationProgress?.progress ?? 0}%`
              : detectSpeakersLabel}
          </span>
        </Button>

        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            void onExportMeeting();
          }}
          disabled={transcriptCount === 0 || isExporting}
          aria-busy={isExporting}
          title={transcriptCount === 0 ? t('common.noTranscript') : t('transcript.exportMeeting')}
        >
          <FileDown className={isExporting ? 'animate-pulse' : undefined} />
          <span className="hidden lg:inline">{t('transcript.export') || <span>Export</span>}</span>
        </Button>

        <Button
          size="sm"
          variant="outline"
          className="xl:px-4"
          onClick={() => {
            onOpenMeetingFolder();
          }}
          title={t('transcript.openRecordingFolder')}
        >
          <FolderOpen className="xl:mr-2" size={18} />
          <span className="hidden lg:inline">{t('transcript.recording')}</span>
        </Button>

        {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
          <Button
            size="sm"
            variant="outline"
            className="bg-gradient-to-r from-info/10 to-info/20 hover:from-info/10 hover:to-info/20 border-info/30 xl:px-4"
            onClick={() => {
              setShowRetranscribeDialog(true);
            }}
            title={t('transcript.retranscribeTitle')}
          >
            <RefreshCw className="xl:mr-2" size={18} />
            <span className="hidden lg:inline">{t('transcript.enhance')}</span>
          </Button>
        )}
      </ButtonGroup>

      {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
        <RetranscribeDialog
          open={showRetranscribeDialog}
          onOpenChange={setShowRetranscribeDialog}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onComplete={handleRetranscribeComplete}
        />
      )}
    </div>
  );
}
