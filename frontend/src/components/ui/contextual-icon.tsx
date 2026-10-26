'use client';

import type { ReactNode } from 'react';
import { AnimatePresence, motion } from 'framer-motion';

import { iconSpring } from '@/lib/motion';
import { cn } from '@/lib/utils';
import { useLiveReducedMotion } from '@/hooks/use-live-reduced-motion';

interface ContextualIconProps {
  stateKey: string;
  children: ReactNode;
  className?: string;
}

export function ContextualIcon({
  stateKey,
  children,
  className,
}: ContextualIconProps) {
  const reduceMotion = useLiveReducedMotion();

  return (
    <span
      aria-hidden="true"
      data-reduced-motion={reduceMotion}
      className={cn('relative inline-flex shrink-0', className)}
    >
      <AnimatePresence initial={false} mode="popLayout">
        <motion.span
          key={stateKey}
          className="absolute inset-0 flex items-center justify-center"
          initial={reduceMotion
            ? { opacity: 0 }
            : { opacity: 0, scale: 0.25, filter: 'blur(4px)' }}
          animate={{ opacity: 1, scale: 1, filter: 'blur(0px)' }}
          exit={reduceMotion
            ? { opacity: 0 }
            : { opacity: 0, scale: 0.25, filter: 'blur(4px)' }}
          transition={reduceMotion ? { duration: 0.1 } : iconSpring}
        >
          {children}
        </motion.span>
      </AnimatePresence>
    </span>
  );
}
