import { useState, useCallback, useEffect, useRef } from 'react';
import {
  MeetingDetails,
  MeetingSummary,
  Transcript,
} from '@/types';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import type { OllamaModel } from '@/contexts/ConfigContext';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { isOllamaNotInstalledError } from '@/lib/utils';
import { BuiltInModelInfo } from '@/lib/builtin-ai';
import {
  detectAndCacheSummaryLanguage,
  readMeetingSummaryLanguage,
  readCachedDetectedSummaryLanguage,
} from '@/lib/summary-language-preferences';
import {
  type MeetingSummaryApiResponse,
  normalizeMeetingSummary,
} from '@/lib/meeting-summary';
import { useI18n } from '@/contexts/I18nContext';

async function resolveSummaryLanguage(
  meetingId: string,
  transcriptTexts: string[],
  translate: ReturnType<typeof useI18n>['t'],
): Promise<string | null> {
  try {
    const perMeeting = await readMeetingSummaryLanguage(meetingId);
    if (perMeeting.language) return perMeeting.language;
  } catch (err) {
    console.warn('Failed to load meeting summary language:', err);
    toast.warning(translate('summary.languageLoadFailed'), {
      description: translate('summary.autoUntilMetadata'),
    });
  }

  try {
    const cachedDetected = await readCachedDetectedSummaryLanguage(meetingId);
    if (cachedDetected) return cachedDetected;
  } catch (err) {
    console.warn('Failed to load cached detected summary language:', err);
  }

  try {
    const detection = await detectAndCacheSummaryLanguage(meetingId, transcriptTexts);
    if (detection.reason === 'tie') {
      toast.warning(translate('summary.bilingualTranscript'), {
        description: translate('summary.bilingualTranscriptDescription'),
      });
    }
    return detection.language;
  } catch (err) {
    console.warn('Failed to detect transcript summary language:', err);
    return null;
  }
}

type SummaryStatus = 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';

interface ProcessTranscriptResponse {
  message: string;
  process_id: string;
}

interface UseSummaryGenerationProps {
  meeting: Pick<MeetingDetails, 'id' | 'created_at'>;
  transcripts: Transcript[];
  modelConfig: ModelConfig;
  isModelConfigLoading: boolean;
  isTemplateSelectionReady?: boolean;
  selectedTemplate: string;
  onMeetingUpdated?: () => Promise<void>;
  updateMeetingTitle: (title: string) => void;
  setAiSummary: (summary: MeetingSummary | null) => void;
  onOpenModelSettings?: () => void;
}

