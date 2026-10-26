import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

const frontendDir = fileURLToPath(new URL('..', import.meta.url));
const eslintCli = fileURLToPath(
  new URL('../node_modules/eslint/bin/eslint.js', import.meta.url),
);

const lifecycleTargets = [
  'src/app/meeting-details/page-content.tsx',
  'src/components/AISummary/index.tsx',
  'src/components/MeetingDetails/RetranscribeDialog.tsx',
  'src/components/MessageToast.tsx',
  'src/components/ParakeetModelManager.tsx',
  'src/components/Sidebar/SidebarProvider.tsx',
  'src/components/Sidebar/index.tsx',
  'src/components/TranscriptRecovery/TranscriptRecovery.tsx',
  'src/components/WhisperModelManager.tsx',
  'src/contexts/RecordingStateContext.tsx',
  'src/contexts/TranscriptContext.tsx',
  'src/hooks/useAudioPlayer.ts',
  'src/hooks/meeting-details/useSummaryGeneration.ts',
  'src/hooks/useRecordingStop.ts',
];

function readSource(relativePath) {
  return fs.readFileSync(path.join(frontendDir, relativePath), 'utf8');
}

function createHookRuntime() {
  const stateSlots = [];
  const setters = [];
  const refSlots = [];
  const callbackSlots = [];
  const memoSlots = [];
  const effectSlots = [];
  let stateIndex = 0;
  let refIndex = 0;
  let callbackIndex = 0;
  let memoIndex = 0;
  let effectIndex = 0;

  const dependenciesMatch = (left, right) =>
    left?.length === right?.length &&
    left.every((value, index) => Object.is(value, right[index]));

  const react = {
    createContext(defaultValue) {
      return { defaultValue, Provider: Symbol('Context.Provider') };
    },
    useContext(context) {
      return context.defaultValue;
    },
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
    useRef(initialValue) {
      const slot = refIndex++;
      refSlots[slot] ??= { current: initialValue };
      return refSlots[slot];
    },
    useCallback(callback, dependencies) {
      const slot = callbackIndex++;
      const previous = callbackSlots[slot];
      if (!previous || !dependenciesMatch(dependencies, previous.dependencies)) {
        callbackSlots[slot] = { callback, dependencies };
      }
      return callbackSlots[slot].callback;
    },
    useMemo(factory, dependencies) {
      const slot = memoIndex++;
      const previous = memoSlots[slot];
      if (!previous || !dependenciesMatch(dependencies, previous.dependencies)) {
        memoSlots[slot] = { value: factory(), dependencies };
      }
      return memoSlots[slot].value;
    },
    useEffect(effect, dependencies) {
      const slot = effectIndex++;
      const previous = effectSlots[slot];
      if (previous && dependenciesMatch(dependencies, previous.dependencies)) return;
      previous?.cleanup?.();
      effectSlots[slot] = {
        cleanup: effect() ?? undefined,
        dependencies,
        effect,
      };
    },
  };

  return {
    react,
    render(renderFunction) {
      stateIndex = 0;
      refIndex = 0;
      callbackIndex = 0;
      memoIndex = 0;
      effectIndex = 0;
      return renderFunction();
    },
    replayEffects() {
      effectSlots.forEach((effect) => effect?.cleanup?.());
      effectSlots.forEach((effect) => {
        if (effect) effect.cleanup = effect.effect() ?? undefined;
      });
    },
    unmount() {
      effectSlots.forEach((effect) => effect?.cleanup?.());
    },
  };
}

function createTimers() {
  const tasks = new Map();
  let nextId = 1;

  return {
    setTimeout(callback) {
      const id = nextId++;
      tasks.set(id, callback);
      return id;
    },
    clearTimeout(id) {
      tasks.delete(id);
    },
    activeCount() {
      return tasks.size;
    },
    runAll() {
      const callbacks = [...tasks.values()];
      tasks.clear();
      callbacks.forEach((callback) => callback());
    },
  };
}

function createIntervals() {
  const tasks = new Map();
  let nextId = 1;

  return {
    setInterval(callback) {
      const id = nextId++;
      tasks.set(id, callback);
      return id;
    },
    clearInterval(id) {
      tasks.delete(id);
    },
    activeCount() {
      return tasks.size;
    },
    runAll() {
      [...tasks.values()].forEach((callback) => callback());
    },
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((complete) => {
    resolve = complete;
  });
  return { promise, resolve };
}

function createDocumentEvents() {
  const listeners = new Map();

  return {
    addEventListener(type, callback) {
      listeners.set(type, callback);
    },
    removeEventListener(type, callback) {
      if (listeners.get(type) === callback) listeners.delete(type);
    },
    dispatch(type, event) {
      listeners.get(type)?.(event);
    },
  };
}

function findJsxNode(node, predicate) {
  if (Array.isArray(node)) {
    for (const child of node) {
      const match = findJsxNode(child, predicate);
      if (match) return match;
    }
    return null;
  }
  if (!node || typeof node !== 'object') return null;
  if (predicate(node)) return node;
  return findJsxNode(node.props?.children, predicate);
}

const quietConsole = {
  error: () => undefined,
  log: () => undefined,
  warn: () => undefined,
};

const jsxRuntime = {
  jsx: (type, props) => ({ type, props }),
  jsxs: (type, props) => ({ type, props }),
};

function loadPageContent(runtime, { getModelConfig, meetingData, summaryGeneration }) {
  return loadModule(
    'src/app/meeting-details/page-content.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      'framer-motion': { motion: { div: Symbol('motion.div') } },
      '@tauri-apps/api/core': { invoke: async () => undefined },
      sonner: { toast: { error: () => undefined, success: () => undefined } },
      '@/components/MeetingDetails/TranscriptPanel': { TranscriptPanel: Symbol('TranscriptPanel') },
      '@/components/MeetingDetails/SummaryPanel': { SummaryPanel: Symbol('SummaryPanel') },
      '@/components/MeetingDetails/MeetingQuestions': { MeetingQuestions: Symbol('MeetingQuestions') },
      '@/hooks/meeting-details/useMeetingData': { useMeetingData: () => meetingData },
      '@/hooks/meeting-details/useSummaryGeneration': { useSummaryGeneration: () => summaryGeneration },
      '@/hooks/meeting-details/useTemplates': {
        useTemplates: () => ({
          availableTemplates: [],
          handleTemplateSelection: () => undefined,
          selectedTemplate: 'default',
        }),
      },
      '@/hooks/meeting-details/useCopyOperations': {
        useCopyOperations: () => ({
          handleCopySummary: () => undefined,
          handleCopyTranscript: () => undefined,
        }),
      },
      '@/hooks/meeting-details/useMeetingOperations': {
        useMeetingOperations: () => ({ handleOpenMeetingFolder: () => undefined }),
      },
      '@/hooks/meeting-details/useMeetingExport': {
        useMeetingExport: () => ({
          handleExportMeeting: () => undefined,
          isExporting: false,
        }),
      },
      '@/hooks/meeting-details/useDiarizationRun': {
        useDiarizationRun: () => ({
          isRecording: false,
          isRunning: false,
          progress: 0,
          runDiarization: () => undefined,
        }),
      },
      '@/contexts/ConfigContext': {
        useConfig: () => ({
          modelConfig: getModelConfig(),
          setModelConfig: () => undefined,
        }),
      },
    },
    { console: quietConsole },
  ).default;
}

