'use client';

import React, { createContext, useContext, useState, useEffect, useRef } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useI18n } from '@/contexts/I18nContext';
import type { MeetingSummaryApiResponse } from '@/lib/meeting-summary';
import type { MeetingSearchResult } from '@/lib/meeting-search';


interface SidebarItem {
  id: string;
  title: string;
  type: 'folder' | 'file';
  children?: SidebarItem[];
  status?: MeetingLifecycleStatus;
  durationSeconds?: number | null;
  startedAtMs?: number | null;
  lifecycleError?: string | null;
}

export type MeetingLifecycleStatus = 'recording' | 'processing' | 'ready' | 'error';

export interface CurrentMeeting {
  id: string;
  title: string;
  status?: MeetingLifecycleStatus;
  durationSeconds?: number | null;
  startedAtMs?: number | null;
  createdAt?: string;
  lifecycleError?: string | null;
}

const INTRO_MEETING: CurrentMeeting = {
  id: 'intro-call',
  title: '+ New Call',
};

interface SidebarContextType {
  currentMeeting: CurrentMeeting | null;
  setCurrentMeeting: (meeting: CurrentMeeting | null) => void;
  sidebarItems: SidebarItem[];
  isCollapsed: boolean;
  toggleCollapse: () => void;
  meetings: CurrentMeeting[];
  setMeetings: (meetings: CurrentMeeting[]) => void;
  isMeetingActive: boolean;
  setIsMeetingActive: (active: boolean) => void;
  handleRecordingToggle: () => void;
  searchTranscripts: (query: string) => Promise<void>;
  searchResults: MeetingSearchResult[];
  isSearching: boolean;
  // Summary polling management
  activeSummaryPolls: Map<string, NodeJS.Timeout>;
  startSummaryPolling: (
    meetingId: string,
    processId: string,
    onUpdate: (result: MeetingSummaryApiResponse) => void,
  ) => void;
  stopSummaryPolling: (meetingId: string) => void;
  // Refetch meetings from backend
  refetchMeetings: () => Promise<void>;

}

const SidebarContext = createContext<SidebarContextType | null>(null);

