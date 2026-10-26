import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import type { TranscriptUpdate } from '@/types';
import {
  indexedDBService,
  type MeetingMetadata,
  type StoredTranscript,
} from '@/services/indexedDBService';

const meeting = {
  id: 'theme-test-meeting',
  title: 'Theme Test Meeting',
  status: 'ready',
  duration_seconds: 1800,
  started_at_ms: Date.parse('2026-06-20T09:00:00Z'),
  created_at: '2026-06-20T09:00:00Z',
  updated_at: '2026-06-20T09:30:00Z',
  folder_path: '/tmp/theme-test-meeting',
  transcripts: [],
};

const completedOnboardingStatus = {
  version: '1.0',
  completed: true,
  current_step: 3,
  model_status: {
    parakeet: 'downloaded',
    summary: 'downloaded',
    selected_summary_model: 'qwen3.5:2b',
  },
  last_updated: '2026-06-20T09:30:00Z',
};

const newOnboardingStatus = {
  ...completedOnboardingStatus,
  completed: false,
  current_step: 1,
};

const browserStore = new Map<string, unknown>();

type TauriCommandJournalEntry = {
  command: string;
  payload?: unknown;
};

type BrowserAudioPermissionState = {
  microphone: 'not_determined' | 'authorized' | 'denied' | 'restricted' | 'unavailable';
  systemAudio: 'unknown' | 'available' | 'denied' | 'unavailable';
};

type BrowserRecordingSnapshot = {
  sessionId: string;
  meetingId: string;
  state: 'recording' | 'paused' | 'completed';
  startedAtMs: number;
  recordingDuration: number;
  activeDuration: number;
  pauseDuration: number;
  microphoneDevice: string | null;
  systemDevice: string | null;
  expectedTracks: Array<'microphone' | 'system'>;
  capturedTracks?: unknown[];
  meetingName: string;
  savePath: string;
  autoSave: boolean;
};

const audioEventSavedMicrophone = 'Studio Microphone';

type BrowserAudioEventState = {
  savedMicrophonePreference: string;
};

function getBrowserAudioEventState() {
  const state = window as typeof window & {
    __audioEventState?: BrowserAudioEventState;
  };
  state.__audioEventState ??= {
    savedMicrophonePreference: audioEventSavedMicrophone,
  };
  return state.__audioEventState;
}

type RecordingFolderPreferences = {
  save_folder: string;
  auto_save: boolean;
  file_format: string;
  preferred_mic_device: string | null;
  preferred_system_device: string | null;
};

type RecordingFolderState = {
  backendMutations: string[];
  openedFolders: string[];
  preferences: RecordingFolderPreferences;
  saveAttempts: RecordingFolderPreferences[];
  saves: RecordingFolderPreferences[];
  releaseSave?: () => void;
  releaseSelection?: () => void;
  selectorCalls: number;
};

type BrowserSummaryTemplate = {
  id: string;
  name: string;
  description: string;
  is_custom: boolean;
  sections: Array<{
    title: string;
    instruction: string;
    format: 'paragraph' | 'list' | 'table' | 'string';
    item_format?: string;
    example_item_format?: string;
  }>;
};

type BrowserSummaryTemplateDraft = Pick<BrowserSummaryTemplate, 'name' | 'description' | 'sections'>;

function parseBrowserSummaryTemplateDraft(templateJson: string | null): BrowserSummaryTemplateDraft {
  let parsed: unknown;
  try {
    parsed = JSON.parse(templateJson || '{}');
  } catch {
    throw new Error('Template JSON is invalid');
  }

  if (typeof parsed !== 'object' || parsed === null) {
    throw new Error('Template JSON must be an object');
  }

  const value = parsed as Record<string, unknown>;
  const sections = Array.isArray(value.sections)
    ? value.sections.map((section) => {
        if (typeof section !== 'object' || section === null) {
          throw new Error('Template sections must be objects');
        }
        const sectionValue = section as Record<string, unknown>;
        const format = sectionValue.format;
        const normalizedFormat: BrowserSummaryTemplate['sections'][number]['format'] =
          format === 'paragraph' || format === 'list' || format === 'table' || format === 'string'
            ? format
            : 'list';
        return {
          title: typeof sectionValue.title === 'string' ? sectionValue.title.trim() : '',
          instruction:
            typeof sectionValue.instruction === 'string' ? sectionValue.instruction.trim() : '',
          format: normalizedFormat,
          ...(typeof sectionValue.item_format === 'string' && sectionValue.item_format.trim()
            ? { item_format: sectionValue.item_format.trim() }
            : {}),
          ...(typeof sectionValue.example_item_format === 'string' && sectionValue.example_item_format.trim()
            ? { example_item_format: sectionValue.example_item_format.trim() }
            : {}),
        };
      })
    : [];

  return {
    name: typeof value.name === 'string' ? value.name.trim() : '',
    description: typeof value.description === 'string' ? value.description.trim() : '',
    sections,
  };
}