function loadSummaryGeneration(runtime, { invoke, sidebar }) {
  return loadModule(
    'src/hooks/meeting-details/useSummaryGeneration.ts',
    {
      react: runtime.react,
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@/components/Sidebar/SidebarProvider': { useSidebar: () => sidebar },
      '@tauri-apps/api/core': { invoke },
      sonner: {
        toast: {
          error: () => undefined,
          info: () => undefined,
          success: () => undefined,
          warning: () => undefined,
        },
      },
      '@/lib/utils': { isOllamaNotInstalledError: () => false },
      '@/lib/summary-language-preferences': {
        detectAndCacheSummaryLanguage: async () => ({ language: 'en', reason: 'detected' }),
        readCachedDetectedSummaryLanguage: async () => null,
        readMeetingSummaryLanguage: async () => ({ language: 'en' }),
      },
      '@/lib/meeting-summary': {
        normalizeMeetingSummary: () => null,
        readMeetingNameFromSummaryPayload: () => null,
      },
    },
    { console: quietConsole },
  ).useSummaryGeneration;
}

function loadModule(relativePath, modules, globals = {}) {
  const source = readSource(relativePath);
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };

  vm.runInNewContext(compiled, {
    console,
    exports: commonJsModule.exports,
    module: commonJsModule,
    require(specifier) {
      if (!(specifier in modules)) {
        throw new Error(`Unexpected dependency: ${specifier}`);
      }
      return modules[specifier];
    },
    ...globals,
  });

  return commonJsModule.exports;
}

test('assigned React lifecycle files pass exhaustive dependency checks', () => {
  const result = spawnSync(
    process.execPath,
    [eslintCli, ...lifecycleTargets, '--max-warnings=0'],
    { cwd: frontendDir, encoding: 'utf8' },
  );

  assert.equal(result.status, 0, `${result.stdout}${result.stderr}`);
});

test('meeting auto-summary starts once when transcripts become available', async () => {
  const runtime = createHookRuntime();
  const meetingData = {
    aiSummary: null,
    blockNoteSummaryRef: { current: null },
    handleSaveSummary: () => undefined,
    handleSummaryChange: () => undefined,
    isSaving: false,
    isTitleDirty: false,
    saveAllChanges: () => undefined,
    setAiSummary: () => undefined,
    setIsSummaryDirty: () => undefined,
    transcripts: [],
    updateMeetingTitle: () => undefined,
  };
  let generationCalls = 0;
  let completionCalls = 0;
  const summaryGeneration = {
    getSummaryStatusMessage: () => '',
    handleGenerateSummary: async () => {
      generationCalls += 1;
      return true;
    },
    handleStopGeneration: () => undefined,
    summaryError: null,
    summaryStatus: 'idle',
  };
  const PageContent = loadPageContent(runtime, {
    getModelConfig: () => ({ model: 'model-a', provider: 'provider-a' }),
    meetingData,
    summaryGeneration,
  });
  const props = {
    meeting: { id: 'meeting-a', title: 'Meeting A', transcripts: [] },
    onAutoGenerateComplete: () => { completionCalls += 1; },
    shouldAutoGenerate: true,
    summaryData: null,
  };

  runtime.render(() => PageContent(props));
  meetingData.transcripts = [{ id: 1, text: 'Ready' }];
  runtime.render(() => PageContent(props));
  await new Promise((resolve) => setImmediate(resolve));

  meetingData.transcripts = [
    { id: 1, text: 'Ready' },
    { id: 2, text: 'Later update' },
  ];
  runtime.render(() => PageContent(props));
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(generationCalls, 1);
  assert.equal(completionCalls, 1);
  runtime.unmount();
});

