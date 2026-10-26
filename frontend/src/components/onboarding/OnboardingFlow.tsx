import React, { useEffect } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { createStepContainerVariants } from '@/lib/motion';
import {
  WelcomeStep,
  PermissionsStep,
  DownloadProgressStep,
  SetupOverviewStep,
} from './steps';
import { useLiveReducedMotion } from '@/hooks/use-live-reduced-motion';

export function OnboardingFlow() {
  const { currentStep } = useOnboarding();
  const [isMac, setIsMac] = React.useState(false);
  const reduceMotion = useLiveReducedMotion();

  useEffect(() => {
    // Check if running on macOS
    const checkPlatform = async () => {
      try {
        // Dynamic import to avoid SSR issues if any
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        console.error('Failed to detect platform:', e);
        // Fallback
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  // 4-Step Onboarding Flow:
  // Step 1: Welcome - Introduce Meetily features
  // Step 2: Setup Overview - Database initialization + summary model selection
  // Step 3: Download Progress - Download Parakeet + selected Summary Model
  // Step 4: Permissions - Request mic + system audio (macOS only)

  const stepContent = currentStep === 1
    ? <WelcomeStep />
    : currentStep === 2
      ? <SetupOverviewStep />
      : currentStep === 3
        ? <DownloadProgressStep />
        : currentStep === 4 && isMac
          ? <PermissionsStep />
          : null;

  return (
    <AnimatePresence initial={false} mode="wait">
      <motion.div
        key={currentStep}
        className="onboarding-flow"
        data-reduced-motion={reduceMotion}
        variants={createStepContainerVariants(reduceMotion)}
        initial="hidden"
        animate="visible"
        exit="exit"
      >
        {stepContent}
      </motion.div>
    </AnimatePresence>
  );
}