function validateBrowserSummaryTemplateDraft(draft: BrowserSummaryTemplateDraft): void {
  if (!draft.name) throw new Error('Template name cannot be empty. Add a short name.');
  if (!draft.description) {
    throw new Error('Template description cannot be empty. Explain when to use it.');
  }
  if (draft.sections.length === 0) {
    throw new Error('Template must have at least one section. Add a title and instruction.');
  }
  for (const [index, section] of draft.sections.entries()) {
    if (!section.title) throw new Error(`Section ${index + 1} needs a title.`);
    if (!section.instruction) {
      throw new Error(`Section '${section.title}' needs an instruction for the summary model.`);
    }
  }
}

type SummaryTemplateState = {
  templates: BrowserSummaryTemplate[];
  availableTemplateIds: string[];
  selectedTemplateId: string;
  selectionReads: number;
  selectionSaves: string[];
  crud: string[];
  processCalls: Array<Record<string, unknown>>;
  selectionReady: boolean;
  releaseSelection?: () => void;
};

function getBrowserRecordingState() {
  const state = window as typeof window & {
    __recordingSessionSnapshot?: BrowserRecordingSnapshot | null;
  };
  if (state.__recordingSessionSnapshot === undefined) {
    const scenario = new URLSearchParams(window.location.search).get('__e2e');
    state.__recordingSessionSnapshot = scenario === 'permissions-audio-events'
      ? {
          sessionId: 'e2e-recording-session',
          meetingId: 'e2e-meeting',
          state: 'recording',
          startedAtMs: Date.now(),
          recordingDuration: 0,
          activeDuration: 0,
          pauseDuration: 0,
          microphoneDevice: getBrowserAudioEventState().savedMicrophonePreference,
          systemDevice: 'System Audio',
          expectedTracks: ['microphone', 'system'],
          meetingName: 'Audio lifecycle',
          savePath: '/tmp/audio-lifecycle',
          autoSave: true,
        }
      : null;
  }
  return state;
}

const initialRecordingFolderPreferences: RecordingFolderPreferences = {
  save_folder: '/tmp/Meetily Recordings',
  auto_save: true,
  file_format: 'mp4',
  preferred_mic_device: null,
  preferred_system_device: null,
};

function getRecordingFolderScenario() {
  const scenario = new URLSearchParams(window.location.search).get('__e2e');
  return scenario?.startsWith('recording-folder-') ? scenario : null;
}

function getAudioPermissionScenario() {
  const scenario = new URLSearchParams(window.location.search).get('__e2e');
  return scenario?.startsWith('permissions-') ? scenario : null;
}

function getAudioPermissionState() {
  const state = window as typeof window & {
    __audioPermissionState?: BrowserAudioPermissionState;
  };
  if (state.__audioPermissionState) return state.__audioPermissionState;

  const scenario = getAudioPermissionScenario();
  state.__audioPermissionState = scenario === 'permissions-focus'
    ? { microphone: 'denied', systemAudio: 'unknown' }
    : scenario === 'permissions-main-microphone'
      ? { microphone: 'not_determined', systemAudio: 'available' }
      : scenario === 'permissions-system-audio-unknown'
        ? { microphone: 'authorized', systemAudio: 'unknown' }
        : scenario === 'permissions-system-audio-unavailable'
          ? { microphone: 'authorized', systemAudio: 'unavailable' }
          : scenario === 'permissions-onboarding-system-audio-denied'
            ? { microphone: 'authorized', systemAudio: 'denied' }
            : scenario === 'permissions-audio-events'
              ? { microphone: 'authorized', systemAudio: 'available' }
              : scenario === 'permissions-system-audio'
                ? { microphone: 'authorized', systemAudio: 'denied' }
                : { microphone: 'not_determined', systemAudio: 'unknown' };
  return state.__audioPermissionState;
}

function getRequestedSystemAudioState() {
  return getAudioPermissionScenario() === 'permissions-system-audio-unavailable'
    ? 'unavailable'
    : 'unknown';
}

function getRecordingFolderState() {
  const state = window as typeof window & {
    __recordingFolderState?: RecordingFolderState;
  };
  state.__recordingFolderState ??= {
    backendMutations: [],
    openedFolders: [],
    preferences: { ...initialRecordingFolderPreferences },
    saveAttempts: [],
    saves: [],
    selectorCalls: 0,
  };
  return state.__recordingFolderState;
}

const defaultSummaryTemplates: BrowserSummaryTemplate[] = [
  {
    id: 'standard_meeting',
    name: 'Standard Meeting Notes',
    description: 'A standard template for general meetings.',
    is_custom: false,
    sections: [
      {
        title: 'Summary',
        instruction: 'Summarize the meeting.',
        format: 'paragraph',
      },
    ],
  },
  {
    id: 'daily_standup',
    name: 'Daily Standup',
    description: 'Time-boxed daily updates.',
    is_custom: false,
    sections: [
      {
        title: 'Today',
        instruction: 'List planned work.',
        format: 'list',
      },
    ],
  },
  {
    id: 'tasks_only',
    name: 'Tasks Only',
    description: 'Only assigned tasks and supporting references.',
    is_custom: false,
    sections: [
      {
        title: 'Action Items',
        instruction: 'List assigned tasks.',
        format: 'list',
      },
    ],
  },
];

function getSummaryTemplateScenario() {
  const scenario = new URLSearchParams(window.location.search).get('__e2e');
  return scenario === 'summary-template-crud' || scenario === 'summary-template-selection-pending'
    ? scenario
    : null;
}

