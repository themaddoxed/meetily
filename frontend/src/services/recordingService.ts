import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type RecordingSessionState =
  | 'starting'
  | 'recording'
  | 'paused'
  | 'stopping'
  | 'finalizing'
  | 'completed'
  | 'interrupted'
  | 'failed';

export type RecordingSourceKind = 'microphone' | 'system';

export interface SourceControl {
  muted: boolean;
  gain: number;
}

export interface CapturedTrack {
  kind: 'microphone' | 'system';
  sampleRate: number;
  channels: number;
  path: string;
  byteLength: number;
  sampleCount: number;
  sha256: string;
}

export interface ContractError {
  code: string;
  message: string;
  retryable: boolean;
  sessionId?: string;
  currentState?: RecordingSessionState;
  details?: string;
}

export function contractErrorFrom(value: unknown): ContractError | null {
  if (!value || typeof value !== 'object') return null;
  const candidate = value as Partial<ContractError>;
  return typeof candidate.code === 'string' && typeof candidate.message === 'string'
    ? candidate as ContractError
    : null;
}

export interface StartRecordingRequest {
  meetingId: string;
  idempotencyKey: string;
  microphoneDevice: string | null;
  microphoneDeviceExplicit?: boolean;
  systemDevice: string | null;
  captureSystem?: boolean;
  meetingName: string;
  meetingVocabulary?: string;
  savePath?: string;
  autoSave?: boolean;
}

export interface RecordingSessionSnapshot {
  sessionId: string;
  meetingId: string;
  meetingVocabulary?: string | null;
  state: RecordingSessionState;
  startedAtMs: number;
  recordingDuration: number;
  activeDuration: number;
  pauseDuration: number;
  microphoneDevice: string | null;
  systemDevice: string | null;
  expectedTracks: Array<'microphone' | 'system'>;
  capturedTracks?: CapturedTrack[];
  error?: ContractError;
  meetingName: string;
  savePath: string;
  autoSave: boolean;
}

export interface RecordingAudioRouteChanged {
  sessionId: string;
  previousDevice: string;
  currentDevice: string;
  reason: string;
}

export interface RecordingAudioDegraded {
  sessionId: string;
  source: 'system';
  reason: string;
}

interface RecordingWakeUp {
  sessionId: string;
}

export class RecordingService {
  startRecording(request: StartRecordingRequest): Promise<RecordingSessionSnapshot> {
    return invoke('start_recording', { request });
  }

  pauseRecording(sessionId: string): Promise<RecordingSessionSnapshot> {
    return invoke('pause_recording', { sessionId });
  }

  resumeRecording(sessionId: string): Promise<RecordingSessionSnapshot> {
    return invoke('resume_recording', { sessionId });
  }

  setSourceControl(
    sessionId: string,
    kind: RecordingSourceKind,
    control: SourceControl,
  ): Promise<RecordingSessionSnapshot> {
    return invoke('set_recording_source_control', { sessionId, kind, control });
  }

  stopRecording(sessionId: string): Promise<RecordingSessionSnapshot> {
    return invoke('stop_recording', { sessionId });
  }

  getRecordingSession(sessionId: string): Promise<RecordingSessionSnapshot> {
    return invoke('get_recording_session', { sessionId });
  }

  recoverRecordingSession(sessionId: string): Promise<RecordingSessionSnapshot> {
    return invoke('recover_recording_session', { sessionId });
  }

  getActiveRecordingSession(): Promise<RecordingSessionSnapshot | null> {
    return invoke('get_active_recording_session');
  }

  getRecordingMeetingName(): Promise<string | null> {
    return invoke('get_recording_meeting_name');
  }

  onRecordingStarted(callback: (sessionId: string) => void): Promise<UnlistenFn> {
    return listen<RecordingWakeUp>('recording-started', (event) => callback(event.payload.sessionId));
  }

  onRecordingStopped(callback: (sessionId: string) => void): Promise<UnlistenFn> {
    return listen<RecordingWakeUp>('recording-stopped', (event) => callback(event.payload.sessionId));
  }

  onRecordingPaused(callback: (sessionId: string) => void): Promise<UnlistenFn> {
    return listen<RecordingWakeUp>('recording-paused', (event) => callback(event.payload.sessionId));
  }

  onRecordingResumed(callback: (sessionId: string) => void): Promise<UnlistenFn> {
    return listen<RecordingWakeUp>('recording-resumed', (event) => callback(event.payload.sessionId));
  }

  onAudioRouteChanged(
    callback: (event: RecordingAudioRouteChanged) => void,
  ): Promise<UnlistenFn> {
    return listen<RecordingAudioRouteChanged>(
      'recording-audio-route-changed',
      (event) => callback(event.payload),
    );
  }

  onAudioDegraded(
    callback: (event: RecordingAudioDegraded) => void,
  ): Promise<UnlistenFn> {
    return listen<RecordingAudioDegraded>(
      'recording-audio-degraded',
      (event) => callback(event.payload),
    );
  }

  onChunkDropWarning(callback: (warning: string) => void): Promise<UnlistenFn> {
    return listen<string>('chunk-drop-warning', (event) => callback(event.payload));
  }

  onSpeechDetected(callback: () => void): Promise<UnlistenFn> {
    return listen('speech-detected', callback);
  }
}

export const recordingService = new RecordingService();
