'use client';

import { useRef, useReducer, startTransition, useEffect, memo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useAutoScroll } from "@/hooks/useAutoScroll";
import { useTranscriptStreaming } from "@/hooks/useTranscriptStreaming";
import { ConfidenceIndicator } from "./ConfidenceIndicator";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { RecordingStatusBar } from "./RecordingStatusBar";
import { motion, AnimatePresence } from "framer-motion";
import { TranscriptSegmentData } from "@/types";
import { SpeakerLabelEditor } from "./MeetingDetails/SpeakerLabelEditor";
import { useI18n } from "@/contexts/I18nContext";

export interface VirtualizedTranscriptViewProps {
    /** Transcript segments to display */
    segments: TranscriptSegmentData[];
    /** Whether recording is in progress */
    isRecording?: boolean;
    /** Whether recording is paused */
    isPaused?: boolean;
    /** Whether processing/finalizing transcription */
    isProcessing?: boolean;
    /** Whether stopping */
    isStopping?: boolean;
    /** Enable streaming effect for latest segment */
    enableStreaming?: boolean;
    /** Show confidence indicators */
    showConfidence?: boolean;
    /** Completely disable auto-scroll behavior (for meeting details page) */
    disableAutoScroll?: boolean;
    /** Transcript segment selected by a meeting search result */
    activeSegmentId?: string;

    // Pagination props (infinite scroll)
    hasMore?: boolean;
    isLoadingMore?: boolean;
    totalCount?: number;
    loadedCount?: number;
    onLoadMore?: () => void;
    meetingId?: string;
    onSpeakerRenamed?: () => Promise<void>;
}

// Threshold for enabling virtualization (below this, use simple rendering)
const VIRTUALIZATION_THRESHOLD = 10;

