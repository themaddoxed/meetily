import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

const frontendDir = fileURLToPath(new URL('../..', import.meta.url));
const layoutPath = `${frontendDir}/src/app/layout.tsx`;

const analyticsPreferencePattern = /analytics(?:optedin|defaultoff|migration|\.json)/i;

function flushMicrotasks() {
  return new Promise((resolve) => setImmediate(resolve));
}

function passthrough({ children }) {
  return children ?? null;
}

function defaultModule(value) {
  return { __esModule: true, default: value };
}

function createHookHarness() {
  const componentStates = new Map();
  let activeState = null;

  function dependenciesMatch(left, right) {
    return left?.length === right?.length
      && left.every((value, index) => Object.is(value, right[index]));
  }

  function createComponentState() {
    const stateSlots = [];
    const refSlots = [];
    const callbackSlots = [];
    const memoSlots = [];
    const effectSlots = [];
    let stateIndex = 0;
    let refIndex = 0;
    let callbackIndex = 0;
    let memoIndex = 0;
    let effectIndex = 0;

    return {
      reset() {
        stateIndex = 0;
        refIndex = 0;
        callbackIndex = 0;
        memoIndex = 0;
        effectIndex = 0;
      },
      useState(initialValue) {
        const slot = stateIndex++;
        if (!(slot in stateSlots)) {
          stateSlots[slot] = typeof initialValue === 'function'
            ? initialValue()
            : initialValue;
        }
        return [
          stateSlots[slot],
          (nextValue) => {
            stateSlots[slot] = typeof nextValue === 'function'
              ? nextValue(stateSlots[slot])
              : nextValue;
          },
        ];
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
        };
      },
    };
  }

  const react = {
    createContext(defaultValue) {
      return { defaultValue, Provider: Symbol('Context.Provider') };
    },
    useContext(context) {
      return context.defaultValue;
    },
    useState(initialValue) {
      return activeState.useState(initialValue);
    },
    useRef(initialValue) {
      return activeState.useRef(initialValue);
    },
    useCallback(callback, dependencies) {
      return activeState.useCallback(callback, dependencies);
    },
    useMemo(factory, dependencies) {
      return activeState.useMemo(factory, dependencies);
    },
    useEffect(effect, dependencies) {
      return activeState.useEffect(effect, dependencies);
    },
  };

  function renderComponent(component, props = {}) {
    let state = componentStates.get(component);
    if (!state) {
      state = createComponentState();
      componentStates.set(component, state);
    }
    state.reset();
    const previousState = activeState;
    activeState = state;
    try {
      return component(props);
    } finally {
      activeState = previousState;
    }
  }

  function renderTree(node) {
    if (Array.isArray(node)) {
      node.forEach(renderTree);
      return;
    }
    if (!node || typeof node !== 'object') return;
    if (typeof node.type === 'function') {
      renderTree(renderComponent(node.type, node.props ?? {}));
      return;
    }
    renderTree(node.props?.children);
  }

  return { react, renderComponent, renderTree };
}

function createLocalStorage(initialValues, reads) {
  const values = new Map(Object.entries(initialValues));
  return {
    getItem(key) {
      reads.push({ operation: 'get', key });
      return values.has(key) ? values.get(key) : null;
    },
    setItem(key, value) {
      reads.push({ operation: 'set', key, value: String(value) });
      values.set(key, String(value));
    },
    removeItem(key) {
      reads.push({ operation: 'remove', key });
      values.delete(key);
    },
  };
}

function createStoreModule(seed, boundaries) {
  const values = new Map(Object.entries(seed));

  async function load(name, options) {
    boundaries.storeCalls.push({ operation: 'load', name, options });
    return {
      async has(key) {
        boundaries.storeCalls.push({ operation: 'has', name, key });
        return values.has(key);
      },
      async get(key) {
        boundaries.storeCalls.push({ operation: 'get', name, key });
        return values.get(key);
      },
      async set(key, value) {
        boundaries.storeCalls.push({ operation: 'set', name, key, value });
        values.set(key, value);
      },
      async save() {
        boundaries.storeCalls.push({ operation: 'save', name });
      },
    };
  }

  return {
    __esModule: true,
    Store: { load },
    load,
  };
}

