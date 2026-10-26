import React, { useState, useEffect, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import { toast } from 'sonner';
import {
  ModelInfo,
  ModelStatus,
  getModelIcon,
  formatFileSize,
  getModelPerformanceBadge,
  isQuantizedModel,
  getModelTagline,
  WhisperAPI
} from '../lib/whisper';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui/accordion';
import { useI18n } from '@/contexts/I18nContext';

interface ModelManagerProps {
  selectedModel?: string;
  onModelSelect?: (modelName: string) => void;
  className?: string;
  autoSave?: boolean;
}

export function ModelManager({
  selectedModel,
  onModelSelect,
  className = '',
  autoSave = false
}: ModelManagerProps) {
  const { t } = useI18n();
  const tRef = useRef(t);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(new Set());
  const [, setHasUserSelection] = useState(false);

  // Refs for stable callbacks
  const onModelSelectRef = useRef(onModelSelect);
  const autoSaveRef = useRef(autoSave);
  const modelsRef = useRef(models);
  const downloadModelRef = useRef<(modelName: string) => Promise<void>>(async () => undefined);

  // Progress throttle map to prevent rapid updates
  const progressThrottleRef = useRef<Map<string, { progress: number; timestamp: number }>>(new Map());

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  // Update refs when props change
  useEffect(() => {
    onModelSelectRef.current = onModelSelect;
    autoSaveRef.current = autoSave;
  }, [onModelSelect, autoSave]);

  useEffect(() => {
    modelsRef.current = models;
  }, [models]);

  // The backend is the source of truth after a restart.  Keep this set only
  // for the current view so a stale browser key cannot resurrect a download.
  const updateDownloadingModels = useCallback((updater: (prev: Set<string>) => Set<string>) => {
    setDownloadingModels(prev => {
      return updater(prev);
    });
  }, []);

  // Initialize models
  useEffect(() => {
    if (initialized) return;

    const initializeModels = async () => {
      try {
        setLoading(true);
        await WhisperAPI.init();
        const modelList = await WhisperAPI.getAvailableModels();

        // Remove the legacy key once, then trust the status returned by Rust.
        // In particular, Error and Missing must remain retryable after a
        // restart instead of becoming a permanent Downloading state.
        try {
          localStorage.removeItem('downloading-models');
        } catch {
          // Browser storage may be unavailable in private or test contexts.
        }
        setModels(modelList);
        setDownloadingModels(new Set(
          modelList
            .filter(model => typeof model.status === 'object' && 'Downloading' in model.status)
            .map(model => model.name)
        ));
        setInitialized(true);
      } catch (err) {
        console.error('Failed to initialize Whisper:', err);
        setError(err instanceof Error ? err.message : tRef.current('model.managerLoadFailed'));
        toast.error(tRef.current('model.loadTranscriptionFailed'), {
          description: err instanceof Error ? err.message : tRef.current('model.unknownError'),
          duration: 5000
        });
      } finally {
        setLoading(false);
      }
    };

    initializeModels();
  }, [initialized, selectedModel, onModelSelect, updateDownloadingModels]);

  // Set up event listeners for download progress
  useEffect(() => {
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;
    let disposed = false;

    const setupListeners = async () => {
      console.log('[ModelManager] Setting up event listeners...');

      // Download progress with throttling
      const progressUnlisten = await listen<{ modelName: string; progress: number }>(
        'model-download-progress',
        (event) => {
          const { modelName, progress } = event.payload;
          const now = Date.now();
          const throttleData = progressThrottleRef.current.get(modelName);

          // Throttle: only update if 300ms passed OR progress jumped by 5%+
          const shouldUpdate = !throttleData ||
            now - throttleData.timestamp > 300 ||
            Math.abs(progress - throttleData.progress) >= 5;

          if (shouldUpdate) {
            console.log(`[ModelManager] Progress update for ${modelName}: ${progress}%`);
            progressThrottleRef.current.set(modelName, { progress, timestamp: now });

            setModels(prevModels =>
              prevModels.map(model =>
                model.name === modelName
                  ? { ...model, status: { Downloading: progress } as ModelStatus }
                  : model
              )
            );
          }
        }
      );
      if (disposed) {
        progressUnlisten();
        return;
      }
      unlistenProgress = progressUnlisten;

      // Download complete
      const completeUnlisten = await listen<{ modelName: string }>(
        'model-download-complete',
        (event) => {
          const { modelName } = event.payload;
          const model = modelsRef.current.find(m => m.name === modelName);
          const displayName = getDisplayName(modelName);

          setModels(prevModels =>
            prevModels.map(model =>
              model.name === modelName
                ? { ...model, status: 'Available' as ModelStatus }
                : model
            )
          );

          setDownloadingModels(prev => {
            const newSet = new Set(prev);
            newSet.delete(modelName);
            return newSet;
          });

          // Clean up throttle data
          progressThrottleRef.current.delete(modelName);

          toast.success(`${getModelIcon(model?.accuracy || 'Good')} ${displayName} ${tRef.current('model.ready').toLowerCase()}!`, {
            description: tRef.current('model.downloadReady'),
            duration: 4000
          });

          // Auto-select after download using stable refs
          if (onModelSelectRef.current) {
            onModelSelectRef.current(modelName);
            if (autoSaveRef.current) {
              saveModelSelection(modelName);
            }
          }
        }
      );
      if (disposed) {
        completeUnlisten();
        return;
      }
      unlistenComplete = completeUnlisten;

      // Download error
      const errorUnlisten = await listen<{ modelName: string; error: string }>(
        'model-download-error',
        (event) => {
          const { modelName, error } = event.payload;

          setModels(prevModels =>
            prevModels.map(model =>
              model.name === modelName
                ? { ...model, status: { Error: error } as ModelStatus }
                : model
            )
          );

          setDownloadingModels(prev => {
            const newSet = new Set(prev);
            newSet.delete(modelName);
            return newSet;
          });

          // Clean up throttle data
          progressThrottleRef.current.delete(modelName);

          toast.error(tRef.current('model.downloadFailed'), {
            description: error,
            duration: 6000,
            action: {
              label: tRef.current('model.retry'),
              onClick: () => downloadModelRef.current(modelName)
            }
          });
        }
      );
      if (disposed) {
        errorUnlisten();
        return;
      }
      unlistenError = errorUnlisten;
    };

    setupListeners();

    return () => {
      disposed = true;
      console.log('[ModelManager] Cleaning up event listeners...');
      if (unlistenProgress) unlistenProgress();
      if (unlistenComplete) unlistenComplete();
      if (unlistenError) unlistenError();
    };
  }, []); // Empty dependency array - listeners use refs for stable callbacks

  const saveModelSelection = async (modelName: string) => {
    try {
      await invoke('api_save_transcript_config', {
        provider: 'localWhisper',
        model: modelName,
        apiKey: null
      });
    } catch (error) {
      console.error('Failed to save model selection:', error);
    }
  };

  const cancelDownload = async (modelName: string) => {
    const displayName = getDisplayName(modelName);

    try {
      await WhisperAPI.cancelDownload(modelName);

      updateDownloadingModels(prev => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      setModels(prevModels =>
        prevModels.map(model =>
          model.name === modelName
            ? { ...model, status: 'Missing' as ModelStatus }
            : model
        )
      );

      // Clean up throttle data
      progressThrottleRef.current.delete(modelName);

      toast.info(tRef.current('model.downloadCancelled', { model: displayName }), {
        duration: 3000
      });
    } catch (err) {
      console.error('Failed to cancel download:', err);
      toast.error(tRef.current('model.cancelFailed'), {
        description: err instanceof Error ? err.message : tRef.current('model.unknownError'),
        duration: 4000
      });
    }
  };

  const downloadModel = useCallback(async (modelName: string) => {
    if (downloadingModels.has(modelName)) return;

    const displayName = getDisplayName(modelName);

    try {
      updateDownloadingModels(prev => new Set([...prev, modelName]));

      setModels(prevModels =>
        prevModels.map(model =>
          model.name === modelName
            ? { ...model, status: { Downloading: 0 } as ModelStatus }
            : model
        )
      );

      toast.info(`${tRef.current('model.downloading')} ${displayName}`, {
        description: tRef.current('model.downloadInMinutes'),
        duration: 5000
      });

      await WhisperAPI.downloadModel(modelName);
    } catch (err) {
      console.error('Download failed:', err);
      const errorMessage = err instanceof Error ? err.message : String(err);

      // The command keeps the cancellation marker inside its existing
      // compatibility wrapper ("Failed to download model: CANCELLED: ...").
      // Cancellation is already finalized by cancelDownload, so do not turn
      // the late command result into a terminal Error state.
      if (errorMessage.includes('CANCELLED:')) {
        return;
      }

      updateDownloadingModels(prev => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      setModels(prev =>
        prev.map(model =>
          model.name === modelName ? { ...model, status: { Error: errorMessage } } : model
        )
      );
    }
  }, [downloadingModels, updateDownloadingModels]);

  useEffect(() => {
    downloadModelRef.current = downloadModel;
  }, [downloadModel]);

  const selectModel = async (modelName: string) => {
    setHasUserSelection(true);

    if (onModelSelect) {
      onModelSelect(modelName);
    }

    if (autoSave) {
      await saveModelSelection(modelName);
    }

    const displayName = getDisplayName(modelName);
    toast.success(`${tRef.current('model.selected')}: ${displayName}`, {
      duration: 3000
    });
  };

  const deleteModel = async (modelName: string) => {
    const displayName = getDisplayName(modelName);

    try {
      await WhisperAPI.deleteCorruptedModel(modelName);

      // Refresh models list
      const modelList = await WhisperAPI.getAvailableModels();
      setModels(modelList);

      toast.success(tRef.current('model.modelDeleted', { model: displayName }), {
        description: tRef.current('model.modelRemoved'),
        duration: 3000
      });

      // If deleted model was selected, clear selection
      if (selectedModel === modelName && onModelSelect) {
        onModelSelect('');
      }
    } catch (err) {
      console.error('Failed to delete model:', err);
      toast.error(tRef.current('model.deleteFailed'), {
        description: err instanceof Error ? err.message : tRef.current('model.deleteFailed'),
        duration: 4000
      });
    }
  };

  const getDisplayName = (modelName: string): string => {
    const modelNameMapping: { [key: string]: string } = {
      "small": "Small",
      "medium-q5_0": "Medium",
      "large-v3-q5_0": "Large V3 Compressed",
      "large-v3-turbo": "Large V3 Turbo",
      "large-v3": "Large V3"
    };

    const basicModelNames = ["small", "medium-q5_0", "large-v3-q5_0", "large-v3-turbo", "large-v3"];
    if (basicModelNames.includes(modelName)) {
      return modelNameMapping[modelName] || modelName;
    }
    return `Whisper ${modelName}`;
  };

  if (loading) {
    return (
      <div className={`space-y-3 ${className}`}>
        <div className="animate-pulse space-y-3">
          <div className="h-20 bg-muted rounded-lg"></div>
          <div className="h-20 bg-muted rounded-lg"></div>
          <div className="h-20 bg-muted rounded-lg"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`bg-destructive/10 border border-destructive/30 rounded-lg p-4 ${className}`}>
        <p className="text-sm text-destructive">{t('model.managerLoadFailed')}</p>
        <p className="text-xs text-destructive mt-1">{error}</p>
      </div>
    );
  }

  const basicModelNames = ["small", "medium-q5_0", "large-v3-q5_0", "large-v3-turbo", "large-v3"];
  const basicModels = models.filter(m => basicModelNames.includes(m.name))
    .sort((a, b) => basicModelNames.indexOf(a.name) - basicModelNames.indexOf(b.name));
  const advancedModels = models.filter(m => !basicModelNames.includes(m.name));

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Basic Models */}
      <div className="space-y-3">
        {basicModels.map((model) => {
          const isRecommended = model.name === 'base';
          return (
            <ModelCard
              key={model.name}
              model={model}
              isSelected={selectedModel === model.name}
              isRecommended={isRecommended}
              onSelect={() => {
                if (model.status === 'Available') {
                  selectModel(model.name);
                }
              }}
              onDownload={() => downloadModel(model.name)}
              onCancel={() => cancelDownload(model.name)}
              onDelete={() => deleteModel(model.name)}
              displayName={getDisplayName(model.name)}
            />
          );
        })}
      </div>

      {/* Advanced Models */}
      {advancedModels.length > 0 && (
        <Accordion type="single" collapsible className="w-full">
          <AccordionItem value="advanced-models">
            <AccordionTrigger>
              <span className='text-lg'>{t('model.advancedModels')}</span>
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-3 pt-4">
                {advancedModels.map((model) => (
                  <ModelCard
                    key={model.name}
                    model={model}
                    isSelected={selectedModel === model.name}
                    isRecommended={false}
                    onSelect={() => {
                      if (model.status === 'Available') {
                        selectModel(model.name);
                      }
                    }}
                    onDownload={() => downloadModel(model.name)}
                    onCancel={() => cancelDownload(model.name)}
                    onDelete={() => deleteModel(model.name)}
                    displayName={getDisplayName(model.name)}
                  />
                ))}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}

      {/* Helper text */}
      {selectedModel && (
        <motion.div
          initial={{ opacity: 0, y: -5 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-xs text-muted-foreground text-center pt-2"
        >
          {t('model.usingForTranscription', { model: getDisplayName(selectedModel) })}
        </motion.div>
      )}
    </div>
  );
}

// Model Card Component
interface ModelCardProps {
  model: ModelInfo;
  isSelected: boolean;
  isRecommended: boolean;
  onSelect: () => void;
  onDownload: () => void;
  onCancel: () => void;
  onDelete: () => void;
  displayName: string;
}

function ModelCard({
  model,
  isSelected,
  isRecommended,
  onSelect,
  onDownload,
  onCancel,
  onDelete,
  displayName
}: ModelCardProps) {
  const { t } = useI18n();
  const [isHovered, setIsHovered] = useState(false);

  const isAvailable = model.status === 'Available';
  const isMissing = model.status === 'Missing';
  const isError = typeof model.status === 'object' && 'Error' in model.status;
  const isCorrupted = typeof model.status === 'object' && 'Corrupted' in model.status;
  const downloadProgress =
    typeof model.status === 'object' && 'Downloading' in model.status
      ? model.status.Downloading
      : null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 5 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        relative rounded-xl border-2 transition-[color,background-color,border-color,box-shadow,transform] cursor-pointer
        ${isSelected && isAvailable
          ? 'border-info bg-info/10'
          : isAvailable
            ? 'border-border hover:border-border bg-card'
            : 'border-border bg-muted/40'
        }
        ${isAvailable ? '' : 'cursor-default'}
      `}
      onClick={() => {
        if (isAvailable) onSelect();
      }}
    >
      {/* Recommended Badge */}
      {isRecommended && (
        <div className="absolute -top-2 -right-2 bg-info text-info-foreground text-xs px-2 py-0.5 rounded-full font-medium">
          {t('model.recommended')}
        </div>
      )}

      <div className="p-3">
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1">
            {/* Model Name and Tagline */}
            <div className="flex items-center gap-2 flex-wrap mb-2">
              <span className="text-2xl">{getModelIcon(model.accuracy)}</span>
              <h3 className="font-semibold text-foreground">{displayName}</h3>
              <span className="text-sm text-muted-foreground">•</span>
              <span className="text-sm text-muted-foreground">{getModelTagline(model.name, model.speed, model.accuracy)}</span>
              {isSelected && isAvailable && (
                <motion.span
                  initial={{ scale: 0 }}
                  animate={{ scale: 1 }}
                  className="bg-info text-info-foreground px-2 py-0.5 rounded-full text-xs font-medium flex items-center gap-1"
                >
                  ✓
                </motion.span>
              )}
              {isQuantizedModel(model.name) && (
                <span className={`px-2 py-0.5 rounded-full text-xs ${getModelPerformanceBadge(model.name).color === 'green'
                  ? 'bg-success/15 text-success'
                  : getModelPerformanceBadge(model.name).color === 'orange'
                    ? 'bg-warning/15 text-warning'
                    : 'bg-muted text-foreground'
                  }`}>
                  {getModelPerformanceBadge(model.name).label}
                </span>
              )}
            </div>

            {/* Model Specs */}
            <div className="flex items-center space-x-4 text-sm text-muted-foreground ml-9 mt-1.5">
              <span className="flex items-center space-x-1">
                <span>📦</span>
                <span>{formatFileSize(model.size_mb)}</span>
              </span>
              <span className="flex items-center space-x-1">
                <span>🎯</span>
                <span>{model.accuracy} {t('model.accuracy')}</span>
              </span>
              <span className="flex items-center space-x-1">
                <span>⚡</span>
                <span>{model.speed} {t('model.processing')}</span>
              </span>
            </div>
          </div>

          {/* Status/Action */}
          <div className="ml-4 flex items-center gap-2">
            {isAvailable && (
              <>
                <div className="flex items-center gap-1.5 text-success">
                  <div className="w-2 h-2 bg-success rounded-full"></div>
                  <span className="text-xs font-medium">{t('model.ready')}</span>
                </div>
                <AnimatePresence>
                  {isHovered && (
                    <motion.button
                      initial={{ opacity: 0, scale: 0.8 }}
                      animate={{ opacity: 1, scale: 1 }}
                      exit={{ opacity: 0, scale: 0.8 }}
                      transition={{ duration: 0.15 }}
                      onClick={(e) => {
                        e.stopPropagation();
                        onDelete();
                      }}
                      className="text-muted-foreground hover:text-destructive transition-colors p-1"
                      title={t('model.deleteSpaceTitle')}
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </motion.button>
                  )}
                </AnimatePresence>
              </>
            )}

            {isMissing && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDownload();
                }}
                className="bg-info text-info-foreground px-3 py-1.5 rounded-md text-sm font-medium hover:bg-info/90 transition-colors"
              >
                {t('model.download')}
              </button>
            )}

            {downloadProgress === null && isError && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDownload();
                }}
                className="bg-destructive text-destructive-foreground px-3 py-1.5 rounded-md text-sm font-medium hover:bg-destructive/90 transition-colors"
              >
                {t('model.retry')}
              </button>
            )}

            {isCorrupted && (
              <div className="flex gap-2">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete();
                  }}
                  className="bg-warning text-warning-foreground px-3 py-1.5 rounded-md text-sm font-medium hover:bg-warning/90 transition-colors"
                >
                  {t('model.delete')}
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDownload();
                  }}
                  className="bg-info text-info-foreground px-3 py-1.5 rounded-md text-sm font-medium hover:bg-info/90 transition-colors"
                >
                  {t('model.redownload')}
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Full-width Download Progress Bar - PROMINENT */}
        {downloadProgress !== null && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-3 pt-3 border-t border-border"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium text-info">{t('model.downloading')}</span>
                <span className="text-sm font-semibold text-info">{Math.round(downloadProgress)}%</span>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onCancel();
                }}
                className="text-xs text-muted-foreground hover:text-destructive font-medium transition-colors px-2 py-1 rounded hover:bg-destructive/10"
                title={t('model.cancelDownloadTitle')}
              >
                {t('model.cancel')}
              </button>
            </div>
            <div className="w-full h-2 bg-muted rounded-full overflow-hidden">
              <motion.div
                className="h-full bg-info rounded-full"
                initial={{ width: 0 }}
                animate={{ width: `${downloadProgress}%` }}
                transition={{ duration: 0.3, ease: 'easeOut' }}
              />
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {model.size_mb ? (
                <>
                  {formatFileSize(model.size_mb * downloadProgress / 100)} / {formatFileSize(model.size_mb)}
                </>
              ) : (
                t('model.downloading')
              )}
            </p>
          </motion.div>
        )}
      </div>
    </motion.div>
  );
}