test('meeting auto-summary retries not-started results after transcript and config rerenders', async () => {
  const runtime = createHookRuntime();
  const meetingData = {
    aiSummary: null,
    blockNoteSummaryRef: { current: null },
    handleSaveSummary: () => undefined,
    handleSummaryChange: () => undefined,
    isSaving: false,
    isTitleDirty: false,
    saveAllChanges: () => undefined,
    setAiSummary: () => undefined,
    setIsSummaryDirty: () => undefined,
    transcripts: [{ id: 1, text: 'Ready' }],
    updateMeetingTitle: () => undefined,
  };
  const results = [false, false, true];
  let generationCalls = 0;
  let completionCalls = 0;
  let model = 'model-a';
  const summaryGeneration = {
    getSummaryStatusMessage: () => '',
    handleGenerateSummary: async () => {
      generationCalls += 1;
      return results.shift();
    },
    handleStopGeneration: () => undefined,
    summaryError: null,
    summaryStatus: 'idle',
  };
  const PageContent = loadPageContent(runtime, {
    getModelConfig: () => ({ model, provider: 'provider-a' }),
    meetingData,
    summaryGeneration,
  });
  const props = {
    meeting: { id: 'meeting-a', title: 'Meeting A', transcripts: meetingData.transcripts },
    onAutoGenerateComplete: () => { completionCalls += 1; },
    shouldAutoGenerate: true,
    summaryData: null,
  };

  runtime.render(() => PageContent(props));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(generationCalls, 1);
  assert.equal(completionCalls, 0);

  meetingData.transcripts = [
    ...meetingData.transcripts,
    { id: 2, text: 'Updated transcript' },
  ];
  runtime.render(() => PageContent(props));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(generationCalls, 2);
  assert.equal(completionCalls, 0);

  model = 'model-b';
  runtime.render(() => PageContent(props));
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(generationCalls, 3);
  assert.equal(completionCalls, 1);
  runtime.unmount();
});

test('meeting auto-summary ignores a stale completion after a meeting transition', async () => {
  const runtime = createHookRuntime();
  const firstAttempt = deferred();
  const meetingData = {
    aiSummary: null,
    blockNoteSummaryRef: { current: null },
    handleSaveSummary: () => undefined,
    handleSummaryChange: () => undefined,
    isSaving: false,
    isTitleDirty: false,
    saveAllChanges: () => undefined,
    setAiSummary: () => undefined,
    setIsSummaryDirty: () => undefined,
    transcripts: [{ id: 1, text: 'Ready' }],
    updateMeetingTitle: () => undefined,
  };
  let currentMeetingId = 'meeting-a';
  const completionMeetings = [];
  const generationMeetings = [];
  const summaryGeneration = {
    getSummaryStatusMessage: () => '',
    handleGenerateSummary: async () => {
      generationMeetings.push(currentMeetingId);
      return currentMeetingId === 'meeting-a' ? firstAttempt.promise : true;
    },
    handleStopGeneration: () => undefined,
    summaryError: null,
    summaryStatus: 'idle',
  };
  const PageContent = loadPageContent(runtime, {
    getModelConfig: () => ({ model: 'model-a', provider: 'provider-a' }),
    meetingData,
    summaryGeneration,
  });
  const props = {
    meeting: { id: currentMeetingId, title: 'Meeting A', transcripts: meetingData.transcripts },
    onAutoGenerateComplete: () => { completionMeetings.push(currentMeetingId); },
    shouldAutoGenerate: true,
    summaryData: null,
  };

  runtime.render(() => PageContent(props));
  currentMeetingId = 'meeting-b';
  props.meeting = { ...props.meeting, id: currentMeetingId, title: 'Meeting B' };
  runtime.render(() => PageContent(props));
  await new Promise((resolve) => setImmediate(resolve));
  firstAttempt.resolve(true);
  await new Promise((resolve) => setImmediate(resolve));

  assert.deepEqual(generationMeetings, ['meeting-a', 'meeting-b']);
  assert.deepEqual(completionMeetings, ['meeting-b']);
  runtime.unmount();
});

test('meeting auto-summary StrictMode effect replay starts and completes once', async () => {
  const runtime = createHookRuntime();
  const attempt = deferred();
  const meetingData = {
    aiSummary: null,
    blockNoteSummaryRef: { current: null },
    handleSaveSummary: () => undefined,
    handleSummaryChange: () => undefined,
    isSaving: false,
    isTitleDirty: false,
    saveAllChanges: () => undefined,
    setAiSummary: () => undefined,
    setIsSummaryDirty: () => undefined,
    transcripts: [{ id: 1, text: 'Ready' }],
    updateMeetingTitle: () => undefined,
  };
  let generationCalls = 0;
  let completionCalls = 0;
  const summaryGeneration = {
    getSummaryStatusMessage: () => '',
    handleGenerateSummary: async () => {
      generationCalls += 1;
      return attempt.promise;
    },
    handleStopGeneration: () => undefined,
    summaryError: null,
    summaryStatus: 'idle',
  };
  const PageContent = loadPageContent(runtime, {
    getModelConfig: () => ({ model: 'model-a', provider: 'provider-a' }),
    meetingData,
    summaryGeneration,
  });
  const props = {
    meeting: { id: 'meeting-a', title: 'Meeting A', transcripts: meetingData.transcripts },
    onAutoGenerateComplete: () => { completionCalls += 1; },
    shouldAutoGenerate: true,
    summaryData: null,
  };

  runtime.render(() => PageContent(props));
  runtime.replayEffects();
  attempt.resolve(true);
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(generationCalls, 1);
  assert.equal(completionCalls, 1);
  runtime.unmount();
});

test('meeting auto-summary does not complete after a real unmount', async () => {
  const runtime = createHookRuntime();
  const attempt = deferred();
  const meetingData = {
    aiSummary: null,
    blockNoteSummaryRef: { current: null },
    handleSaveSummary: () => undefined,
    handleSummaryChange: () => undefined,
    isSaving: false,
    isTitleDirty: false,
    saveAllChanges: () => undefined,
    setAiSummary: () => undefined,
    setIsSummaryDirty: () => undefined,
    transcripts: [{ id: 1, text: 'Ready' }],
    updateMeetingTitle: () => undefined,
  };
  let completionCalls = 0;
  const summaryGeneration = {
    getSummaryStatusMessage: () => '',
    handleGenerateSummary: async () => attempt.promise,
    handleStopGeneration: () => undefined,
    summaryError: null,
    summaryStatus: 'idle',
  };
  const PageContent = loadPageContent(runtime, {
    getModelConfig: () => ({ model: 'model-a', provider: 'provider-a' }),
    meetingData,
    summaryGeneration,
  });

  runtime.render(() => PageContent({
    meeting: { id: 'meeting-a', title: 'Meeting A', transcripts: meetingData.transcripts },
    onAutoGenerateComplete: () => { completionCalls += 1; },
    shouldAutoGenerate: true,
    summaryData: null,
  }));
  runtime.unmount();
  attempt.resolve(true);
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(completionCalls, 0);
});