function createAnalyticsModule(boundaries) {
  const analytics = {};
  const methods = [
    'init',
    'disable',
    'track',
    'identify',
    'startSession',
    'endSession',
    'trackSessionStarted',
    'trackSessionEnded',
    'trackAppStarted',
    'getPersistentUserId',
    'getDeviceInfo',
    'checkAndTrackFirstLaunch',
    'checkAndTrackDailyUsage',
    'cleanup',
  ];

  for (const method of methods) {
    analytics[method] = async (...args) => {
      boundaries.analyticsCalls.push({ method, args });
      if (method === 'getPersistentUserId') return 'test-user';
      if (method === 'getDeviceInfo') {
        return { platform: 'test', os_version: 'test', architecture: 'test' };
      }
      if (method === 'startSession') return 'test-session';
      return undefined;
    };
  }

  return { __esModule: true, default: analytics, Analytics: analytics };
}

function createBoundaries(scenario) {
  const localStorageReads = [];
  const boundaries = {
    analyticsCalls: [],
    fetchCalls: [],
    invokeCalls: [],
    listenCalls: [],
    storeCalls: [],
    windowDispatches: [],
    windowListeners: [],
    documentListeners: [],
    localStorageReads,
  };

  const localStorage = createLocalStorage(scenario.localStorage, localStorageReads);
  const window = {
    localStorage,
    matchMedia() {
      return {
        matches: false,
        addEventListener() {},
        removeEventListener() {},
      };
    },
    addEventListener(type, listener) {
      boundaries.windowListeners.push({ operation: 'add', type, listener });
    },
    removeEventListener(type, listener) {
      boundaries.windowListeners.push({ operation: 'remove', type, listener });
    },
    dispatchEvent(event) {
      boundaries.windowDispatches.push(event.type);
      return true;
    },
  };
  const document = {
    documentElement: { classList: { contains: () => false } },
    addEventListener(type, listener) {
      boundaries.documentListeners.push({ operation: 'add', type, listener });
    },
    removeEventListener(type, listener) {
      boundaries.documentListeners.push({ operation: 'remove', type, listener });
    },
  };

  async function invoke(command, args) {
    boundaries.invokeCalls.push({ command, args });
    if (command === 'get_onboarding_status') return { completed: true };
    if (command === 'get_ollama_models') return [];
    if (command === 'api_get_api_key') return null;
    if (command === 'api_get_meetings') {
      if (scenario.meetingsError) throw new Error('test meeting list failure');
      return scenario.meetings ?? [{ id: 'meeting-1', title: 'Test meeting' }];
    }
    return undefined;
  }

  async function listen(event, callback) {
    boundaries.listenCalls.push({ event, callback });
    return () => undefined;
  }

  async function fetch(...args) {
    boundaries.fetchCalls.push(args);
    return {
      ok: true,
      status: 200,
      async json() { return {}; },
      async text() { return ''; },
    };
  }

  return {
    ...boundaries,
    scenario,
    document,
    fetch,
    invoke,
    listen,
    localStorage,
    storeModule: createStoreModule(scenario.store, boundaries),
    window,
    analyticsModule: createAnalyticsModule(boundaries),
  };
}