function getSummaryTemplateState() {
  const state = window as typeof window & {
    __summaryTemplateState?: SummaryTemplateState;
  };
  const persistedSelection = window.localStorage.getItem('e2e-summary-template-selection');
  state.__summaryTemplateState ??= {
    templates: defaultSummaryTemplates.map((template) => ({
      ...template,
      sections: template.sections.map((section) => ({ ...section })),
    })),
    availableTemplateIds: defaultSummaryTemplates.map((template) => template.id),
    selectedTemplateId: persistedSelection || 'standard_meeting',
    selectionReads: 0,
    selectionSaves: [],
    crud: [],
    processCalls: [],
    selectionReady: false,
  };
  return state.__summaryTemplateState;
}

const knownNoopCommands = new Set([
  'set_language_preference',
  'set_notification_settings',
  'set_recording_preferences',
  'start_audio_level_monitoring',
  'start_import_audio_command',
  'start_recording_with_devices_and_meeting',
  'start_retranscription_command',
  'stop_audio_level_monitoring',
]);

const commandFixtures: Record<string, unknown> = {
  get_onboarding_status: completedOnboardingStatus,
  get_recording_state: { is_recording: false, is_paused: false },
  is_recording: false,
  api_get_meetings: [meeting],
  api_get_meeting: meeting,
  api_get_meeting_metadata: meeting,
  api_get_meeting_transcripts: {
    transcripts: [
      {
        id: 'theme-test-transcript',
        text: 'We approved the dark theme.',
        timestamp: '09:00:00',
        audio_start_time: 0,
        audio_end_time: 4.2,
        duration: 4.2,
        confidence: 0.98,
      },
    ],
    has_more: false,
    total_count: 1,
  },
  api_get_summary: {
    status: 'completed',
    meetingName: meeting.title,
    meeting_id: meeting.id,
    start: '2026-06-20T09:00:00Z',
    end: '2026-06-20T09:30:00Z',
    data: {
      markdown: '# Theme Test Meeting\n## Decisions\nUse semantic tokens.',
    },
    error: null,
  },
  api_get_meeting_summary_language: {
    language: null,
    storage: 'metadata',
  },
  api_get_model_config: {
    provider: 'builtin-ai',
    model: 'qwen3.5:2b',
    whisperModel: 'parakeet-tdt-0.6b-v3-int8',
    ollamaEndpoint: null,
  },
  api_get_transcript_config: {
    provider: 'parakeet',
    model: 'parakeet-tdt-0.6b-v3-int8',
    apiKey: null,
    whisperVocabulary: null,
  },
  api_get_transcript_vocabulary: null,
  api_get_meeting_vocabulary: {
    globalVocabulary: null,
    meetingVocabulary: null,
    effectiveVocabulary: null,
    supportsVocabulary: true,
  },
  api_save_transcript_vocabulary: null,
  api_save_meeting_vocabulary: null,
  api_get_custom_openai_config: null,
  api_get_api_key: null,
  api_get_transcript_api_key: null,
  api_get_auto_generate_setting: false,
  api_list_templates: defaultSummaryTemplates.map((template) => ({ ...template })),
  api_get_summary_template_selection: {
    templateId: 'standard_meeting',
    replaced: false,
    previousTemplateId: 'standard_meeting',
  },
  api_save_summary_template_selection: {
    templateId: 'standard_meeting',
    replaced: false,
    previousTemplateId: 'standard_meeting',
  },
  api_create_custom_template: 'custom_template',
  api_copy_template: 'custom_template-copy',
  api_update_custom_template: 'custom_template',
  api_save_custom_template: 'custom_template',
  api_delete_custom_template: null,
  api_is_custom_template: false,
  get_notification_settings: null,
  get_database_directory: '/tmp/Meetily',
  whisper_get_models_directory: '/tmp/Meetily/models',
  get_default_recordings_folder_path: '/tmp/Meetily Recordings',
  get_recording_preferences: {
    save_folder: '/tmp/Meetily Recordings',
    auto_save: true,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null,
  },
  get_audio_devices: [],
  get_ollama_models: [],
  builtin_ai_list_models: [
    {
      name: 'qwen3.5:2b',
      display_name: 'Qwen 3.5 2B (Balanced)',
      status: { type: 'available' },
      path: '/tmp/models/Qwen3.5-2B-Q4_K_M.gguf',
      size_mb: 1221,
      context_size: 32768,
      description: 'Balanced local model for built-in summaries.',
      gguf_file: 'Qwen3.5-2B-Q4_K_M.gguf',
    },
    {
      name: 'qwen3.5:4b',
      display_name: 'Qwen 3.5 4B (High Quality)',
      status: { type: 'not_downloaded' },
      path: '/tmp/models/Qwen3.5-4B-Q4_K_M.gguf',
      size_mb: 2614,
      context_size: 32768,
      description: 'Higher-quality local model for built-in summaries.',
      gguf_file: 'Qwen3.5-4B-Q4_K_M.gguf',
    },
  ],
  builtin_ai_get_recommended_model: 'qwen3.5:2b',
  builtin_ai_download_model: null,
  check_first_launch: false,
  check_default_legacy_database: null,
  check_homebrew_database: null,
  initialize_fresh_database: null,
  import_and_initialize_database: null,
  parakeet_init: null,
  parakeet_has_available_models: true,
  parakeet_download_model: null,
  parakeet_retry_download: null,
  parakeet_get_available_models: [
    {
      name: 'parakeet-tdt-0.6b-v3-int8',
      path: '/tmp/models/parakeet-tdt-0.6b-v3-int8',
      size_mb: 670,
      speed: 'Ultra Fast (v3)',
      status: 'Available',
      description: 'Recommended local transcription model.',
      quantization: 'Int8',
    },
  ],
  save_onboarding_status_cmd: null,
  complete_onboarding: null,
  whisper_get_available_models: [],
};