test('AI summary mount does not create a duplicate undo entry', () => {
  const runtime = createHookRuntime();
  const documentEvents = createDocumentEvents();
  const summary = {
    Agenda: {
      title: 'Agenda',
      blocks: [{ id: 'Agenda-1', type: 'text', content: 'One', color: 'default' }],
    },
  };
  let summaryChanges = 0;
  const { AISummary } = loadModule(
    'src/components/AISummary/index.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      './Section': { Section: Symbol('Section') },
      '@heroicons/react/24/outline': { ExclamationTriangleIcon: Symbol('icon') },
    },
    {
      console: quietConsole,
      document: documentEvents,
      navigator: { clipboard: { writeText: () => undefined } },
    },
  );
  const renderSummary = () => runtime.render(() => AISummary({
    error: null,
    onSummaryChange: () => { summaryChanges += 1; },
    status: 'completed',
    summary,
  }));

  renderSummary();
  renderSummary();
  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: false,
  });

  assert.equal(summaryChanges, 0);
  runtime.unmount();
});

test('AI summary undo and redo navigate history without adding loop entries', () => {
  const runtime = createHookRuntime();
  const documentEvents = createDocumentEvents();
  const section = Symbol('Section');
  const firstSummary = {
    Agenda: {
      title: 'Agenda',
      blocks: [{ id: 'Agenda-1', type: 'text', content: 'One', color: 'default' }],
    },
  };
  const secondSummary = {
    Agenda: {
      title: 'Agenda',
      blocks: [{ id: 'Agenda-1', type: 'text', content: 'Two', color: 'default' }],
    },
  };
  const changes = [];
  const { AISummary } = loadModule(
    'src/components/AISummary/index.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      './Section': { Section: section },
      '@heroicons/react/24/outline': { ExclamationTriangleIcon: Symbol('icon') },
    },
    {
      console: quietConsole,
      document: documentEvents,
      navigator: { clipboard: { writeText: () => undefined } },
    },
  );
  const renderSummary = (summary) => runtime.render(() => AISummary({
    error: null,
    onSummaryChange: (nextSummary) => { changes.push(nextSummary); },
    status: 'completed',
    summary,
  }));

  renderSummary(firstSummary);
  renderSummary(secondSummary);
  renderSummary(secondSummary);
  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: false,
  });
  assert.equal(changes.at(-1).Agenda.blocks[0].id, 'Agenda-1');
  assert.equal(changes.at(-1).Agenda.blocks[0].content, 'One');

  renderSummary(firstSummary);
  renderSummary(firstSummary);
  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: true,
  });
  assert.equal(changes.at(-1).Agenda.blocks[0].id, 'Agenda-1');
  assert.equal(changes.at(-1).Agenda.blocks[0].content, 'Two');

  renderSummary(secondSummary);
  renderSummary(secondSummary);
  const changeCount = changes.length;
  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: true,
  });
  assert.equal(changes.length, changeCount);
  runtime.unmount();
});

test('AI summary history preserves normalized backend block IDs through undo and redo', () => {
  const runtime = createHookRuntime();
  const documentEvents = createDocumentEvents();
  const section = Symbol('Section');
  const firstSummary = {
    Agenda: {
      title: 'Agenda',
      blocks: [{ id: 'backend-first', type: 'text', content: 'One', color: 'default' }],
    },
  };
  const secondSummary = {
    Agenda: {
      title: 'Agenda',
      blocks: [{ id: 'backend-second', type: 'text', content: 'Two', color: 'default' }],
    },
  };
  const changes = [];
  const { AISummary } = loadModule(
    'src/components/AISummary/index.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      './Section': { Section: section },
      '@heroicons/react/24/outline': { ExclamationTriangleIcon: Symbol('icon') },
    },
    {
      console: quietConsole,
      document: documentEvents,
      navigator: { clipboard: { writeText: () => undefined } },
    },
  );
  const renderSummary = (summary) => runtime.render(() => AISummary({
    error: null,
    onSummaryChange: (nextSummary) => { changes.push(nextSummary); },
    status: 'completed',
    summary,
  }));
  const renderedBlockId = (rendered) =>
    findJsxNode(rendered, (node) => node.type === section).props.section.blocks[0].id;

  const firstRendered = renderSummary(firstSummary);
  const normalizedFirstId = renderedBlockId(firstRendered);
  const secondRendered = renderSummary(secondSummary);
  const normalizedSecondId = renderedBlockId(secondRendered);
  renderSummary(secondSummary);

  assert.notEqual(normalizedFirstId, 'backend-first');
  assert.notEqual(normalizedSecondId, 'backend-second');

  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: false,
  });
  const undoneSummary = changes.at(-1);
  assert.equal(undoneSummary.Agenda.blocks[0].id, normalizedFirstId);
  assert.equal(renderedBlockId(renderSummary(undoneSummary)), normalizedFirstId);
  renderSummary(undoneSummary);

  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: true,
  });
  const redoneSummary = changes.at(-1);
  assert.equal(redoneSummary.Agenda.blocks[0].id, normalizedSecondId);
  assert.equal(renderedBlockId(renderSummary(redoneSummary)), normalizedSecondId);
  renderSummary(redoneSummary);

  const changeCount = changes.length;
  documentEvents.dispatch('keydown', {
    key: 'z',
    metaKey: true,
    preventDefault: () => undefined,
    shiftKey: true,
  });
  assert.equal(changes.length, changeCount);
  runtime.unmount();
});

