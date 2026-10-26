"use client";
import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { MeetingDetails, MeetingSummary, SummaryResponse, TranscriptSegmentData } from '@/types';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { TranscriptPanel } from '@/components/MeetingDetails/TranscriptPanel';
import { SummaryPanel } from '@/components/MeetingDetails/SummaryPanel';
import { MeetingQuestions } from '@/components/MeetingDetails/MeetingQuestions';
import { ModelConfig } from '@/components/ModelSettingsModal';

// Custom hooks
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import { useTemplates } from '@/hooks/meeting-details/useTemplates';
import { useCopyOperations } from '@/hooks/meeting-details/useCopyOperations';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useMeetingExport } from '@/hooks/meeting-details/useMeetingExport';
import { useDiarizationRun } from '@/hooks/meeting-details/useDiarizationRun';
import { useConfig } from '@/contexts/ConfigContext';

export default function PageContent({
  meeting,
  summaryData,
  shouldAutoGenerate = false,
  onAutoGenerateComplete,
  onMeetingUpdated,
  onRefetchTranscripts,
  activeTranscriptId,
  // Pagination props for efficient transcript loading
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
}: {
  meeting: MeetingDetails;
  summaryData: MeetingSummary | null;
  shouldAutoGenerate?: boolean;
  onAutoGenerateComplete?: () => void;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  activeTranscriptId?: string | null;
  // Pagination props
  segments?: TranscriptSegmentData[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
}) {
  console.log('📄 PAGE CONTENT: Initializing with data:', {
    meetingId: meeting.id,
    summaryDataKeys: summaryData ? Object.keys(summaryData) : null,
    transcriptsCount: meeting.transcripts?.length
  });

  // State
  const [customPrompt, setCustomPrompt] = useState<string>('');
  const [summaryResponse] = useState<SummaryResponse | null>(null);

  // Ref to store the modal open function from SummaryGeneratorButtonGroup
  const openModelSettingsRef = useRef<(() => void) | null>(null);

  // Get model config from ConfigContext
  const { modelConfig, setModelConfig } = useConfig();

  // Custom hooks
  const meetingData = useMeetingData({ meeting, summaryData, onMeetingUpdated });
  const templates = useTemplates();

  // Callback to register the modal open function
  const handleRegisterModalOpen = (openFn: () => void) => {
    console.log('📝 Registering modal open function in PageContent');
    openModelSettingsRef.current = openFn;
  };

  // Callback to trigger modal open (called from error handler)
  const handleOpenModelSettings = () => {
    console.log('🔔 Opening model settings from PageContent');
    if (openModelSettingsRef.current) {
      openModelSettingsRef.current();
    } else {
      console.warn('⚠️ Modal open function not yet registered');
    }
  };

  // Save model config to backend database and sync via event
  const handleSaveModelConfig = async (config?: ModelConfig) => {
    if (!config) return;
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey ?? null,
        ollamaEndpoint: config.ollamaEndpoint ?? null,
      });

      // Emit event so ConfigContext and other listeners stay in sync
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      toast.success('Model settings saved successfully');
    } catch (error) {
      console.error('Failed to save model config:', error);
      toast.error('Failed to save model settings');
    }
  };

  const summaryGeneration = useSummaryGeneration({
    meeting,
    transcripts: meetingData.transcripts,
    modelConfig: modelConfig,
    isModelConfigLoading: false, // ConfigContext loads on mount
    isTemplateSelectionReady: templates.isSelectionReady,
    selectedTemplate: templates.selectedTemplate,
    onMeetingUpdated,
    updateMeetingTitle: meetingData.updateMeetingTitle,
    setAiSummary: meetingData.setAiSummary,
    onOpenModelSettings: handleOpenModelSettings,
  });
  const autoGenerationDependenciesRef = useRef({
    handleGenerateSummary: summaryGeneration.handleGenerateSummary,
    model: modelConfig.model,
    onAutoGenerateComplete,
    provider: modelConfig.provider,
  });
  const autoGenerationInFlightMeetingRef = useRef<string | null>(null);
  const autoGenerationCompletedMeetingRef = useRef<string | null>(null);
  const autoGenerationMountedRef = useRef(false);
  const latestMeetingIdRef = useRef(meeting.id);
  const transcriptCount = meetingData.transcripts.length;

  useEffect(() => {
    latestMeetingIdRef.current = meeting.id;
    autoGenerationDependenciesRef.current = {
      handleGenerateSummary: summaryGeneration.handleGenerateSummary,
      model: modelConfig.model,
      onAutoGenerateComplete,
      provider: modelConfig.provider,
    };
  }, [
    meeting.id,
    modelConfig.model,
    modelConfig.provider,
    onAutoGenerateComplete,
    summaryGeneration.handleGenerateSummary,
  ]);

  useEffect(() => {
    autoGenerationMountedRef.current = true;
    return () => {
      autoGenerationMountedRef.current = false;
    };
  }, []);

  const copyOperations = useCopyOperations({
    meeting,
    transcripts: meetingData.transcripts,
    meetingTitle: meetingData.meetingTitle,
    aiSummary: meetingData.aiSummary,
    blockNoteSummaryRef: meetingData.blockNoteSummaryRef,
  });

  const meetingExport = useMeetingExport({
    meeting,
    meetingTitle: meetingData.meetingTitle,
    aiSummary: meetingData.aiSummary,
    blockNoteSummaryRef: meetingData.blockNoteSummaryRef,
  });

  const meetingOperations = useMeetingOperations({
    meeting,
  });

  const diarization = useDiarizationRun({
    meetingId: meeting.id,
    onRefetchTranscripts,
  });

  // Auto-generate summary when flag is set
  useEffect(() => {
    if (
      !shouldAutoGenerate ||
      templates.isSelectionReady === false ||
      transcriptCount === 0 ||
      autoGenerationInFlightMeetingRef.current === meeting.id ||
      autoGenerationCompletedMeetingRef.current === meeting.id
    ) {
      return;
    }

    const meetingId = meeting.id;
    autoGenerationInFlightMeetingRef.current = meetingId;

    const autoGenerate = async () => {
      try {
        const { handleGenerateSummary, model, provider } = autoGenerationDependenciesRef.current;
        console.log(`🤖 Auto-generating summary with ${provider}/${model}...`);
        const didStart = await handleGenerateSummary('');

        if (
          didStart &&
          autoGenerationMountedRef.current &&
          latestMeetingIdRef.current === meetingId &&
          autoGenerationCompletedMeetingRef.current !== meetingId
        ) {
          autoGenerationCompletedMeetingRef.current = meetingId;
          autoGenerationDependenciesRef.current.onAutoGenerateComplete?.();
        }
      } finally {
        if (autoGenerationInFlightMeetingRef.current === meetingId) {
          autoGenerationInFlightMeetingRef.current = null;
        }
      }
    };

    void autoGenerate();
  }, [
    meeting.id,
    modelConfig.model,
    modelConfig.provider,
    shouldAutoGenerate,
    summaryGeneration.handleGenerateSummary,
    templates.isSelectionReady,
    transcriptCount,
  ]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      className="flex flex-col h-screen bg-muted/40"
    >
      <div className="flex flex-1 overflow-hidden">
        <TranscriptPanel
          transcripts={meetingData.transcripts}
          customPrompt={customPrompt}
          onPromptChange={setCustomPrompt}
          onCopyTranscript={copyOperations.handleCopyTranscript}
          onExportMeeting={meetingExport.handleExportMeeting}
          isExporting={meetingExport.isExporting}
          onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
          isRecording={diarization.isRecording}
          onRunDiarization={diarization.runDiarization}
          isDiarizationRunning={diarization.isRunning}
          diarizationProgress={diarization.progress}
          disableAutoScroll={true}
          activeSegmentId={activeTranscriptId ?? undefined}
          // Pagination props for efficient loading
          usePagination={true}
          segments={segments}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
          // Retranscription props
          meetingId={meeting.id}
          meetingFolderPath={meeting.folder_path}
          onRefetchTranscripts={onRefetchTranscripts}
        />
        <SummaryPanel
          meeting={meeting}
          isTitleDirty={meetingData.isTitleDirty}
          summaryRef={meetingData.blockNoteSummaryRef}
          isSaving={meetingData.isSaving}
          onSaveAll={meetingData.saveAllChanges}
          onCopySummary={copyOperations.handleCopySummary}
          aiSummary={meetingData.aiSummary}
          summaryStatus={summaryGeneration.summaryStatus}
          transcripts={meetingData.transcripts}
          modelConfig={modelConfig}
          setModelConfig={setModelConfig}
          onSaveModelConfig={handleSaveModelConfig}
          onGenerateSummary={async (prompt) => {
            await summaryGeneration.handleGenerateSummary(prompt);
          }}
          onStopGeneration={summaryGeneration.handleStopGeneration}
          customPrompt={customPrompt}
          summaryResponse={summaryResponse}
          onSaveSummary={meetingData.handleSaveSummary}
          onSummaryChange={meetingData.handleSummaryChange}
          onDirtyChange={meetingData.setIsSummaryDirty}
          summaryError={summaryGeneration.summaryError}
          getSummaryStatusMessage={summaryGeneration.getSummaryStatusMessage}
          availableTemplates={templates.availableTemplates}
          selectedTemplate={templates.selectedTemplate}
          onTemplateSelect={templates.handleTemplateSelection}
          templateManager={templates}
          isModelConfigLoading={false}
          onOpenModelSettings={handleRegisterModalOpen}
        />
      </div>
      <MeetingQuestions meetingId={meeting.id} modelConfig={modelConfig} />
    </motion.div>
  );
}