function loadSidebarProvider(harness, boundaries) {
  const source = fs.readFileSync(`${frontendDir}/src/components/Sidebar/SidebarProvider.tsx`, 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  const modules = {
    react: harness.react,
    'react/jsx-runtime': {
      Fragment: Symbol('Fragment'),
      jsx: (type, props) => ({ type, props }),
      jsxs: (type, props) => ({ type, props }),
    },
    'next/navigation': {
      usePathname: () => '/',
      useRouter: () => ({ push() {} }),
    },
    '@tauri-apps/api/core': { invoke: boundaries.invoke },
    '@tauri-apps/api/event': { listen: boundaries.listen },
    '@/contexts/RecordingStateContext': { useRecordingState: () => ({ isRecording: false }) },
    '@/contexts/I18nContext': { useI18n: () => ({ locale: 'en', t: (key) => key }) },
    '@/lib/analytics': boundaries.analyticsModule,
  };

  vm.runInNewContext(compiled, {
    CustomEvent: class CustomEvent {
      constructor(type, init = {}) {
        this.type = type;
        this.detail = init.detail;
      }
    },
    URL,
    clearInterval() {},
    console: { error() {}, log() {}, warn() {} },
    exports: commonJsModule.exports,
    module: commonJsModule,
    require(specifier) {
      if (!(specifier in modules)) throw new Error(`Unexpected dependency: ${specifier}`);
      return modules[specifier];
    },
    setInterval() { return 1; },
    window: boundaries.window,
  });

  return commonJsModule.exports.SidebarProvider;
}

function loadRootLayout(harness, boundaries) {
  const source = fs.readFileSync(layoutPath, 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  const noop = () => null;
  const noopNamed = (name) => ({ __esModule: true, [name]: noop });
  const SidebarProvider = loadSidebarProvider(harness, boundaries);
  const modules = {
    './globals.css': {},
    'sonner/dist/styles.css': {},
    react: harness.react,
    'react/jsx-runtime': {
      Fragment: Symbol('Fragment'),
      jsx: (type, props) => ({ type, props }),
      jsxs: (type, props) => ({ type, props }),
    },
    '@tauri-apps/api/core': { invoke: boundaries.invoke },
    '@tauri-apps/api/event': { listen: boundaries.listen },
    '@tauri-apps/plugin-store': boundaries.storeModule,
    '@/lib/analytics': boundaries.analyticsModule,
    '@/components/Sidebar': defaultModule(noop),
    '@/components/Sidebar/SidebarProvider': { __esModule: true, SidebarProvider },
    '@/components/MainContent': defaultModule(passthrough),
    sonner: {
      __esModule: true,
      Toaster: noop,
      toast: { error: () => undefined },
    },
    '@/components/ui/tooltip': { __esModule: true, TooltipProvider: passthrough },
    '@/contexts/RecordingStateContext': {
      __esModule: true,
      RecordingStateProvider: passthrough,
      useRecordingState: () => ({ isRecording: false }),
    },
    '@/contexts/OllamaDownloadContext': { __esModule: true, OllamaDownloadProvider: passthrough },
    '@/contexts/TranscriptContext': { __esModule: true, TranscriptProvider: passthrough },
    '@/contexts/ConfigContext': {
      __esModule: true,
      ConfigProvider: passthrough,
      useConfig: () => ({ betaFeatures: { importAndRetranscribe: false } }),
    },
    '@/contexts/OnboardingContext': { __esModule: true, OnboardingProvider: passthrough },
    '@/components/onboarding': noopNamed('OnboardingFlow'),
    '@/types/betaFeatures': { loadBetaFeatures: () => ({ importAndRetranscribe: false }) },
    '@/components/shared/DownloadProgressToast': {
      __esModule: true,
      DownloadProgressToastProvider: passthrough,
    },
    '@/contexts/RecordingPostProcessingProvider': {
      __esModule: true,
      RecordingPostProcessingProvider: passthrough,
    },
    '@/components/ImportAudio': {
      __esModule: true,
      ImportAudioDialog: noop,
      ImportDropOverlay: noop,
    },
    '@/contexts/ImportDialogContext': { __esModule: true, ImportDialogProvider: passthrough },
    '@/constants/audioFormats': {
      isAudioExtension: () => false,
      getAudioFormatsDisplayList: () => '',
    },
    '@/testing/E2EBootstrap': noopNamed('E2EBootstrap'),
    '@/components/ThemeScript': noopNamed('ThemeScript'),
    '@/contexts/I18nContext': {
      __esModule: true,
      I18nProvider: passthrough,
      useI18n: () => ({ locale: 'en', t: (key) => key }),
    },
  };

  vm.runInNewContext(compiled, {
    CustomEvent: class CustomEvent {
      constructor(type, init = {}) {
        this.type = type;
        this.detail = init.detail;
      }
    },
    URL,
    clearTimeout() {},
    console: { error() {}, log() {}, warn() {} },
    document: boundaries.document,
    exports: commonJsModule.exports,
    fetch: boundaries.fetch,
    globalThis: undefined,
    module: commonJsModule,
    navigator: { userAgent: 'test' },
    process: { env: { NODE_ENV: 'test' } },
    require(specifier) {
      if (!(specifier in modules)) throw new Error(`Unexpected dependency: ${specifier}`);
      return modules[specifier];
    },
    setTimeout() { return 1; },
    window: boundaries.window,
  });

  return commonJsModule.exports.default;
}

function loadConfigProvider(harness, boundaries) {
  const source = fs.readFileSync(`${frontendDir}/src/contexts/ConfigContext.tsx`, 'utf8');
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
  }).outputText;
  const commonJsModule = { exports: {} };
  const configService = {
    async getTranscriptConfig() { return null; },
    async getModelConfig() { return null; },
    async getCustomOpenAIConfig() { return null; },
    async getRecordingPreferences() { return null; },
  };
  const modules = {
    react: harness.react,
    'react/jsx-runtime': {
      Fragment: Symbol('Fragment'),
      jsx: (type, props) => ({ type, props }),
      jsxs: (type, props) => ({ type, props }),
    },
    '@tauri-apps/api/core': { invoke: boundaries.invoke },
    '@tauri-apps/api/event': { listen: boundaries.listen },
    '@tauri-apps/plugin-store': boundaries.storeModule,
    '@/components/TranscriptSettings': {},
    '@/components/DeviceSelection': {},
    '@/services/configService': { configService },
    '@/types/betaFeatures': {
      loadBetaFeatures: () => ({ importAndRetranscribe: false }),
      saveBetaFeatures() {},
    },
    '@/lib/theme': {
      applyEffectiveTheme() {},
      createNativeThemeSyncController: () => ({
        async sync() {},
        dispose() {},
      }),
      readStoredThemeMode: () => 'system',
      resolveEffectiveTheme: () => 'light',
      THEME_STORAGE_KEY: 'theme',
    },
    '@/lib/analytics': boundaries.analyticsModule,
  };

  vm.runInNewContext(compiled, {
    CustomEvent: class CustomEvent {
      constructor(type, init = {}) {
        this.type = type;
        this.detail = init.detail;
      }
    },
    URL,
    clearTimeout() {},
    console: { error() {}, log() {}, warn() {} },
    document: boundaries.document,
    exports: commonJsModule.exports,
    fetch: boundaries.fetch,
    localStorage: boundaries.localStorage,
    module: commonJsModule,
    navigator: { userAgent: 'test' },
    process: { env: { NODE_ENV: 'test' } },
    require(specifier) {
      if (!(specifier in modules)) throw new Error(`Unexpected dependency: ${specifier}`);
      return modules[specifier];
    },
    setTimeout() { return 1; },
    window: boundaries.window,
  });

  return commonJsModule.exports.ConfigProvider;
}

