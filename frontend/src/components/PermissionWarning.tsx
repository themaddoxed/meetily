import React from 'react';
import { AlertTriangle, Mic, RefreshCw, Speaker } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { useIsLinux } from '@/hooks/usePlatform';
import { useI18n } from '@/contexts/I18nContext';
import type {
  MicrophonePermissionState,
  SystemAudioAccessState,
} from '@/lib/audioPermissions';

interface PermissionWarningProps {
  microphone: MicrophonePermissionState;
  systemAudio: SystemAudioAccessState;
  onRecheck: () => void;
  requestMicrophone: () => Promise<unknown>;
  requestSystemAudio: () => Promise<unknown>;
  openSettings: (kind: 'microphone' | 'systemAudio') => Promise<unknown>;
  isRechecking?: boolean;
  isRecording?: boolean;
}

export function PermissionWarning({
  microphone,
  systemAudio,
  onRecheck,
  requestMicrophone,
  requestSystemAudio,
  openSettings,
  isRechecking = false,
  isRecording = false,
}: PermissionWarningProps) {
  const isLinux = useIsLinux();
  const { t } = useI18n();

  if (isLinux || (microphone === 'authorized' && systemAudio === 'available')) {
    return null;
  }

  const microphoneNeedsAccess = microphone !== 'authorized';
  const canOpenMicrophoneSettings = microphone === 'denied' || microphone === 'restricted';
  const microphoneCanRetry = microphone === 'not_determined' || microphone === 'unavailable';

  return (
    <div className="mb-4 max-w-md space-y-3">
      {microphoneNeedsAccess && (
        <Alert variant="destructive" className="border-warning/30 bg-warning/10">
          <AlertTriangle className="h-5 w-5 text-warning" />
          <AlertTitle className="flex items-center gap-2 font-semibold text-warning">
            <Mic className="h-4 w-4" />
            {t('permission.microphoneRequired')}
          </AlertTitle>
          <AlertDescription className="mt-2 text-warning">
            {t('permission.microphoneAccessDescription')}
          </AlertDescription>
          <div className="mt-4 flex flex-wrap gap-2">
            {microphoneCanRetry && (
              <button
                onClick={() => void requestMicrophone()}
                disabled={isRechecking || isRecording}
                className="inline-flex items-center gap-2 rounded-md bg-warning px-4 py-2 text-sm font-medium text-warning-foreground transition-colors hover:bg-warning/90 disabled:opacity-50"
              >
                <Mic className="h-4 w-4" />
                {microphone === 'not_determined'
                  ? t('permission.enableMicrophone')
                  : t('permission.tryAgain')}
              </button>
            )}
            {canOpenMicrophoneSettings && (
              <button
                onClick={() => void openSettings('microphone')}
                className="inline-flex items-center gap-2 rounded-md bg-warning px-4 py-2 text-sm font-medium text-warning-foreground transition-colors hover:bg-warning/90"
              >
                <Mic className="h-4 w-4" />
                {t('permission.openMicrophoneSettings')}
              </button>
            )}
            <button
              onClick={onRecheck}
              disabled={isRechecking || isRecording}
              className="inline-flex items-center gap-2 rounded-md bg-warning/15 px-4 py-2 text-sm font-medium text-warning transition-colors hover:bg-warning/20 disabled:opacity-50"
            >
              <RefreshCw className={`h-4 w-4 ${isRechecking ? 'animate-spin' : ''}`} />
              {t('permission.refreshStatus')}
            </button>
          </div>
        </Alert>
      )}

      {systemAudio !== 'available' && (
        <Alert className="border-info/30 bg-info/10">
          <Speaker className="h-5 w-5 text-info" />
          <AlertTitle className="font-semibold text-info">{t('permission.audioCaptureTitle')}</AlertTitle>
          <AlertDescription className="mt-2 text-info">
            {t(systemAudio === 'unknown'
              ? 'permission.systemAudioUnconfirmed'
              : 'permission.systemAudioMicrophoneOnly')}
          </AlertDescription>
          <div className="mt-4 flex flex-wrap gap-2">
            {(systemAudio === 'denied' || systemAudio === 'unknown') && (
              <button
                onClick={() => void openSettings('systemAudio')}
                className="inline-flex items-center gap-2 rounded-md bg-info px-4 py-2 text-sm font-medium text-info-foreground transition-colors hover:bg-info/90"
              >
                <Speaker className="h-4 w-4" />
                {t('permission.openAudioCaptureSettings')}
              </button>
            )}
            <button
              onClick={() => void requestSystemAudio()}
              disabled={isRechecking || isRecording}
              className="inline-flex items-center gap-2 rounded-md bg-info px-4 py-2 text-sm font-medium text-info-foreground transition-colors hover:bg-info/90 disabled:opacity-50"
            >
              <Speaker className="h-4 w-4" />
              {t('permission.tryAgain')}
            </button>
            <button
              onClick={onRecheck}
              disabled={isRechecking || isRecording}
              className="inline-flex items-center gap-2 rounded-md bg-info/15 px-4 py-2 text-sm font-medium text-info transition-colors hover:bg-info/20 disabled:opacity-50"
            >
              <RefreshCw className={`h-4 w-4 ${isRechecking ? 'animate-spin' : ''}`} />
              {t('permission.refreshStatus')}
            </button>
          </div>
        </Alert>
      )}
    </div>
  );
}