export const useSidebar = () => {
  const context = useContext(SidebarContext);
  if (!context) {
    throw new Error('useSidebar must be used within a SidebarProvider');
  }
  return context;
};

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [currentMeeting, setCurrentMeeting] = useState<CurrentMeeting | null>(INTRO_MEETING);
  const [isCollapsed, setIsCollapsed] = useState(true);
  const [meetings, setMeetings] = useState<CurrentMeeting[]>([]);
  const [sidebarItems, setSidebarItems] = useState<SidebarItem[]>([]);
  const [isMeetingActive, setIsMeetingActive] = useState(false);
  const [searchResults, setSearchResults] = useState<MeetingSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const searchRequestIdRef = useRef(0);
  const [activeSummaryPolls, setActiveSummaryPolls] = useState<Map<string, NodeJS.Timeout>>(new Map());

  // Use recording state from RecordingStateContext (single source of truth)
  const { isRecording } = useRecordingState();
  const { locale, t } = useI18n();

  const pathname = usePathname();
  const router = useRouter();
  const mountedRef = React.useRef(true);
  const fetchGenerationRef = React.useRef(0);

  useEffect(() => () => {
    mountedRef.current = false;
  }, []);

  // Extract fetchMeetings as a reusable function
  const fetchMeetings = React.useCallback(async () => {
    const generation = ++fetchGenerationRef.current;
    try {
      const meetings = await invoke<Array<CurrentMeeting & {
        duration_seconds?: number | null;
        started_at_ms?: number | null;
        created_at?: string;
        lifecycle_error?: string | null;
      }>>('api_get_meetings');
      if (!mountedRef.current || generation !== fetchGenerationRef.current) return;
      const transformedMeetings: CurrentMeeting[] = meetings.map((meeting) => ({
        id: meeting.id,
        title: meeting.title,
        status: meeting.status ?? 'ready',
        durationSeconds: meeting.durationSeconds ?? meeting.duration_seconds ?? null,
        startedAtMs: meeting.startedAtMs ?? meeting.started_at_ms ?? null,
        createdAt: meeting.createdAt ?? meeting.created_at,
        lifecycleError: meeting.lifecycleError ?? meeting.lifecycle_error ?? null,
      }));
      setMeetings(transformedMeetings);
    } catch (error) {
      if (!mountedRef.current || generation !== fetchGenerationRef.current) return;
      console.error('Error fetching meetings:', error);
      setMeetings([]);
    }
  }, []);

  useEffect(() => {
    fetchMeetings();
  }, [fetchMeetings]);

  // Backend lifecycle events are wake-ups; the next list read remains the
  // source of truth for status, title, time, and duration.
  useEffect(() => {
    const lifecycleEvents = [
      'recording-started',
      'recording-processing',
      'recording-stopped',
      'import-started',
      'import-complete',
      'import-error',
      'retranscription-started',
      'retranscription-complete',
      'retranscription-error',
      'meeting-lifecycle-updated',
    ];
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const retain = (unlisten: UnlistenFn) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    };
    lifecycleEvents.forEach((eventName) => {
      void listen(eventName, () => {
        void fetchMeetings();
      }).then(retain);
    });
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [fetchMeetings]);

  // The Rust session computes elapsed time and persists terminal lifecycle
  // states. Polling while any row is active keeps the list current even when
  // the meeting details page is not mounted.
  const hasPendingLifecycle = isRecording || meetings.some(
    (meeting) => meeting.status === 'recording' || meeting.status === 'processing',
  );

  useEffect(() => {
    if (!hasPendingLifecycle) return;

    let disposed = false;
    let inFlight = false;
    const refresh = async () => {
      if (disposed || inFlight) return;
      inFlight = true;
      try {
        await fetchMeetings();
      } finally {
        inFlight = false;
      }
    };

    void refresh();
    const interval = setInterval(() => {
      void refresh();
    }, 1000);
    return () => {
      disposed = true;
      clearInterval(interval);
    };
  }, [fetchMeetings, hasPendingLifecycle]);

  const baseItems = React.useMemo<SidebarItem[]>(() => [
    (() => {
      const groups = new Map<string, { label: string; timestamp: number; meetings: CurrentMeeting[] }>();
      for (const meeting of meetings) {
        const timestamp = meeting.startedAtMs ?? (meeting.createdAt ? Date.parse(meeting.createdAt) : 0);
        const date = Number.isFinite(timestamp) && timestamp > 0 ? new Date(timestamp) : null;
        const key = date
          ? `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
          : 'unknown';
        const label = date
          ? new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(date)
          : t('meetings.unknownDate');
        const group = groups.get(key) ?? { label, timestamp: date?.getTime() ?? 0, meetings: [] };
        group.meetings.push(meeting);
        groups.set(key, group);
      }

      const dayFolders = [...groups.entries()]
        .sort(([, left], [, right]) => right.timestamp - left.timestamp)
        .map(([key, group]) => ({
          id: `meeting-day-${key}`,
          title: group.label,
          type: 'folder' as const,
          children: group.meetings.map((meeting) => ({
            id: meeting.id,
            title: meeting.title,
            type: 'file' as const,
            status: meeting.status,
            durationSeconds: meeting.durationSeconds,
            startedAtMs: meeting.startedAtMs,
            lifecycleError: meeting.lifecycleError,
          })),
        }));

      return {
        id: 'meetings',
        title: 'Meeting Notes',
        type: 'folder' as const,
        children: dayFolders,
      };
    })(),
  ], [locale, meetings, t]);

  const toggleCollapse = () => {
    setIsCollapsed(!isCollapsed);
  };

  // Update current meeting when on home page
  useEffect(() => {
    if (pathname === '/') {
      setCurrentMeeting(INTRO_MEETING);
    }
  }, [pathname]);

  // Update sidebar items when meetings change
  useEffect(() => {
    setSidebarItems(baseItems);
  }, [baseItems]);

  // Function to handle recording toggle from sidebar
  const handleRecordingToggle = () => {
    if (!isRecording) {
      // Check if already on home page
      if (pathname === '/') {
        // Already on home - trigger recording directly via custom event
        console.log('Triggering recording from sidebar (already on home page)');
        window.dispatchEvent(new CustomEvent('start-recording-from-sidebar'));
      } else {
        // Not on home - navigate and use auto-start mechanism
        console.log('Navigating to home page with auto-start flag');
        sessionStorage.setItem('autoStartRecording', 'true');
        router.push('/');
      }

    }
    // The actual recording start/stop is handled in the Home component
  };

  // Function to search through meeting transcripts
  const searchTranscripts = async (query: string) => {
    const requestId = ++searchRequestIdRef.current;
    if (!query.trim()) {
      setSearchResults([]);
      setIsSearching(false);
      return;
    }

    try {
      setIsSearching(true);

      const results = await invoke<MeetingSearchResult[]>('api_search_transcripts', { query });
      if (requestId === searchRequestIdRef.current) {
        setSearchResults(results);
      }
    } catch (error) {
      console.error('Error searching transcripts:', error);
      if (requestId === searchRequestIdRef.current) {
        setSearchResults([]);
      }
    } finally {
      if (requestId === searchRequestIdRef.current) {
        setIsSearching(false);
      }
    }
  };

  // Summary polling management
  const startSummaryPolling = React.useCallback((
    meetingId: string,
    processId: string,
    onUpdate: (result: MeetingSummaryApiResponse) => void
  ) => {
    // Stop existing poll for this meeting if any
    if (activeSummaryPolls.has(meetingId)) {
      clearInterval(activeSummaryPolls.get(meetingId)!);
    }

    console.log(`📊 Starting polling for meeting ${meetingId}, process ${processId}`);

    let pollCount = 0;
    const MAX_POLLS = 200; // ~16.5 minutes at 5-second intervals (slightly longer than backend's 15-min timeout to avoid race conditions)

    const pollInterval = setInterval(async () => {
      pollCount++;

      // Timeout safety: Stop after 10 minutes
      if (pollCount >= MAX_POLLS) {
        console.warn(`⏱️ Polling timeout for ${meetingId} after ${MAX_POLLS} iterations`);
        clearInterval(pollInterval);
        setActiveSummaryPolls(prev => {
          const next = new Map(prev);
          next.delete(meetingId);
          return next;
        });
        onUpdate({
          status: 'error',
          error: 'Summary generation timed out after 15 minutes. Please try again or check your model configuration.'
        });
        return;
      }
      try {
        const result = await invoke<MeetingSummaryApiResponse>('api_get_summary', {
          meetingId: meetingId,
        });

        console.log(`📊 Polling update for ${meetingId}:`, result.status);

        // Call the update callback with result
        onUpdate(result);
        void fetchMeetings();

        // Stop polling if completed, error, failed, cancelled, or idle (after initial processing)
        if (result.status === 'completed' || result.status === 'error' || result.status === 'failed' || result.status === 'cancelled') {
          console.log(`Polling completed for ${meetingId}, status: ${result.status}`);
          clearInterval(pollInterval);
          setActiveSummaryPolls(prev => {
            const next = new Map(prev);
            next.delete(meetingId);
            return next;
          });
        } else if (result.status === 'idle' && pollCount > 1) {
          // If we get 'idle' after polling started, process completed/disappeared
          console.log(`Process completed or not found for ${meetingId}, stopping poll`);
          clearInterval(pollInterval);
          setActiveSummaryPolls(prev => {
            const next = new Map(prev);
            next.delete(meetingId);
            return next;
          });
        }
      } catch (error) {
        console.error(`Polling error for ${meetingId}:`, error);
        // Report error to callback
        onUpdate({
          status: 'error',
          error: error instanceof Error ? error.message : 'Unknown error'
        });
        clearInterval(pollInterval);
        setActiveSummaryPolls(prev => {
          const next = new Map(prev);
          next.delete(meetingId);
          return next;
        });
      }
    }, 5000); // Poll every 5 seconds

    setActiveSummaryPolls(prev => new Map(prev).set(meetingId, pollInterval));
  }, [activeSummaryPolls, fetchMeetings]);

  const stopSummaryPolling = React.useCallback((meetingId: string) => {
    const pollInterval = activeSummaryPolls.get(meetingId);
    if (pollInterval) {
      console.log(`⏹️ Stopping polling for meeting ${meetingId}`);
      clearInterval(pollInterval);
      setActiveSummaryPolls(prev => {
        const next = new Map(prev);
        next.delete(meetingId);
        return next;
      });
    }
  }, [activeSummaryPolls]);

  // Cleanup all polling intervals on unmount
  useEffect(() => {
    return () => {
      console.log('🧹 Cleaning up all summary polling intervals');
      activeSummaryPolls.forEach(interval => clearInterval(interval));
    };
  }, [activeSummaryPolls]);



  return (
    <SidebarContext.Provider value={{
      currentMeeting,
      setCurrentMeeting,
      sidebarItems,
      isCollapsed,
      toggleCollapse,
      meetings,
      setMeetings,
      isMeetingActive,
      setIsMeetingActive,
      handleRecordingToggle,
      searchTranscripts,
      searchResults,
      isSearching,
      activeSummaryPolls,
      startSummaryPolling,
      stopSummaryPolling,
      refetchMeetings: fetchMeetings,

    }}>
      {children}
    </SidebarContext.Provider>
  );
}
