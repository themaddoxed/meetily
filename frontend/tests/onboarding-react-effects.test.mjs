import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((complete, fail) => {
    resolve = complete;
    reject = fail;
  });
  return { promise, reject, resolve };
}

async function flush() {
  for (let index = 0; index < 10; index += 1) {
    await Promise.resolve();
  }
}

function createTimers() {
  const tasks = new Map();
  let nextId = 1;

  return {
    clearTimeout(id) {
      tasks.delete(id);
    },
    runAll() {
      const callbacks = [...tasks.values()];
      tasks.clear();
      callbacks.forEach((callback) => callback());
    },
    setTimeout(callback) {
      const id = nextId++;
      tasks.set(id, callback);
      return id;
    },
  };
}

function createHookRuntime() {
  const stateSlots = [];
  const setters = [];
  const refSlots = [];
  const callbackSlots = [];
  const effectSlots = [];
  let stateIndex = 0;
  let refIndex = 0;
  let callbackIndex = 0;
  let effectIndex = 0;
  let pendingEffects = [];
  let unmounted = false;
  let updatesAfterUnmount = 0;

  const dependenciesMatch = (left, right) =>
    left?.length === right?.length &&
    left.every((value, index) => Object.is(value, right[index]));

  const react = {
    createContext(defaultValue) {
      return { defaultValue, Provider: Symbol('Context.Provider') };
    },
    useCallback(callback, dependencies) {
      const slot = callbackIndex++;
      const previous = callbackSlots[slot];
      if (!previous || !dependenciesMatch(dependencies, previous.dependencies)) {
        callbackSlots[slot] = { callback, dependencies };
      }
      return callbackSlots[slot].callback;
    },
    useContext(context) {
      return context.defaultValue;
    },
    useEffect(effect, dependencies) {
      const slot = effectIndex++;
      const previous = effectSlots[slot];
      if (previous && dependenciesMatch(dependencies, previous.dependencies)) return;
      effectSlots[slot] = {
        cleanup: previous?.cleanup,
        dependencies,
        effect,
      };
      pendingEffects.push({ effect, previousCleanup: previous?.cleanup, slot });
    },
    useRef(initialValue) {
      const slot = refIndex++;
      refSlots[slot] ??= { current: initialValue };
      return refSlots[slot];
    },
    useState(initialValue) {
      const slot = stateIndex++;
      if (!(slot in stateSlots)) {
        stateSlots[slot] = typeof initialValue === 'function' ? initialValue() : initialValue;
      }
      setters[slot] ??= (nextValue) => {
        if (unmounted) updatesAfterUnmount += 1;
        stateSlots[slot] = typeof nextValue === 'function'
          ? nextValue(stateSlots[slot])
          : nextValue;
      };
      return [stateSlots[slot], setters[slot]];
    },
  };

  return {
    react,
    render(renderFunction) {
      unmounted = false;
      stateIndex = 0;
      refIndex = 0;
      callbackIndex = 0;
      effectIndex = 0;
      return renderFunction();
    },
    flushEffects() {
      const effects = pendingEffects;
      pendingEffects = [];
      effects.forEach(({ effect, previousCleanup, slot }) => {
        const current = effectSlots[slot];
        if (!current || current.effect !== effect) return;
        previousCleanup?.();
        current.cleanup = current.effect() ?? undefined;
      });
    },
    replayEffects() {
      effectSlots.forEach((effect) => effect?.cleanup?.());
      effectSlots.forEach((effect) => {
        if (effect) effect.cleanup = effect.effect() ?? undefined;
      });
    },
    stateUpdatesAfterUnmount() {
      return updatesAfterUnmount;
    },
    unmount() {
      unmounted = true;
      pendingEffects = [];
      effectSlots.forEach((effect) => effect?.cleanup?.());
    },
  };
}

function createListeners() {
  const handlers = new Map();

  return {
    emit(name, payload) {
      handlers.get(name)?.forEach((handler) => handler({ payload }));
    },
    listen(name, handler) {
      const eventHandlers = handlers.get(name) ?? new Set();
      eventHandlers.add(handler);
      handlers.set(name, eventHandlers);
      return Promise.resolve(() => eventHandlers.delete(handler));
    },
  };
}

