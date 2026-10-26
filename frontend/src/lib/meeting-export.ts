import type { Transcript } from '@/types';

interface MeetingExportMetadata {
  id: string;
  title: string;
  created_at: string;
}

export interface MeetingMarkdownExportInput {
  meeting: MeetingExportMetadata;
  transcripts: Transcript[];
  summaryMarkdown?: string | null;
}

export function formatRecordingTime(
  seconds: number | undefined,
  fallbackTimestamp: string,
): string {
  if (seconds === undefined || !Number.isFinite(seconds) || seconds < 0) {
    return fallbackTimestamp;
  }

  const totalSeconds = Math.floor(seconds);
  const minutes = Math.floor(totalSeconds / 60);
  const remainingSeconds = totalSeconds % 60;

  return `[${minutes.toString().padStart(2, '0')}:${remainingSeconds
    .toString()
    .padStart(2, '0')}]`;
}

function normalizeHeading(value: string): string {
  return value.replace(/[\r\n]+/g, ' ').trim();
}

function formatTranscriptLine(transcript: Transcript): string {
  const timestamp = formatRecordingTime(transcript.audio_start_time, transcript.timestamp);
  const speakerLabel = transcript.is_overlap
    ? 'Multiple speakers'
    : transcript.speaker_label?.trim();

  if (!speakerLabel) {
    return `${timestamp} ${transcript.text}`;
  }

  return `${timestamp} **${speakerLabel}:** ${transcript.text}`;
}

export function formatMeetingMarkdown({
  meeting,
  transcripts,
  summaryMarkdown,
}: MeetingMarkdownExportInput): string {
  const orderedTranscripts = [...transcripts].sort(
    (left, right) => (left.audio_start_time ?? 0) - (right.audio_start_time ?? 0),
  );
  const transcriptMarkdown = orderedTranscripts.map(formatTranscriptLine).join('\n\n');
  const normalizedSummary = summaryMarkdown?.trim();

  const sections = [
    `# Meeting: ${normalizeHeading(meeting.title)}`,
    `**Meeting ID:** ${meeting.id}\n**Date:** ${meeting.created_at}`,
    `## Transcript\n\n${transcriptMarkdown}`,
  ];

  if (normalizedSummary) {
    sections.push(`## Summary\n\n${normalizedSummary}`);
  }

  return `${sections.join('\n\n')}\n`;
}
