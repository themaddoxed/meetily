import { useState, useCallback, useRef, useEffect, useLayoutEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Transcript, MeetingMetadata, PaginatedTranscriptsResponse, TranscriptSegmentData } from "@/types";
import { loadPagesUntilTarget } from "@/lib/transcript-pagination";

const DEFAULT_PAGE_SIZE = 100;

interface UsePaginatedTranscriptsProps {
    meetingId: string | null;
    /** Optional transcript id from a search result that should be revealed. */
    targetTranscriptId?: string;
}

interface UsePaginatedTranscriptsReturn {
    metadata: MeetingMetadata | null;
    segments: TranscriptSegmentData[];
    transcripts: Transcript[];
    isLoading: boolean;
    isLoadingMore: boolean;
    hasMore: boolean;
    totalCount: number;
    loadedCount: number;
    error: string | null;

    // Actions
    loadMore: () => Promise<void>;
    reset: () => void;
    refetch: () => Promise<void>;
}

/**
 * Convert Transcript array to TranscriptSegmentData for virtualized display
 */
export function convertTranscriptsToSegments(transcripts: Transcript[]): TranscriptSegmentData[] {
    return transcripts.map(t => ({
        id: t.id,
        timestamp: t.audio_start_time ?? 0,
        endTime: t.audio_end_time,
        text: t.text,
        confidence: t.confidence,
        speaker_id: t.speaker_id,
        speaker_label: t.speaker_label,
        speaker_color: t.speaker_color,
        is_overlap: t.is_overlap,
        diarization_status: t.diarization_status,
        diarization_method: t.diarization_method,
        diarization_confidence: t.diarization_confidence,
    }));
}