test('AI summary keyboard delete uses the latest selected blocks', () => {
  const runtime = createHookRuntime();
  const documentEvents = createDocumentEvents();
  const section = Symbol('Section');
  const summary = {
    Agenda: {
      title: 'Agenda',
      blocks: [
        { id: 'Agenda-1', type: 'text', content: 'One', color: 'default' },
        { id: 'Agenda-2', type: 'text', content: 'Two', color: 'default' },
        { id: 'Agenda-3', type: 'text', content: 'Three', color: 'default' },
      ],
    },
  };
  let changedSummary;
  const { AISummary } = loadModule(
    'src/components/AISummary/index.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      './Section': { Section: section },
      '@heroicons/react/24/outline': { ExclamationTriangleIcon: Symbol('icon') },
    },
    {
      console: quietConsole,
      document: documentEvents,
      navigator: { clipboard: { writeText: () => undefined } },
    },
  );
  const renderSummary = () => runtime.render(() => AISummary({
    error: null,
    onSummaryChange: (nextSummary) => { changedSummary = nextSummary; },
    status: 'completed',
    summary,
  }));

  let rendered = renderSummary();
  let sectionNode = findJsxNode(rendered, (node) => node.type === section);
  sectionNode.props.onBlockMouseDown('Agenda-1', { shiftKey: false });
  rendered = renderSummary();
  sectionNode = findJsxNode(rendered, (node) => node.type === section);
  sectionNode.props.onBlockMouseEnter('Agenda-2');
  renderSummary();

  documentEvents.dispatch('keydown', {
    ctrlKey: false,
    key: 'Delete',
    metaKey: false,
    preventDefault: () => undefined,
  });

  assert.deepEqual(
    changedSummary.Agenda.blocks.map((block) => block.id),
    ['Agenda-3'],
  );
  runtime.unmount();
});

test('summary generation model-required error calls the latest settings handler', async () => {
  const runtime = createHookRuntime();
  let pollingCallback;
  let firstCalls = 0;
  let latestCalls = 0;
  const sidebar = {
    startSummaryPolling: (_meetingId, _processId, callback) => { pollingCallback = callback; },
    stopSummaryPolling: () => undefined,
  };
  const transcript = {
    audio_start_time: 0,
    id: 1,
    is_partial: false,
    sequence_id: 1,
    text: 'Transcript',
    timestamp: '00:00',
  };
  const invoke = async (command) => {
    if (command === 'api_get_meeting_transcripts') {
      return { has_more: false, total_count: 1, transcripts: [transcript] };
    }
    if (command === 'api_process_transcript') {
      return { message: 'started', process_id: 'process-a' };
    }
    throw new Error(`Unexpected command: ${command}`);
  };
  const { useSummaryGeneration } = loadModule(
    'src/hooks/meeting-details/useSummaryGeneration.ts',
    {
      react: runtime.react,
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@/components/Sidebar/SidebarProvider': { useSidebar: () => sidebar },
      '@tauri-apps/api/core': { invoke },
      sonner: {
        toast: {
          error: () => undefined,
          info: () => undefined,
          success: () => undefined,
          warning: () => undefined,
        },
      },
      '@/lib/utils': { isOllamaNotInstalledError: () => false },
      '@/lib/summary-language-preferences': {
        detectAndCacheSummaryLanguage: async () => ({ language: 'en', reason: 'detected' }),
        readCachedDetectedSummaryLanguage: async () => null,
        readMeetingSummaryLanguage: async () => ({ language: 'en' }),
      },
      '@/lib/meeting-summary': {
        normalizeMeetingSummary: () => null,
        readMeetingNameFromSummaryPayload: () => null,
      },
    },
    { console: quietConsole },
  );
  const stableProps = {
    isModelConfigLoading: false,
    meeting: { created_at: '2026-07-15T00:00:00Z', id: 'meeting-a' },
    modelConfig: { model: 'model-a', provider: 'openrouter' },
    onMeetingUpdated: async () => undefined,
    selectedTemplate: 'default',
    setAiSummary: () => undefined,
    transcripts: [],
    updateMeetingTitle: () => undefined,
  };

  runtime.render(() => useSummaryGeneration({
    ...stableProps,
    onOpenModelSettings: () => { firstCalls += 1; },
  }));
  const latest = runtime.render(() => useSummaryGeneration({
    ...stableProps,
    onOpenModelSettings: () => { latestCalls += 1; },
  }));

  const didStart = await latest.handleGenerateSummary();
  await pollingCallback({ error: 'model is required', status: 'error' });

  assert.equal(didStart, true);
  assert.equal(firstCalls, 0);
  assert.equal(latestCalls, 1);
  runtime.unmount();
});

test('summary generation reports whether polling actually started', async () => {
  const runtime = createHookRuntime();
  let mode = 'no-transcripts';
  let pollingStarts = 0;
  const transcript = {
    audio_start_time: 0,
    id: 1,
    is_partial: false,
    sequence_id: 1,
    text: 'Transcript',
    timestamp: '00:00',
  };
  const invoke = async (command) => {
    if (command === 'api_get_meeting_transcripts') {
      return mode === 'no-transcripts'
        ? { has_more: false, total_count: 0, transcripts: [] }
        : { has_more: false, total_count: 1, transcripts: [transcript] };
    }
    if (command === 'api_process_transcript') {
      if (mode === 'setup-error') throw new Error('setup failed');
      return { message: 'started', process_id: 'process-a' };
    }
    throw new Error(`Unexpected command: ${command}`);
  };
  const useSummaryGeneration = loadSummaryGeneration(runtime, {
    invoke,
    sidebar: {
      startSummaryPolling: () => { pollingStarts += 1; },
      stopSummaryPolling: () => undefined,
    },
  });
  const hook = runtime.render(() => useSummaryGeneration({
    isModelConfigLoading: false,
    meeting: { created_at: '2026-07-15T00:00:00Z', id: 'meeting-a' },
    modelConfig: { model: 'model-a', provider: 'openrouter' },
    selectedTemplate: 'default',
    setAiSummary: () => undefined,
    transcripts: [],
    updateMeetingTitle: () => undefined,
  }));

  assert.equal(await hook.handleGenerateSummary(), false);
  mode = 'setup-error';
  assert.equal(await hook.handleGenerateSummary(), false);
  mode = 'success';
  assert.equal(await hook.handleGenerateSummary(), true);
  assert.equal(pollingStarts, 1);
  runtime.unmount();
});

