export type VocabularyLoadStatus = 'idle' | 'loading' | 'ready' | 'error';

/**
 * A response can update the dialog only while its request is current and
 * mounted. This keeps a slow response for a previous meeting from replacing
 * the current meeting's draft.
 */
export function isCurrentVocabularyRequest(
  requestId: number,
  activeRequestId: number,
  disposed: boolean,
): boolean {
  return !disposed && requestId === activeRequestId;
}

export function canStartRetranscription(status: VocabularyLoadStatus): boolean {
  return status === 'ready';
}