export function useSummaryGeneration({
  meeting,
  modelConfig,
  isModelConfigLoading,
  isTemplateSelectionReady = true,
  selectedTemplate,
  onMeetingUpdated,
  updateMeetingTitle,
  setAiSummary,
  onOpenModelSettings,
}: UseSummaryGenerationProps) {
  const [summaryStatus, setSummaryStatus] = useState<SummaryStatus>('idle');
  const [summaryError, setSummaryError] = useState<string | null>(null);
  const { t } = useI18n();
  const tRef = useRef(t);

  const { startSummaryPolling, stopSummaryPolling, currentMeeting } = useSidebar();
  const currentMeetingIdRef = useRef(currentMeeting?.id);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  useEffect(() => {
    currentMeetingIdRef.current = currentMeeting?.id;
  }, [currentMeeting?.id]);

  // Helper to get status message
  const getSummaryStatusMessage = useCallback((status: SummaryStatus) => {
    switch (status) {
      case 'processing':
        return tRef.current('summary.processingTranscript');
      case 'summarizing':
        return tRef.current('summary.generatingToast');
      case 'regenerating':
        return tRef.current('summary.regeneratingToast');
      case 'completed':
        return tRef.current('summary.completed');
      case 'error':
        return tRef.current('summary.errorGenerating');
      default:
        return '';
    }
  }, []);

  // Unified summary processing logic
  const processSummary = useCallback(async ({
    transcriptText,
    transcriptTexts,
    customPrompt = '',
    isRegeneration = false,
  }: {
    transcriptText: string;
    transcriptTexts?: string[];
    customPrompt?: string;
    isRegeneration?: boolean;
  }) => {
    setSummaryStatus(isRegeneration ? 'regenerating' : 'processing');
    setSummaryError(null);

    try {
      if (!transcriptText.trim()) {
        throw new Error(tRef.current('summary.noTranscriptText'));
      }

      console.log('Processing transcript with template:', selectedTemplate);

      // Show toast notification for generation start
      toast.info(tRef.current(isRegeneration ? 'summary.regeneratingToast' : 'summary.generatingToast'), {
        description: tRef.current('summary.usingModel', {
          provider: modelConfig.provider,
          model: modelConfig.model,
        }),
        duration: 3000,
      });

      // Resolve explicit metadata override first; Auto detects the transcript language.
      const summaryLanguage = await resolveSummaryLanguage(
        meeting.id,
        transcriptTexts?.length ? transcriptTexts : [transcriptText],
        tRef.current,
      );

      // Process transcript and get process_id
      const result = await invokeTauri<ProcessTranscriptResponse>('api_process_transcript', {
        text: transcriptText,
        model: modelConfig.provider,
        modelName: modelConfig.model,
        meetingId: meeting.id,
        chunkSize: 40000,
        overlap: 1000,
        customPrompt: customPrompt,
        systemPrompt: modelConfig.summarySystemPrompt || null,
        templateId: selectedTemplate,
        summaryLanguage,
      });

      const process_id = result.process_id;
      console.log('Process ID:', process_id);

      // Start global polling via context
      startSummaryPolling(meeting.id, process_id, async (pollingResult: MeetingSummaryApiResponse) => {
        console.log('Summary status:', pollingResult);

        // Handle cancellation
        if (pollingResult.status === 'cancelled') {
          console.log('Summary generation was cancelled');

          // Reload summary from database (backend has already restored from backup)
          try {
            const existingSummary = await invokeTauri<MeetingSummaryApiResponse>('api_get_summary', {
              meetingId: meeting.id
            });
            const restoredSummary = normalizeMeetingSummary(existingSummary.data);

            if (restoredSummary) {
              console.log('Restored previous summary after cancellation');
              setAiSummary(restoredSummary);
              setSummaryStatus('completed');
            } else {
              setSummaryStatus('idle');
            }
          } catch (error) {
            console.error('Failed to reload summary after cancellation:', error);
            setSummaryStatus('idle');
          }

          setSummaryError(null);
          return;
        }

        // Handle errors
        if (pollingResult.status === 'error' || pollingResult.status === 'failed') {
          console.error('Backend returned error:', pollingResult.error);
          const errorMessage = pollingResult.error || `Summary ${isRegeneration ? 'regeneration' : 'generation'} failed`;

          // If this was a regeneration, try to restore previous summary from database
          if (isRegeneration) {
            try {
              const existingSummary = await invokeTauri<MeetingSummaryApiResponse>('api_get_summary', {
                meetingId: meeting.id
              });
              const restoredSummary = normalizeMeetingSummary(existingSummary.data);

              if (restoredSummary) {
                console.log('Restored previous summary after regeneration failure');
                setAiSummary(restoredSummary);
                setSummaryStatus('completed');
                setSummaryError(null);

                // Show error toast with restoration message
                toast.error(tRef.current('summary.regenerationFailed'), {
                  description: `${errorMessage}. ${tRef.current('summary.previousSummaryRestored')}`,
                });

                return;
              }
            } catch (error) {
              console.error('Failed to reload summary after error:', error);
            }
          }

          // Continue with normal error handling if not regeneration or reload failed
          setSummaryError(errorMessage);
          setSummaryStatus('error');

          // Check if this is a "model is required" error
          const isModelRequiredError = errorMessage.includes('model is required') ||
            errorMessage.includes('"model":"required"') ||
            errorMessage.toLowerCase().includes('model') && errorMessage.toLowerCase().includes('required');

          // Show error toast
          toast.error(tRef.current(isRegeneration ? 'summary.regenerationFailed' : 'summary.generationFailed'), {
            description: errorMessage.includes('Connection refused')
              ? tRef.current('summary.connectionFailedDescription')
              : errorMessage,
          });

          // Auto-open model settings modal if model is missing
          if (isModelRequiredError && onOpenModelSettings) {
            console.log('🔧 Model required error detected, opening model settings...');
            onOpenModelSettings();
          }

          return;
        }

        // Handle successful completion
        if (pollingResult.status === 'completed' && pollingResult.data !== undefined) {
          console.log('Summary generation completed:', pollingResult.data);

          const normalizedSummary = normalizeMeetingSummary(pollingResult.data);
          if (!normalizedSummary) {
            console.error('Summary completed with malformed or empty content');
            setSummaryError(tRef.current('summary.invalidContent'));
            setSummaryStatus('error');

            return;
          }

          // The summary payload is stale as soon as a user saves a manual
          // title. Read the backend row after completion, where the atomic
          // manual-title guard is the source of truth, and update only the
          // meeting that is still open in the UI.
          let authoritativeMeetingTitle: string | null = null;
          try {
            const metadata = await invokeTauri<{ title: string }>('api_get_meeting_metadata', {
              meetingId: meeting.id,
            });
            authoritativeMeetingTitle = metadata.title;
            if (metadata.title && currentMeetingIdRef.current === meeting.id) {
              updateMeetingTitle(metadata.title);
            }
          } catch (error) {
            console.warn('Could not reload authoritative meeting title:', error);
          }

          setAiSummary(normalizedSummary);
          setSummaryStatus('completed');

          // Show success toast
          toast.success(tRef.current('summary.generated'), {
            description: tRef.current('summary.summaryReady'),
            duration: 4000,
          });

          if (authoritativeMeetingTitle && onMeetingUpdated && currentMeetingIdRef.current === meeting.id) {
            await onMeetingUpdated();
          }
        }
      });
      return true;
    } catch (error) {
      console.error(`Failed to ${isRegeneration ? 'regenerate' : 'generate'} summary:`, error);
      const errorMessage = error instanceof Error ? error.message : tRef.current('model.unknownError');
      setSummaryError(errorMessage);
      setSummaryStatus('error');
      // Note: We don't clear the summary here because the backend has already restored from backup

      toast.error(tRef.current(isRegeneration ? 'summary.regenerationFailed' : 'summary.generationFailed'), {
        description: errorMessage,
      });

      return false;
    }
  }, [
    meeting.id,
    modelConfig,
    selectedTemplate,
    startSummaryPolling,
    setAiSummary,
    updateMeetingTitle,
    onMeetingUpdated,
    onOpenModelSettings,
  ]);

  // Helper function to fetch ALL transcripts for summary generation
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for meeting:', meetingId);

      // First, get total count by fetching first page
      const firstPage = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: 1,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      const totalCount = firstPage.total_count;
      console.log(`📊 Total transcripts in database: ${totalCount}`);

      if (totalCount === 0) {
        return [];
      }

      // Fetch all transcripts in one call
      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: totalCount,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      console.log(`✅ Fetched ${allData.transcripts.length} transcripts from database`);
      return allData.transcripts;
    } catch (error) {
      console.error('❌ Error fetching all transcripts:', error);
      toast.error(tRef.current('summary.transcriptsFetchFailed'));
      return [];
    }
  }, []);

  const buildSummaryTranscriptPayload = useCallback((allTranscripts: Transcript[]) => {
    const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
      if (seconds === undefined) {
        return fallbackTimestamp;
      }
      const totalSecs = Math.floor(seconds);
      const mins = Math.floor(totalSecs / 60);
      const secs = totalSecs % 60;
      return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
    };

    return {
      transcriptText: allTranscripts
        .map(t => `${formatTime(t.audio_start_time, t.timestamp)} ${t.text}`)
        .join('\n'),
      transcriptTexts: allTranscripts.map(t => t.text),
    };
  }, []);

  // Public API: Generate summary from transcripts
  const handleGenerateSummary = useCallback(async (customPrompt: string = '') => {
    if (!isTemplateSelectionReady) {
      toast.info(tRef.current('summary.loadingTemplate'));
      return false;
    }

    // Check if model config is still loading
    if (isModelConfigLoading) {
      console.log('⏳ Model configuration is still loading, please wait...');
      toast.info(tRef.current('summary.loadingModelConfig'));
      return false;
    }

    // CHANGE: Fetch ALL transcripts from database, not from pagination state
    console.log('📊 Fetching all transcripts for summary generation...');
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      const error_msg = tRef.current('summary.noTranscriptsForSummary');
      console.log(error_msg);
      toast.error(error_msg);
      return false;
    }

    console.log(`✅ Proceeding with ${allTranscripts.length} transcripts`);

    console.log('🚀 Starting summary generation with config:', {
      provider: modelConfig.provider,
      model: modelConfig.model,
      template: selectedTemplate
    });

    // Check if Ollama provider has models available
    if (modelConfig.provider === 'ollama') {
      try {
        const endpoint = modelConfig.ollamaEndpoint || null;
        const models = await invokeTauri<OllamaModel[]>('get_ollama_models', { endpoint });

        if (!models || models.length === 0) {
          toast.error(
            tRef.current('summary.noOllamaModels'),
            { duration: 5000 }
          );
          return false;
        }
      } catch (error) {
        console.error('Error checking Ollama models:', error);
        const errorMessage = error instanceof Error ? error.message : String(error);

        if (isOllamaNotInstalledError(errorMessage)) {
          // Ollama is not installed - show specific message with download link
          toast.error(
            tRef.current('summary.ollamaNotInstalled'),
            {
              description: tRef.current('summary.ollamaInstallDescription'),
              duration: 7000,
              action: {
                label: tRef.current('summary.download'),
                onClick: () => invokeTauri('open_external_url', { url: 'https://ollama.com/download' })
              }
            }
          );
        } else {
          // Other error - generic message
          toast.error(
            tRef.current('summary.ollamaCheckFailed'),
            { duration: 5000 }
          );
        }
        return false;
      }
    }

    // Check if built-in AI provider has models available
    if (modelConfig.provider === 'builtin-ai') {
      try {
        const selectedModel = modelConfig.model;

        if (!selectedModel) {
          toast.error(tRef.current('summary.noBuiltInModel'), {
            description: tRef.current('summary.selectModelDescription'),
            duration: 5000,
          });
          if (onOpenModelSettings) {
            onOpenModelSettings();
          }
          return false;
        }

        // Check model readiness with filesystem refresh
        const isReady = await invokeTauri<boolean>('builtin_ai_is_model_ready', {
          modelName: selectedModel,
          refresh: true,
        });

        if (!isReady) {
          // Get detailed model status
          const modelInfo = await invokeTauri<BuiltInModelInfo | null>('builtin_ai_get_model_info', {
            modelName: selectedModel,
          });

          if (modelInfo) {
            const status = modelInfo.status;

            if (status.type === 'downloading') {
              toast.info(tRef.current('summary.downloadInProgress'), {
                description: tRef.current('summary.downloadProgressDescription', { model: selectedModel, progress: status.progress }),
                duration: 5000,
              });
              return false;
            }

            if (status.type === 'not_downloaded') {
              toast.error(tRef.current('summary.modelNotDownloaded'), {
                description: tRef.current('summary.notDownloadedDescription', { model: selectedModel }),
                duration: 7000,
              });
              if (onOpenModelSettings) {
                onOpenModelSettings();
              }
              return false;
            }

            if (status.type === 'corrupted' || status.type === 'error') {
              const errorDesc = status.type === 'error'
                ? status.Error || tRef.current('summary.modelErrorDescription')
                : tRef.current('summary.corruptedDescription', { model: selectedModel });
              toast.error(tRef.current('summary.modelUnavailable'), {
                description: `${errorDesc} ${tRef.current('summary.checkModelSettingsDescription')}`,
                duration: 7000,
              });
              if (onOpenModelSettings) {
                onOpenModelSettings();
              }
              return false;
            }
          }

          // Fallback if we couldn't get model info
          toast.error(tRef.current('summary.builtInModelNotReady'), {
            description: tRef.current('summary.builtInModelDescription'),
            duration: 5000,
          });
          if (onOpenModelSettings) {
            onOpenModelSettings();
          }
          return false;
        }

        // Model is ready, continue to backend call
      } catch (error) {
        console.error('Error validating built-in AI model:', error);
        toast.error(tRef.current('summary.builtInModelCheckFailed'), {
          description: error instanceof Error ? error.message : String(error),
          duration: 5000,
        });
        return false;
      }
    }

    const summaryPayload = buildSummaryTranscriptPayload(allTranscripts);

    return processSummary({
      ...summaryPayload,
      customPrompt,
    });
  }, [meeting.id, fetchAllTranscripts, buildSummaryTranscriptPayload, processSummary, modelConfig, isModelConfigLoading, isTemplateSelectionReady, selectedTemplate, onOpenModelSettings]);

  // Public API: Regenerate summary from the current saved transcript
  const handleRegenerateSummary = useCallback(async () => {
    if (!isTemplateSelectionReady) {
      toast.info(tRef.current('summary.loadingTemplate'));
      return;
    }

    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      console.error('No transcripts available for regeneration');
      toast.error(tRef.current('summary.noTranscripts'));
      return;
    }

    await processSummary({
      ...buildSummaryTranscriptPayload(allTranscripts),
      isRegeneration: true
    });
  }, [meeting.id, fetchAllTranscripts, buildSummaryTranscriptPayload, processSummary, isTemplateSelectionReady]);

  // Public API: Stop ongoing summary generation
  const handleStopGeneration = useCallback(async () => {
    console.log('Stopping summary generation for meeting:', meeting.id);

    try {
      // Call backend to cancel the summary generation
      await invokeTauri('api_cancel_summary', {
        meetingId: meeting.id
      });
      console.log('✓ Backend cancellation request sent for meeting:', meeting.id);
    } catch (error) {
      console.error('Failed to cancel summary generation:', error);
      // Continue with frontend cleanup even if backend call fails
    }

    // Stop polling
    stopSummaryPolling(meeting.id);

    // Reset status to idle
    setSummaryStatus('idle');
    setSummaryError(null);

    // Show toast notification
    toast.info(tRef.current('summary.generationStopped'), {
      description: tRef.current('summary.generationStoppedDescription'),
      duration: 3000,
    });
  }, [meeting.id, stopSummaryPolling]);

  return {
    summaryStatus,
    summaryError,
    handleGenerateSummary,
    handleRegenerateSummary,
    handleStopGeneration,
    getSummaryStatusMessage,
  };
}