const freshBuiltInModelFixtures = (
  commandFixtures.builtin_ai_list_models as Array<Record<string, unknown>>
).map((model) => ({
  ...model,
  status: { type: 'not_downloaded' },
  path: null,
}));

const macosGemmaModelFixtures = [
  ...(commandFixtures.builtin_ai_list_models as Array<Record<string, unknown>>),
  {
    name: 'gemma4:e2b',
    display_name: 'Gemma 4 E2B (Enhanced)',
    status: { type: 'not_downloaded' },
    path: '/tmp/models/gemma-4-E2B-it-Q4_K_M.gguf',
    size_mb: 2963,
    context_size: 32768,
    description: 'Enhanced Gemma 4 model for built-in summaries.',
    gguf_file: 'gemma-4-E2B-it-Q4_K_M.gguf',
  },
  {
    name: 'gemma4:e4b',
    display_name: 'Gemma 4 E4B (High Quality)',
    status: { type: 'not_downloaded' },
    path: '/tmp/models/gemma-4-E4B-it-Q4_K_M.gguf',
    size_mb: 5088,
    context_size: 32768,
    description: 'High-quality Gemma 4 model for built-in summaries.',
    gguf_file: 'gemma-4-E4B-it-Q4_K_M.gguf',
  },
];

let installed = false;

