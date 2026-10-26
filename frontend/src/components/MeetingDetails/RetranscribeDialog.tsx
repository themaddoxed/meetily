import React, { useState, useEffect, useRef, useMemo } from 'react';
import { RefreshCw, Globe, Loader2, AlertCircle, X, Cpu } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { useConfig } from '@/contexts/ConfigContext';
import { LANGUAGES } from '@/constants/languages';
import { useTranscriptionModels, ModelOption } from '@/hooks/useTranscriptionModels';
import {
  canStartRetranscription,
  isCurrentVocabularyRequest,
  type VocabularyLoadStatus,
} from '@/lib/retranscription-vocabulary';
import { useI18n } from '@/contexts/I18nContext';

interface RetranscribeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  meetingId: string;
  meetingFolderPath: string | null;
  onComplete?: () => void;
}

interface RetranscriptionProgress {
  meeting_id: string;
  stage: string;
  progress_percentage: number;
  message: string;
}

interface RetranscriptionResult {
  meeting_id: string;
  segments_count: number;
  duration_seconds: number;
  language: string | null;
}

interface RetranscriptionError {
  meeting_id: string;
  error: string;
}

interface MeetingVocabularyConfig {
  globalVocabulary: string | null;
  meetingVocabulary: string | null;
  effectiveVocabulary: string | null;
  supportsVocabulary: boolean;
}