test('MessageToast keeps one timeout and calls the latest close callback', () => {
  const runtime = createHookRuntime();
  const timers = createTimers();
  const { MessageToast } = loadModule(
    'src/components/MessageToast.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': { jsx: () => null },
    },
    timers,
  );
  let firstCalls = 0;
  let latestCalls = 0;

  runtime.render(() =>
    MessageToast({
      message: 'Saved',
      type: 'success',
      show: true,
      setShow: () => { firstCalls += 1; },
    }),
  );
  runtime.render(() =>
    MessageToast({
      message: 'Saved',
      type: 'success',
      show: true,
      setShow: () => { latestCalls += 1; },
    }),
  );

  assert.equal(timers.activeCount(), 1);
  timers.runAll();
  assert.equal(firstCalls, 0);
  assert.equal(latestCalls, 1);
  runtime.unmount();
});

test('SidebarProvider fetches meetings natively without legacy server addresses', async () => {
  const runtime = createHookRuntime();
  const invocations = [];
  const { SidebarProvider } = loadModule(
    'src/components/Sidebar/SidebarProvider.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': {
        jsx: (type, props) => ({ type, props }),
        jsxs: (type, props) => ({ type, props }),
      },
      'next/navigation': {
        usePathname: () => '/meeting-details',
        useRouter: () => ({ push: () => undefined }),
      },
      '@tauri-apps/api/core': {
        invoke: async (command) => {
          invocations.push(command);
          return [{ id: 'native-meeting', title: 'Native Meeting' }];
        },
      },
      '@tauri-apps/api/event': {
        listen: async () => () => undefined,
      },
      '@/contexts/RecordingStateContext': {
        useRecordingState: () => ({ isRecording: false }),
      },
      '@/contexts/I18nContext': {
        useI18n: () => ({ locale: 'en', t: (key) => key }),
      },
    },
  );

  const rendered = runtime.render(() => SidebarProvider({ children: null }));
  await Promise.resolve();
  await Promise.resolve();

  assert.deepEqual(invocations, ['api_get_meetings']);
  assert.equal('serverAddress' in rendered.props.value, false);
  assert.equal('setServerAddress' in rendered.props.value, false);
  assert.equal('transcriptServerAddress' in rendered.props.value, false);
  assert.equal('setTranscriptServerAddress' in rendered.props.value, false);

  await rendered.props.value.refetchMeetings();
  assert.deepEqual(invocations, ['api_get_meetings', 'api_get_meetings']);
  runtime.unmount();
});

test('SidebarProvider refreshes processing rows until a background error without details mounted', async () => {
  const runtime = createHookRuntime();
  const intervals = createIntervals();
  const responses = [
    [{ id: 'meeting-1', title: 'Meeting 1', status: 'processing' }],
    [{ id: 'meeting-1', title: 'Meeting 1', status: 'processing' }],
    [{ id: 'meeting-1', title: 'Meeting 1', status: 'error', lifecycle_error: 'Summary failed' }],
  ];
  let reads = 0;
  const { SidebarProvider } = loadModule(
    'src/components/Sidebar/SidebarProvider.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      'next/navigation': {
        usePathname: () => '/',
        useRouter: () => ({ push: () => undefined }),
      },
      '@tauri-apps/api/core': {
        invoke: async (command) => {
          assert.equal(command, 'api_get_meetings');
          const response = responses[Math.min(reads, responses.length - 1)];
          reads += 1;
          return response;
        },
      },
      '@tauri-apps/api/event': {
        listen: async () => () => undefined,
      },
      '@/contexts/RecordingStateContext': {
        useRecordingState: () => ({ isRecording: false }),
      },
      '@/contexts/I18nContext': {
        useI18n: () => ({ locale: 'en', t: (key) => key }),
      },
    },
    { console: quietConsole, ...intervals },
  );
  const renderProvider = () =>
    runtime.render(() => SidebarProvider({ children: null }));

  renderProvider();
  await new Promise((resolve) => setImmediate(resolve));

  let rendered = renderProvider();
  assert.equal(reads, 2);
  assert.equal(intervals.activeCount(), 1);

  await new Promise((resolve) => setImmediate(resolve));
  intervals.runAll();
  await new Promise((resolve) => setImmediate(resolve));

  rendered = renderProvider();
  assert.equal(reads, 3);
  assert.equal(rendered.props.value.meetings[0].status, 'error');
  assert.equal(intervals.activeCount(), 0);
  runtime.unmount();
});

