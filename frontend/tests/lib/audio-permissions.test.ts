import { describe, expect, it } from 'vitest';

import {
  microphoneReady,
  permissionAction,
  systemAudioReady,
} from '@/lib/audioPermissions';

describe('audio permissions', () => {
  it.each([
    [{ microphone: 'authorized', systemAudio: 'unknown' }, true],
    [{ microphone: 'denied', systemAudio: 'available' }, false],
  ] as const)('reports microphone readiness for %o', (snapshot, expected) => {
    expect(microphoneReady(snapshot)).toBe(expected);
  });

  it.each([
    [{ microphone: 'authorized', systemAudio: 'available' }, true],
    [{ microphone: 'authorized', systemAudio: 'unknown' }, false],
  ] as const)('reports system audio readiness for %o', (snapshot, expected) => {
    expect(systemAudioReady(snapshot)).toBe(expected);
  });

  it('blocks microphone settings when access is restricted', () => {
    expect(permissionAction('microphone', 'restricted')).toEqual({
      label: 'Open Settings',
      blocking: true,
    });
  });

  it('keeps unknown system audio unconfirmed and nonblocking', () => {
    expect(permissionAction('systemAudio', 'unknown')).toEqual({ label: 'Try Again', blocking: false });
  });

  it('keeps unavailable system audio nonblocking', () => {
    expect(permissionAction('systemAudio', 'unavailable')).toEqual({
      label: 'Try Again',
      blocking: false,
    });
  });
});
