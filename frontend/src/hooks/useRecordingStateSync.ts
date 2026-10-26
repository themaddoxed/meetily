import { useState } from 'react';
import { useRecordingState } from '@/contexts/RecordingStateContext';

interface UseRecordingStateSyncReturn {
  isBackendRecording: boolean;
  isRecordingDisabled: boolean;
  setIsRecordingDisabled: (value: boolean) => void;
}

export function useRecordingStateSync(): UseRecordingStateSyncReturn {
  const { isRecording } = useRecordingState();
  const [isRecordingDisabled, setIsRecordingDisabled] = useState(false);
  return {
    isBackendRecording: isRecording,
    isRecordingDisabled,
    setIsRecordingDisabled,
  };
}