test('SidebarProvider meeting refresh does not reset the current meeting on home', () => {
  const runtime = createHookRuntime();
  const jsxRuntime = {
    jsx: (type, props) => ({ type, props }),
    jsxs: (type, props) => ({ type, props }),
  };
  const { SidebarProvider } = loadModule(
    'src/components/Sidebar/SidebarProvider.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      'next/navigation': {
        usePathname: () => '/',
        useRouter: () => ({ push: () => undefined }),
      },
      '@tauri-apps/api/core': {
        invoke: () => new Promise(() => undefined),
      },
      '@tauri-apps/api/event': {
        listen: async () => () => undefined,
      },
      '@/contexts/RecordingStateContext': {
        useRecordingState: () => ({ isRecording: false }),
      },
      '@/contexts/I18nContext': {
        useI18n: () => ({ locale: 'en', t: (key) => key }),
      },
    },
  );
  const renderProvider = () =>
    runtime.render(() => SidebarProvider({ children: null }));

  let rendered = renderProvider();
  rendered = renderProvider();
  rendered.props.value.setCurrentMeeting({ id: 'selected', title: 'Selected' });
  rendered = renderProvider();
  assert.equal(rendered.props.value.currentMeeting.id, 'selected');

  rendered.props.value.setMeetings([{ id: 'new-meeting', title: 'New Meeting' }]);
  renderProvider();
  rendered = renderProvider();

  assert.equal(rendered.props.value.currentMeeting.id, 'selected');
  runtime.unmount();
});

test('TranscriptContext history sync is triggered by recording state, not local clearing', () => {
  const runtime = createHookRuntime();
  const recordingState = { isRecording: false };
  let historyReads = 0;
  const never = () => new Promise(() => undefined);
  const { TranscriptProvider } = loadModule(
    'src/contexts/TranscriptContext.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      sonner: { toast: { success: () => undefined } },
      './RecordingStateContext': {
        useRecordingState: () => recordingState,
      },
      '@/services/transcriptService': {
        transcriptService: {
          getTranscriptHistory: () => {
            historyReads += 1;
            return never();
          },
          onTranscriptUpdate: never,
        },
      },
      '@/services/recordingService': {
        recordingService: {
          getRecordingMeetingName: never,
          onRecordingStarted: never,
          onRecordingStopped: () => {
            throw new Error('recording listener setup should remain blocked on init');
          },
        },
      },
      '@/services/indexedDBService': {
        indexedDBService: { init: never },
      },
    },
    {
      console: quietConsole,
      navigator: { clipboard: { writeText: () => undefined } },
      sessionStorage: {
        getItem: () => null,
        removeItem: () => undefined,
        setItem: () => undefined,
      },
    },
  );
  const renderProvider = () =>
    runtime.render(() => TranscriptProvider({ children: null }));

  let rendered = renderProvider();
  recordingState.isRecording = true;
  rendered = renderProvider();
  assert.equal(historyReads, 1);

  rendered.props.value.addTranscript({
    is_partial: false,
    sequence_id: 1,
    text: 'Local transcript',
    timestamp: '00:01',
  });
  rendered = renderProvider();
  rendered.props.value.clearTranscripts();
  renderProvider();

  assert.equal(historyReads, 1);
  runtime.unmount();
});

test('model listener resolving after unmount is immediately removed without registering more', async () => {
  const runtime = createHookRuntime();
  const firstListener = deferred();
  let listenCalls = 0;
  let lateUnlistenCalls = 0;
  const never = () => new Promise(() => undefined);
  const motion = new Proxy({}, { get: () => Symbol('motion') });
  const { ParakeetModelManager } = loadModule(
    'src/components/ParakeetModelManager.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@tauri-apps/api/event': {
        listen: () => {
          listenCalls += 1;
          return firstListener.promise;
        },
      },
      '@tauri-apps/api/core': { invoke: never },
      'framer-motion': { AnimatePresence: Symbol('AnimatePresence'), motion },
      sonner: {
        toast: {
          error: () => undefined,
          info: () => undefined,
          success: () => undefined,
        },
      },
      '../lib/parakeet': {
        ParakeetAPI: {
          init: never,
          getAvailableModels: never,
        },
        formatFileSize: () => '',
        getModelDisplayInfo: () => undefined,
        getModelDisplayName: (name) => name,
      },
    },
    { console: quietConsole },
  );

  runtime.render(() => ParakeetModelManager({}));
  runtime.unmount();
  firstListener.resolve(() => { lateUnlistenCalls += 1; });
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(lateUnlistenCalls, 1);
  assert.equal(listenCalls, 1);
});

test('recording listener resolving after unmount is immediately removed without registering more', async () => {
  const runtime = createHookRuntime();
  const firstListener = deferred();
  let startedRegistrations = 0;
  let stoppedRegistrations = 0;
  let routeRegistrations = 0;
  let degradationRegistrations = 0;
  let lateUnlistenCalls = 0;
  const never = () => new Promise(() => undefined);
  const { RecordingStateProvider } = loadModule(
    'src/contexts/RecordingStateContext.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      sonner: { toast: { warning: () => undefined } },
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@/services/recordingService': {
        recordingService: {
          getActiveRecordingSession: never,
          getRecordingSession: never,
          onRecordingStarted: () => {
            startedRegistrations += 1;
            return firstListener.promise;
          },
          onRecordingStopped: () => {
            stoppedRegistrations += 1;
            return never();
          },
          onRecordingPaused: never,
          onRecordingResumed: never,
          onAudioRouteChanged: () => {
            routeRegistrations += 1;
            return never();
          },
          onAudioDegraded: () => {
            degradationRegistrations += 1;
            return never();
          },
        },
      },
    },
    { console: quietConsole },
  );

  runtime.render(() => RecordingStateProvider({ children: null }));
  runtime.unmount();
  firstListener.resolve(() => { lateUnlistenCalls += 1; });
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(startedRegistrations, 1);
  assert.equal(stoppedRegistrations, 1);
  assert.equal(routeRegistrations, 1);
  assert.equal(degradationRegistrations, 1);
  assert.equal(lateUnlistenCalls, 1);
});