const jsxRuntime = {
  jsx: (type, props) => ({ type, props }),
  jsxs: (type, props) => ({ type, props }),
};

const quietConsole = {
  error() {},
  log() {},
  warn() {},
};

function loadModule(relativePath, modules, globals = {}) {
  const source = fs.readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');
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
    console: quietConsole,
    exports: commonJsModule.exports,
    module: commonJsModule,
    require(specifier) {
      if (!(specifier in modules)) throw new Error(`Unexpected dependency: ${specifier}`);
      return modules[specifier];
    },
    ...globals,
  });

  return commonJsModule.exports;
}

function loadOnboardingProvider(runtime, { invoke, listeners, timers }) {
  return loadModule(
    'src/contexts/OnboardingContext.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      '@tauri-apps/api/core': { invoke },
      '@tauri-apps/api/event': { listen: listeners.listen },
      '@/lib/onboarding-summary-model': {
        resolveOnboardingSummaryModelStatus: ({ selectedModel, recommendedModel, selectedModelReady }) => ({
          selectedSummaryModel: selectedModel || recommendedModel,
          summaryModelDownloaded: selectedModelReady,
        }),
      },
    },
    {
      clearTimeout: timers.clearTimeout,
      navigator: { platform: 'Linux' },
      setTimeout: timers.setTimeout,
    },
  ).OnboardingProvider;
}

function loadDownloadProgressStep(runtime, { getContext, invoke, listeners }) {
  const symbols = new Proxy({}, { get: (_, name) => Symbol(String(name)) });
  return loadModule(
    'src/components/onboarding/steps/DownloadProgressStep.tsx',
    {
      react: runtime.react,
      'react/jsx-runtime': jsxRuntime,
      '@tauri-apps/api/core': { invoke },
      '@tauri-apps/api/event': { listen: listeners.listen },
      '@tauri-apps/plugin-os': { platform: () => 'linux' },
      'lucide-react': symbols,
      '@/components/ui/button': { Button: Symbol('Button') },
      '../OnboardingContainer': { OnboardingContainer: Symbol('OnboardingContainer') },
      '@/contexts/OnboardingContext': { useOnboarding: () => getContext() },
      sonner: { toast: new Proxy({}, { get: () => () => undefined }) },
      'framer-motion': {
        AnimatePresence: Symbol('AnimatePresence'),
        motion: new Proxy({}, { get: (_, name) => Symbol(`motion.${String(name)}`) }),
      },
      '@/lib/onboarding-summary-model': {
        getSummaryModelSizeLabel: () => '1 GB',
        getSummaryModelSizeMb: () => 1024,
      },
    },
    {
      navigator: { userAgent: 'Linux' },
      setTimeout: () => 1,
      window: { location: { reload() {} } },
    },
  ).DownloadProgressStep;
}

function onboardingContext(overrides = {}) {
  return {
    completeOnboarding: async () => undefined,
    goNext() {},
    parakeetDownloaded: false,
    recommendedSummaryModel: 'summary-a',
    selectedSummaryModel: 'summary-a',
    setParakeetDownloaded() {},
    setSummaryModelDownloaded() {},
    startBackgroundDownloads: async () => undefined,
    summaryModelDownloaded: false,
    ...overrides,
  };
}

test('StrictMode replay starts onboarding initialization once and late completion does not commit after unmount', async () => {
  const runtime = createHookRuntime();
  const listeners = createListeners();
  const timers = createTimers();
  const status = deferred();
  const firstLaunch = deferred();
  const calls = [];
  const Provider = loadOnboardingProvider(runtime, {
    listeners,
    timers,
    invoke(command) {
      calls.push(command);
      if (command === 'get_onboarding_status') return status.promise;
      if (command === 'check_first_launch') return firstLaunch.promise;
      return Promise.resolve(null);
    },
  });

  runtime.render(() => Provider({ children: null }));
  runtime.flushEffects();
  runtime.replayEffects();

  assert.equal(calls.filter((command) => command === 'get_onboarding_status').length, 1);
  assert.equal(calls.filter((command) => command === 'check_first_launch').length, 2);

  runtime.unmount();
  status.resolve({
    version: '1.0',
    completed: true,
    current_step: 4,
    model_status: {
      parakeet: 'downloaded',
      summary: 'downloaded',
      selected_summary_model: 'summary-a',
    },
    last_updated: '2026-07-15T00:00:00.000Z',
  });
  firstLaunch.resolve(false);
  await flush();

  assert.equal(runtime.stateUpdatesAfterUnmount(), 0);
});

