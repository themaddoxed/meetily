'use client';

import React, { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';
import {
  recordingService,
  type RecordingSessionSnapshot,
} from '@/services/recordingService';
import { useI18n } from '@/contexts/I18nContext';

export enum RecordingStatus {
  IDLE = 'idle',
  STARTING = 'starting',
  RECORDING = 'recording',
  STOPPING = 'stopping',
  PROCESSING_TRANSCRIPTS = 'processing',
  SAVING = 'saving',
  COMPLETED = 'completed',
  ERROR = 'error',
}

const NONTERMINAL = new Set(['starting', 'recording', 'paused', 'stopping', 'finalizing']);

interface RecordingStateContextType {
  snapshot: RecordingSessionSnapshot | null;
  sessionId: string | null;
  isRecording: boolean;
  isPaused: boolean;
  isActive: boolean;
  recordingDuration: number | null;
  activeDuration: number | null;
  status: RecordingStatus;
  statusMessage?: string;
  setStatus: (status: RecordingStatus, message?: string) => void;
  projectSnapshot: (snapshot: RecordingSessionSnapshot | null) => void;
  refreshSnapshot: (sessionId?: string) => Promise<RecordingSessionSnapshot | null>;
  isStopping: boolean;
  isProcessing: boolean;
  isSaving: boolean;
}

const RecordingStateContext = createContext<RecordingStateContextType | null>(null);

export const useRecordingState = () => {
  const context = useContext(RecordingStateContext);
  if (!context) throw new Error('useRecordingState must be used within a RecordingStateProvider');
  return context;
};

export function RecordingStateProvider({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
  const translateRef = useRef(t);
  translateRef.current = t;
  const [snapshot, setSnapshot] = useState<RecordingSessionSnapshot | null>(null);
  const [status, setStatusValue] = useState(RecordingStatus.IDLE);
  const [statusMessage, setStatusMessage] = useState<string>();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const snapshotRef = useRef<RecordingSessionSnapshot | null>(null);
  const refreshGenerationRef = useRef(0);

  const applySnapshot = useCallback((next: RecordingSessionSnapshot | null) => {
    snapshotRef.current = next;
    setSnapshot(next);
    if (!next) {
      setStatusValue(RecordingStatus.IDLE);
    } else if (next.state === 'starting') {
      setStatusValue(RecordingStatus.STARTING);
    } else if (next.state === 'recording' || next.state === 'paused') {
      setStatusValue(RecordingStatus.RECORDING);
    } else if (next.state === 'stopping' || next.state === 'finalizing') {
      setStatusValue(RecordingStatus.STOPPING);
    } else if (next.state === 'completed') {
      setStatusValue(RecordingStatus.COMPLETED);
    } else if (next.state === 'failed' || next.state === 'interrupted') {
      setStatusValue(RecordingStatus.ERROR);
    }
  }, []);

  const projectSnapshot = useCallback((next: RecordingSessionSnapshot | null) => {
    refreshGenerationRef.current += 1;
    applySnapshot(next);
  }, [applySnapshot]);

  const refreshSnapshot = useCallback(async (sessionId?: string) => {
    const generation = ++refreshGenerationRef.current;
    const next = sessionId
      ? await recordingService.getRecordingSession(sessionId)
      : await recordingService.getActiveRecordingSession();
    if (generation !== refreshGenerationRef.current) return snapshotRef.current;
    if (sessionId && next?.sessionId !== sessionId) return snapshotRef.current;
    const current = snapshotRef.current;
    if (
      sessionId &&
      current &&
      current.sessionId !== sessionId &&
      NONTERMINAL.has(current.state)
    ) return current;
    applySnapshot(next);
    return next;
  }, [applySnapshot]);

  const setStatus = useCallback((next: RecordingStatus, message?: string) => {
    setStatusValue(next);
    setStatusMessage(message);
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const retain = (unlisten: () => void) => disposed ? unlisten() : unlisteners.push(unlisten);
    const refreshWakeUp = (sessionId: string) => {
      if (disposed) return;
      void refreshSnapshot(sessionId).catch((error) => {
        console.error('[RecordingState] Failed to reconcile session snapshot:', error);
      });
    };

    void recordingService.onRecordingStarted(refreshWakeUp).then(retain);
    void recordingService.onRecordingStopped(refreshWakeUp).then(retain);
    void recordingService.onRecordingPaused(refreshWakeUp).then(retain);
    void recordingService.onRecordingResumed(refreshWakeUp).then(retain);
    void recordingService.onAudioRouteChanged((event) => {
      if (disposed || snapshotRef.current?.sessionId !== event.sessionId) return;
      toast.warning(
        translateRef.current('recording.microphoneDisconnectedContinues', {
          previousDevice: event.previousDevice,
          currentDevice: event.currentDevice,
        }),
        { duration: 8000 },
      );
      refreshWakeUp(event.sessionId);
    }).then(retain);
    void recordingService.onAudioDegraded((event) => {
      if (disposed || snapshotRef.current?.sessionId !== event.sessionId) return;
      toast.warning(translateRef.current('recording.systemAudioStoppedContinues'), { duration: 8000 });
    }).then(retain);
    void refreshSnapshot().catch((error) => {
      console.error('[RecordingState] Failed to restore active session:', error);
    });

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [refreshSnapshot]);

  useEffect(() => {
    if (timerRef.current) clearInterval(timerRef.current);
    timerRef.current = null;
    if (!snapshot || !NONTERMINAL.has(snapshot.state)) return;
    timerRef.current = setInterval(() => {
      const sessionId = snapshotRef.current?.sessionId;
      if (sessionId) void refreshSnapshot(sessionId);
    }, 1000);
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
      timerRef.current = null;
    };
  }, [refreshSnapshot, snapshot]);

  const value = useMemo<RecordingStateContextType>(() => ({
    snapshot,
    sessionId: snapshot?.sessionId ?? null,
    isRecording: Boolean(snapshot && NONTERMINAL.has(snapshot.state)),
    isPaused: snapshot?.state === 'paused',
    isActive: snapshot?.state === 'recording',
    recordingDuration: snapshot?.recordingDuration ?? null,
    activeDuration: snapshot?.activeDuration ?? null,
    status,
    statusMessage,
    setStatus,
    projectSnapshot,
    refreshSnapshot,
    isStopping: status === RecordingStatus.STOPPING,
    isProcessing: status === RecordingStatus.PROCESSING_TRANSCRIPTS,
    isSaving: status === RecordingStatus.SAVING,
  }), [projectSnapshot, refreshSnapshot, setStatus, snapshot, status, statusMessage]);

  return <RecordingStateContext.Provider value={value}>{children}</RecordingStateContext.Provider>;
}