test('audio lifecycle callbacks delivered after unmount do not notify or refresh before listener resolution', async () => {
  const runtime = createHookRuntime();
  const intervals = createIntervals();
  const routeRegistration = deferred();
  const degradationRegistration = deferred();
  const warnings = [];
  let routeCallback;
  let degradationCallback;
  let sessionReads = 0;
  let lateUnlistenCalls = 0;
  const activeState = {
    activeDuration: 5,
    autoSave: true,
    expectedTracks: ['microphone', 'system'],
    meetingId: 'meeting-1',
    meetingName: 'Meeting 1',
    microphoneDevice: 'Studio Microphone',
    pauseDuration: 0,
    recordingDuration: 5,
    savePath: '/tmp/meeting-1',
    sessionId: 'session-1',
    startedAtMs: 1,
    state: 'recording',
    systemDevice: 'System Audio',
  };
  const unlisten = async () => () => undefined;
  const { RecordingStateProvider } = loadModule(
    'src/contexts/RecordingStateContext.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      sonner: { toast: { warning: (message) => warnings.push(message) } },
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@/services/recordingService': {
        degradationMessage: () => 'degraded',
        routeChangeMessage: () => 'route changed',
        recordingService: {
          getActiveRecordingSession: async () => activeState,
          getRecordingSession: async () => {
            sessionReads += 1;
            return activeState;
          },
          onRecordingStarted: unlisten,
          onRecordingStopped: unlisten,
          onRecordingPaused: unlisten,
          onRecordingResumed: unlisten,
          onAudioRouteChanged: (callback) => {
            routeCallback = callback;
            return routeRegistration.promise;
          },
          onAudioDegraded: (callback) => {
            degradationCallback = callback;
            return degradationRegistration.promise;
          },
        },
      },
    },
    { console: quietConsole, ...intervals },
  );

  const render = () => runtime.render(() => RecordingStateProvider({ children: null }));
  render();
  await new Promise((resolve) => setImmediate(resolve));
  render();
  runtime.unmount();

  routeCallback({
    sessionId: 'session-1',
    previousDevice: 'Studio Microphone',
    currentDevice: 'Built-in Microphone',
    reason: 'active microphone became unavailable',
  });
  degradationCallback({
    sessionId: 'session-1',
    source: 'system',
    reason: 'system stream stopped',
  });
  await new Promise((resolve) => setImmediate(resolve));

  assert.deepEqual(warnings, []);
  assert.equal(sessionReads, 0);

  routeRegistration.resolve(() => { lateUnlistenCalls += 1; });
  degradationRegistration.resolve(() => { lateUnlistenCalls += 1; });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(lateUnlistenCalls, 2);
});

test('active backend recording on reload starts one polling interval and cleans it up', async () => {
  const runtime = createHookRuntime();
  const intervals = createIntervals();
  let stateReads = 0;
  const unlisten = async () => () => undefined;
  const activeState = {
    activeDuration: 5,
    capturedTracks: undefined,
    expectedTracks: ['microphone'],
    meetingId: 'meeting-1',
    meetingName: 'Meeting 1',
    microphoneDevice: null,
    pauseDuration: 0,
    recordingDuration: 5,
    savePath: '/tmp/meeting-1',
    sessionId: 'session-1',
    startedAtMs: 1,
    state: 'recording',
    systemDevice: null,
  };
  const { RecordingStateProvider } = loadModule(
    'src/contexts/RecordingStateContext.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      sonner: { toast: { warning: () => undefined } },
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@/services/recordingService': {
        recordingService: {
          getActiveRecordingSession: async () => {
            stateReads += 1;
            return activeState;
          },
          getRecordingSession: async () => activeState,
          onRecordingStarted: unlisten,
          onRecordingStopped: unlisten,
          onRecordingPaused: unlisten,
          onRecordingResumed: unlisten,
          onAudioRouteChanged: unlisten,
          onAudioDegraded: unlisten,
        },
      },
    },
    { console: quietConsole, ...intervals },
  );
  const renderProvider = () =>
    runtime.render(() => RecordingStateProvider({ children: null }));

  renderProvider();
  await new Promise((resolve) => setImmediate(resolve));
  renderProvider();
  assert.equal(intervals.activeCount(), 1);

  renderProvider();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(intervals.activeCount(), 1);
  assert.equal(stateReads, 1);

  runtime.unmount();
  assert.equal(intervals.activeCount(), 0);
});

test('older_snapshot_response_cannot_overwrite_newer_session', async () => {
  const runtime = createHookRuntime();
  const intervals = createIntervals();
  const oldActive = deferred();
  let startedListener;
  const unlisten = async (callback) => {
    startedListener = callback;
    return () => undefined;
  };
  const snapshot = (sessionId) => ({
    activeDuration: 1,
    autoSave: true,
    expectedTracks: ['microphone'],
    meetingId: `meeting-${sessionId}`,
    meetingName: sessionId,
    microphoneDevice: null,
    pauseDuration: 0,
    recordingDuration: 1,
    savePath: `/tmp/${sessionId}`,
    sessionId,
    startedAtMs: 1,
    state: 'recording',
    systemDevice: null,
  });
  const { RecordingStateProvider } = loadModule(
    'src/contexts/RecordingStateContext.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      sonner: { toast: { warning: () => undefined } },
      '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
      '@/services/recordingService': {
        recordingService: {
          getActiveRecordingSession: () => oldActive.promise,
          getRecordingSession: async (sessionId) => snapshot(sessionId),
          onRecordingStarted: unlisten,
          onRecordingStopped: async () => () => undefined,
          onRecordingPaused: async () => () => undefined,
          onRecordingResumed: async () => () => undefined,
          onAudioRouteChanged: async () => () => undefined,
          onAudioDegraded: async () => () => undefined,
        },
      },
    },
    { console: quietConsole, ...intervals },
  );

  const render = () => runtime.render(() => RecordingStateProvider({ children: null }));
  render();
  startedListener('new-session');
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(render().props.value.snapshot.sessionId, 'new-session');

  oldActive.resolve(snapshot('old-session'));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(render().props.value.snapshot.sessionId, 'new-session');
  runtime.unmount();
});
