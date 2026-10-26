'use client';

import { motion } from 'framer-motion';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useEffect, useState } from 'react';
import { ContextualIcon } from '@/components/ui/contextual-icon';
import { useLiveReducedMotion } from '@/hooks/use-live-reduced-motion';

interface RecordingStatusBarProps {
  isPaused?: boolean;
}

export const RecordingStatusBar: React.FC<RecordingStatusBarProps> = ({ isPaused = false }) => {
  // Get recording duration from backend-synced context (in seconds)
  // Backend polls every 500ms, providing smooth updates
  const { activeDuration } = useRecordingState();
  const reduceMotion = useLiveReducedMotion();

  // Display state synced from backend
  const [displaySeconds, setDisplaySeconds] = useState(0);

  // Sync with backend duration when it changes (handles refresh/navigation)
  useEffect(() => {
    if (activeDuration !== null) {
      // Round to nearest second to avoid decimal issues
      setDisplaySeconds(Math.floor(activeDuration));
    }
  }, [activeDuration]);

  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <motion.div
      initial={reduceMotion ? { opacity: 0 } : { opacity: 0, y: -10 }}
      animate={reduceMotion ? { opacity: 1 } : { opacity: 1, y: 0 }}
      exit={reduceMotion ? { opacity: 0 } : { opacity: 0, y: -10 }}
      transition={{ duration: 0.2 }}
      className="flex items-center gap-2 px-3 py-2 bg-muted/40 rounded-lg mb-2"
    >
      <ContextualIcon stateKey={isPaused ? 'paused' : 'recording'} className="h-3 w-3">
        <span className={`h-2 w-2 rounded-full ${isPaused ? 'bg-warning' : 'bg-recording animate-pulse motion-reduce:animate-none'}`} />
      </ContextualIcon>
      <span className={`tabular-nums text-sm ${isPaused ? 'text-warning' : 'text-recording'}`}>
        {isPaused ? 'Paused' : 'Recording'} • {formatDuration(displaySeconds)}
      </span>
    </motion.div>
  );
};
