'use client';

import { useCallback, useEffect, useState } from 'react';
import { Play, Pause, Square, Mic, AlertCircle, X, Tags, Volume2, VolumeX } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useI18n } from '@/contexts/I18nContext';
import {
  contractErrorFrom,
  recordingService,
  type RecordingSourceKind,
  type SourceControl,
} from '@/services/recordingService';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface RecordingControlsProps {
  isRecording: boolean;
  barHeights: string[];
  onRecordingStop: (callApi?: boolean) => void;
  onRecordingStart: () => void;
  onTranscriptionError?: (message: string) => void;
  onStopInitiated?: () => void; // Called immediately when stop button is clicked
  isRecordingDisabled: boolean;
  isParentProcessing: boolean;
  selectedDevices?: {
    micDevice: string | null;
    systemDevice: string | null;
  };
  meetingName?: string;
  transcriptProvider: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';
  meetingVocabulary: string;
  onMeetingVocabularyChange: (vocabulary: string) => void;
}

const DEFAULT_SOURCE_CONTROLS: Record<RecordingSourceKind, SourceControl> = {
  microphone: { muted: false, gain: 1 },
  system: { muted: false, gain: 1 },
};

export const RecordingControls: React.FC<RecordingControlsProps> = ({
  isRecording,
  barHeights,
  onRecordingStop,
  onRecordingStart,
  onTranscriptionError,
  onStopInitiated,
  isRecordingDisabled,
  isParentProcessing,
  selectedDevices,
  meetingName,
  transcriptProvider,
  meetingVocabulary,
  onMeetingVocabularyChange,
}) => {
  // Use global recording state context for pause state (syncs with tray operations)
  const recordingState = useRecordingState();
  const { t } = useI18n();
  const isPaused = recordingState.isPaused;

  const [showPlayback, setShowPlayback] = useState(false);
  const [, setTranscript] = useState<string>('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [isPausing, setIsPausing] = useState(false);
  const [isResuming, setIsResuming] = useState(false);
  const [, setTranscriptionErrors] = useState(0);
  const [isValidatingModel] = useState(false);
  const [, setSpeechDetected] = useState(false);
  const [deviceError, setDeviceError] = useState<{ title: string, message: string } | null>(null);
  const [showVocabularyDialog, setShowVocabularyDialog] = useState(false);
  const [sourceControls, setSourceControls] = useState(DEFAULT_SOURCE_CONTROLS);
  const [sourceControlBusy, setSourceControlBusy] = useState<RecordingSourceKind | null>(null);
  const supportsVocabulary = transcriptProvider === 'localWhisper';

  const currentTime = 0;
  const duration = 0;
  const progress = 0;

  const sourceLabel = useCallback((kind: RecordingSourceKind) => (
    kind === 'microphone' ? t('recording.microphoneSource') : t('recording.systemAudioSource')
  ), [t]);

  const formatTime = (time: number) => {
    const minutes = Math.floor(time / 60);
    const seconds = Math.floor(time % 60);
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  };

  const handleStartRecording = useCallback(async () => {
    if (isStarting || isValidatingModel) return;
    setDeviceError(null);
    setIsStarting(true);
    console.log('Starting recording...');
    console.log('Selected devices:', selectedDevices);
    console.log('Meeting name:', meetingName);
    console.log('Current isRecording state:', isRecording);

    setShowPlayback(false);
    setTranscript(''); // Clear any previous transcript
    setSpeechDetected(false); // Reset speech detection on new recording

    try {
      // Call the validation callback which will:
      // 1. Check if model is ready
      // 2. Show appropriate toast/modal
      // 3. Call backend if valid
      // 4. Update UI state
      await onRecordingStart();
    } catch (error) {
      console.error('Failed to start recording:', error);
      console.error('Error details:', {
        message: error instanceof Error ? error.message : String(error),
        name: error instanceof Error ? error.name : 'Unknown',
        stack: error instanceof Error ? error.stack : undefined
      });

      const contract = contractErrorFrom(error);
      if (contract?.code === 'device_unavailable') {
        setDeviceError({
          title: t('recording.audioDeviceUnavailable'),
          message: t('recording.audioDeviceUnavailableDescription')
        });
      } else if (contract?.code === 'permission_denied') {
        setDeviceError({
          title: t('recording.permissionRequired'),
          message: t('recording.permissionDescription')
        });
      } else {
        setDeviceError({
          title: t('recording.failed'),
          message: t('recording.failedDescription')
        });
      }
    } finally {
      setIsStarting(false);
    }
  }, [onRecordingStart, isStarting, isValidatingModel, selectedDevices, meetingName, isRecording, t]);

  useEffect(() => {
    if (isRecording || recordingState.isRecording) {
      setDeviceError(null);
    }
  }, [isRecording, recordingState.isRecording]);

  useEffect(() => {
    if (!isRecording && !recordingState.isRecording) {
      setSourceControls(DEFAULT_SOURCE_CONTROLS);
      setSourceControlBusy(null);
    }
  }, [isRecording, recordingState.isRecording]);

  const commitSourceControl = useCallback(async (
    kind: RecordingSourceKind,
    control: SourceControl,
  ) => {
    const sessionId = recordingState.sessionId;
    if (!sessionId || isStopping) return;
    setSourceControlBusy(kind);
    try {
      const snapshot = await recordingService.setSourceControl(sessionId, kind, control);
      setSourceControls((current) => ({ ...current, [kind]: control }));
      recordingState.projectSnapshot(snapshot);
    } catch (error) {
      console.error(`Failed to update ${kind} source control:`, error);
      setDeviceError({
        title: t('recording.failed'),
        message: t('recording.sourceControlFailed', { source: sourceLabel(kind) }),
      });
    } finally {
      setSourceControlBusy(null);
    }
  }, [isStopping, recordingState, sourceLabel, t]);

  const renderSourceControl = useCallback((kind: RecordingSourceKind) => {
    const control = sourceControls[kind];
    const label = sourceLabel(kind);
    const isBusy = sourceControlBusy === kind;
    const muteLabel = t(control.muted ? 'recording.unmuteSource' : 'recording.muteSource', {
      source: label,
    });
    return (
      <div key={kind} className="flex items-center gap-2 px-2 py-1" data-source-control={kind}>
        <span className="w-24 truncate text-xs text-muted-foreground" title={label}>{label}</span>
        <button
          type="button"
          aria-label={muteLabel}
          title={muteLabel}
          disabled={isBusy || isStopping}
          onClick={() => void commitSourceControl(kind, { ...control, muted: !control.muted })}
          className="flex h-7 w-7 items-center justify-center rounded-full border border-border text-muted-foreground transition-colors hover:bg-muted/40 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {control.muted ? <VolumeX size={14} /> : <Volume2 size={14} />}
        </button>
        <input
          type="range"
          min="0"
          max="2"
          step="0.05"
          value={control.gain}
          aria-label={t('recording.sourceVolume', { source: label })}
          disabled={isBusy || isStopping}
          onChange={(event) => {
            const gain = Number(event.target.value);
            setSourceControls((current) => ({
              ...current,
              [kind]: { ...current[kind], gain },
            }));
          }}
          onBlur={(event) => {
            const gain = Number(event.currentTarget.value);
            void commitSourceControl(kind, { ...sourceControls[kind], gain });
          }}
          className="w-28 accent-primary"
        />
        <span className="w-10 text-right text-xs tabular-nums text-muted-foreground">
          {Math.round(control.gain * 100)}%
        </span>
      </div>
    );
  }, [commitSourceControl, isStopping, sourceControlBusy, sourceControls, sourceLabel, t]);

  const stopRecordingAction = useCallback(async () => {
    console.log('Executing stop recording...');
    try {
      setIsProcessing(true);
      if (!recordingState.sessionId) return;
      const result = await recordingService.stopRecording(recordingState.sessionId);
      recordingState.projectSnapshot(result);
      console.log('stop_recording command completed successfully:', result);
      // setShowPlayback(true);
      setIsProcessing(false);
      onRecordingStop(true);
    } catch (error) {
      console.error('Failed to stop recording:', error);
      const contract = contractErrorFrom(error);
      if (contract?.code === 'not_found' && !contract.currentState) {
        return;
      }
      if (error instanceof Error) {
        console.error('Error details:', {
          message: error.message,
          name: error.name,
          stack: error.stack,
        });
      }
      setIsProcessing(false);
      onRecordingStop(false);
    } finally {
      setIsStopping(false);
    }
  }, [onRecordingStop, recordingState]);

  const handleStopRecording = useCallback(async () => {
    console.log('handleStopRecording called - isRecording:', isRecording, 'isStarting:', isStarting, 'isStopping:', isStopping);
    if (!isRecording || isStarting || isStopping) {
      console.log('Early return from handleStopRecording due to state check');
      return;
    }

    console.log('Stopping recording...');

    setIsStopping(true);
    onStopInitiated?.();

    // Immediately trigger the stop action
    await stopRecordingAction();
  }, [isRecording, isStarting, isStopping, onStopInitiated, stopRecordingAction]);

  const handlePauseRecording = useCallback(async () => {
    if (!isRecording || isPaused || isPausing) return;

    console.log('Pausing recording...');
    setIsPausing(true);

    try {
      if (!recordingState.sessionId) return;
      recordingState.projectSnapshot(await recordingService.pauseRecording(recordingState.sessionId));
      // isPaused state now managed by RecordingStateContext via events
      console.log('Recording paused successfully');
    } catch (error) {
      console.error('Failed to pause recording:', error);
      alert(t('recording.pauseFailed'));
    } finally {
      setIsPausing(false);
    }
  }, [isRecording, isPaused, isPausing, recordingState, t]);

  const handleResumeRecording = useCallback(async () => {
    if (!isRecording || !isPaused || isResuming) return;

    console.log('Resuming recording...');
    setIsResuming(true);

    try {
      if (!recordingState.sessionId) return;
      recordingState.projectSnapshot(await recordingService.resumeRecording(recordingState.sessionId));
      // isPaused state now managed by RecordingStateContext via events
      console.log('Recording resumed successfully');
    } catch (error) {
      console.error('Failed to resume recording:', error);
      alert(t('recording.resumeFailed'));
    } finally {
      setIsResuming(false);
    }
  }, [isRecording, isPaused, isResuming, recordingState, t]);

  useEffect(() => {
    return () => {
      // Cleanup on unmount if needed
    };
  }, []);

  useEffect(() => {
    console.log('Setting up recording event listeners');
    let unsubscribes: (() => void)[] = [];

    const setupListeners = async () => {
      try {
        // Transcript error listener - handles both regular and actionable errors
        const transcriptErrorUnsubscribe = await listen('transcript-error', (event) => {
          console.log('transcript-error event received:', event);
          console.error('Transcription error received:', event.payload);
          const errorMessage = event.payload as string;

          setTranscriptionErrors(prev => {
            const newCount = prev + 1;
            console.log('Transcription error count incremented:', newCount);
            return newCount;
          });
          setIsProcessing(false);
          console.log('Calling onRecordingStop(false) due to transcript error');
          onRecordingStop(false);
          if (onTranscriptionError) {
            onTranscriptionError(errorMessage);
          }
        });

        // Transcription error listener - handles structured error objects with actionable flag
        const transcriptionErrorUnsubscribe = await listen('transcription-error', (event) => {
          console.log('transcription-error event received:', event);
          console.error('Transcription error received:', event.payload);

          setTranscriptionErrors(prev => {
            const newCount = prev + 1;
            console.log('Transcription error count incremented:', newCount);
            return newCount;
          });
          setIsProcessing(false);
          console.log('Calling onRecordingStop(false) due to transcription error');
          onRecordingStop(false);

          // For actionable errors (like model loading failures), the main page will handle showing the model selector
          // For regular errors, they are handled by useModalState global listener which shows a toast
          // We don't want to show a modal (via onTranscriptionError) AND a toast, so we skip the callback here
          /* if (onTranscriptionError && !isActionable) {
            onTranscriptionError(errorMessage);
          } */
        });

        // Pause/Resume events are now handled by RecordingStateContext
        // No need for duplicate listeners here

        // Speech detected listener - for UX feedback when VAD detects speech
        const speechDetectedUnsubscribe = await listen('speech-detected', (event) => {
          console.log('speech-detected event received:', event);
          setSpeechDetected(true);
        });

        unsubscribes = [
          transcriptErrorUnsubscribe,
          transcriptionErrorUnsubscribe,
          speechDetectedUnsubscribe
        ];
        console.log('Recording event listeners set up successfully');
      } catch (error) {
        console.error('Failed to set up recording event listeners:', error);
      }
    };

    setupListeners();

    return () => {
      console.log('Cleaning up recording event listeners');
      unsubscribes.forEach(unsubscribe => {
        if (unsubscribe && typeof unsubscribe === 'function') {
          unsubscribe();
        }
      });
    };
  }, [onRecordingStop, onTranscriptionError]);

  return (
    <TooltipProvider>
      <div className="flex flex-col space-y-2">
        <div className="flex items-center space-x-2 bg-card rounded-full shadow-lg px-4 py-2">
          {isProcessing && !isParentProcessing ? (
            <div className="flex items-center space-x-2">
              <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-primary"></div>
              <span className="text-sm text-muted-foreground">{t('recording.processing')}</span>
            </div>
          ) : (
            <>
              {showPlayback ? (
                <>
                  <button
                    onClick={handleStartRecording}
                    aria-label={t('recording.start')}
                    className="w-10 h-10 flex items-center justify-center bg-destructive rounded-full text-destructive-foreground hover:bg-destructive/90 transition-colors"
                  >
                    <Mic size={16} />
                  </button>

                  <div className="w-px h-6 bg-muted mx-1" />

                  <div className="flex items-center space-x-1 mx-2">
                    <div className="min-w-[40px] tabular-nums text-sm text-muted-foreground">
                      {formatTime(currentTime)}
                    </div>
                    <div
                      className="relative w-24 h-1 bg-muted rounded-full"
                    >
                      <div
                        className="absolute h-full bg-info rounded-full"
                        style={{ width: `${progress}%` }}
                      />
                    </div>
                    <div className="min-w-[40px] tabular-nums text-sm text-muted-foreground">
                      {formatTime(duration)}
                    </div>
                  </div>

                  <button
                    className="w-10 h-10 flex items-center justify-center bg-muted rounded-full text-muted-foreground cursor-not-allowed"
                    disabled
                  >
                    <Play size={16} />
                  </button>
                </>
              ) : (
                <>
                  {!isRecording ? (
                    // Meeting vocabulary and start recording controls
                    <>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span tabIndex={supportsVocabulary ? undefined : 0}>
                            <button
                              onClick={() => setShowVocabularyDialog(true)}
                              disabled={!supportsVocabulary || isStarting || isProcessing || isRecordingDisabled || isValidatingModel}
                              className="w-10 h-10 flex items-center justify-center rounded-full border border-border bg-card text-muted-foreground hover:bg-muted/40 transition-colors disabled:cursor-not-allowed disabled:opacity-60"
                              aria-label={supportsVocabulary
                                ? t('recording.setVocabulary')
                                : t('recording.vocabularyOnlyWhisper')}
                            >
                              <Tags size={17} />
                            </button>
                          </span>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{supportsVocabulary
                            ? t('recording.setVocabulary')
                            : t('recording.vocabularyOnlyWhisper')}</p>
                        </TooltipContent>
                      </Tooltip>
                      {!supportsVocabulary && (
                        <span className="max-w-[180px] text-xs text-muted-foreground">
                          {t('recording.vocabularyOnlyWhisperShort')}
                        </span>
                      )}

                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            aria-label={t('recording.start')}
                            onClick={() => {
                              handleStartRecording();
                            }}
                            disabled={isStarting || isProcessing || isRecordingDisabled || isValidatingModel}
                            className={`w-12 h-12 flex items-center justify-center ${isStarting || isProcessing || isRecordingDisabled || isValidatingModel ? 'bg-muted text-muted-foreground' : 'bg-destructive text-destructive-foreground hover:bg-destructive/90'
                              } rounded-full transition-colors relative`}
                          >
                            {isStarting || isValidatingModel ? (
                              <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-current"></div>
                            ) : (
                              <Mic size={20} />
                            )}
                          </button>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{t('recording.start')}</p>
                        </TooltipContent>
                      </Tooltip>
                    </>
                  ) : (
                    // Recording controls (pause/resume + stop)
                    <>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            aria-label={isPaused ? t('recording.resume') : t('recording.pause')}
                            onClick={() => {
                              if (isPaused) {
                                handleResumeRecording();
                              } else {
                                handlePauseRecording();
                              }
                            }}
                            disabled={isPausing || isResuming || isStopping}
                            className={`w-10 h-10 flex items-center justify-center ${isPausing || isResuming || isStopping
                              ? 'bg-muted border-2 border-border text-muted-foreground'
                              : 'bg-card border-2 border-border text-muted-foreground hover:border-border hover:bg-muted/40'
                              } rounded-full transition-colors relative`}
                          >
                            {isPaused ? <Play size={16} /> : <Pause size={16} />}
                            {(isPausing || isResuming) && (
                              <div className="absolute -top-8 text-muted-foreground font-medium text-xs">
                                {isPausing ? t('recording.pausing') : t('recording.resuming')}
                              </div>
                            )}
                          </button>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{isPaused ? t('recording.resume') : t('recording.pause')}</p>
                        </TooltipContent>
                      </Tooltip>

                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            aria-label={t('recording.stop')}
                            onClick={() => {
                              handleStopRecording();
                            }}
                            disabled={isStopping || isPausing || isResuming}
                            className={`w-10 h-10 flex items-center justify-center ${isStopping || isPausing || isResuming ? 'bg-muted text-muted-foreground' : 'bg-destructive text-destructive-foreground hover:bg-destructive/90'
                              } rounded-full transition-colors relative`}
                          >
                            <Square size={16} />
                            {isStopping && (
                              <div className="absolute -top-8 text-muted-foreground font-medium text-xs">
                                {t('recording.stopping')}
                              </div>
                            )}
                          </button>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>{t('recording.stop')}</p>
                        </TooltipContent>
                      </Tooltip>
                    </>
                  )}

                  <div className="flex items-center space-x-1 mx-4">
                    {barHeights.map((height, index) => (
                      <div
                        key={index}
                        className={`w-1 rounded-full transition-[height,background-color] duration-200 ${isPaused ? 'bg-warning' : 'bg-destructive'
                          }`}
                        style={{
                          height: isRecording && !isPaused ? height : '4px',
                          opacity: isPaused ? 0.6 : 1,
                        }}
                      />
                    ))}
                  </div>
                </>
              )}
            </>
          )}
        </div>

        {(isRecording || recordingState.isRecording) && (
          <div
            className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 rounded-lg border border-border/60 bg-card/80 px-2 py-1"
            aria-label={t('recording.sourceControls')}
          >
            {renderSourceControl('microphone')}
            {renderSourceControl('system')}
          </div>
        )}

        {/* Show validation status only */}
        {isValidatingModel && (
          <div className="text-xs text-muted-foreground text-center mt-2">
            {t('recording.validating')}
          </div>
        )}

        {/* Device error alert */}
        {deviceError && (
          <Alert variant="destructive" className="mt-4 border-destructive/40 bg-destructive/10">
            <AlertCircle className="h-5 w-5 text-destructive" />
            <button
              onClick={() => setDeviceError(null)}
              className="absolute right-3 top-3 text-destructive hover:text-destructive transition-colors"
              aria-label={t('recording.closeAlert')}
            >
              <X className="h-4 w-4" />
            </button>
            <AlertTitle className="text-destructive font-semibold mb-2">
              {deviceError.title}
            </AlertTitle>
            <AlertDescription className="text-destructive">
              {deviceError.message.split('\n').map((line, i) => (
                <div key={i} className={i > 0 ? 'ml-2' : ''}>
                  {line}
                </div>
              ))}
            </AlertDescription>
          </Alert>
        )}

        {/* {showPlayback && recordingPath && (
        <div className="text-sm text-muted-foreground px-4">
          Recording saved to: {recordingPath}
        </div>
      )} */}

        <Dialog open={supportsVocabulary && showVocabularyDialog} onOpenChange={setShowVocabularyDialog}>
          <DialogContent className="sm:max-w-[460px]">
            <DialogHeader>
              <DialogTitle>{t('recording.vocabulary')}</DialogTitle>
              <DialogDescription>
                {t('recording.vocabularyDescription')}
              </DialogDescription>
            </DialogHeader>
            <Textarea
              value={meetingVocabulary}
              onChange={(event) => onMeetingVocabularyChange(event.target.value)}
              placeholder={t('recording.vocabularyPlaceholder')}
              className="min-h-[110px] resize-y"
            />
            <p className="text-xs text-muted-foreground">
              {t('recording.vocabularySaved')}
            </p>
            <DialogFooter>
              <Button type="button" onClick={() => setShowVocabularyDialog(false)}>
                {t('common.done')}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  );
};
