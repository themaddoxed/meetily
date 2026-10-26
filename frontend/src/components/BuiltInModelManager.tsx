'use client';

import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { cn } from '@/lib/utils';
import { Download, RefreshCw, BadgeAlert, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { formatSummaryModelSizeLabelFromMb } from '@/lib/onboarding-summary-model';
import {
  getFirstSelectableBuiltInModelName,
  isSelectableBuiltInModelStatus,
} from '@/lib/builtin-ai-models';
import type { BuiltInModelInfo } from '@/lib/builtin-ai';
import { useI18n } from '@/contexts/I18nContext';

interface BuiltInModelDownloadProgress {
  model: string;
  progress: number;
  downloaded_mb?: number;
  total_mb?: number;
  speed_mbps?: number;
  status: 'downloading' | 'completed' | 'error' | 'cancelled';
  error?: string;
}

interface DownloadProgressInfo {
  downloadedMb: number;
  totalMb: number;
  speedMbps: number;
}

interface BuiltInModelManagerProps {
  selectedModel: string;
  onModelSelect: (model: string) => void;
  layout?: 'inline' | 'dialog';
}

export function BuiltInModelManager({
  selectedModel,
  onModelSelect,
  layout = 'inline',
}: BuiltInModelManagerProps) {
  const { t } = useI18n();
  const tRef = useRef(t);
  const [models, setModels] = useState<BuiltInModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasFetched, setHasFetched] = useState<boolean>(false);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [downloadProgressInfo, setDownloadProgressInfo] = useState<Record<string, DownloadProgressInfo>>({});
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(new Set());
  const selectedModelRef = useRef(selectedModel);
  const onModelSelectRef = useRef(onModelSelect);
  const fetchGenerationRef = useRef(0);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  useEffect(() => {
    selectedModelRef.current = selectedModel;
  }, [selectedModel]);

  useEffect(() => {
    onModelSelectRef.current = onModelSelect;
  }, [onModelSelect]);

  const selectModel = useCallback((modelName: string) => {
    selectedModelRef.current = modelName;
    onModelSelectRef.current(modelName);
  }, []);

  const fetchModels = useCallback(async () => {
    const generation = ++fetchGenerationRef.current;
    try {
      setIsLoading(true);
      const data = await invoke<BuiltInModelInfo[]>('builtin_ai_list_models');
      if (generation !== fetchGenerationRef.current) return;

      setModels(data);

      if (data.length > 0 && !selectedModelRef.current) {
        const firstSelectableModelName = getFirstSelectableBuiltInModelName(data);
        if (firstSelectableModelName) {
          selectModel(firstSelectableModelName);
        }
      }
    } catch (error) {
      if (generation !== fetchGenerationRef.current) return;
      console.error('Failed to fetch built-in AI models:', error);
      toast.error(tRef.current('model.builtInLoadFailed'));
    } finally {
      if (generation === fetchGenerationRef.current) {
        setIsLoading(false);
        setHasFetched(true);
      }
    }
  }, [selectModel]);

  useEffect(() => {
    fetchModels();
    return () => {
      fetchGenerationRef.current += 1;
    };
  }, [fetchModels]);

  // Listen for download progress events
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    const setupListener = async () => {
      const nextUnlisten = await listen<BuiltInModelDownloadProgress>('builtin-ai-download-progress', (event) => {
        if (disposed) return;

        const { model, progress, downloaded_mb, total_mb, speed_mbps, status } = event.payload;

        // Update percentage progress
        setDownloadProgress((prev) => ({
          ...prev,
          [model]: progress,
        }));

        // Update detailed progress info (MB, speed)
        setDownloadProgressInfo((prev) => ({
          ...prev,
          [model]: {
            downloadedMb: downloaded_mb ?? 0,
            totalMb: total_mb ?? 0,
            speedMbps: speed_mbps ?? 0,
          },
        }));

        // Handle downloading status - restore downloadingModels state on modal reopen
        if (status === 'downloading') {
          setDownloadingModels((prev) => {
            if (!prev.has(model)) {
              const newSet = new Set(prev);
              newSet.add(model);
              return newSet;
            }
            return prev;
          });
        }

        // Handle completed status
        if (status === 'completed') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const next = { ...prev };
            delete next[model];
            return next;
          });
          setDownloadProgressInfo((prev) => {
            const next = { ...prev };
            delete next[model];
            return next;
          });
          // Refresh models list
          fetchModels();
          toast.success(tRef.current('model.downloadedSuccessfully', { model }));
        }

        // Handle cancelled status
        if (status === 'cancelled') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const next = { ...prev };
            delete next[model];
            return next;
          });
          setDownloadProgressInfo((prev) => {
            const next = { ...prev };
            delete next[model];
            return next;
          });
          // Refresh models list
          fetchModels();
        }

        // Handle error status
        if (status === 'error') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const next = { ...prev };
            delete next[model];
            return next;
          });
          setDownloadProgressInfo((prev) => {
            const next = { ...prev };
            delete next[model];
            return next;
          });

          // Update model status to error locally instead of fetching from backend
          // Backend doesn't persist error status, so fetchModels() would return not_downloaded
          setModels((prevModels) =>
            prevModels.map((m) =>
              m.name === model
                ? {
                    ...m,
                    status: {
                      type: 'error',
                      Error: tRef.current('model.errorOccurred'),
                    },
                  }
                : m
            )
          );

          // Don't show error toast here - DownloadProgressToast already handles it
          // Don't call fetchModels() - it would overwrite error status with not_downloaded
        }
      });

      if (disposed) {
        nextUnlisten();
        return;
      }
      unlisten = nextUnlisten;
    };

    setupListener();

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [fetchModels]);

  const downloadModel = async (modelName: string) => {
    selectModel(modelName);

    try {
      // Optimistically add to downloadingModels for immediate UI feedback
      setDownloadingModels((prev) => new Set([...prev, modelName]));

      await invoke('builtin_ai_download_model', { modelName });
    } catch (error) {
      console.error('Failed to download model:', error);

      // Check if this is a cancellation error (starts with "CANCELLED:")
      const errorMsg = String(error);
      if (errorMsg.startsWith('CANCELLED:')) {
        // Cancel handler already removed from downloadingModels
        // Don't show error toast for cancellations - cancel function already shows info toast
        return;
      }

      // For real errors, show toast and remove from downloading
      toast.error(tRef.current('model.downloadFailed'), {
        description: modelName,
      });

      setDownloadingModels((prev) => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      // Refresh model list to get updated Error status from backend
      fetchModels();
    }
  };

  const cancelDownload = async (modelName: string) => {
    try {
      await invoke('builtin_ai_cancel_download', { modelName });
      toast.info(tRef.current('model.downloadOfCancelled', { model: modelName }));
      setDownloadingModels((prev) => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const deleteModel = async (modelName: string) => {
    try {
      await invoke('builtin_ai_delete_model', { modelName });
      toast.success(tRef.current('model.modelDeleted', { model: modelName }));
      fetchModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
      toast.error(tRef.current('model.deleteFailed'), {
        description: modelName,
      });
    }
  };

  // Don't show loading spinner if we have downloads in progress - show the model list instead
  if (isLoading && downloadingModels.size === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <RefreshCw className="mx-auto h-8 w-8 animate-spin mb-2" />
        {t('model.loading')}
      </div>
    );
  }

  // Only show "no models" message after fetch has completed
  if (hasFetched && models.length === 0) {
    return (
      <Alert>
        <AlertDescription>
          {t('model.noModelsBuiltIn')}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h4 className="text-sm font-bold">{t('model.managerTitle')}</h4>
      </div>

      <div
        className={cn(
          'grid gap-4',
          layout === 'dialog' && 'max-h-[50vh] overflow-y-auto pr-2 pb-2'
        )}
      >
        {models.map((model) => {
          const progress = downloadProgress[model.name];
          const progressInfo = downloadProgressInfo[model.name];
          const modelIsDownloading = downloadingModels.has(model.name);
          const isAvailable = model.status.type === 'available';
          const isNotDownloaded = model.status.type === 'not_downloaded';
          const isCorrupted = model.status.type === 'corrupted';
          const isError = model.status.type === 'error';
          const isSelectable = isSelectableBuiltInModelStatus(model.status.type, modelIsDownloading);

          return (
            <div
              key={model.name}
              className={cn(
                'p-4 rounded-lg border transition-colors',
                modelIsDownloading
                  ? 'bg-card border-border'
                  : 'bg-card',
                selectedModel === model.name
                  ? 'ring-2 ring-ring border-primary'
                  : 'border-border hover:border-border',
                isSelectable && 'cursor-pointer'
              )}
              onClick={() => {
                if (isSelectable) {
                  selectModel(model.name);
                }
              }}
            >
            <div className="space-y-3">
              <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div className="min-w-0 flex-1">
                  <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                    <span className="min-w-0 break-words text-base font-bold leading-snug text-foreground">{model.display_name || model.name}</span>
                    {isAvailable && (
                      <span className="flex shrink-0 items-center gap-1 text-xs font-medium text-success">
                        <span className="h-2 w-2 rounded-full bg-success"></span>
                        {t('model.ready')}
                      </span>
                    )}
                    {selectedModel === model.name && (
                      <span className="shrink-0 rounded bg-info/15 px-2 py-0.5 text-xs font-medium text-info">
                        {t('model.selected')}
                      </span>
                    )}
                    {isCorrupted && (
                      <span className="flex shrink-0 items-center gap-1 rounded bg-destructive/15 px-2 py-0.5 text-xs font-medium text-destructive">
                        <BadgeAlert className="h-3 w-3" />
                        {t('model.corrupted')}
                      </span>
                    )}
                    {isError && (
                      <span className="shrink-0 rounded bg-destructive/15 px-2 py-0.5 text-xs font-medium text-destructive">
                        {t('model.error')}
                      </span>
                    )}
                  </div>
                </div>
                <div className="flex w-full shrink-0 flex-wrap items-center gap-2 sm:ml-4 sm:w-auto sm:justify-end">
                  {/* Not Downloaded - Show Download button */}
                  {isNotDownloaded && !modelIsDownloading && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="min-w-[100px]"
                      onClick={(e) => {
                        e.stopPropagation();
                        downloadModel(model.name);
                      }}
                    >
                      <Download className="mr-2 h-4 w-4" />
                      {t('model.download')}
                    </Button>
                  )}
                  {/* Downloading - Show Cancel button */}
                  {modelIsDownloading && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="min-w-[100px]"
                      onClick={(e) => {
                        e.stopPropagation();
                        cancelDownload(model.name);
                      }}
                    >
                      {t('model.cancel')}
                    </Button>
                  )}
                  {/* Error - Show Retry button */}
                  {isError && !modelIsDownloading && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="min-w-[100px]"
                      onClick={(e) => {
                        e.stopPropagation();
                        downloadModel(model.name);
                      }}
                    >
                      <RefreshCw className="mr-2 h-4 w-4" />
                      {t('model.retry')}
                    </Button>
                  )}
                  {/* Corrupted - Show both Retry and Delete buttons */}
                  {isCorrupted && !modelIsDownloading && (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          downloadModel(model.name);
                        }}
                      >
                        <RefreshCw className="mr-2 h-4 w-4" />
                        {t('model.retry')}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteModel(model.name);
                        }}
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        {t('model.delete')}
                      </Button>
                    </>
                  )}
                  {/* Available - Show small trash icon (only if not currently selected) */}
                  {isAvailable && !modelIsDownloading && selectedModel !== model.name && (
                    <button
                      className="p-2 rounded hover:bg-muted transition-colors text-muted-foreground hover:text-destructive"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteModel(model.name);
                      }}
                      title={t('model.deleteTitle')}
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  )}
                </div>
              </div>
              <div className="text-sm text-muted-foreground">
                {model.description && (
                  <p className="mb-1">{model.description}</p>
                )}
                {(isError || isCorrupted) && (
                  <p className="mb-1 text-xs text-destructive">
                    {model.status.type === 'error'
                      ? model.status.Error
                      : isCorrupted
                      ? t('model.fileCorrupted')
                      : t('model.errorOccurred')}
                  </p>
                )}
                <div className="text-xs text-muted-foreground">
                  <span>{formatSummaryModelSizeLabelFromMb(model.size_mb)} • {model.context_size} tokens</span>
                </div>
                </div>
              </div>

              {/* Download progress bar */}
              {modelIsDownloading && progress !== undefined && (
                <div className="mt-3 pt-3 border-t border-border">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-medium text-foreground">{t('model.downloading')}</span>
                    <span className="text-sm font-semibold text-foreground">
                      {Math.round(progress)}%
                    </span>
                  </div>
                  <div className="text-sm text-muted-foreground mb-2">
                    {progressInfo?.totalMb > 0 ? (
                      <>
                        {progressInfo.downloadedMb.toFixed(1)} MiB / {progressInfo.totalMb.toFixed(1)} MiB
                        {progressInfo.speedMbps > 0 && (
                          <span className="ml-2 text-muted-foreground">
                            ({progressInfo.speedMbps.toFixed(1)} MiB/s)
                          </span>
                        )}
                      </>
                    ) : (
                      <span>{formatSummaryModelSizeLabelFromMb(model.size_mb)}</span>
                    )}
                  </div>
                  <div className="w-full h-2.5 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary rounded-full transition-[width] duration-300"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