// Helper function to format seconds as recording-relative time [MM:SS]
function formatRecordingTime(seconds: number | undefined): string {
    if (seconds === undefined) return '[--:--]';

    const totalSeconds = Math.floor(seconds);
    const minutes = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;

    return `[${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
}

// Helper function to remove filler words and repetitions
function cleanStopWords(text: string): string {
    const stopWords = ['uh', 'um', 'er', 'ah', 'hmm', 'hm', 'eh', 'oh'];

    let cleanedText = text;
    stopWords.forEach(word => {
        const pattern = new RegExp(`\\b${word}\\b[,\\s]*`, 'gi');
        cleanedText = cleanedText.replace(pattern, ' ');
    });

    return cleanedText.replace(/\s+/g, ' ').trim();
}

// Memoized transcript segment component
const TranscriptSegment = memo(function TranscriptSegment({
    segment,
    text,
    isActive,
    isStreaming,
    showConfidence,
    meetingId,
    onSpeakerRenamed,
}: {
    segment: TranscriptSegmentData;
    text: string;
    isActive: boolean;
    isStreaming: boolean;
    showConfidence: boolean;
    meetingId?: string;
    onSpeakerRenamed?: () => Promise<void>;
}) {
    const { t } = useI18n();
    const displayText = cleanStopWords(text) || (text.trim() === '' ? t('transcript.silence') : text);
    const multipleSpeakersLabel = t('transcript.multipleSpeakers') || 'Multiple speakers';
    const canRename = Boolean(
        meetingId && segment.speaker_id && segment.speaker_label && onSpeakerRenamed,
    );

    return (
        <div
            id={`segment-${segment.id}`}
            className={`mb-3 rounded-md transition-colors ${isActive ? 'bg-info/15 ring-1 ring-info/40' : ''}`}
            aria-current={isActive ? 'location' : undefined}
        >
            <div className="flex items-start gap-2">
                <Tooltip>
                    <TooltipTrigger>
                        <span className="text-xs text-muted-foreground mt-1 flex-shrink-0 min-w-[50px]">
                            {formatRecordingTime(segment.timestamp)}
                        </span>
                    </TooltipTrigger>
                    <TooltipContent>
                        {segment.confidence !== undefined && showConfidence && (
                            <ConfidenceIndicator confidence={segment.confidence} showIndicator={showConfidence} />
                        )}
                    </TooltipContent>
                </Tooltip>
                <div className="flex-1">
                    {segment.is_overlap ? (
                        <div className="mb-1 inline-flex items-center rounded-full bg-warning/10 px-2 py-0.5 text-xs font-medium text-warning-foreground">
                            {multipleSpeakersLabel}
                        </div>
                    ) : canRename ? (
                        <SpeakerLabelEditor
                            meetingId={meetingId!}
                            speakerId={segment.speaker_id!}
                            speakerLabel={segment.speaker_label!}
                            speakerColor={segment.speaker_color}
                            diarizationStatus={segment.diarization_status}
                            onRenamed={onSpeakerRenamed!}
                        />
                    ) : segment.speaker_label ? (
                        <div className="mb-1 flex items-center gap-1.5 text-xs font-medium text-foreground">
                            <span
                                aria-hidden="true"
                                className="h-2.5 w-2.5 rounded-full border border-border"
                                style={{ backgroundColor: segment.speaker_color || '#94a3b8' }}
                            />
                            <span>{segment.speaker_label}</span>
                            {segment.diarization_status === 'provisional' && (
                                <span className="font-normal text-muted-foreground">{t('transcript.provisional')}</span>
                            )}
                        </div>
                    ) : null}
                    {isStreaming ? (
                        <div className="bg-muted border border-border rounded-lg px-3 py-2">
                            <p className="text-base text-foreground leading-relaxed">{displayText}</p>
                        </div>
                    ) : (
                        <p className="text-base text-foreground leading-relaxed">{displayText}</p>
                    )}
                </div>
            </div>
        </div>
    );
});

export const VirtualizedTranscriptView: React.FC<VirtualizedTranscriptViewProps> = ({
    segments,
    isRecording = false,
    isPaused = false,
    isProcessing = false,
    isStopping = false,
    enableStreaming = false,
    showConfidence = true,
    disableAutoScroll = false,
    activeSegmentId,
    hasMore = false,
    isLoadingMore = false,
    totalCount = 0,
    loadedCount = 0,
    onLoadMore,
    meetingId,
    onSpeakerRenamed,
}) => {
    const { t } = useI18n();
    // Create scroll ref first - shared between virtualizer and auto-scroll hook
    const scrollRef = useRef<HTMLDivElement>(null);
    // Ref for infinite scroll trigger element
    const loadMoreTriggerRef = useRef<HTMLDivElement>(null);

    // Force re-render without flushSync (avoids React warning)
    const [, rerender] = useReducer((x: number) => x + 1, 0);

    // Setup virtualizer for efficient rendering of large lists
    const virtualizer = useVirtualizer({
        count: segments.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => 60, // Estimated height per segment
        overscan: 10, // Render extra items above/below viewport
        onChange: () => {
            startTransition(() => {
                rerender();
            });
        },
    });

    // Custom hook for auto-scrolling (supports both virtualized and non-virtualized)
    useAutoScroll({
        scrollRef,
        segments,
        isRecording,
        isPaused,
        virtualizer,
        virtualizationThreshold: VIRTUALIZATION_THRESHOLD,
        disableAutoScroll,
        activeSegmentId,
    });

    // Streaming text effect hook (typewriter animation for new transcripts)
    const { streamingSegmentId, getDisplayText } = useTranscriptStreaming(
        segments,
        isRecording,
        enableStreaming
    );

    // Infinite scroll: IntersectionObserver to trigger loading more
    useEffect(() => {
        if (!onLoadMore || !hasMore || isLoadingMore || isRecording || segments.length === 0) {
            return;
        }

        const triggerElement = loadMoreTriggerRef.current;
        if (!triggerElement) return;

        const observer = new IntersectionObserver(
            (entries) => {
                if (entries[0].isIntersecting && hasMore && !isLoadingMore) {
                    onLoadMore();
                }
            },
            {
                root: null,
                rootMargin: '100px',
                threshold: 0,
            }
        );

        observer.observe(triggerElement);

        return () => observer.disconnect();
    }, [hasMore, isLoadingMore, onLoadMore, isRecording, segments.length]);

    // Scroll-based fallback for fast scrolling
    useEffect(() => {
        if (!onLoadMore || !hasMore || isLoadingMore || isRecording) return;

        const scrollElement = scrollRef.current;
        if (!scrollElement) return;

        let ticking = false;

        const handleScroll = () => {
            if (ticking || isLoadingMore || !hasMore) return;

            ticking = true;
            requestAnimationFrame(() => {
                const { scrollTop, scrollHeight, clientHeight } = scrollElement;
                const scrollBottom = scrollHeight - scrollTop - clientHeight;

                // Trigger load when within 200px of bottom
                if (scrollBottom < 200 && hasMore && !isLoadingMore) {
                    onLoadMore();
                }
                ticking = false;
            });
        };

        scrollElement.addEventListener('scroll', handleScroll, { passive: true });
        return () => scrollElement.removeEventListener('scroll', handleScroll);
    }, [onLoadMore, hasMore, isLoadingMore, isRecording]);

    // Use simple rendering for small lists, virtualization for large lists
    const useVirtualization = segments.length >= VIRTUALIZATION_THRESHOLD;

    return (
        <div ref={scrollRef} className="flex flex-col h-full overflow-y-auto px-4 py-2">
            {/* Recording Status Bar - Sticky at top, always visible when recording */}
            <AnimatePresence>
                {isRecording && (
                    <div className="sticky top-0 z-10 bg-card pb-2">
                        <RecordingStatusBar isPaused={isPaused} />
                    </div>
                )}
            </AnimatePresence>

            {/* Content - add padding when recording to prevent overlap */}
            <div className={isRecording ? 'pt-2' : ''}>
            {segments.length === 0 ? (
                // Empty state
                <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="text-center text-muted-foreground mt-8"
                >
                    {isRecording ? (
                        <>
                            <div className="flex items-center justify-center mb-3">
                                <div className={`w-3 h-3 rounded-full ${isPaused ? 'bg-warning' : 'bg-info animate-pulse'}`}></div>
                            </div>
                            <p className="text-sm text-muted-foreground">
                                {isPaused ? t('recording.paused') : t('recording.listeningForSpeech')}
                            </p>
                            <p className="text-xs mt-1 text-muted-foreground">
                                {isPaused ? t('recording.resumeToContinue') : t('recording.speakToSee')}
                            </p>
                        </>
                    ) : (
                        <>
                            <p className="text-lg font-semibold">{t('transcript.welcome')}</p>
                            <p className="text-xs mt-1">{t('transcript.startToSee')}</p>
                        </>
                    )}
                </motion.div>
            ) : useVirtualization ? (
                // Virtualized rendering for large lists
                <>
                    <div
                        style={{
                            height: virtualizer.getTotalSize(),
                            width: "100%",
                            position: "relative",
                        }}
                    >
                        {virtualizer.getVirtualItems().map((virtualRow) => {
                            const segment = segments[virtualRow.index];
                            const isStreaming = streamingSegmentId === segment.id;

                            return (
                                <div
                                    key={segment.id}
                                    data-index={virtualRow.index}
                                    ref={virtualizer.measureElement}
                                    style={{
                                        position: "absolute",
                                        top: 0,
                                        left: 0,
                                        width: "100%",
                                        transform: `translateY(${virtualRow.start}px)`,
                                    }}
                                >
                                    <TranscriptSegment
                                        segment={segment}
                                        text={getDisplayText(segment)}
                                        isActive={activeSegmentId === segment.id}
                                        isStreaming={isStreaming}
                                        showConfidence={showConfidence}
                                        meetingId={meetingId}
                                        onSpeakerRenamed={onSpeakerRenamed}
                                    />
                                </div>
                            );
                        })}
                    </div>

                    {/* Infinite scroll trigger and loading indicator */}
                    {(hasMore || isLoadingMore) && !isRecording && segments.length > 0 && (
                        <div ref={loadMoreTriggerRef} className="flex justify-center items-center py-4 mt-2">
                            {isLoadingMore ? (
                                <div className="flex items-center gap-2 text-muted-foreground">
                                    <div className="w-4 h-4 border-2 border-border border-t-muted-foreground rounded-full animate-spin" />
                                    <span className="text-sm">{t('transcript.loadingMore')}</span>
                                </div>
                            ) : hasMore && totalCount > 0 ? (
                                    <span className="text-sm text-muted-foreground">
                                    {t('transcript.showingSegments', { loaded: loadedCount, total: totalCount })}
                                </span>
                            ) : null}
                        </div>
                    )}

                    {/* Listening indicator when recording */}
                    {!isStopping && isRecording && !isPaused && !isProcessing && segments.length > 0 && (
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center gap-2 mt-4 text-muted-foreground"
                        >
                            <div className="w-2 h-2 bg-info rounded-full animate-pulse"></div>
                            <span className="text-sm">{t('transcript.listening')}</span>
                        </motion.div>
                    )}
                </>
            ) : (
                // Simple rendering for small lists (better animations)
                <>
                    <div className="space-y-1">
                        {segments.map((segment) => {
                            const isStreaming = streamingSegmentId === segment.id;

                            return (
                                <motion.div
                                    key={segment.id}
                                    initial={{ opacity: 0, y: 5 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    transition={{ duration: 0.15 }}
                                >
                                    <TranscriptSegment
                                        segment={segment}
                                        text={getDisplayText(segment)}
                                        isActive={activeSegmentId === segment.id}
                                        isStreaming={isStreaming}
                                        showConfidence={showConfidence}
                                        meetingId={meetingId}
                                        onSpeakerRenamed={onSpeakerRenamed}
                                    />
                                </motion.div>
                            );
                        })}
                    </div>

                    {/* Infinite scroll trigger (for small lists that grow) */}
                    {(hasMore || isLoadingMore) && !isRecording && segments.length > 0 && (
                        <div ref={loadMoreTriggerRef} className="flex justify-center items-center py-4 mt-2">
                            {isLoadingMore ? (
                                <div className="flex items-center gap-2 text-muted-foreground">
                                    <div className="w-4 h-4 border-2 border-border border-t-muted-foreground rounded-full animate-spin" />
                                    <span className="text-sm">{t('transcript.loadingMore')}</span>
                                </div>
                            ) : hasMore && totalCount > 0 ? (
                                    <span className="text-sm text-muted-foreground">
                                    {t('transcript.showingSegments', { loaded: loadedCount, total: totalCount })}
                                </span>
                            ) : null}
                        </div>
                    )}

                    {/* Listening indicator when recording */}
                    {!isStopping && isRecording && !isPaused && !isProcessing && segments.length > 0 && (
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center gap-2 mt-4 text-muted-foreground"
                        >
                            <div className="w-2 h-2 bg-info rounded-full animate-pulse"></div>
                            <span className="text-sm">{t('transcript.listening')}</span>
                        </motion.div>
                    )}
                </>
            )}
            </div>
        </div>
    );
};
