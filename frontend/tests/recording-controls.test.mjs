import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

function createHookRuntime() {
  const stateSlots = [];
  const setters = [];
  const callbackSlots = [];
  const effectSlots = [];
  let stateIndex = 0;
  let callbackIndex = 0;
  let effectIndex = 0;

  const dependenciesMatch = (left, right) =>
    left?.length === right?.length &&
    left.every((value, index) => Object.is(value, right[index]));

  const react = {
    useState(initialValue) {
      const slot = stateIndex++;
      if (!(slot in stateSlots)) {
        stateSlots[slot] =
          typeof initialValue === 'function' ? initialValue() : initialValue;
      }
      setters[slot] ??= (nextValue) => {
        stateSlots[slot] =
          typeof nextValue === 'function'
            ? nextValue(stateSlots[slot])
            : nextValue;
      };
      return [stateSlots[slot], setters[slot]];
    },
    useCallback(callback, dependencies) {
      const slot = callbackIndex++;
      const previous = callbackSlots[slot];
      if (!previous || !dependenciesMatch(dependencies, previous.dependencies)) {
        callbackSlots[slot] = { callback, dependencies };
      }
      return callbackSlots[slot].callback;
    },
    useEffect(effect, dependencies) {
      const slot = effectIndex++;
      const previous = effectSlots[slot];
      if (previous && dependenciesMatch(dependencies, previous.dependencies)) return;
      previous?.cleanup?.();
      effectSlots[slot] = {
        cleanup: effect() ?? undefined,
        dependencies,
      };
    },
  };

  return {
    react,
    render(renderFunction) {
      stateIndex = 0;
      callbackIndex = 0;
      effectIndex = 0;
      return renderFunction();
    },
    unmount() {
      effectSlots.forEach((effect) => effect?.cleanup?.());
    },
  };
}

const jsxRuntime = {
  Fragment: Symbol('Fragment'),
  jsx: (type, props) => ({ type, props: props ?? {} }),
  jsxs: (type, props) => ({ type, props: props ?? {} }),
};

function loadRecordingControls(runtime, { recordingState, recordingService }) {
  const source = fs.readFileSync(
    new URL('../src/components/RecordingControls.tsx', import.meta.url),
    'utf8',
  );
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  const alert = Symbol('Alert');
  const ui = (name) => Symbol(name);
  const icons = new Proxy({}, { get: (_, name) => Symbol(String(name)) });

  vm.runInNewContext(compiled, {
    console: { error() {}, log() {}, warn() {} },
    exports: commonJsModule.exports,
    module: commonJsModule,
    require(specifier) {
      const modules = {
        react: runtime.react,
        'react/jsx-runtime': jsxRuntime,
        'lucide-react': icons,
        '@tauri-apps/api/event': { listen: async () => () => undefined },
        '@/components/ui/alert': {
          Alert: alert,
          AlertDescription: ui('AlertDescription'),
          AlertTitle: ui('AlertTitle'),
        },
        '@/components/ui/tooltip': {
          Tooltip: ui('Tooltip'),
          TooltipContent: ui('TooltipContent'),
          TooltipProvider: ui('TooltipProvider'),
          TooltipTrigger: ui('TooltipTrigger'),
        },
        '@/contexts/RecordingStateContext': {
          useRecordingState: () => recordingState,
        },
        '@/contexts/I18nContext': {
          useI18n: () => ({
            t: (key, variables = {}) => {
              const template = ({
              'recording.start': 'Start recording',
              'recording.audioDeviceUnavailable': 'Audio device unavailable',
              'recording.audioDeviceUnavailableDescription': 'Choose another audio device.',
              'recording.permissionRequired': 'Permission required',
              'recording.permissionDescription': 'Allow access to record audio.',
              'recording.failed': 'Recording failed',
              'recording.failedDescription': 'Unable to start recording.',
              'recording.closeAlert': 'Close alert',
              'recording.microphoneSource': 'Microphone',
              'recording.systemAudioSource': 'System audio',
              'recording.muteSource': 'Mute {source}',
              'recording.unmuteSource': 'Unmute {source}',
              'recording.sourceVolume': '{source} volume',
              'recording.sourceControlFailed': 'Could not update {source} control.',
              'recording.sourceControls': 'Audio source controls',
              }[key] ?? key);
              return template.replace(/\{(\w+)\}/g, (_, name) => String(variables[name] ?? `{${name}}`));
            },
          }),
        },
        '@/services/recordingService': {
          contractErrorFrom: (value) =>
            value && typeof value === 'object' &&
            typeof value.code === 'string' && typeof value.message === 'string'
              ? value
              : null,
          recordingService,
        },
        '@/components/ui/button': { Button: ui('Button') },
        '@/components/ui/textarea': { Textarea: ui('Textarea') },
        '@/components/ui/dialog': {
          Dialog: ui('Dialog'),
          DialogContent: ui('DialogContent'),
          DialogDescription: ui('DialogDescription'),
          DialogFooter: ui('DialogFooter'),
          DialogHeader: ui('DialogHeader'),
          DialogTitle: ui('DialogTitle'),
        },
      };
      if (!(specifier in modules)) {
        throw new Error(`Unexpected dependency: ${specifier}`);
      }
      return modules[specifier];
    },
  });

  return { RecordingControls: commonJsModule.exports.RecordingControls, alert };
}

