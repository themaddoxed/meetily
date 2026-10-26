export type MicrophonePermissionState =
  | 'not_determined'
  | 'authorized'
  | 'denied'
  | 'restricted'
  | 'unavailable';

export type SystemAudioAccessState =
  | 'unknown'
  | 'available'
  | 'denied'
  | 'unavailable';

export interface AudioPermissionSnapshot {
  microphone: MicrophonePermissionState;
  systemAudio: SystemAudioAccessState;
}

export const microphoneReady = (value: AudioPermissionSnapshot) =>
  value.microphone === 'authorized';

export const systemAudioReady = (value: AudioPermissionSnapshot) =>
  value.systemAudio === 'available';

type PermissionKind = 'microphone' | 'systemAudio';
type PermissionAction = { label: string; blocking: boolean };

function unexpectedPermissionState(state: never): never {
  throw new Error(`Unsupported audio permission state: ${state}`);
}

export function permissionAction(
  kind: 'microphone',
  state: MicrophonePermissionState,
): PermissionAction;
export function permissionAction(
  kind: 'systemAudio',
  state: SystemAudioAccessState,
): PermissionAction;
export function permissionAction(
  kind: PermissionKind,
  state: MicrophonePermissionState | SystemAudioAccessState,
): PermissionAction {
  if (kind === 'microphone') {
    return microphoneAction(state as MicrophonePermissionState);
  }
  return systemAudioAction(state as SystemAudioAccessState);
}

function microphoneAction(state: MicrophonePermissionState): PermissionAction {
  switch (state) {
    case 'authorized':
      return { label: 'Access Granted', blocking: false };
    case 'not_determined':
      return { label: 'Enable', blocking: true };
    case 'denied':
    case 'restricted':
      return { label: 'Open Settings', blocking: true };
    case 'unavailable':
      return { label: 'Try Again', blocking: true };
    default:
      return unexpectedPermissionState(state);
  }
}

function systemAudioAction(state: SystemAudioAccessState): PermissionAction {
  switch (state) {
    case 'available':
      return { label: 'Access Granted', blocking: false };
    case 'unknown':
      return { label: 'Try Again', blocking: false };
    case 'denied':
      return { label: 'Open Settings', blocking: false };
    case 'unavailable':
      return { label: 'Try Again', blocking: false };
    default:
      return unexpectedPermissionState(state);
  }
}
