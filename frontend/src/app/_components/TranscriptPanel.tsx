import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { PermissionWarning } from '@/components/PermissionWarning';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, GlobeIcon } from 'lucide-react';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { ModalType } from '@/hooks/useModalState';
import { useIsLinux } from '@/hooks/usePlatform';
import { useMemo } from 'react';
import { useI18n } from '@/contexts/I18nContext';
import type {
  MicrophonePermissionState,
  SystemAudioAccessState,
} from '@/lib/audioPermissions';

/**
 * TranscriptPanel Component
 *
 * Displays transcript content with controls for copying and language settings.
 * Uses TranscriptContext, ConfigContext, and RecordingStateContext internally.
 */

interface TranscriptPanelProps {
  // indicates stop-processing state for transcripts; derived from backend statuses.
  isProcessingStop: boolean;
  isStopping: boolean;
  showModal: (name: ModalType, message?: string) => void;
  microphonePermission: MicrophonePermissionState;
  systemAudioAccess: SystemAudioAccessState;
  checkPermissions: () => Promise<unknown>;
  requestMicrophone: () => Promise<unknown>;
  requestSystemAudio: () => Promise<unknown>;
  isCheckingPermissions: boolean;
  openPermissionSettings: (kind: 'microphone' | 'systemAudio') => Promise<unknown>;
}

export function TranscriptPanel({
  isProcessingStop,
  isStopping,
  showModal,
  microphonePermission,
  systemAudioAccess,
  checkPermissions,
  requestMicrophone,
  requestSystemAudio,
  isCheckingPermissions,
  openPermissionSettings,
}: TranscriptPanelProps) {
  const { t } = useI18n();
  // Contexts
  const { transcripts, transcriptContainerRef, copyTranscript } = useTranscripts();
  const { transcriptModelConfig } = useConfig();
  const { isRecording, isPaused } = useRecordingState();
  const isLinux = useIsLinux();

  // Convert transcripts to segments for virtualized view
  const segments = useMemo(() =>
    transcripts.map(t => ({
      id: t.id,
      timestamp: t.audio_start_time ?? 0,
      endTime: t.audio_end_time,
      text: t.text,
      confidence: t.confidence,
    })),
    [transcripts]
  );

  return (
    <div ref={transcriptContainerRef} className="w-full border-r border-border bg-card flex flex-col overflow-y-auto">
      {/* Title area - Sticky header */}
      <div className="sticky top-0 z-10 bg-card p-4 border-border">
        <div className="flex flex-col space-y-3">
          <div className="flex  flex-col space-y-2">
            <div className="flex justify-center  items-center space-x-2">
              <ButtonGroup>
                {transcripts?.length > 0 && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={copyTranscript}
                    title={t('transcript.copyTitle')}
                  >
                    <Copy />
                    <span className='hidden md:inline'>
                      {t('transcript.copy')}
                    </span>
                  </Button>
                )}
                {transcriptModelConfig.provider === "localWhisper" &&
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => showModal('languageSettings')}
                    title={t('transcript.language')}
                  >
                    <GlobeIcon />
                    <span className='hidden md:inline'>
                      {t('transcript.language')}
                    </span>
                  </Button>
                }
              </ButtonGroup>
            </div>
          </div>
        </div>
      </div>

      {/* Permission Warning - Not needed on Linux */}
      {!isLinux && (!isCheckingPermissions || isRecording) && (
        <div className="flex justify-center px-4 pt-4">
          <PermissionWarning
            microphone={microphonePermission}
            systemAudio={systemAudioAccess}
            onRecheck={checkPermissions}
            requestMicrophone={requestMicrophone}
            requestSystemAudio={requestSystemAudio}
            openSettings={openPermissionSettings}
            isRechecking={isCheckingPermissions}
            isRecording={isRecording || isPaused}
          />
        </div>
      )}

      {/* Transcript content */}
      <div className="pb-20">
        <div className="flex justify-center">
          <div className="w-2/3 max-w-[750px]">
            <VirtualizedTranscriptView
              segments={segments}
              isRecording={isRecording}
              isPaused={isPaused}
              isProcessing={isProcessingStop}
              isStopping={isStopping}
              enableStreaming={isRecording}
              showConfidence={true}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