test('Parakeet and summary downloads start once across callback and progress rerenders', async () => {
  const runtime = createHookRuntime();
  const listeners = createListeners();
  const calls = [];
  let context = onboardingContext({
    startBackgroundDownloads: async (options) => { calls.push({ owner: 'initial', ...options }); },
  });
  const Step = loadDownloadProgressStep(runtime, {
    getContext: () => context,
    invoke: async () => undefined,
    listeners,
  });

  runtime.render(() => Step());
  runtime.flushEffects();
  await flush();
  context = onboardingContext({
    parakeetDownloaded: true,
    summaryModelDownloaded: true,
    startBackgroundDownloads: async (options) => { calls.push({ owner: 'latest', ...options }); },
  });
  runtime.render(() => Step());
  runtime.flushEffects();
  listeners.emit('parakeet-model-download-progress', {
    modelName: 'parakeet-tdt-0.6b-v3-int8',
    progress: 100,
    status: 'completed',
  });
  listeners.emit('builtin-ai-download-progress', {
    model: 'summary-a',
    progress: 100,
    status: 'completed',
  });
  runtime.render(() => Step());
  runtime.flushEffects();

  assert.deepEqual(
    calls.map((call) => JSON.parse(JSON.stringify(call))),
    [
      { owner: 'initial', includeParakeet: true, includeSummary: false },
      { owner: 'initial', includeParakeet: false, includeSummary: true, summaryModel: 'summary-a' },
    ],
  );
  runtime.unmount();
});

test('download completion events call the latest context setters', async () => {
  const runtime = createHookRuntime();
  const listeners = createListeners();
  const setterCalls = { initial: 0, latest: 0 };
  let context = onboardingContext({
    setParakeetDownloaded: () => { setterCalls.initial += 1; },
    setSummaryModelDownloaded: () => { setterCalls.initial += 1; },
  });
  const Step = loadDownloadProgressStep(runtime, {
    getContext: () => context,
    invoke: async () => undefined,
    listeners,
  });

  runtime.render(() => Step());
  runtime.flushEffects();
  await flush();
  context = onboardingContext({
    setParakeetDownloaded: () => { setterCalls.latest += 1; },
    setSummaryModelDownloaded: () => { setterCalls.latest += 1; },
  });
  runtime.render(() => Step());
  runtime.flushEffects();
  listeners.emit('parakeet-model-download-complete', {
    modelName: 'parakeet-tdt-0.6b-v3-int8',
  });
  listeners.emit('builtin-ai-download-progress', {
    model: 'summary-a',
    progress: 100,
    status: 'completed',
  });

  assert.deepEqual(setterCalls, { initial: 0, latest: 2 });
  runtime.unmount();
});

test('debounced onboarding persistence waits for hydration then saves the latest status once', async () => {
  const runtime = createHookRuntime();
  const listeners = createListeners();
  const timers = createTimers();
  const status = deferred();
  const saves = [];
  const Provider = loadOnboardingProvider(runtime, {
    listeners,
    timers,
    invoke(command, payload) {
      if (command === 'get_onboarding_status') return status.promise;
      if (command === 'check_first_launch') return Promise.resolve(false);
      if (command === 'save_onboarding_status_cmd') saves.push(payload.status);
      return Promise.resolve(null);
    },
  });
  const render = () => {
    const tree = runtime.render(() => Provider({ children: null }));
    runtime.flushEffects();
    return tree;
  };

  let tree = render();
  tree.props.value.goNext();
  tree.props.value.setParakeetDownloaded(true);
  tree.props.value.setSelectedSummaryModel('summary-latest');
  tree = render();
  timers.runAll();
  await flush();

  assert.equal(saves.length, 0);

  status.reject(new Error('status unavailable'));
  await flush();
  render();
  timers.runAll();
  await flush();
  render();
  timers.runAll();
  await flush();

  assert.equal(saves.length, 1);
  assert.equal(saves[0].current_step, 2);
  assert.equal(saves[0].model_status.parakeet, 'downloaded');
  assert.equal(saves[0].model_status.selected_summary_model, 'summary-latest');
  runtime.unmount();
});