function getScenarioFixture(command: string, payload?: unknown) {
  const scenario = new URLSearchParams(window.location.search).get('__e2e');

  const summaryTemplateScenario = getSummaryTemplateScenario();
  if (summaryTemplateScenario) {
    const state = getSummaryTemplateState();
    const templatePayload = (payload: unknown, key: string) => {
      if (typeof payload !== 'object' || payload === null || !(key in payload)) return null;
      const value = (payload as Record<string, unknown>)[key];
      return typeof value === 'string' ? value : null;
    };

    if (command === 'api_get_summary_template_selection') {
      state.selectionReads += 1;
      const resolveSelection = () => {
        const previousTemplateId = state.selectedTemplateId;
        if (!state.templates.some((template) => template.id === state.selectedTemplateId)) {
          state.selectedTemplateId = 'standard_meeting';
          window.localStorage.setItem('e2e-summary-template-selection', state.selectedTemplateId);
        }
        state.availableTemplateIds = state.templates.map((template) => template.id);
        state.selectionReady = true;
        return {
          templateId: state.selectedTemplateId,
          replaced: previousTemplateId !== state.selectedTemplateId,
          previousTemplateId,
        };
      };
      if (summaryTemplateScenario === 'summary-template-selection-pending') {
        return new Promise((resolve) => {
          const release = () => resolve(resolveSelection());
          state.releaseSelection = release;
          (window as typeof window & {
            __releaseSummaryTemplateSelection?: () => void;
          }).__releaseSummaryTemplateSelection = release;
        });
      }
      return resolveSelection();
    }

    if (command === 'api_list_templates') {
      state.availableTemplateIds = state.templates.map((template) => template.id);
      return state.templates.map((template) => ({ ...template }));
    }

    if (command === 'api_save_summary_template_selection') {
      const templateId = templatePayload(payload, 'templateId');
      if (!templateId || !state.templates.some((template) => template.id === templateId)) {
        throw new Error(`Summary template '${templateId || ''}' is not available`);
      }
      state.selectedTemplateId = templateId;
      window.localStorage.setItem('e2e-summary-template-selection', templateId);
      state.selectionSaves.push(templateId);
      state.crud.push(`save-selection:${templateId}`);
      state.availableTemplateIds = state.templates.map((template) => template.id);
      return {
        templateId,
        replaced: false,
        previousTemplateId: templateId,
      };
    }

    if (command === 'api_get_template_details') {
      const templateId = templatePayload(payload, 'templateId');
      const template = state.templates.find((candidate) => candidate.id === templateId);
      if (!template) throw new Error(`Template '${templateId || ''}' was not found`);
      return {
        ...template,
        sections: template.sections.map((section) => ({ ...section })),
      };
    }

    if (command === 'api_validate_template') {
      const draft = parseBrowserSummaryTemplateDraft(templatePayload(payload, 'templateJson'));
      validateBrowserSummaryTemplateDraft(draft);
      return draft.name;
    }

    if (command === 'api_create_custom_template') {
      const templateId = templatePayload(payload, 'templateId') || 'browser-custom';
      const draft = parseBrowserSummaryTemplateDraft(templatePayload(payload, 'templateJson'));
      validateBrowserSummaryTemplateDraft(draft);
      const created: BrowserSummaryTemplate = {
        id: templateId,
        name: draft.name,
        description: draft.description,
        sections: draft.sections.map((section) => ({ ...section })),
        is_custom: true,
      };
      state.templates = [...state.templates.filter((template) => template.id !== templateId), created];
      state.availableTemplateIds = state.templates.map((template) => template.id);
      state.crud.push(`create:${templateId}`);
      return templateId;
    }

    if (command === 'api_copy_template') {
      const sourceTemplateId = templatePayload(payload, 'sourceTemplateId');
      const source = state.templates.find((template) => template.id === sourceTemplateId);
      if (!source) throw new Error(`Template '${sourceTemplateId || ''}' was not found`);

      const requestedTemplateId = templatePayload(payload, 'requestedTemplateId')?.trim();
      const baseTemplateId = requestedTemplateId || `${source.id}-copy`;
      let copiedTemplateId = baseTemplateId;
      let suffix = 2;
      while (state.templates.some((template) => template.id === copiedTemplateId)) {
        copiedTemplateId = `${baseTemplateId}-${suffix}`;
        suffix += 1;
      }

      const copied: BrowserSummaryTemplate = {
        ...source,
        id: copiedTemplateId,
        name: `${source.name} Copy`,
        is_custom: true,
        sections: source.sections.map((section) => ({ ...section })),
      };
      state.templates = [...state.templates, copied];
      state.availableTemplateIds = state.templates.map((template) => template.id);
      state.crud.push(`copy:${source.id}->${copiedTemplateId}`);
      return copiedTemplateId;
    }

    if (command === 'api_update_custom_template') {
      const templateId = templatePayload(payload, 'templateId');
      const draft = parseBrowserSummaryTemplateDraft(templatePayload(payload, 'templateJson'));
      validateBrowserSummaryTemplateDraft(draft);
      const index = state.templates.findIndex((template) => template.id === templateId);
      if (index < 0 || !state.templates[index].is_custom) {
        throw new Error(`Template '${templateId || ''}' is not a custom template`);
      }
      state.templates = state.templates.map((template, templateIndex) =>
        templateIndex === index
          ? {
              ...template,
              name: draft.name,
              description: draft.description,
              sections: draft.sections.map((section) => ({ ...section })),
            }
          : template,
      );
      state.crud.push(`update:${templateId}`);
      return templateId;
    }

    if (command === 'api_delete_custom_template') {
      const templateId = templatePayload(payload, 'templateId');
      const template = state.templates.find((candidate) => candidate.id === templateId);
      if (!template?.is_custom) throw new Error(`Template '${templateId || ''}' is not a custom template`);
      state.templates = state.templates.filter((candidate) => candidate.id !== templateId);
      state.availableTemplateIds = state.templates.map((candidate) => candidate.id);
      state.crud.push(`delete:${templateId}`);
      return null;
    }

    if (command === 'api_is_custom_template') {
      const templateId = templatePayload(payload, 'templateId');
      return state.templates.some((template) => template.id === templateId && template.is_custom);
    }

    if (command === 'api_process_transcript') {
      state.processCalls.push((payload || {}) as Record<string, unknown>);
      return { message: 'Summary process started', process_id: 'summary-process-e2e' };
    }
  }

  if (scenario?.startsWith('recording-folder-')) {
    const state = getRecordingFolderState();
    if (command === 'get_recording_preferences') return { ...state.preferences };
    if (command === 'get_default_recordings_folder_path') {
      return state.preferences.save_folder;
    }
    if (command === 'select_recording_folder') {
      state.selectorCalls += 1;
      if (scenario === 'recording-folder-picker-pending') {
        return new Promise<string>((resolve) => {
          state.releaseSelection = () => resolve('/tmp/Meeting Captures');
        });
      }
      return scenario === 'recording-folder-cancel'
        ? null
        : '/tmp/Meeting Captures';
    }
    if (command === 'set_recording_preferences') return undefined;
  }

  if (
    (scenario === 'onboarding' || scenario === 'onboarding-gemma') &&
    command === 'get_onboarding_status'
  ) {
    return newOnboardingStatus;
  }

  if (
    (scenario === 'permissions-onboarding' ||
      scenario === 'permissions-focus' ||
      scenario === 'permissions-onboarding-system-audio-denied') &&
    command === 'get_onboarding_status'
  ) {
    return { ...newOnboardingStatus, current_step: 4 };
  }

  if (
    scenario === 'onboarding-gemma' &&
    command === 'builtin_ai_get_recommended_model'
  ) {
    return 'gemma4:e2b';
  }

  if (scenario === 'onboarding-gemma' && command === 'builtin_ai_list_models') {
    return macosGemmaModelFixtures;
  }

  if (scenario === 'empty-home' && command === 'api_get_meetings') {
    return [];
  }

  if (
    scenario === 'missing-local-summary-model' &&
    command === 'api_get_model_config'
  ) {
    return {
      ...(commandFixtures.api_get_model_config as Record<string, unknown>),
      model: 'qwen3.5:4b',
    };
  }

  if (
    scenario === 'missing-local-summary-model' &&
    command === 'builtin_ai_is_model_ready'
  ) {
    return false;
  }

  if (
    scenario === 'missing-local-summary-model' &&
    command === 'builtin_ai_get_model_info'
  ) {
    return (commandFixtures.builtin_ai_list_models as Array<Record<string, unknown>>)
      .find((model) => model.name === 'qwen3.5:4b');
  }

  if (scenario === 'missing-builtin-model' && command === 'api_get_model_config') {
    return {
      ...(commandFixtures.api_get_model_config as Record<string, unknown>),
      model: '',
    };
  }

  if (scenario === 'missing-builtin-model' && command === 'builtin_ai_list_models') {
    return freshBuiltInModelFixtures;
  }

  if (scenario === 'delayed-builtin-models' && command === 'api_get_model_config') {
    return {
      ...(commandFixtures.api_get_model_config as Record<string, unknown>),
      provider: 'claude',
      model: '',
    };
  }

  if (scenario === 'delayed-builtin-models' && command === 'get_ollama_models') {
    return [
      {
        id: 'llama3.2:latest',
        name: 'llama3.2:latest',
        size: '2.0 GiB',
      },
    ];
  }

  if (scenario === 'delayed-builtin-models' && command === 'builtin_ai_list_models') {
    const state = window as typeof window & {
      __pendingBuiltinModelRequests?: number;
      __builtinModelRequestResolvers?: Array<() => void>;
      __releaseBuiltinModelRequests?: () => void;
    };

    return new Promise((resolve) => {
      const resolvers = state.__builtinModelRequestResolvers ?? [];
      state.__builtinModelRequestResolvers = resolvers;
      resolvers.push(() => resolve(freshBuiltInModelFixtures));
      state.__pendingBuiltinModelRequests = resolvers.length;
      state.__releaseBuiltinModelRequests = () => {
        const pendingResolvers = resolvers.splice(0);
        state.__pendingBuiltinModelRequests = 0;
        pendingResolvers.forEach((release) => release());
      };
    });
  }

  if (scenario === 'recovery-audio-error' && command === 'has_audio_checkpoints') {
    throw new Error('Audio checkpoint inspection unavailable');
  }

  return undefined;
}

