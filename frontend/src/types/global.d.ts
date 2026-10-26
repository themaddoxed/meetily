interface Window {
  __TAURI__?: unknown;
  handleRecordingStop?: (callApi?: boolean) => void;
  webkitAudioContext?: typeof AudioContext;
}
