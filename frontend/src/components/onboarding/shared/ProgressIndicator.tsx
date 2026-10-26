import React from 'react';
import { Check, Lock, Download, CheckCircle2, BrainCircuit } from 'lucide-react';
import { ContextualIcon } from '@/components/ui/contextual-icon';

interface ProgressIndicatorProps {
  current: number;
  total: number;
  onStepClick?: (step: number) => void;
}

const stepIcons = [
  Lock,         // 1. Welcome
  BrainCircuit, // 2. Setup Overview
  Download,     // 3. Download Progress
  // Step 4 (Permissions) doesn't need icon - auto-skipped on non-macOS
];

export function ProgressIndicator({ current, total, onStepClick }: ProgressIndicatorProps) {
  const visibleSteps = Array.from({ length: total }, (_, i) => i + 1);

  return (
    <div className="mb-8">
      <div className="flex items-center justify-center gap-1 sm:gap-2">
        {visibleSteps.map((step, index) => {
          const isActive = step === current;
          const isCompleted = step < current;
          const isClickable = isCompleted && onStepClick;
          const StepIcon = stepIcons[step - 1] || CheckCircle2;

          return (
            <React.Fragment key={step}>
              {/* Step Circle */}
              <button
                onClick={() => isClickable && onStepClick(step)}
                disabled={!isClickable}
                aria-label={`Step ${step}: ${isCompleted ? 'completed' : isActive ? 'current' : 'pending'}`}
                aria-current={isActive ? 'step' : undefined}
                className={`relative flex h-10 w-10 shrink-0 items-center justify-center rounded-full transition-[transform,box-shadow] duration-300 motion-reduce:transform-none motion-reduce:transition-none ${
                  isClickable ? 'cursor-pointer motion-safe:hover:scale-110 hover:shadow-md' : 'cursor-default'
                }`}
              >
                <ContextualIcon
                  stateKey={isCompleted ? 'completed' : isActive ? 'active' : 'pending'}
                  className={`h-8 w-8 rounded-full ${
                    isCompleted
                      ? 'bg-success text-success-foreground'
                      : isActive
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted-foreground/30 text-muted-foreground'
                  }`}
                >
                  {isCompleted ? <Check className="h-4 w-4" /> : <StepIcon className="h-4 w-4" />}
                </ContextualIcon>
              </button>

              {/* Connector Line */}
              {index < visibleSteps.length - 1 && (
                <div
                  className={`h-0.5 w-2 shrink-0 transition-colors duration-300 sm:w-6 ${
                    isCompleted ? 'bg-success' : 'bg-muted-foreground/30'
                  }`}
                />
              )}
            </React.Fragment>
          );
        })}
      </div>
    </div>
  );
}
