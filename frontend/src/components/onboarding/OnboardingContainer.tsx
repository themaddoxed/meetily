import React from 'react';
import { motion } from 'framer-motion';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ProgressIndicator } from './shared/ProgressIndicator';
import { useOnboarding } from '@/contexts/OnboardingContext';
import type { OnboardingContainerProps } from '@/types/onboarding';
import { createStepItemVariants } from '@/lib/motion';
import { useLiveReducedMotion } from '@/hooks/use-live-reduced-motion';

export function OnboardingContainer({
  title,
  description,
  children,
  step,
  totalSteps = 5,
  stepOffset = 0,
  hideProgress = false,
  className,
  showNavigation = false,
  onNext,
  onPrevious,
  canGoNext = true,
  canGoPrevious = true,
}: OnboardingContainerProps) {
  const { goToStep, goPrevious, goNext } = useOnboarding();
  const reduceMotion = useLiveReducedMotion();
  const itemVariants = createStepItemVariants(reduceMotion);

  const handlePrevious = () => {
    if (onPrevious) {
      onPrevious();
    } else {
      goPrevious();
    }
  };

  const handleNext = () => {
    if (onNext) {
      onNext();
    } else {
      goNext();
    }
  };

  const handleStepClick = (s: number) => {
    goToStep(s + stepOffset);
  };

  return (
    <div className="fixed inset-0 bg-muted/40 flex items-center justify-center z-50 overflow-hidden">
      <div className={cn('w-full max-w-2xl h-full max-h-screen flex flex-col px-6 py-6', className)}>
        {/* Progress Indicator with Navigation - Fixed */}
        {step && !hideProgress && (
          <motion.div className="mb-2 relative flex-shrink-0" variants={itemVariants}>
            {/* Navigation Buttons */}
            {showNavigation && (
              <div className="absolute top-1/2 -translate-y-1/2 left-0 right-0 flex justify-between pointer-events-none">
                <button
                  onClick={handlePrevious}
                  disabled={!canGoPrevious || step === 1}
                  className={cn(
                    'pointer-events-auto w-10 h-10 rounded-full bg-card border border-border shadow-sm flex items-center justify-center transition-[color,background-color,box-shadow,transform,opacity] duration-200 motion-reduce:transform-none motion-reduce:transition-none',
                    canGoPrevious && step !== 1
                      ? 'hover:bg-muted/40 hover:shadow-md motion-safe:hover:scale-110 text-foreground'
                      : 'opacity-0 cursor-not-allowed'
                  )}
                >
                  <ChevronLeft className="w-4 h-4" />
                </button>

                <button
                  onClick={handleNext}
                  disabled={!canGoNext || step === totalSteps}
                  className={cn(
                    'pointer-events-auto w-10 h-10 rounded-full bg-card border border-border shadow-sm flex items-center justify-center transition-[color,background-color,box-shadow,transform,opacity] duration-200 motion-reduce:transform-none motion-reduce:transition-none',
                    canGoNext && step !== totalSteps
                      ? 'hover:bg-muted/40 hover:shadow-md motion-safe:hover:scale-110 text-foreground'
                      : 'opacity-0 cursor-not-allowed'
                  )}
                >
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            )}

            {/* Progress Indicator */}
            <ProgressIndicator current={step} total={totalSteps} onStepClick={handleStepClick} />
          </motion.div>
        )}

        {/* Header - Fixed */}
        <motion.div className="mb-4 text-center space-y-3 flex-shrink-0" variants={itemVariants}>
          <h1 className="text-balance text-4xl font-semibold text-foreground">{title}</h1>
          {description && (
            <p className="mx-auto max-w-md text-pretty text-base text-muted-foreground">
              {description}
            </p>
          )}
        </motion.div>

        {/* Content - Scrollable */}
        <motion.div className="flex-1 overflow-y-auto pr-2" variants={itemVariants}>
          <div className="space-y-6">{children}</div>
        </motion.div>
      </div>
    </div>
  );
}