export function RetranscribeDialog({
  open,
  onOpenChange,
  meetingId,
  meetingFolderPath,
  onComplete,
}: RetranscribeDialogProps) {
  const { selectedLanguage, transcriptModelConfig } = useConfig();
  const { t } = useI18n();
  const [isProcessing, setIsProcessing] = useState(false);
  const [progress, setProgress] = useState<RetranscriptionProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedLang, setSelectedLang] = useState(selectedLanguage || 'auto');
  const [meetingVocabulary, setMeetingVocabulary] = useState('');
  const [effectiveVocabulary, setEffectiveVocabulary] = useState<string | null>(null);
  const [vocabularyLoadState, setVocabularyLoadState] = useState<VocabularyLoadStatus>('idle');
  const [vocabularyLoadError, setVocabularyLoadError] = useState<string | null>(null);

  // Use centralized model fetching hook
  const {
    availableModels,
    selectedModelKey,
    setSelectedModelKey,
    loadingModels,
    fetchModels,
    resetSelection,
  } = useTranscriptionModels(transcriptModelConfig);

  // Stable refs for callbacks to avoid listener re-registration
  const onCompleteRef = useRef(onComplete);
  const onOpenChangeRef = useRef(onOpenChange);
  useEffect(() => { onCompleteRef.current = onComplete; }, [onComplete]);
  useEffect(() => { onOpenChangeRef.current = onOpenChange; }, [onOpenChange]);

  // Track previous open state to only reset on closed→open transition
  const prevOpenRef = useRef(false);
  const vocabularyRequestRef = useRef(0);

  // Helper to get selected model details (memoized)
  const selectedModelDetails = useMemo((): ModelOption | undefined => {
    if (!selectedModelKey) return undefined;
    const colonIndex = selectedModelKey.indexOf(':');
    if (colonIndex === -1) return undefined;
    const provider = selectedModelKey.slice(0, colonIndex);
    const name = selectedModelKey.slice(colonIndex + 1);
    return availableModels.find(m => m.provider === provider && m.name === name);
  }, [selectedModelKey, availableModels]);
  const selectedProvider = selectedModelDetails?.provider || transcriptModelConfig.provider;
  const isParakeetModel = selectedProvider === 'parakeet';

  useEffect(() => {
    if (isParakeetModel && selectedLang !== 'auto') {
      setSelectedLang('auto');
    }
  }, [isParakeetModel, selectedLang]);

  // Reset state only when dialog transitions from closed to open
  // This prevents re-initialization when config changes while dialog is already open
  useEffect(() => {
    const wasOpen = prevOpenRef.current;
    prevOpenRef.current = open;

    if (open && !wasOpen) {
      resetSelection();
      setIsProcessing(false);
      setProgress(null);
      setError(null);
      setSelectedLang(selectedLanguage || 'auto');
      setMeetingVocabulary('');
      setEffectiveVocabulary(null);

      // Fetch available models using centralized hook
      fetchModels();
    }
  }, [open, selectedLanguage, transcriptModelConfig, fetchModels, resetSelection]);

  useEffect(() => {
    if (!open) return;
    const requestId = ++vocabularyRequestRef.current;
    let disposed = false;
    setVocabularyLoadState('loading');
    setVocabularyLoadError(null);
    setMeetingVocabulary('');
    setEffectiveVocabulary(null);

    invoke<MeetingVocabularyConfig>('api_get_meeting_vocabulary', { meetingId })
      .then((config) => {
        if (!isCurrentVocabularyRequest(requestId, vocabularyRequestRef.current, disposed)) return;
        setMeetingVocabulary(config.meetingVocabulary || '');
        setEffectiveVocabulary(config.effectiveVocabulary);
        setVocabularyLoadState('ready');
      })
      .catch((loadError) => {
        if (!isCurrentVocabularyRequest(requestId, vocabularyRequestRef.current, disposed)) return;
        const message = loadError instanceof Error ? loadError.message : String(loadError);
        setVocabularyLoadError(message);
        setVocabularyLoadState('error');
        console.error('Failed to load meeting vocabulary:', loadError);
      });

    return () => {
      disposed = true;
    };
  }, [open, meetingId]);

  // Listen for retranscription events
  useEffect(() => {
    if (!open) return;

    const unlisteners: UnlistenFn[] = [];
    const cleanedUpRef = { current: false };

    const setupListeners = async () => {
      // Progress events
      const unlistenProgress = await listen<RetranscriptionProgress>(
        'retranscription-progress',
        (event) => {
          if (event.payload.meeting_id === meetingId) {
            setProgress(event.payload);
          }
        }
      );
      if (cleanedUpRef.current) {
        unlistenProgress();
        return;
      }
      unlisteners.push(unlistenProgress);

      // Completion event
      const unlistenComplete = await listen<RetranscriptionResult>(
        'retranscription-complete',
        (event) => {
          if (event.payload.meeting_id === meetingId) {
            setIsProcessing(false);
            toast.success(
              t('retranscribe.completeToast', { count: event.payload.segments_count })
            );
            onCompleteRef.current?.();
            onOpenChangeRef.current(false);
          }
        }
      );
      if (cleanedUpRef.current) {
        unlistenComplete();
        unlisteners.forEach(u => u());
        return;
      }
      unlisteners.push(unlistenComplete);

      // Error event
      const unlistenError = await listen<RetranscriptionError>(
        'retranscription-error',
        (event) => {
          if (event.payload.meeting_id === meetingId) {
            setIsProcessing(false);
            setError(event.payload.error);
          }
        }
      );
      if (cleanedUpRef.current) {
        unlistenError();
        unlisteners.forEach(u => u());
        return;
      }
      unlisteners.push(unlistenError);
    };

    setupListeners();

    return () => {
      cleanedUpRef.current = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [open, meetingId, t]);

  const handleStartRetranscription = async () => {
    if (!meetingFolderPath) {
      setError(t('retranscribe.meetingFolderMissing'));
      return;
    }
    if (!canStartRetranscription(vocabularyLoadState)) {
      setError(
        vocabularyLoadState === 'error'
          ? t('retranscribe.vocabularyCouldNotLoad')
          : t('retranscribe.vocabularyStillLoading'),
      );
      return;
    }

    setIsProcessing(true);
    setError(null);
    setProgress(null);

    try {
      const languageToSend = isParakeetModel ? null : selectedLang === 'auto' ? null : selectedLang;
      if (!isParakeetModel) {
        await invoke('api_save_meeting_vocabulary', {
          meetingId,
          vocabulary: meetingVocabulary,
        });
      }

      await invoke('start_retranscription_command', {
        meetingId,
        meetingFolderPath,
        language: languageToSend,
        model: selectedModelDetails?.name || null,
        provider: selectedModelDetails?.provider || null,
      });
    } catch (err: unknown) {
      setIsProcessing(false);
      const errorMsg = err instanceof Error
        ? err.message
        : typeof err === 'string'
          ? err
          : String(err);
      setError(errorMsg);

    }
  };

  const handleCancel = async () => {
    if (isProcessing) {
      try {
        await invoke('cancel_retranscription_command');
        setIsProcessing(false);
        setProgress(null);
        toast.info(t('retranscribe.cancelled'));
      } catch (err) {
        console.error('Failed to cancel retranscription:', err);
      }
    }
    onOpenChange(false);
  };

  // Prevent closing during processing
  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen && isProcessing) {
      return;
    }
    onOpenChange(newOpen);
  };

  const handleEscapeKeyDown = (event: KeyboardEvent) => {
    if (isProcessing) {
      event.preventDefault();
    }
  };

  const handleInteractOutside = (event: Event) => {
    if (isProcessing) {
      event.preventDefault();
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        className="sm:max-w-[450px]"
        onEscapeKeyDown={handleEscapeKeyDown}
        onInteractOutside={handleInteractOutside}
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {isProcessing ? (
              <>
                <Loader2 className="h-5 w-5 animate-spin text-info" />
                {t('retranscribe.processingTitle')}
              </>
            ) : error ? (
              <>
                <AlertCircle className="h-5 w-5 text-destructive" />
                {t('retranscribe.failedTitle')}
              </>
            ) : (
              <>
                <RefreshCw className="h-5 w-5 text-info" />
                {t('retranscribe.title')}
              </>
            )}
          </DialogTitle>
          <DialogDescription>
            {isProcessing
              ? progress?.message || t('import.processing')
              : error
                ? t('retranscribe.failedTitle')
                : t('retranscribe.description')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {!isProcessing && !error && (
            !isParakeetModel ? (
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                  <Globe className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{t('common.language')}</span>
                </div>
                <Select value={selectedLang} onValueChange={setSelectedLang}>
                  <SelectTrigger className="w-full" aria-label={t('common.selectLanguage')}>
                    <SelectValue placeholder={t('common.selectLanguage')} />
                  </SelectTrigger>
                  <SelectContent className="max-h-60">
                    {LANGUAGES.map((lang) => (
                      <SelectItem key={lang.code} value={lang.code}>
                        {lang.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {t('retranscribe.selectLanguageHint')}
                </p>
              </div>
            ) : (
              <div className="space-y-3">
                <div className="flex items-center gap-2">
                  <Globe className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{t('common.language')}</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  {t('retranscribe.parakeetLanguage')}
                </p>
              </div>
            )
          )}

          {!isProcessing && !error && !isParakeetModel && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">{t('retranscribe.meetingVocabulary')}</span>
              </div>
              <Textarea
                value={meetingVocabulary}
                onChange={(event) => setMeetingVocabulary(event.target.value)}
                placeholder={t('retranscribe.vocabularyPlaceholder')}
                className="min-h-[76px] resize-y"
                disabled={isProcessing || !canStartRetranscription(vocabularyLoadState)}
              />
              {vocabularyLoadState === 'loading' && (
                <p className="text-xs text-muted-foreground" role="status">
                  {t('retranscribe.loadingVocabulary')}
                </p>
              )}
              {vocabularyLoadState === 'error' && (
                <p className="text-xs text-destructive" role="alert">
                  {t('retranscribe.vocabularyLoadFailed')}{vocabularyLoadError ? `: ${vocabularyLoadError}` : '.'}
                </p>
              )}
              <p className="text-xs text-muted-foreground">
                {t('retranscribe.vocabularyHint')}
                {effectiveVocabulary && t('retranscribe.changesSaved')}
              </p>
            </div>
          )}

          {!isProcessing && !error && availableModels.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">{t('common.model')}</span>
              </div>
              <Select value={selectedModelKey} onValueChange={setSelectedModelKey} disabled={loadingModels}>
                <SelectTrigger className="w-full" aria-label={loadingModels ? t('common.loadingModels') : t('common.selectModel')}>
                  <SelectValue placeholder={loadingModels ? t('common.loadingModels') : t('common.selectModel')} />
                </SelectTrigger>
                <SelectContent>
                  {availableModels.map((model) => (
                    <SelectItem key={`${model.provider}:${model.name}`} value={`${model.provider}:${model.name}`}>
                      {model.displayName} ({Math.round(model.size_mb)} MB)
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t('retranscribe.modelHint')}
              </p>
            </div>
          )}

          {isProcessing && progress && (
            <div className="space-y-2">
              <div className="relative">
                <div className="w-full bg-muted rounded-full h-3">
                  <div
                    className="bg-info h-3 rounded-full transition-[width] duration-300 ease-out"
                    style={{ width: `${Math.min(progress.progress_percentage, 100)}%` }}
                  />
                </div>
                <div className="flex justify-between text-xs text-muted-foreground mt-1">
                  <span>{progress.stage}</span>
                  <span>{Math.round(progress.progress_percentage)}%</span>
                </div>
              </div>
              <p className="text-sm text-muted-foreground text-center">
                {progress.message}
              </p>
            </div>
          )}

          {error && (
            <div className="bg-destructive/10 border border-destructive/30 rounded-lg p-3">
              <p className="text-sm text-destructive">{error}</p>
            </div>
          )}
        </div>

        <DialogFooter>
          {!isProcessing && !error && (
            <>
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {t('common.cancel')}
              </Button>
              <Button
                onClick={handleStartRetranscription}
                className="bg-info hover:bg-info/90"
                disabled={!meetingFolderPath || !canStartRetranscription(vocabularyLoadState)}
              >
                <RefreshCw className="h-4 w-4 mr-2" />
                {t('retranscribe.start')}
              </Button>
            </>
          )}
          {isProcessing && (
            <Button variant="outline" onClick={handleCancel}>
              <X className="h-4 w-4 mr-2" />
              {t('common.cancel')}
            </Button>
          )}
          {error && (
            <>
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                {t('common.close')}
              </Button>
              <Button
                onClick={() => {
                  setError(null);
                  setProgress(null);
                }}
                variant="outline"
              >
                {t('common.tryAgain')}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
