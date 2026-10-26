import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { fileURLToPath } from 'node:url';

const frontendDir = fileURLToPath(new URL('..', import.meta.url));
const eslintCli = fileURLToPath(new URL('../node_modules/eslint/bin/eslint.js', import.meta.url));
const targets = [
  'src/components/BuiltInModelManager.tsx',
  'src/components/ModelSettingsModal.tsx',
  'src/contexts/ConfigContext.tsx',
];

function deferred() {
  let resolve;
  const promise = new Promise((complete) => {
    resolve = complete;
  });
  return { promise, resolve };
}

async function flush() {
  for (let index = 0; index < 10; index += 1) {
    await Promise.resolve();
  }
}

function createLocalStorage(initial = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem(key) {
      return values.has(key) ? values.get(key) : null;
    },
    setItem(key, value) {
      values.set(key, String(value));
    },
  };
}

function createHookRuntime({ passiveEffects = false } = {}) {
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
    useContext(context) {
      return context.defaultValue;
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
      if (passiveEffects) {
        effectSlots[slot] = {
          cleanup: previous?.cleanup,
          dependencies,
          effect,
        };
        pendingEffects.push({ previousCleanup: previous?.cleanup, slot, effect });
        return;
      }
      previous?.cleanup?.();
      effectSlots[slot] = { effect, dependencies, cleanup: effect() ?? undefined };
    },
  };

  return {
    react,
    render(renderFunction) {
      unmounted = false;
      stateIndex = 0;
      refIndex = 0;
      callbackIndex = 0;
      memoIndex = 0;
      effectIndex = 0;
      return renderFunction();
    },
    flushEffects() {
      const effects = pendingEffects;
      pendingEffects = [];
      effects.forEach(({ previousCleanup, slot, effect }) => {
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

const jsxRuntime = {
  Fragment: Symbol('Fragment'),
  jsx: (type, props) => ({ type, props }),
  jsxs: (type, props) => ({ type, props }),
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
    console: { error() {}, log() {}, warn() {} },
    exports: commonJsModule.exports,
    module: commonJsModule,
    require(specifier) {
      if (!(specifier in modules)) throw new Error(`Unexpected dependency: ${specifier}`);
      return modules[specifier];
    },
    URL,
    clearTimeout() {},
    setTimeout() { return 1; },
    ...globals,
  });

  return commonJsModule.exports;
}

const uiModule = (name) => ({ [name]: Symbol(name) });
const icons = new Proxy({}, { get: (_, name) => Symbol(String(name)) });

function loadModelSettings(runtime, { getContext, invoke, localStorage, downloadingModels = new Set() }) {
  const modules = {
    react: runtime.react,
    'react/jsx-runtime': jsxRuntime,
    '@tauri-apps/api/core': { invoke },
    '@/components/ui/button': uiModule('Button'),
    '@/contexts/OllamaDownloadContext': {
      useOllamaDownload: () => ({
        downloadingModels,
        getProgress: () => 0,
        isDownloading: () => false,
      }),
    },
    '@/components/BuiltInModelManager': uiModule('BuiltInModelManager'),
    '@/components/ui/input': uiModule('Input'),
    '@/components/ui/label': uiModule('Label'),
    '@/contexts/ConfigContext': { useConfig: () => getContext() },
    '@/contexts/I18nContext': {
      useI18n: () => ({
        t: (key) => ({
          'model.customEndpoint': 'Custom Endpoint (optional)',
          'model.using': 'Using:',
          'common.loadingModels': 'Loading models...',
          'transcript.enterApiKey': 'Enter your API key',
        }[key] || key),
      }),
    },
    '@/components/ui/select': {
      Select: Symbol('Select'),
      SelectContent: Symbol('SelectContent'),
      SelectItem: Symbol('SelectItem'),
      SelectTrigger: Symbol('SelectTrigger'),
      SelectValue: Symbol('SelectValue'),
    },
    '@/components/ui/alert': {
      Alert: Symbol('Alert'),
      AlertDescription: Symbol('AlertDescription'),
    },
    '@/components/ui/scroll-area': uiModule('ScrollArea'),
    'lucide-react': icons,
    '@/components/ui/popover': {
      Popover: Symbol('Popover'),
      PopoverContent: Symbol('PopoverContent'),
      PopoverTrigger: Symbol('PopoverTrigger'),
    },
    '@/components/ui/command': {
      Command: Symbol('Command'),
      CommandEmpty: Symbol('CommandEmpty'),
      CommandGroup: Symbol('CommandGroup'),
      CommandInput: Symbol('CommandInput'),
      CommandItem: Symbol('CommandItem'),
      CommandList: Symbol('CommandList'),
    },
    '@/lib/utils': { cn: (...values) => values.filter(Boolean).join(' '), isOllamaNotInstalledError: () => false },
    sonner: { toast: new Proxy({}, { get: () => () => undefined }) },
    '@/lib/builtin-ai-models': {
      getFirstSelectableBuiltInModelName: (models) => models[0]?.name ?? null,
    },
  };

  return loadModule('src/components/ModelSettingsModal.tsx', modules, { localStorage })
    .ModelSettingsModal;
}

function loadConfigProvider(
  runtime,
  { configService, invoke, listen = async () => () => undefined, localStorage },
) {
  const window = {
    dispatchEvent() {},
    localStorage,
    matchMedia: () => ({
      addEventListener() {},
      matches: false,
      removeEventListener() {},
    }),
  };
  const modules = {
    react: runtime.react,
    'react/jsx-runtime': jsxRuntime,
    '@/services/configService': { configService },
    '@tauri-apps/api/core': { invoke },
    '@tauri-apps/api/event': { listen },
    '@/types/betaFeatures': {
      loadBetaFeatures: () => ({}),
      saveBetaFeatures: () => undefined,
    },
    '@/lib/theme': {
      THEME_STORAGE_KEY: 'theme',
      applyEffectiveTheme: () => undefined,
      createNativeThemeSyncController: () => ({ dispose() {}, sync: async () => undefined }),
      readStoredThemeMode: () => 'system',
      resolveEffectiveTheme: () => 'light',
    },
  };

  return loadModule('src/contexts/ConfigContext.tsx', modules, {
    CustomEvent: class CustomEvent {},
    document: { documentElement: { classList: { contains: () => false } } },
    localStorage,
    window,
  }).ConfigProvider;
}

function loadBuiltInModelManager(runtime, { invoke, listen }) {
  const modules = {
    react: runtime.react,
    'react/jsx-runtime': jsxRuntime,
    '@/contexts/I18nContext': { useI18n: () => ({ t: (key) => key }) },
    '@tauri-apps/api/core': { invoke },
    '@tauri-apps/api/event': { listen },
    '@/components/ui/button': uiModule('Button'),
    '@/components/ui/alert': {
      Alert: Symbol('Alert'),
      AlertDescription: Symbol('AlertDescription'),
    },
    '@/lib/utils': { cn: (...values) => values.filter(Boolean).join(' ') },
    'lucide-react': icons,
    sonner: { toast: new Proxy({}, { get: () => () => undefined }) },
    '@/lib/onboarding-summary-model': {
      formatSummaryModelSizeLabelFromMb: (size) => String(size),
    },
    '@/lib/builtin-ai-models': {
      getFirstSelectableBuiltInModelName: (models) => models[0]?.name ?? null,
      isSelectableBuiltInModelStatus: () => true,
    },
  };

  return loadModule('src/components/BuiltInModelManager.tsx', modules)
    .BuiltInModelManager;
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

function textContent(node) {
  if (Array.isArray(node)) return node.map(textContent).join(' ');
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (!node || typeof node !== 'object') return '';
  return textContent(node.props?.children);
}

function clickEnabled(node) {
  assert.ok(node);
  assert.equal(node.props.disabled, false);
  node.props.onClick();
}

function baseConfig(overrides = {}) {
  return {
    provider: 'ollama',
    model: '',
    whisperModel: 'large-v3',
    ollamaEndpoint: null,
    ...overrides,
  };
}

test('assigned model/config files pass exhaustive dependency checks', () => {
  const result = spawnSync(process.execPath, [eslintCli, ...targets, '--max-warnings=0'], {
    cwd: frontendDir,
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, `${result.stdout}${result.stderr}`);
});

test('model settings initial work is idempotent and resolves through the latest setter', async () => {
  const runtime = createHookRuntime();
  const response = deferred();
  const calls = [];
  const setterCalls = [0, 0];
  let setterIndex = 0;
  const context = baseConfig({ provider: 'claude', model: 'claude-model' });
  const Modal = loadModelSettings(runtime, {
    getContext: () => ({
      modelConfig: context,
      providerApiKeys: null,
      setModelConfig: () => { setterCalls[setterIndex] += 1; },
    }),
    invoke: (command) => {
      calls.push(command);
      if (command === 'api_get_model_config') return response.promise;
      return Promise.resolve([]);
    },
    localStorage: createLocalStorage(),
  });
  const render = () => runtime.render(() => Modal({
    modelConfig: context,
    setModelConfig() {},
    onSave() {},
  }));

  render();
  setterIndex = 1;
  render();
  response.resolve(baseConfig({ provider: 'claude', model: 'saved' }));
  await flush();

  assert.equal(calls.filter((command) => command === 'api_get_model_config').length, 1);
  assert.deepEqual(setterCalls, [0, 1]);

  const strictRuntime = createHookRuntime();
  let ollamaFetches = 0;
  const ollamaContext = baseConfig();
  const OllamaModal = loadModelSettings(strictRuntime, {
    getContext: () => ({ modelConfig: ollamaContext, setModelConfig() {} }),
    invoke: (command) => {
      if (command === 'get_ollama_models') ollamaFetches += 1;
      return Promise.resolve([]);
    },
    localStorage: createLocalStorage(),
  });
  strictRuntime.render(() => OllamaModal({
    modelConfig: ollamaContext,
    setModelConfig() {},
    onSave() {},
    skipInitialFetch: true,
  }));
  strictRuntime.replayEffects();
  assert.equal(ollamaFetches, 1);
});

test('StrictMode initial config replay shares one data load and only the live subscription commits', async () => {
  const runtime = createHookRuntime({ passiveEffects: true });
  const configResponse = deferred();
  const apiKeyResponse = deferred();
  const customConfigResponse = deferred();
  const calls = [];
  let configCommits = 0;
  const context = baseConfig({ provider: 'custom-openai', model: '' });
  const Modal = loadModelSettings(runtime, {
    getContext: () => ({
      modelConfig: context,
      providerApiKeys: null,
      setModelConfig() { configCommits += 1; },
    }),
    invoke: (command) => {
      calls.push(command);
      if (command === 'api_get_model_config') return configResponse.promise;
      if (command === 'api_get_api_key') return apiKeyResponse.promise;
      if (command === 'api_get_custom_openai_config') return customConfigResponse.promise;
      return Promise.resolve([]);
    },
    localStorage: createLocalStorage(),
  });

  runtime.render(() => Modal({
    modelConfig: context,
    setModelConfig() {},
    onSave() {},
  }));
  runtime.flushEffects();
  runtime.replayEffects();
  assert.equal(calls.filter((command) => command === 'api_get_model_config').length, 1);

  configResponse.resolve(baseConfig({ provider: 'custom-openai', model: '' }));
  await flush();
  assert.equal(calls.filter((command) => command === 'api_get_api_key').length, 1);
  apiKeyResponse.resolve('custom-key');
  await flush();
  assert.equal(calls.filter((command) => command === 'api_get_custom_openai_config').length, 1);
  customConfigResponse.resolve({
    apiKey: 'custom-key',
    endpoint: 'http://custom',
    model: 'custom-model',
  });
  await flush();
  assert.equal(configCommits, 1);

  const unmountedRuntime = createHookRuntime({ passiveEffects: true });
  const unmountedResponse = deferred();
  let unmountedCommits = 0;
  const UnmountedModal = loadModelSettings(unmountedRuntime, {
    getContext: () => ({
      modelConfig: context,
      providerApiKeys: null,
      setModelConfig() { unmountedCommits += 1; },
    }),
    invoke: (command) => command === 'api_get_model_config'
      ? unmountedResponse.promise
      : Promise.resolve([]),
    localStorage: createLocalStorage(),
  });
  unmountedRuntime.render(() => UnmountedModal({
    modelConfig: context,
    setModelConfig() {},
    onSave() {},
  }));
  unmountedRuntime.flushEffects();
  unmountedRuntime.unmount();
  unmountedResponse.resolve(baseConfig({ provider: 'ollama', model: 'saved' }));
  await flush();

  assert.equal(unmountedCommits, 0);
  assert.equal(unmountedRuntime.stateUpdatesAfterUnmount(), 0);
});

test('changing an Ollama endpoint invalidates the request and re-enables the rendered fetch button', async () => {
  const runtime = createHookRuntime({ passiveEffects: true });
  const requests = new Map();
  const config = baseConfig();
  const Modal = loadModelSettings(runtime, {
    getContext: () => ({ modelConfig: config, setModelConfig() {} }),
    invoke: (command, payload) => {
      if (command !== 'get_ollama_models') return Promise.resolve([]);
      if (payload.endpoint === null) return Promise.resolve([]);
      const request = deferred();
      requests.set(payload.endpoint, request);
      return request.promise;
    },
    localStorage: createLocalStorage(),
  });
  const render = () => runtime.render(() => Modal({
    modelConfig: config,
    setModelConfig() {},
    onSave() {},
    skipInitialFetch: true,
  }));

  let tree = render();
  runtime.flushEffects();
  await flush();
  tree = render();
  runtime.flushEffects();
  const endpointToggle = findJsxNode(
    tree,
    (node) => typeof node.props?.onClick === 'function' && textContent(node).includes('Custom Endpoint (optional)'),
  );
  assert.ok(endpointToggle);
  endpointToggle.props.onClick();
  tree = render();
  runtime.flushEffects();
  const endpointInput = () => findJsxNode(tree, (node) => node.props?.placeholder === 'http://localhost:11434');
  const fetchButton = () => findJsxNode(
    tree,
    (node) => typeof node.props?.onClick === 'function' && node.props?.className === 'whitespace-nowrap',
  );

  endpointInput().props.onChange({ target: { value: 'http://first:11434' } });
  tree = render();
  runtime.flushEffects();
  tree = render();
  assert.equal(fetchButton().props.disabled, false);
  clickEnabled(fetchButton());
  tree = render();
  assert.equal(fetchButton().props.disabled, true);

  endpointInput().props.onChange({ target: { value: 'http://second:11434' } });
  tree = render();
  runtime.flushEffects();
  tree = render();
  assert.equal(fetchButton().props.disabled, false);
  clickEnabled(fetchButton());

  requests.get('http://second:11434').resolve([{ name: 'second-model', id: '2', size: '', modified: '' }]);
  await flush();
  requests.get('http://first:11434').resolve([{ name: 'first-model', id: '1', size: '', modified: '' }]);
  await flush();
  tree = render();
  runtime.flushEffects();

  assert.match(textContent(tree), /second-model/);
  assert.doesNotMatch(textContent(tree), /first-model/);
});

test('reverting to a cached Ollama endpoint invalidates the pending request and restores its models', async () => {
  const runtime = createHookRuntime({ passiveEffects: true });
  const requests = new Map();
  const config = baseConfig();
  const Modal = loadModelSettings(runtime, {
    getContext: () => ({ modelConfig: config, setModelConfig() {} }),
    invoke: (command, payload) => {
      if (command !== 'get_ollama_models') return Promise.resolve([]);
      if (payload.endpoint === null) return Promise.resolve([]);
      const request = deferred();
      requests.set(payload.endpoint, request);
      return request.promise;
    },
    localStorage: createLocalStorage(),
  });
  const render = () => runtime.render(() => Modal({
    modelConfig: config,
    setModelConfig() {},
    onSave() {},
    skipInitialFetch: true,
  }));

  let tree = render();
  runtime.flushEffects();
  await flush();
  tree = render();
  runtime.flushEffects();
  const endpointToggle = findJsxNode(
    tree,
    (node) => typeof node.props?.onClick === 'function' && textContent(node).includes('Custom Endpoint (optional)'),
  );
  assert.ok(endpointToggle);
  endpointToggle.props.onClick();
  tree = render();
  runtime.flushEffects();
  const endpointInput = () => findJsxNode(tree, (node) => node.props?.placeholder === 'http://localhost:11434');
  const fetchButton = () => findJsxNode(
    tree,
    (node) => typeof node.props?.onClick === 'function' && node.props?.className === 'whitespace-nowrap',
  );

  endpointInput().props.onChange({ target: { value: 'http://first:11434' } });
  tree = render();
  runtime.flushEffects();
  tree = render();
  clickEnabled(fetchButton());
  requests.get('http://first:11434').resolve([{ name: 'first-model', id: '1', size: '', modified: '' }]);
  await flush();
  tree = render();
  runtime.flushEffects();
  assert.match(textContent(tree), /first-model/);
  assert.match(textContent(tree), /Using:\s*http:\/\/first:11434/);

  endpointInput().props.onChange({ target: { value: 'http://second:11434' } });
  tree = render();
  runtime.flushEffects();
  tree = render();
  clickEnabled(fetchButton());
  tree = render();
  assert.equal(fetchButton().props.disabled, true);

  endpointInput().props.onChange({ target: { value: 'http://first:11434' } });
  tree = render();
  runtime.flushEffects();
  tree = render();
  assert.equal(fetchButton().props.disabled, false);
  assert.match(textContent(tree), /first-model/);
  assert.match(textContent(tree), /Using:\s*http:\/\/first:11434/);

  requests.get('http://second:11434').resolve([{ name: 'second-model', id: '2', size: '', modified: '' }]);
  await flush();
  tree = render();
  runtime.flushEffects();

  assert.equal(fetchButton().props.disabled, false);
  assert.match(textContent(tree), /first-model/);
  assert.doesNotMatch(textContent(tree), /second-model/);
  assert.match(textContent(tree), /Using:\s*http:\/\/first:11434/);
});

test('provider keys synchronize without stale fetches and cached fallback preserves a user choice', async () => {
  const runtime = createHookRuntime();
  const modelCalls = [];
  const queuedSetters = [];
  let config = baseConfig({ provider: 'openai', model: '' });
  const providerApiKeys = {
    claude: 'claude-key',
    groq: null,
    openai: 'openai-key',
    openrouter: null,
  };
  const Modal = loadModelSettings(runtime, {
    getContext: () => ({
      modelConfig: config,
      providerApiKeys,
      setModelConfig: (next) => queuedSetters.push(next),
    }),
    invoke: (command, payload) => {
      if (command === 'get_openai_models' || command === 'get_anthropic_models') {
        modelCalls.push([command, payload.apiKey]);
      }
      return Promise.resolve([]);
    },
    localStorage: createLocalStorage({ providerModelMap: JSON.stringify({ openai: 'gpt-4o' }) }),
  });
  const render = () => runtime.render(() => Modal({
    modelConfig: config,
    setModelConfig() {},
    onSave() {},
    skipInitialFetch: true,
  }));

  render();
  render();
  config = baseConfig({ provider: 'claude', model: 'user-choice' });
  render();
  render();
  await flush();

  assert.equal(modelCalls.some(([command, key]) => command === 'get_anthropic_models' && key === 'openai-key'), false);
  const fallback = queuedSetters.find((value) => typeof value === 'function');
  assert.ok(fallback);
  const selected = fallback(config);
  assert.equal(selected.model, 'user-choice');
});

test('same-provider key changes keep only the latest OpenAI, Claude, and Groq response', async () => {
  const providers = [
    ['openai', 'get_openai_models'],
    ['claude', 'get_anthropic_models'],
    ['groq', 'get_groq_models'],
  ];

  for (const [provider, command] of providers) {
    const runtime = createHookRuntime({ passiveEffects: true });
    const requests = new Map();
    const config = baseConfig({ provider, model: '', apiKey: null });
    const Modal = loadModelSettings(runtime, {
      getContext: () => ({
        modelConfig: config,
        providerApiKeys: null,
        setModelConfig() {},
      }),
      invoke: (invokedCommand, payload) => {
        if (invokedCommand !== command) return Promise.resolve([]);
        const request = deferred();
        requests.set(payload.apiKey, request);
        return request.promise;
      },
      localStorage: createLocalStorage(),
    });
    const render = () => runtime.render(() => Modal({
      modelConfig: config,
      setModelConfig() {},
      onSave() {},
      skipInitialFetch: true,
    }));

    let tree = render();
    runtime.flushEffects();
    tree = render();
    runtime.flushEffects();
    const apiKeyInput = () => findJsxNode(
      tree,
      (node) => node.props?.placeholder === 'Enter your API key',
    );

    apiKeyInput().props.onChange({ target: { value: `${provider}-A` } });
    tree = render();
    runtime.flushEffects();
    apiKeyInput().props.onChange({ target: { value: `${provider}-B` } });
    tree = render();
    runtime.flushEffects();

    requests.get(`${provider}-B`).resolve([{ id: `${provider}-model-B` }]);
    await flush();
    tree = render();
    runtime.flushEffects();
    assert.match(textContent(tree), new RegExp(`${provider}-model-B`));
    assert.doesNotMatch(textContent(tree), /Loading models/);

    requests.get(`${provider}-A`).resolve([{ id: `${provider}-model-A` }]);
    await flush();
    tree = render();
    runtime.flushEffects();
    assert.match(textContent(tree), new RegExp(`${provider}-model-B`));
    assert.doesNotMatch(textContent(tree), new RegExp(`${provider}-model-A`));
  }
});

test('ConfigContext keeps model options stable and only applies the latest endpoint response', async () => {
  const runtime = createHookRuntime();
  const requests = new Map();
  const Provider = loadConfigProvider(runtime, {
    configService: {
      getCustomOpenAIConfig: async () => null,
      getModelConfig: async () => null,
      getRecordingPreferences: async () => null,
      getTranscriptConfig: async () => null,
    },
    invoke: (command, payload) => {
      if (command === 'get_ollama_models') {
        const key = payload.endpoint ?? 'default';
        if (key === 'default') return Promise.resolve([]);
        const request = deferred();
        requests.set(key, request);
        return request.promise;
      }
      if (command === 'api_get_api_key') return Promise.resolve(null);
      return Promise.resolve(null);
    },
    localStorage: createLocalStorage(),
  });
  const render = () => runtime.render(() => Provider({ children: null }));

  let tree = render();
  await flush();
  tree = render();
  const firstOptions = tree.props.value.modelOptions;
  tree = render();
  assert.equal(tree.props.value.modelOptions, firstOptions);

  tree.props.value.setModelConfig((previous) => ({ ...previous, ollamaEndpoint: 'http://first' }));
  tree = render();
  tree.props.value.setModelConfig((previous) => ({ ...previous, ollamaEndpoint: 'http://second' }));
  tree = render();

  requests.get('http://second').resolve([{ name: 'second', id: '2', size: '', modified: '' }]);
  await flush();
  requests.get('http://first').resolve([{ name: 'first', id: '1', size: '', modified: '' }]);
  await flush();
  tree = render();

  assert.equal(tree.props.value.models[0]?.name, 'second');
});

test('BuiltInModelManager removes a late listener and ignores events after unmount', async () => {
  const runtime = createHookRuntime({ passiveEffects: true });
  const listener = deferred();
  let eventCallback;
  let listCalls = 0;
  let listenCalls = 0;
  let unlistenCalls = 0;
  const Manager = loadBuiltInModelManager(runtime, {
    invoke: (command) => {
      if (command === 'builtin_ai_list_models') listCalls += 1;
      return Promise.resolve([]);
    },
    listen: (_eventName, callback) => {
      listenCalls += 1;
      eventCallback = callback;
      return listener.promise;
    },
  });

  runtime.render(() => Manager({ selectedModel: '', onModelSelect() {} }));
  runtime.flushEffects();
  runtime.unmount();
  eventCallback({
    payload: { model: 'model', progress: 100, status: 'completed' },
  });
  listener.resolve(() => { unlistenCalls += 1; });
  await flush();

  assert.equal(listCalls, 1);
  assert.equal(listenCalls, 1);
  assert.equal(unlistenCalls, 1);
  assert.equal(runtime.stateUpdatesAfterUnmount(), 0);
});

test('ConfigContext removes a late listener and ignores config events after unmount', async () => {
  const runtime = createHookRuntime({ passiveEffects: true });
  const listener = deferred();
  let eventCallback;
  let listenCalls = 0;
  let unlistenCalls = 0;
  const Provider = loadConfigProvider(runtime, {
    configService: {
      getCustomOpenAIConfig: async () => null,
      getModelConfig: async () => null,
      getRecordingPreferences: async () => null,
      getTranscriptConfig: async () => null,
    },
    invoke: (command) => command === 'get_ollama_models'
      ? Promise.resolve([])
      : Promise.resolve(null),
    listen: (_eventName, callback) => {
      listenCalls += 1;
      eventCallback = callback;
      return listener.promise;
    },
    localStorage: createLocalStorage(),
  });

  runtime.render(() => Provider({ children: null }));
  runtime.flushEffects();
  await flush();
  assert.equal(listenCalls, 1);
  runtime.unmount();
  eventCallback({
    payload: baseConfig({ provider: 'openai', model: 'new-model', apiKey: 'new-key' }),
  });
  listener.resolve(() => { unlistenCalls += 1; });
  await flush();

  assert.equal(listenCalls, 1);
  assert.equal(unlistenCalls, 1);
  assert.equal(runtime.stateUpdatesAfterUnmount(), 0);
});
