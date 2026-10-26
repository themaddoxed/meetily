/**
 * useTranscriptRecovery Hook
 *
 * Orchestrates transcript recovery operations for interrupted meetings.
 * Provides functionality to detect, preview, and recover meetings from IndexedDB.
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { indexedDBService, MeetingMetadata, StoredTranscript } from '@/services/indexedDBService';
import { storageService } from '@/services/storageService';
import { applyPinnedSummaryLanguageToMeeting } from '@/lib/summary-language-preferences';
import { toast } from 'sonner';

interface AudioRecoveryStatus {
  status: string; // "success" | "partial" | "failed" | "none"
  chunk_count: number;
  estimated_duration_seconds: number;
  audio_file_path?: string;
  message: string;
}

type RecoveryResult = {
  success: boolean;
  audioRecoveryStatus?: AudioRecoveryStatus | null;
  meetingId?: string;
};

export interface UseTranscriptRecoveryReturn {
  recoverableMeetings: MeetingMetadata[];
  isLoading: boolean;
  isRecovering: boolean;
  checkForRecoverableTranscripts: () => Promise<void>;
  recoverMeeting: (meetingId: string) => Promise<RecoveryResult>;
  loadMeetingTranscripts: (meetingId: string) => Promise<StoredTranscript[]>;
  deleteRecoverableMeeting: (meetingId: string) => Promise<void>;
}

export function useTranscriptRecovery(): UseTranscriptRecoveryReturn {
  const [recoverableMeetings, setRecoverableMeetings] = useState<MeetingMetadata[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRecovering, setIsRecovering] = useState(false);
  const isMountedRef = useRef(true);
  const recoveryRequestGenerationRef = useRef(0);
  const inFlightRecoveriesRef = useRef(new Map<string, Promise<RecoveryResult>>());

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      recoveryRequestGenerationRef.current += 1;
    };
  }, []);

  /**
   * Check for recoverable meetings in IndexedDB
   */
  const checkForRecoverableTranscripts = useCallback(async () => {
    if (!isMountedRef.current) return;

    const requestGeneration = ++recoveryRequestGenerationRef.current;
    const isCurrentRequest = () =>
      isMountedRef.current &&
      requestGeneration === recoveryRequestGenerationRef.current;
    setIsLoading(true);
    try {
      const meetings = await indexedDBService.getAllMeetings();

      // Exclude only the live or mid-save meeting. Session storage is cleared by a
      // process restart, so a crashed meeting remains eligible on the first relaunch.
      const activeMeetingId = sessionStorage.getItem('indexeddb_current_meeting_id');

      // getAllMeetings already excludes records persisted successfully to SQLite.
      // A freshness delay would hide a recent crash until a second app launch.
      const cutoffTime = Date.now() - (7 * 24 * 60 * 60 * 1000);

      const recentMeetings = meetings.filter(m =>
        m.lastUpdated > cutoffTime && m.meetingId !== activeMeetingId
      );

      // Verify audio checkpoint availability for each meeting
      const meetingsWithAudioStatus = await Promise.all(
        recentMeetings.map(async (meeting) => {
          if (meeting.folderPath) {
            try {
              const hasAudio = await invoke<boolean>('has_audio_checkpoints', {
                meetingFolder: meeting.folderPath
              });

              // If no audio files, clear folderPath to show "No audio" in UI
              return {
                ...meeting,
                folderPath: hasAudio ? meeting.folderPath : undefined
              };
            } catch (error) {
              console.warn('Failed to check audio for meeting:', error);
              // On error, assume no audio to be safe
              return { ...meeting, folderPath: undefined };
            }
          }
          return meeting;
        })
      );

      if (isCurrentRequest()) {
        setRecoverableMeetings(meetingsWithAudioStatus);
      }
    } catch (error) {
      if (isCurrentRequest()) {
        console.error('Failed to check for recoverable transcripts:', error);
        setRecoverableMeetings([]);
      }
    } finally {
      if (isCurrentRequest()) {
        setIsLoading(false);
      }
    }
  }, []);

  /**
   * Load transcripts for preview
   */
  const loadMeetingTranscripts = useCallback(async (meetingId: string): Promise<StoredTranscript[]> => {
    try {
      const transcripts = await indexedDBService.getTranscripts(meetingId);
      // Sort by sequence ID
      transcripts.sort((a, b) => (a.sequenceId || 0) - (b.sequenceId || 0));
      return transcripts;
    } catch (error) {
      console.error('Failed to load meeting transcripts:', error);
      throw error;
    }
  }, []);

  /**
   * Recover a meeting from IndexedDB
   */
  const recoverMeeting = useCallback((meetingId: string): Promise<RecoveryResult> => {
    const existingRecovery = inFlightRecoveriesRef.current.get(meetingId);
    if (existingRecovery) return existingRecovery;

    const recovery = (async (): Promise<RecoveryResult> => {
      setIsRecovering(true);
      try {
      // 1. Load meeting metadata
      const metadata = await indexedDBService.getMeetingMetadata(meetingId);
      if (!metadata) {
        throw new Error('Meeting metadata not found');
      }

      // 2. Load all transcripts
      const transcripts = await loadMeetingTranscripts(meetingId);

      // 3. Check for folder path
      let folderPath = metadata.folderPath;


      if (!folderPath) {
        // Try to get from backend (might exist if only app crashed, not system)
        try {
          folderPath = await invoke<string>('get_meeting_folder_path');
        } catch {
          folderPath = undefined;
        }
      }

      // 4. Attempt audio recovery if folder path exists
      let audioRecoveryStatus: AudioRecoveryStatus | null = null;
      if (folderPath) {
        try {
          audioRecoveryStatus = await invoke<AudioRecoveryStatus>(
            'recover_audio_from_checkpoints',
            { meetingFolder: folderPath, sampleRate: 48000 }
          );
        } catch (error) {
          console.error('Audio recovery failed:', error);
          audioRecoveryStatus = {
            status: 'failed',
            chunk_count: 0,
            estimated_duration_seconds: 0,
            message: error instanceof Error ? error.message : 'Unknown error'
          };
        }
      } else {
        audioRecoveryStatus = {
          status: 'none',
          chunk_count: 0,
          estimated_duration_seconds: 0,
          message: 'No folder path available'
        };
      }

      // A crash can happen before STT emits its first segment. In that case,
      // recovered audio is still a valid meeting artifact. If neither durable
      // transcript nor a recovered audio file exists, leave the recovery item
      // pending instead of marking an empty meeting as saved.
      const hasRecoveredAudio =
        (audioRecoveryStatus.status === 'success' || audioRecoveryStatus.status === 'partial')
        && Boolean(audioRecoveryStatus.audio_file_path);
      const hasCompleteAudioRecovery =
        audioRecoveryStatus.status === 'success'
        && typeof audioRecoveryStatus.audio_file_path === 'string'
        && audioRecoveryStatus.audio_file_path.trim().length > 0;
      if (transcripts.length === 0 && !hasRecoveredAudio) {
        throw new Error('No transcript or recoverable audio found for this meeting');
      }

      // 5. Convert StoredTranscripts to the format expected by storageService
      const formattedTranscripts = transcripts.map((t, index) => ({
        id: t.id?.toString() || `${Date.now()}-${index}`,
        text: t.text,
        timestamp: t.timestamp,
        sequence_id: t.sequenceId ?? index,
        chunk_start_time: t.chunkStartTime,
        is_partial: t.isPartial ?? false,
        confidence: t.confidence,
        audio_start_time: t.audioStartTime,
        audio_end_time: t.audioEndTime,
        duration: t.duration,
      }));

      // 6. Allocate the permanent ID before touching SQLite. Every retry sends
      // the same ID, so a crash after the backend commit cannot create a duplicate.
      const savedMeetingId = await indexedDBService.getOrCreateSQLiteMeetingId(
        meetingId,
        crypto.randomUUID(),
      );
      const saveResponse = await storageService.saveMeeting(
        metadata.title,
        formattedTranscripts,
        folderPath ?? null,
        savedMeetingId,
        metadata.startTime,
      );
      if (saveResponse.meeting_id !== savedMeetingId) {
        throw new Error('SQLite returned a different meeting ID during recovery');
      }

      try {
        await applyPinnedSummaryLanguageToMeeting(savedMeetingId);
      } catch (error) {
        console.warn('Failed to apply pinned summary language to recovered meeting:', error);
        toast.warning('Could not apply default summary language', {
          description: 'The recovered meeting was saved, but the default summary language was not applied.',
        });
      }

      // 7. Recovery remains pending until a complete audio file exists. Failed,
      // partial, unavailable, or malformed recovery must retain raw checkpoints.
      if (hasCompleteAudioRecovery) {
        await indexedDBService.markMeetingSaved(meetingId);

        try {
          await invoke('cleanup_checkpoints', { meetingFolder: folderPath });
        } catch (error) {
          // Non-fatal - don't fail recovery if cleanup fails
          console.warn('Checkpoint cleanup failed (non-fatal):', error);
        }

        // 8. Remove only completely recovered meetings from the pending list.
        setRecoverableMeetings(prev => prev.filter(m => m.meetingId !== meetingId));
      }

      return {
        success: true,
        audioRecoveryStatus,
        meetingId: savedMeetingId
      };
      } catch (error) {
        console.error('Failed to recover meeting:', error);
        throw error;
      } finally {
        setIsRecovering(false);
      }
    })();

    const trackedRecovery = recovery.finally(() => {
      if (inFlightRecoveriesRef.current.get(meetingId) === trackedRecovery) {
        inFlightRecoveriesRef.current.delete(meetingId);
      }
    });
    inFlightRecoveriesRef.current.set(meetingId, trackedRecovery);
    return trackedRecovery;
  }, [loadMeetingTranscripts]);

  /**
   * Delete a recoverable meeting
   */
  const deleteRecoverableMeeting = useCallback(async (meetingId: string): Promise<void> => {
    try {
      await indexedDBService.deleteMeeting(meetingId);
      setRecoverableMeetings(prev => prev.filter(m => m.meetingId !== meetingId));
    } catch (error) {
      console.error('Failed to delete meeting:', error);
      throw error;
    }
  }, []);

  return {
    recoverableMeetings,
    isLoading,
    isRecovering,
    checkForRecoverableTranscripts,
    recoverMeeting,
    loadMeetingTranscripts,
    deleteRecoverableMeeting
  };
}
