import React from 'react';
import { Mic, Volume2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { PermissionRow } from '../shared';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { useI18n } from '@/contexts/I18nContext';

export function PermissionsStep() {
  const { t } = useI18n();
  const { completeOnboarding } = useOnboarding();
  const {
    snapshot,
    isChecking,
    requestMicrophone,
    requestSystemAudio,
    openSettings,
  } = usePermissionCheck();

  const handleMicrophoneAction = () => {
    if (snapshot.microphone === 'denied' || snapshot.microphone === 'restricted') {
      return openSettings('microphone');
    }
    return requestMicrophone();
  };

  const handleSystemAudioAction = () => {
    return requestSystemAudio();
  };

  const handleFinish = async () => {
    try {
      await completeOnboarding();
      window.location.reload();
    } catch (error) {
      console.error('Failed to complete onboarding:', error);
    }
  };

  const canFinish = snapshot.microphone === 'authorized';

  return (
    <OnboardingContainer
      title={t('onboarding.permissionsTitle')}
      description={t('onboarding.permissionsDescription')}
      step={4}
      hideProgress={true}
      showNavigation={canFinish}
      canGoNext={canFinish}
    >
      <div className="mx-auto max-w-lg space-y-6">
        <div className="space-y-4">
          <PermissionRow
            icon={<Mic className="h-5 w-5" />}
            title={t('settings.microphone')}
            description={t('onboarding.microphoneDescription')}
            kind="microphone"
            state={snapshot.microphone}
            isPending={isChecking}
            onAction={() => void handleMicrophoneAction()}
          />
          <PermissionRow
            icon={<Volume2 className="h-5 w-5" />}
            title={t('onboarding.systemAudioOptionalTitle')}
            description={t(snapshot.systemAudio === 'unknown'
              ? 'permission.systemAudioUnconfirmed'
              : 'onboarding.systemAudioDescription')}
            kind="systemAudio"
            state={snapshot.systemAudio}
            isPending={isChecking}
            onAction={() => void handleSystemAudioAction()}
            onOpenSettings={
              (snapshot.systemAudio === 'denied' || snapshot.systemAudio === 'unknown')
                ? () => void openSettings('systemAudio')
                : undefined
            }
          />
        </div>

        <div className="flex flex-col gap-3 pt-4">
          <Button onClick={handleFinish} disabled={!canFinish} className="h-11 w-full">
            {t('onboarding.finishSetup')}
          </Button>
          {!canFinish && (
            <p className="text-center text-xs text-muted-foreground">
              {t('onboarding.grantMicrophoneToFinish')}
            </p>
          )}
        </div>
      </div>
    </OnboardingContainer>
  );
}