function findNode(node, predicate) {
  if (Array.isArray(node)) {
    for (const child of node) {
      const match = findNode(child, predicate);
      if (match) return match;
    }
    return null;
  }
  if (!node || typeof node !== 'object') return null;
  if (predicate(node)) return node;
  return findNode(node.props?.children, predicate);
}

function deferred() {
  let resolve;
  const promise = new Promise((complete) => {
    resolve = complete;
  });
  return { promise, resolve };
}

async function settle() {
  for (let index = 0; index < 8; index += 1) {
    await Promise.resolve();
  }
}

function createProps(overrides = {}) {
  return {
    isRecording: false,
    barHeights: [],
    onRecordingStop: () => undefined,
    onRecordingStart: async () => undefined,
    isRecordingDisabled: false,
    isParentProcessing: false,
    selectedDevices: { micDevice: null, systemDevice: null },
    meetingName: 'Meeting',
    transcriptProvider: 'deepgram',
    meetingVocabulary: '',
    onMeetingVocabularyChange: () => undefined,
    ...overrides,
  };
}

function createRecordingState() {
  return {
    isRecording: false,
    isPaused: false,
    sessionId: null,
    projectSnapshot: () => undefined,
  };
}

function findStartButton(tree) {
  return findNode(
    tree,
    (node) => node.type === 'button' && node.props?.['aria-label'] === 'Start recording',
  );
}

function findButtonByLabel(tree, label) {
  return findNode(
    tree,
    (node) => node.type === 'button' && node.props?.['aria-label'] === label,
  );
}

function findSourceControl(tree, kind) {
  return findNode(tree, (node) => node.props?.['data-source-control'] === kind);
}

function findAlert(tree, alertType) {
  return findNode(tree, (node) => node.type === alertType);
}

