import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { useRecordingState } from '@/contexts/RecordingStateContext';

export interface DiarizationProgress {
  meetingId: string;
  stage: string;
  progress: number;
  message: string;
}

interface DiarizationRunResult {
  segmentCount: number;
  speakerCount: number;
}

interface UseDiarizationRunOptions {
  meetingId: string;
  onRefetchTranscripts?: () => Promise<void>;
}

const ERROR_COPY: Record<string, { title: string; description: string }> = {
  already_running: {
    title: 'Speaker detection is already running',
    description: 'Wait for the current run to finish.',
  },
  feature_disabled: {
    title: 'Speaker detection is disabled',
    description: 'Enable post-call speaker detection in Settings.',
  },
  inference_failed: {
    title: 'Could not detect speakers',
    description: 'Keep the recording and try again.',
  },
  insufficient_speech: {
    title: 'Not enough speech to detect speakers',
    description: 'The transcript and existing labels were not changed.',
  },
  invalid_audio: {
    title: 'The recording could not be read',
    description: 'Open the recording folder and check the audio file.',
  },
  invalid_output: {
    title: 'Speaker results need another run',
    description: 'The transcript and existing labels were not changed.',
  },
  meeting_not_found: {
    title: 'Meeting not found',
    description: 'Return to the meetings list and open the meeting again.',
  },
  missing_recording: {
    title: 'Recording file not found',
    description: 'The transcript remains available, but speaker detection needs the audio file.',
  },
  model_invalid: {
    title: 'Offline speaker models are invalid',
    description: 'Reinstall this Meetily build before trying again.',
  },
  model_unavailable: {
    title: 'Offline speaker models are missing',
    description: 'Reinstall this Meetily build before trying again.',
  },
  persistence_failed: {
    title: 'Speaker labels could not be saved',
    description: 'The transcript and existing labels were not changed.',
  },
  recording_active: {
    title: 'Stop recording first',
    description: 'Speaker detection runs after the recording is finalized.',
  },
  recording_incomplete: {
    title: 'Recording is still being finalized',
    description: 'Wait a moment, then try speaker detection again.',
  },
};

function errorCode(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error) {
    return String(error.message);
  }
  return 'inference_failed';
}

export function useDiarizationRun({
  meetingId,
  onRefetchTranscripts,
}: UseDiarizationRunOptions) {
  const [isRunning, setIsRunning] = useState(false);
  const { snapshot } = useRecordingState();
  const isRecording = Boolean(snapshot && !['completed', 'failed', 'interrupted'].includes(snapshot.state));
  const [progress, setProgress] = useState<DiarizationProgress | null>(null);
  const runTokenRef = useRef(0);
  const onRefetchTranscriptsRef = useRef(onRefetchTranscripts);

  useEffect(() => {
    onRefetchTranscriptsRef.current = onRefetchTranscripts;
  }, [onRefetchTranscripts]);

  useEffect(() => {
    let ignore = false;
    const unlisteners: UnlistenFn[] = [];
    runTokenRef.current += 1;
    setIsRunning(false);
    setProgress(null);

    const subscribe = async () => {
      const unlistenProgress = await listen<DiarizationProgress>('meeting-diarization-progress', (event) => {
        if (ignore || event.payload.meetingId !== meetingId) return;
        setProgress(event.payload);
      });
      if (ignore) {
        unlistenProgress();
        return;
      }
      unlisteners.push(unlistenProgress);

    };

    void subscribe().catch((subscriptionError) => {
      unlisteners.forEach((unlisten) => unlisten());
      if (!ignore) {
        console.error('Failed to subscribe to diarization events:', subscriptionError);
      }
    });
    return () => {
      ignore = true;
      runTokenRef.current += 1;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [meetingId]);

  const runDiarization = useCallback(async () => {
    if (!meetingId || isRunning || isRecording) return;
    const runToken = ++runTokenRef.current;
    setIsRunning(true);
    setProgress({
      meetingId,
      stage: 'starting',
      progress: 0,
      message: 'Starting speaker detection',
    });

    try {
      const result = await invoke<DiarizationRunResult>('run_meeting_diarization', {
        request: { meetingId },
      });
      if (runTokenRef.current !== runToken) return;

      try {
        await onRefetchTranscriptsRef.current?.();
      } catch (refetchError) {
        console.error('Speaker labels were saved but transcript refresh failed:', refetchError);
        toast.warning('Speaker labels were saved', {
          description: 'Reopen the meeting to refresh the transcript.',
        });
        return;
      }
      if (runTokenRef.current !== runToken) return;
      toast.success('Speakers detected', {
        description: `${result.speakerCount} speaker${result.speakerCount === 1 ? '' : 's'} found.`,
      });
      setProgress(null);
    } catch (error) {
      if (runTokenRef.current !== runToken) return;
      const code = errorCode(error);
      const copy = ERROR_COPY[code] ?? ERROR_COPY.inference_failed;
      toast.error(copy.title, { description: copy.description });
      setProgress(null);
    } finally {
      if (runTokenRef.current === runToken) setIsRunning(false);
    }
  }, [isRecording, isRunning, meetingId]);

  return {
    isRecording,
    isRunning,
    progress,
    runDiarization,
  };
}
