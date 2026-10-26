import type { Block, MeetingSummary, Section } from '@/types';

function readMarkdown(summary: unknown): string {
  if (
    summary !== null &&
    typeof summary === 'object' &&
    'markdown' in summary &&
    typeof summary.markdown === 'string'
  ) {
    return summary.markdown;
  }
  return '';
}

function isLegacyBlock(value: unknown): value is Block {
  return (
    value !== null &&
    typeof value === 'object' &&
    'id' in value &&
    typeof value.id === 'string' &&
    'type' in value &&
    typeof value.type === 'string' &&
    'content' in value &&
    typeof value.content === 'string' &&
    'color' in value &&
    typeof value.color === 'string'
  );
}

function readLegacySection(value: unknown): Section | null {
  if (
    value === null ||
    typeof value !== 'object' ||
    !('title' in value) ||
    typeof value.title !== 'string' ||
    !('blocks' in value) ||
    !Array.isArray(value.blocks) ||
    !value.blocks.every(isLegacyBlock)
  ) {
    return null;
  }

  return {
    title: value.title,
    blocks: value.blocks,
  };
}

export function summaryToMarkdown(summary: MeetingSummary | null): string {
  const markdown = readMarkdown(summary);
  if (markdown) {
    return markdown;
  }

  if (!summary) {
    return '';
  }

  return Object.entries(summary)
    .filter(
      ([key]) =>
        !['markdown', 'summary_json', '_section_order', 'MeetingName'].includes(key),
    )
    .map(([, value]) => {
      const section = readLegacySection(value);
      if (!section) {
        return '';
      }

      const content = section.blocks.map((block) => `- ${block.content}`).join('\n');
      return `## ${section.title}\n\n${content}`;
    })
    .filter((section) => section.trim())
    .join('\n\n');
}
