import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  microphoneReady,
  systemAudioReady,
  type AudioPermissionSnapshot,
} from '@/lib/audioPermissions';

const initialSnapshot: AudioPermissionSnapshot = {
  microphone: 'not_determined',
  systemAudio: 'unknown',
};

export function usePermissionCheck() {
  const [snapshot, setSnapshot] = useState<AudioPermissionSnapshot>(initialSnapshot);
  const [isChecking, setIsChecking] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generationRef = useRef(0);

  const applySnapshot = useCallback((generation: number, next: AudioPermissionSnapshot) => {
    if (generation !== generationRef.current) return;
    setSnapshot(next);
    setIsChecking(false);
    setError(null);
  }, []);

  const checkPermissions = useCallback(async () => {
    const generation = ++generationRef.current;
    setIsChecking(true);
    setError(null);

    try {
      const next = await invoke<AudioPermissionSnapshot>('get_audio_permission_snapshot');
      applySnapshot(generation, next);
      return next;
    } catch (cause) {
      if (generation === generationRef.current) {
        setIsChecking(false);
        setError(cause instanceof Error ? cause.message : 'Failed to check permissions');
      }
      return null;
    }
  }, [applySnapshot]);

  const requestPermission = useCallback(async (
    command: 'request_microphone_permission' | 'request_system_audio_permission',
  ) => {
    const generation = ++generationRef.current;
    setIsChecking(true);
    setError(null);

    try {
      const next = await invoke<AudioPermissionSnapshot>(command);
      applySnapshot(generation, next);
      return next;
    } catch (cause) {
      if (generation === generationRef.current) {
        setIsChecking(false);
        setError(cause instanceof Error ? cause.message : 'Failed to request permission');
      }
      return null;
    }
  }, [applySnapshot]);

  const requestMicrophone = useCallback(
    () => requestPermission('request_microphone_permission'),
    [requestPermission],
  );
  const requestSystemAudio = useCallback(
    () => requestPermission('request_system_audio_permission'),
    [requestPermission],
  );
  const openSettings = useCallback(
    (kind: 'microphone' | 'systemAudio') =>
      invoke('open_audio_permission_settings', { kind }),
    [],
  );

  useEffect(() => {
    void checkPermissions();
    const handleFocus = () => void checkPermissions();
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [checkPermissions]);

  return {
    snapshot,
    hasMicrophone: microphoneReady(snapshot),
    hasSystemAudio: systemAudioReady(snapshot),
    isChecking,
    error,
    checkPermissions,
    requestMicrophone,
    requestSystemAudio,
    openSettings,
  };
}