function assertNoAnalyticsActivity(boundaries) {
  assert.deepEqual(
    boundaries.analyticsCalls,
    [],
    'startup must not initialize or call an analytics client',
  );
  assert.deepEqual(
    boundaries.storeCalls,
    [],
    'startup must not read or write the legacy analytics store',
  );
  assert.deepEqual(
    boundaries.fetchCalls,
    [],
    'startup must not issue a product analytics network request',
  );
  assert.deepEqual(
    boundaries.windowDispatches,
    [],
    'startup must not dispatch a product analytics event',
  );
  assert.deepEqual(
    boundaries.windowListeners.filter(({ type }) => /analytics|beforeunload/i.test(type)),
    [],
    'startup must not register an analytics lifecycle listener',
  );
  assert.deepEqual(
    boundaries.localStorageReads.filter(({ key }) => analyticsPreferencePattern.test(key)),
    [],
    'startup must not read legacy analytics preference keys',
  );
  assert.equal(
    boundaries.invokeCalls.filter(({ command }) => /analytics|track_|identify_/i.test(command)).length,
    0,
    'startup must not invoke an analytics Tauri command',
  );
  assert.equal(
    boundaries.listenCalls.filter(({ event }) => /analytics/i.test(event)).length,
    0,
    'startup must not subscribe to an analytics event',
  );
}