test('clears a failed-start alert during a retry and keeps it cleared after success', async () => {
  const runtime = createHookRuntime();
  const recordingState = createRecordingState();
  const pendingStart = deferred();
  let startMode = 'fail';
  let attempts = 0;
  const onRecordingStart = async () => {
    attempts += 1;
    if (startMode === 'fail') {
      throw { code: 'device_unavailable', message: 'No device', retryable: true };
    }
    return pendingStart.promise;
  };
  const { RecordingControls, alert } = loadRecordingControls(runtime, {
    recordingState,
    recordingService: {},
  });
  const props = createProps({ onRecordingStart });
  const render = () => runtime.render(() => RecordingControls(props));

  let tree = render();
  findStartButton(tree).props.onClick();
  await settle();
  tree = render();
  assert.ok(findAlert(tree, alert), 'the failed start should show an alert');

  startMode = 'pending';
  const retry = findStartButton(tree);
  retry.props.onClick();
  await settle();
  tree = render();
  assert.equal(findAlert(tree, alert), null, 'a new start attempt should clear the old alert');
  assert.equal(attempts, 2);

  pendingStart.resolve();
  await settle();
  tree = render();
  assert.equal(findAlert(tree, alert), null, 'a successful retry should leave no stale alert');
  runtime.unmount();
});

test('clears a failed-start alert when recording becomes active through context or props', async () => {
  const runtime = createHookRuntime();
  const recordingState = createRecordingState();
  let starts = 0;
  const onRecordingStart = async () => {
    starts += 1;
    throw { code: 'permission_denied', message: 'Denied', retryable: true };
  };
  const { RecordingControls, alert } = loadRecordingControls(runtime, {
    recordingState,
    recordingService: {},
  });
  const props = createProps({ onRecordingStart });
  const render = () => runtime.render(() => RecordingControls(props));

  let tree = render();
  findStartButton(tree).props.onClick();
  await settle();
  tree = render();
  assert.ok(findAlert(tree, alert));

  recordingState.isRecording = true;
  render();
  tree = render();
  assert.equal(findAlert(tree, alert), null, 'native/sidebar state should clear the stale alert');

  recordingState.isRecording = false;
  props.isRecording = false;
  tree = render();
  findStartButton(tree).props.onClick();
  await settle();
  tree = render();
  assert.ok(findAlert(tree, alert));

  props.isRecording = true;
  render();
  tree = render();
  assert.equal(findAlert(tree, alert), null, 'confirmed active props should clear the stale alert');
  assert.equal(starts, 2);
  runtime.unmount();
});

test('source controls call the recording service without exposing the source samples', async () => {
  const runtime = createHookRuntime();
  const recordingState = {
    ...createRecordingState(),
    isRecording: true,
    sessionId: 'session-1',
    projectSnapshot(snapshot) {
      this.lastSnapshot = snapshot;
    },
  };
  const calls = [];
  const recordingService = {
    setSourceControl(sessionId, kind, control) {
      calls.push({ sessionId, kind, control });
      return Promise.resolve({ sessionId, state: 'recording' });
    },
  };
  const { RecordingControls } = loadRecordingControls(runtime, {
    recordingState,
    recordingService,
  });
  const render = () => runtime.render(() => RecordingControls(createProps({
    isRecording: true,
  })));

  let tree = render();
  const microphone = findSourceControl(tree, 'microphone');
  assert.ok(microphone, 'microphone control row should be rendered during recording');
  const mute = findButtonByLabel(tree, 'Mute Microphone');
  assert.ok(mute, 'microphone mute button should be accessible');
  mute.props.onClick();
  await settle();
  assert.equal(JSON.stringify(calls), JSON.stringify([{
    sessionId: 'session-1',
    kind: 'microphone',
    control: { muted: true, gain: 1 },
  }]));

  tree = render();
  const volume = findNode(
    tree,
    (node) => node.type === 'input' && node.props?.['aria-label'] === 'Microphone volume',
  );
  assert.ok(volume, 'microphone volume slider should be accessible');
  volume.props.onChange({ target: { value: '1.5' } });
  tree = render();
  findNode(tree, (node) => node.type === 'input' && node.props?.['aria-label'] === 'Microphone volume')
    .props.onBlur({ currentTarget: { value: '1.5' } });
  await settle();
  assert.equal(JSON.stringify(calls.at(-1)), JSON.stringify({
    sessionId: 'session-1',
    kind: 'microphone',
    control: { muted: true, gain: 1.5 },
  }));
  runtime.unmount();
});
