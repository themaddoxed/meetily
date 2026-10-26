export interface TranscriptWithId {
  id: string;
}

/** Load pages in order until a requested transcript row is present. */
export async function loadPagesUntilTarget<T extends TranscriptWithId>(
  targetId: string,
  loadPage: () => Promise<ReadonlyArray<T>>,
  hasMore: () => boolean,
  isCurrent: () => boolean = () => true,
): Promise<boolean> {
  while (isCurrent() && hasMore()) {
    const page = await loadPage();
    if (!isCurrent()) {
      return false;
    }

    if (page.some((transcript) => transcript.id === targetId)) {
      return true;
    }

    // A failed or empty page cannot make progress, even if the server still
    // reports more rows.
    if (page.length === 0) {
      return false;
    }
  }

  return false;
}
