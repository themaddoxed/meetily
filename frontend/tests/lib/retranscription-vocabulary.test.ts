import { describe, expect, it } from 'vitest';
import {
  canStartRetranscription,
  isCurrentVocabularyRequest,
} from '@/lib/retranscription-vocabulary';

describe('retranscription vocabulary loading', () => {
  it('ignores a delayed response after the meeting changes', () => {
    expect(isCurrentVocabularyRequest(1, 2, false)).toBe(false);
    expect(isCurrentVocabularyRequest(2, 2, false)).toBe(true);
    expect(isCurrentVocabularyRequest(2, 2, true)).toBe(false);
  });

  it('allows saving only after a successful vocabulary load', () => {
    expect(canStartRetranscription('loading')).toBe(false);
    expect(canStartRetranscription('error')).toBe(false);
    expect(canStartRetranscription('ready')).toBe(true);
  });
});