const scenarios = [
  {
    name: 'fresh preferences',
    localStorage: {},
    store: {},
  },
  {
    name: 'persisted legacy explicit opt-in and migration marker',
    localStorage: {
      analyticsOptedIn: 'true',
      analyticsDefaultOffMigrationV1: 'true',
    },
    store: {
      analyticsOptedIn: true,
      analyticsDefaultOffMigrationV1: true,
    },
  },
  {
    name: 'persisted explicit opt-in without migration marker',
    localStorage: {
      analyticsOptedIn: 'true',
    },
    store: {
      analyticsOptedIn: true,
    },
  },
];

for (const scenario of scenarios) {
  test(`RootLayout startup stays analytics-free with ${scenario.name}`, async () => {
    const boundaries = createBoundaries(scenario);
    const harness = createHookHarness();
    const RootLayout = loadRootLayout(harness, boundaries);

    const tree = harness.renderComponent(RootLayout, { children: 'test page' });
    harness.renderTree(tree);
    await flushMicrotasks();

    assert.deepEqual(
      boundaries.invokeCalls.map(({ command }) => command).filter((command) => command === 'get_onboarding_status'),
      ['get_onboarding_status'],
      'RootLayout should retain its ordinary onboarding status check',
    );
    assert.ok(
      boundaries.invokeCalls.some(({ command }) => command === 'api_get_meetings'),
      'the actual SidebarProvider should fetch its ordinary meeting list on startup',
    );
    assert.deepEqual(
      ['request-recording-toggle', 'tauri://drag-drop', 'tauri://drag-enter', 'tauri://drag-leave']
        .every((event) => boundaries.listenCalls.some((entry) => entry.event === event)),
      true,
      'RootLayout should retain ordinary recording and drag event subscriptions',
    );
    assertNoAnalyticsActivity(boundaries);
  });

  test(`ConfigProvider startup stays analytics-free with ${scenario.name}`, async () => {
    const boundaries = createBoundaries(scenario);
    const harness = createHookHarness();
    const ConfigProvider = loadConfigProvider(harness, boundaries);

    harness.renderComponent(ConfigProvider, { children: null });
    await flushMicrotasks();
    harness.renderComponent(ConfigProvider, { children: null });
    await flushMicrotasks();

    assert.ok(
      boundaries.invokeCalls.some(({ command }) => command === 'get_ollama_models'),
      'the actual startup provider should retain its ordinary model-list request',
    );
    assertNoAnalyticsActivity(boundaries);
  });
}

test('SidebarProvider startup stays analytics-free when the initial meeting fetch succeeds', async () => {
  const scenario = {
    name: 'successful meeting list',
    localStorage: {},
    store: {},
    meetings: [{ id: 'meeting-success', title: 'Meeting loaded' }],
  };
  const boundaries = createBoundaries(scenario);
  const harness = createHookHarness();
  const SidebarProvider = loadSidebarProvider(harness, boundaries);

  harness.renderComponent(SidebarProvider, { children: null });
  await flushMicrotasks();
  const tree = harness.renderComponent(SidebarProvider, { children: null });

  assert.equal(tree.props.value.meetings.length, 1);
  assert.equal(tree.props.value.meetings[0].id, 'meeting-success');
  assertNoAnalyticsActivity(boundaries);
});

test('SidebarProvider startup stays analytics-free when the initial meeting fetch fails', async () => {
  const scenario = {
    name: 'failed meeting list',
    localStorage: {},
    store: {},
    meetingsError: true,
  };
  const boundaries = createBoundaries(scenario);
  const harness = createHookHarness();
  const SidebarProvider = loadSidebarProvider(harness, boundaries);

  harness.renderComponent(SidebarProvider, { children: null });
  await flushMicrotasks();
  harness.renderComponent(SidebarProvider, { children: null });
  await flushMicrotasks();

  assert.equal(
    boundaries.invokeCalls.filter(({ command }) => command === 'api_get_meetings').length,
    1,
    'SidebarProvider should make one initial meeting-list request',
  );
  assertNoAnalyticsActivity(boundaries);
});