export function usePaginatedTranscripts({
    meetingId,
    targetTranscriptId,
}: UsePaginatedTranscriptsProps): UsePaginatedTranscriptsReturn {
    const [metadata, setMetadata] = useState<MeetingMetadata | null>(null);
    const [transcripts, setTranscripts] = useState<Transcript[]>([]);
    const [totalCount, setTotalCount] = useState(0);
    const [isLoading, setIsLoading] = useState(true);
    const [isLoadingMore, setIsLoadingMore] = useState(false);
    const [hasMore, setHasMore] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const offsetRef = useRef(0);
    const loadedMeetingIdRef = useRef<string | null>(null);
    const isLoadingRef = useRef(false);
    const hasMoreRef = useRef(false);
    const lastLoadTimeRef = useRef(0); // Debounce protection
    const targetLoadInFlightRef = useRef(false);
    const requestGenerationRef = useRef(0);
    const activeMeetingIdRef = useRef(meetingId);

    // Invalidate pending work before passive effects start loading another meeting.
    useLayoutEffect(() => {
        activeMeetingIdRef.current = meetingId;
        requestGenerationRef.current += 1;
        return () => {
            requestGenerationRef.current += 1;
            loadedMeetingIdRef.current = null;
        };
    }, [meetingId]);

    // Reset state when meeting changes
    const reset = useCallback(() => {
        requestGenerationRef.current += 1;
        isLoadingRef.current = false;
        setMetadata(null);
        setTranscripts([]);
        setTotalCount(0);
        setIsLoading(true);
        setIsLoadingMore(false);
        setHasMore(false);
        setError(null);
        offsetRef.current = 0;
        hasMoreRef.current = false;
        targetLoadInFlightRef.current = false;
    }, []);

    // Load meeting metadata
    const loadMetadata = useCallback(async (): Promise<MeetingMetadata | null> => {
        if (!meetingId) return null;
        const requestMeetingId = meetingId;
        const requestGeneration = requestGenerationRef.current;
        const isCurrentRequest = () =>
            requestGeneration === requestGenerationRef.current &&
            requestMeetingId === activeMeetingIdRef.current;
        if (!isCurrentRequest()) return null;

        try {
            const data = await invoke<MeetingMetadata>('api_get_meeting_metadata', {
                meetingId: requestMeetingId,
            });
            if (!isCurrentRequest()) return null;
            setMetadata((current) => isCurrentRequest() ? data : current);
            return data;
        } catch (err) {
            if (!isCurrentRequest()) return null;
            console.error('Failed to load meeting metadata:', err);
            setError((current) => isCurrentRequest() ? 'Failed to load meeting details' : current);
            return null;
        }
    }, [meetingId]);

    // Load transcripts at specific offset
    const loadTranscriptsAtOffset = useCallback(async (
        offset: number,
        append: boolean = true,
        requestGeneration: number = requestGenerationRef.current,
    ): Promise<Transcript[]> => {
        if (
            !meetingId ||
            loadedMeetingIdRef.current !== meetingId ||
            activeMeetingIdRef.current !== meetingId ||
            requestGeneration !== requestGenerationRef.current
        ) return [];
        const requestMeetingId = meetingId;
        const isCurrentRequest = () =>
            requestGeneration === requestGenerationRef.current &&
            requestMeetingId === activeMeetingIdRef.current;

        try {
            const response = await invoke<PaginatedTranscriptsResponse>(
                'api_get_meeting_transcripts',
                {
                    meetingId: requestMeetingId,
                    limit: DEFAULT_PAGE_SIZE,
                    offset,
                }
            );
            if (!isCurrentRequest()) return [];

            const newTranscripts = response.transcripts;

            if (append) {
                setTranscripts(prev => {
                    if (!isCurrentRequest()) return prev;
                    // Deduplicate by id
                    const existingIds = new Set(prev.map(t => t.id));
                    const uniqueNew = newTranscripts.filter(t => !existingIds.has(t.id));
                    // Sort by audio_start_time
                    return [...prev, ...uniqueNew].sort((a, b) =>
                        (a.audio_start_time ?? 0) - (b.audio_start_time ?? 0)
                    );
                });
            } else {
                setTranscripts((prev) => isCurrentRequest() ? newTranscripts : prev);
            }

            setHasMore((current) => isCurrentRequest() ? response.has_more : current);
            hasMoreRef.current = response.has_more;
            setTotalCount((current) => isCurrentRequest() ? response.total_count : current);
            offsetRef.current = offset + newTranscripts.length;

            return newTranscripts;
        } catch (err) {
            if (!isCurrentRequest()) return [];
            console.error('Failed to load transcripts:', err);
            setError((current) => isCurrentRequest() ? 'Failed to load transcripts' : current);
            return [];
        }
    }, [meetingId]);

    // Load next page with debounce protection
    const loadMore = useCallback(async () => {
        const now = Date.now();
        // Debounce: require at least 100ms between calls
        if (now - lastLoadTimeRef.current < 100) {
            return;
        }

        if (
            isLoadingRef.current ||
            !hasMore ||
            !meetingId ||
            activeMeetingIdRef.current !== meetingId ||
            isLoading
        ) return;

        lastLoadTimeRef.current = now;
        const requestGeneration = requestGenerationRef.current;
        isLoadingRef.current = true;
        setIsLoadingMore(true);
        try {
            await loadTranscriptsAtOffset(offsetRef.current, true, requestGeneration);
        } finally {
            if (requestGeneration === requestGenerationRef.current) {
                setIsLoadingMore(false);
                isLoadingRef.current = false;
            }
        }
    }, [hasMore, meetingId, loadTranscriptsAtOffset, isLoading]);

    // Search navigation uses a dedicated loader so a fast page cannot be
    // swallowed by the scroll loader's debounce window. Pages are fetched in
    // order until the target row is found or the server is exhausted.
    const loadTargetTranscript = useCallback(async () => {
        if (
            !targetTranscriptId ||
            !meetingId ||
            loadedMeetingIdRef.current !== meetingId ||
            activeMeetingIdRef.current !== meetingId ||
            isLoading ||
            transcripts.some((transcript) => transcript.id === targetTranscriptId)
        ) {
            return;
        }

        const requestMeetingId = meetingId;
        const requestGeneration = requestGenerationRef.current;
        const isCurrentRequest = () =>
            requestGeneration === requestGenerationRef.current &&
            requestMeetingId === activeMeetingIdRef.current;

        await loadPagesUntilTarget(
            targetTranscriptId,
            async () => {
                if (!isCurrentRequest() || isLoadingRef.current || !hasMoreRef.current) {
                    return [];
                }

                isLoadingRef.current = true;
                setIsLoadingMore(true);
                try {
                    return await loadTranscriptsAtOffset(
                        offsetRef.current,
                        true,
                        requestGeneration,
                    );
                } finally {
                    if (isCurrentRequest()) {
                        setIsLoadingMore(false);
                        isLoadingRef.current = false;
                    }
                }
            },
            () => hasMoreRef.current,
            isCurrentRequest,
        );
    }, [
        targetTranscriptId,
        meetingId,
        isLoading,
        transcripts,
        loadTranscriptsAtOffset,
    ]);

    // Force refetch of data (e.g., after retranscription)
    const refetch = useCallback(async () => {
        if (!meetingId) return;

        reset();
        const requestGeneration = requestGenerationRef.current;
        const isCurrentRequest = () =>
            requestGeneration === requestGenerationRef.current &&
            activeMeetingIdRef.current === meetingId;
        setIsLoading((current) => isCurrentRequest() ? true : current);
        try {
            await loadMetadata();
            if (isCurrentRequest()) {
                await loadTranscriptsAtOffset(0, false, requestGeneration);
            }
        } finally {
            setIsLoading((current) => isCurrentRequest() ? false : current);
        }
    }, [meetingId, reset, loadMetadata, loadTranscriptsAtOffset]);

    // Search results can point past the first page. Continue loading pages
    // until the exact transcript row is present or the meeting is exhausted.
    useEffect(() => {
        if (
            !targetTranscriptId ||
            !meetingId ||
            loadedMeetingIdRef.current !== meetingId ||
            isLoading ||
            isLoadingMore ||
            !hasMore ||
            targetLoadInFlightRef.current ||
            transcripts.some((transcript) => transcript.id === targetTranscriptId)
        ) {
            return;
        }

        const requestGeneration = requestGenerationRef.current;
        targetLoadInFlightRef.current = true;
        void loadTargetTranscript().finally(() => {
            if (requestGeneration === requestGenerationRef.current) {
                targetLoadInFlightRef.current = false;
            }
        });
    }, [
        targetTranscriptId,
        meetingId,
        isLoading,
        isLoadingMore,
        hasMore,
        transcripts,
        loadTargetTranscript,
    ]);

    // Initial load
    useEffect(() => {
        if (!meetingId) {
            loadedMeetingIdRef.current = null;
            reset();
            return;
        }

        // Avoid reloading the same meeting
        if (loadedMeetingIdRef.current === meetingId) return;
        loadedMeetingIdRef.current = meetingId;

        reset();
        const requestGeneration = requestGenerationRef.current;
        const isCurrentRequest = () =>
            requestGeneration === requestGenerationRef.current &&
            activeMeetingIdRef.current === meetingId;

        const loadInitial = async () => {
            setIsLoading((current) => isCurrentRequest() ? true : current);
            try {
                await loadMetadata();
                if (isCurrentRequest()) {
                    await loadTranscriptsAtOffset(0, false, requestGeneration);
                }
            } finally {
                setIsLoading((current) => isCurrentRequest() ? false : current);
            }
        };

        loadInitial();
    }, [meetingId, reset, loadMetadata, loadTranscriptsAtOffset]);

    // Convert to segments (memoized)
    const segments = useMemo(() =>
        convertTranscriptsToSegments(transcripts),
        [transcripts]
    );

    return {
        metadata,
        segments,
        transcripts,
        isLoading,
        isLoadingMore,
        hasMore,
        totalCount,
        loadedCount: transcripts.length,
        error,
        loadMore,
        reset,
        refetch,
    };
}