test('late onboarding listener callbacks do not commit and delayed registrations unlisten after unmount', async () => {
  const runtime = createHookRuntime();
  const timers = createTimers();
  const registrations = [];
  const listeners = {
    listen(name, handler) {
      const ready = deferred();
      const registration = { handler, name, ready, unlistenCalls: 0 };
      registrations.push(registration);
      return ready.promise;
    },
  };
  const Provider = loadOnboardingProvider(runtime, {
    listeners,
    timers,
    invoke(command) {
      if (command === 'get_onboarding_status' || command === 'check_first_launch') {
        return new Promise(() => undefined);
      }
      return Promise.resolve(null);
    },
  });

  let tree = runtime.render(() => Provider({ children: null }));
  runtime.flushEffects();
  tree.props.value.setSelectedSummaryModel('summary-a');
  tree = runtime.render(() => Provider({ children: null }));
  runtime.flushEffects();
  assert.equal(registrations.length, 5);
  runtime.unmount();

  registrations
    .find(({ name }) => name === 'parakeet-model-download-progress')
    .handler({
      payload: {
        modelName: 'parakeet-tdt-0.6b-v3-int8',
        progress: 100,
        status: 'completed',
      },
    });
  registrations
    .find(({ name }) => name === 'parakeet-model-download-complete')
    .handler({ payload: { modelName: 'parakeet-tdt-0.6b-v3-int8' } });
  [...registrations]
    .reverse()
    .find(({ name }) => name === 'builtin-ai-download-progress')
    .handler({
      payload: {
        model: 'summary-a',
        progress: 100,
        status: 'completed',
      },
    });

  assert.equal(runtime.stateUpdatesAfterUnmount(), 0);

  registrations.forEach((registration) => {
    registration.ready.resolve(() => {
      registration.unlistenCalls += 1;
    });
  });
  await flush();
  assert.ok(registrations.every(({ unlistenCalls }) => unlistenCalls === 1));
});

test('rejected listener registrations are handled after onboarding components unmount', async () => {
  const providerRuntime = createHookRuntime();
  const providerTimers = createTimers();
  const providerRegistration = deferred();
  let providerListenCalls = 0;
  const Provider = loadOnboardingProvider(providerRuntime, {
    listeners: {
      listen() {
        providerListenCalls += 1;
        return providerListenCalls === 1
          ? providerRegistration.promise
          : Promise.resolve(() => undefined);
      },
    },
    timers: providerTimers,
    invoke(command) {
      if (command === 'get_onboarding_status' || command === 'check_first_launch') {
        return new Promise(() => undefined);
      }
      return Promise.resolve(null);
    },
  });
  providerRuntime.render(() => Provider({ children: null }));
  providerRuntime.flushEffects();
  providerRuntime.unmount();

  const stepRuntime = createHookRuntime();
  const stepRegistration = deferred();
  let stepListenCalls = 0;
  const Step = loadDownloadProgressStep(stepRuntime, {
    getContext: () => onboardingContext(),
    invoke: async () => undefined,
    listeners: {
      listen() {
        stepListenCalls += 1;
        return stepListenCalls === 1
          ? stepRegistration.promise
          : Promise.resolve(() => undefined);
      },
    },
  });
  stepRuntime.render(() => Step());
  stepRuntime.flushEffects();
  stepRuntime.unmount();

  providerRegistration.reject(new Error('provider listen failed'));
  stepRegistration.reject(new Error('step listen failed'));
  await flush();
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(providerRuntime.stateUpdatesAfterUnmount(), 0);
  assert.equal(stepRuntime.stateUpdatesAfterUnmount(), 0);
});