export function installTauriBrowserMocks() {
  if (process.env.NODE_ENV === 'production') {
    throw new Error('E2E browser mocks cannot run in production');
  }

  if (
    installed ||
    typeof window === 'undefined' ||
    process.env.NEXT_PUBLIC_E2E_TESTING !== '1'
  ) {
    return;
  }

  installed = true;
  const testWindow = window as typeof window & {
    __tauriCommandJournal?: TauriCommandJournalEntry[];
  };
  testWindow.__tauriCommandJournal = [];
  mockWindows('main');
  mockIPC(
    (command, payload) => {
      testWindow.__tauriCommandJournal?.push({ command, payload });
      if (getAudioPermissionScenario()) {
        const permissionState = getAudioPermissionState();
        if (command === 'get_audio_permission_snapshot') return { ...permissionState };
        if (command === 'request_microphone_permission') {
          permissionState.microphone = 'authorized';
          return { ...permissionState };
        }
        if (command === 'request_system_audio_permission') {
          permissionState.systemAudio = getRequestedSystemAudioState();
          return { ...permissionState };
        }
        if (command === 'open_audio_permission_settings') return null;
        if (
          getAudioPermissionScenario() === 'permissions-audio-events'
          && command === 'get_recording_preferences'
        ) {
          return {
            ...(commandFixtures.get_recording_preferences as Record<string, unknown>),
            preferred_mic_device: getBrowserAudioEventState().savedMicrophonePreference,
          };
        }
        if (
          getAudioPermissionScenario() === 'permissions-audio-events'
          && command === 'set_recording_preferences'
        ) {
          const preferences = (payload as { preferences: RecordingFolderPreferences })
            .preferences;
          getBrowserAudioEventState().savedMicrophonePreference =
            preferences.preferred_mic_device ?? '';
          return null;
        }
      }
      const scenarioFixture = getScenarioFixture(command, payload);
      if (scenarioFixture !== undefined) return scenarioFixture;
      const recordingFolderScenario = getRecordingFolderScenario();
      if (recordingFolderScenario) {
        const state = getRecordingFolderState();
        if (command === 'set_recording_preferences') {
          const preferences = (payload as { preferences: RecordingFolderPreferences })
            .preferences;
          state.saveAttempts.push({ ...preferences });
          if (recordingFolderScenario === 'recording-folder-failure') {
            const previousPreferences = state.preferences;
            state.preferences = { ...preferences };
            state.backendMutations.push(preferences.save_folder);
            state.preferences = previousPreferences;
            state.backendMutations.push(previousPreferences.save_folder);
            throw new Error('Recording preferences store unavailable');
          }
          if (recordingFolderScenario === 'recording-folder-pending') {
            return new Promise<null>((resolve) => {
              state.releaseSave = () => {
                state.preferences = { ...preferences };
                state.saves.push({ ...preferences });
                resolve(null);
              };
            });
          }
          state.preferences = { ...preferences };
          state.saves.push({ ...preferences });
          return null;
        }
        if (command === 'open_recordings_folder') {
          if (recordingFolderScenario === 'recording-folder-open-failure') {
            throw new Error('Unable to open persisted recording folder');
          }
          state.openedFolders.push(state.preferences.save_folder);
          return null;
        }
      }
      if (command === 'api_save_model_config') {
        (
          window as typeof window & {
            __savedModelConfig?: Record<string, unknown>;
          }
        ).__savedModelConfig = payload as Record<string, unknown>;
        return null;
      }
      if (command === 'plugin:app|set_app_theme') {
        const theme =
          typeof payload === 'object' &&
          payload !== null &&
          'theme' in payload &&
          (payload.theme === 'light' ||
            payload.theme === 'dark' ||
            payload.theme === null)
            ? payload.theme
            : undefined;
        if (theme === undefined) {
          throw new Error(
            '[E2E fixture invalid] set_app_theme requires light, dark, or null',
          );
        }
        const state = window as typeof window & {
          __failNativeTheme?: boolean;
          __nativeThemeCalls?: Array<'light' | 'dark' | null>;
          __nativeThemeDelays?: Partial<
            Record<'light' | 'dark' | 'system', number>
          >;
          __nativeThemeFailures?: number;
          __nativeThemeStarts?: Array<'light' | 'dark' | null>;
        };
        state.__nativeThemeStarts = [...(state.__nativeThemeStarts ?? []), theme];
        const delay = state.__nativeThemeDelays?.[
          theme === null ? 'system' : theme
        ] ?? 0;
        return new Promise<null>((resolve, reject) => {
          window.setTimeout(() => {
            state.__nativeThemeCalls = [
              ...(state.__nativeThemeCalls ?? []),
              theme,
            ];
            if (state.__failNativeTheme) {
              state.__nativeThemeFailures =
                (state.__nativeThemeFailures ?? 0) + 1;
              reject(new Error('Native theme unavailable'));
              return;
            }
            resolve(null);
          }, delay);
        });
      }
      if (command === 'plugin:app|version') return '0.4.1';
      if (command === 'plugin:store|load') return 1;
      if (
        command === 'plugin:store|has' ||
        command === 'plugin:store|get' ||
        command === 'plugin:store|set'
      ) {
        const key =
          typeof payload === 'object' &&
          payload !== null &&
          'key' in payload &&
          typeof payload.key === 'string'
            ? payload.key
            : '';

        if (command === 'plugin:store|has') return browserStore.has(key);
        if (command === 'plugin:store|get') {
          return [browserStore.get(key), browserStore.has(key)];
        }

        const value =
          typeof payload === 'object' &&
          payload !== null &&
          'value' in payload
            ? payload.value
            : undefined;
        browserStore.set(key, value);
        return null;
      }
      if (command === 'plugin:store|save') return null;
      if (command === 'builtin_ai_is_model_ready') {
        return (
          typeof payload === 'object' &&
          payload !== null &&
          'modelName' in payload &&
          payload.modelName === 'qwen3.5:2b'
        );
      }
      if (command === 'get_active_recording_session') {
        return getBrowserRecordingState().__recordingSessionSnapshot;
      }
      if (command === 'get_recording_session') {
        const snapshot = getBrowserRecordingState().__recordingSessionSnapshot;
        const sessionId =
          typeof payload === 'object' &&
          payload !== null &&
          'sessionId' in payload &&
          typeof payload.sessionId === 'string'
            ? payload.sessionId
            : '';
        if (snapshot?.sessionId === sessionId) return snapshot;
        throw new Error(`Recording session not found: ${sessionId}`);
      }
      if (command === 'recover_recording_session') {
        const snapshot = getBrowserRecordingState().__recordingSessionSnapshot;
        const sessionId =
          typeof payload === 'object' &&
          payload !== null &&
          'sessionId' in payload &&
          typeof payload.sessionId === 'string'
            ? payload.sessionId
            : '';
        if (snapshot?.sessionId !== sessionId) {
          throw new Error(`Recording session not found: ${sessionId}`);
        }
        const recovered = { ...snapshot, state: 'completed' as const };
        getBrowserRecordingState().__recordingSessionSnapshot = recovered;
        return recovered;
      }
      if (command === 'start_recording') {
        const request =
          typeof payload === 'object' &&
          payload !== null &&
          'request' in payload &&
          typeof payload.request === 'object' &&
          payload.request !== null
            ? (payload.request as Record<string, unknown>)
            : {};
        const expectedTracks: Array<'microphone' | 'system'> = [];
        expectedTracks.push('microphone');
        if (request.captureSystem !== false) expectedTracks.push('system');
        const snapshot: BrowserRecordingSnapshot = {
          sessionId: 'e2e-recording-session',
          meetingId:
            typeof request.meetingId === 'string' ? request.meetingId : 'e2e-meeting',
          state: 'recording',
          startedAtMs: Date.now(),
          recordingDuration: 0,
          activeDuration: 0,
          pauseDuration: 0,
          microphoneDevice:
            typeof request.microphoneDevice === 'string'
              ? request.microphoneDevice
              : null,
          systemDevice:
            typeof request.systemDevice === 'string' ? request.systemDevice : null,
          expectedTracks,
          meetingName:
            typeof request.meetingName === 'string' ? request.meetingName : '',
          savePath: typeof request.savePath === 'string' ? request.savePath : '',
          autoSave: typeof request.autoSave === 'boolean' ? request.autoSave : true,
        };
        getBrowserRecordingState().__recordingSessionSnapshot = snapshot;
        return snapshot;
      }
      if (command === 'pause_recording' || command === 'resume_recording') {
        const state = getBrowserRecordingState();
        if (!state.__recordingSessionSnapshot) {
          throw new Error('No active recording session');
        }
        state.__recordingSessionSnapshot = {
          ...state.__recordingSessionSnapshot,
          state: command === 'pause_recording' ? 'paused' : 'recording',
        };
        return state.__recordingSessionSnapshot;
      }
      if (command === 'set_recording_source_control') {
        const state = getBrowserRecordingState();
        if (!state.__recordingSessionSnapshot) {
          throw new Error('No active recording session');
        }
        const request = payload as {
          kind?: unknown;
          control?: { muted?: unknown; gain?: unknown };
        };
        if (request.kind !== 'microphone' && request.kind !== 'system') {
          throw new Error('Unknown recording source');
        }
        if (
          !request.control ||
          typeof request.control.muted !== 'boolean' ||
          typeof request.control.gain !== 'number' ||
          !Number.isFinite(request.control.gain) ||
          request.control.gain < 0 ||
          request.control.gain > 4
        ) {
          throw new Error('Invalid recording source control');
        }
        return state.__recordingSessionSnapshot;
      }
      if (command === 'stop_recording') {
        const state = getBrowserRecordingState();
        if (!state.__recordingSessionSnapshot) {
          throw new Error('No active recording session');
        }
        state.__recordingSessionSnapshot = {
          ...state.__recordingSessionSnapshot,
          state: 'completed',
          capturedTracks: [],
        };
        return state.__recordingSessionSnapshot;
      }
      if (command === 'has_audio_checkpoints') return true;
      if (command in commandFixtures) return commandFixtures[command];

      if (knownNoopCommands.has(command)) return null;

      throw new Error(`[E2E fixture missing] ${command}`);
    },
    { shouldMockEvents: true },
  );

  (
    window as typeof window & {
      __emitTauriEvent?: (
        event: string,
        payload: Record<string, unknown>,
      ) => Promise<void>;
    }
  ).__emitTauriEvent = emit;

  const audioEventWindow = window as typeof window & {
    __emitRecordingAudioRouteChanged?: (currentDevice: string) => Promise<void>;
    __emitRecordingAudioDegraded?: () => Promise<void>;
    __getSavedMicrophonePreference?: () => string;
  };
  audioEventWindow.__emitRecordingAudioRouteChanged = async (currentDevice) => {
    const state = getBrowserRecordingState();
    const snapshot = state.__recordingSessionSnapshot;
    if (!snapshot) throw new Error('No active recording session');
    state.__recordingSessionSnapshot = {
      ...snapshot,
      microphoneDevice: currentDevice,
    };
    await emit('recording-audio-route-changed', {
      sessionId: snapshot.sessionId,
      previousDevice: snapshot.microphoneDevice ?? audioEventSavedMicrophone,
      currentDevice,
      reason: 'active microphone became unavailable',
    });
  };
  audioEventWindow.__emitRecordingAudioDegraded = () => {
    const snapshot = getBrowserRecordingState().__recordingSessionSnapshot;
    if (!snapshot) throw new Error('No active recording session');
    return emit('recording-audio-degraded', {
      sessionId: snapshot.sessionId,
      source: 'system',
      reason: 'system stream stopped',
    });
  };
  audioEventWindow.__getSavedMicrophonePreference = () =>
    getBrowserAudioEventState().savedMicrophonePreference;

  const recoveryTestWindow = window as typeof window & {
    __saveRecoveryTranscript?: (
      meetingId: string,
      transcript: TranscriptUpdate,
    ) => Promise<void>;
    __getRecoveryTranscripts?: (
      meetingId: string,
    ) => Promise<StoredTranscript[]>;
    __saveRecoveryMetadata?: (metadata: MeetingMetadata) => Promise<void>;
    __getOrCreateRecoverySQLiteMeetingId?: (
      meetingId: string,
      proposedId: string,
    ) => Promise<string>;
    __getRecoveryMetadata?: (meetingId: string) => Promise<MeetingMetadata | null>;
  };
  recoveryTestWindow.__saveRecoveryTranscript = (meetingId, transcript) =>
    indexedDBService.saveTranscript(meetingId, transcript);
  recoveryTestWindow.__getRecoveryTranscripts = (meetingId) =>
    indexedDBService.getTranscripts(meetingId);
  recoveryTestWindow.__saveRecoveryMetadata = (metadata) =>
    indexedDBService.saveMeetingMetadata(metadata);
  recoveryTestWindow.__getOrCreateRecoverySQLiteMeetingId = (meetingId, proposedId) =>
    indexedDBService.getOrCreateSQLiteMeetingId(meetingId, proposedId);
  recoveryTestWindow.__getRecoveryMetadata = (meetingId) =>
    indexedDBService.getMeetingMetadata(meetingId);
}
