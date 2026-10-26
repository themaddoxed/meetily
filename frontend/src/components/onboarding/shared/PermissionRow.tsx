import React from 'react';
import { CheckCircle2, Loader2, XCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useI18n } from '@/contexts/I18nContext';
import {
  microphoneReady,
  permissionAction,
  systemAudioReady,
  type MicrophonePermissionState,
  type SystemAudioAccessState,
} from '@/lib/audioPermissions';

type PermissionRowProps = {
  icon: React.ReactNode;
  title: string;
  description: string;
  kind: 'microphone';
  state: MicrophonePermissionState;
  isPending?: boolean;
  onAction: () => void;
  onOpenSettings?: never;
} | {
  icon: React.ReactNode;
  title: string;
  description: string;
  kind: 'systemAudio';
  state: SystemAudioAccessState;
  isPending?: boolean;
  onAction: () => void;
  onOpenSettings?: () => void;
};

export function PermissionRow({
  icon,
  title,
  description,
  kind,
  state,
  isPending = false,
  onAction,
  onOpenSettings,
}: PermissionRowProps) {
  const { t } = useI18n();
  const isAuthorized = kind === 'microphone'
    ? microphoneReady({ microphone: state, systemAudio: 'unknown' })
    : systemAudioReady({ microphone: 'not_determined', systemAudio: state });
  const action = kind === 'microphone'
    ? permissionAction(kind, state)
    : permissionAction(kind, state);
  const needsAttention = !isAuthorized && action.blocking;
  const actionLabel = kind === 'systemAudio' && state === 'denied'
    ? t('permission.tryAgain')
    : action.label === 'Access Granted'
      ? t('permission.accessGranted')
      : action.label === 'Enable'
        ? t('permission.enable')
        : action.label === 'Open Settings'
          ? t('permission.openSettings')
          : action.label === 'Try Again'
            ? t('permission.tryAgain')
            : action.label;

  return (
    <div
      className={cn(
        'flex items-center justify-between rounded-2xl border px-6 py-5',
        'transition-[color,background-color,border-color,box-shadow] duration-200',
        isAuthorized
          ? 'border-primary bg-muted'
          : needsAttention
            ? 'border-destructive/40 bg-destructive/10'
            : 'bg-card border-border',
      )}
    >
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <div
          className={cn(
            'flex size-10 flex-shrink-0 items-center justify-center rounded-full',
            isAuthorized ? 'bg-muted' : needsAttention ? 'bg-destructive/15' : 'bg-muted/40',
          )}
        >
          <div className={cn(isAuthorized ? 'text-foreground' : needsAttention ? 'text-destructive' : 'text-muted-foreground')}>
            {icon}
          </div>
        </div>

        <div className="min-w-0 flex-1">
          <div className="truncate font-medium text-foreground">{title}</div>
          <div className="text-sm text-muted-foreground">
            {isAuthorized ? (
              <span className="flex items-center gap-1 text-success">
                <CheckCircle2 className="h-3.5 w-3.5" />
                {t('permission.accessGranted')}
              </span>
            ) : needsAttention ? (
              <span className="flex items-center gap-1 text-destructive">
                <XCircle className="h-3.5 w-3.5" />
                {t('permission.accessNeedsAttention')}
              </span>
            ) : (
              <span>{description}</span>
            )}
          </div>
        </div>
      </div>

      <div className="ml-3 flex flex-shrink-0 items-center gap-2">
        {!isAuthorized && (
          <>
            {onOpenSettings && (
              <Button
                variant="outline"
                size="sm"
                onClick={onOpenSettings}
                disabled={isPending}
              >
                {t('permission.openSettings')}
              </Button>
            )}
            <Button
              variant={needsAttention ? 'destructive' : 'outline'}
              size="sm"
              onClick={onAction}
              disabled={isPending}
              className="min-w-[100px]"
            >
              {isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {isPending ? t('permission.checking') : actionLabel}
            </Button>
          </>
        )}
        {isAuthorized && (
          <div className="flex size-8 items-center justify-center rounded-full bg-success/15">
            <CheckCircle2 className="h-4 w-4 text-success" />
          </div>
        )}
      </div>
    </div>
  );
}
