import {
  indexedDBService,
  type IndexedDBMigrationSnapshot,
  type MeetingMetadata,
  type StoredTranscript,
} from '@/services/indexedDBService';

export const WEB_MIGRATION_FORMAT_VERSION = 1 as const;
export const WEB_MIGRATION_SCHEMA_VERSION = 1 as const;

const EXACT_LOCAL_STORAGE_KEYS = new Set([
  'betaFeatures',
  'interfaceLanguage',
  'isAutoSummary',
  'primaryLanguage',
  'providerModelMap',
  'showConfidenceIndicator',
  'summaryLanguageDefault',
  'summaryLanguageRecents',
  'summarySystemPrompt',
  'themeMode',
]);

const LOCAL_STORAGE_PREFIXES = [
  'detectedSummaryLanguageFallback:',
  'summaryLanguageFallback:',
] as const;

export interface WebMigrationLocalStorageEntry {
  key: string;
  value: string;
  sha256: string;
}

export interface WebMigrationSnapshot {
  localStorage: WebMigrationLocalStorageEntry[];
  meetings: MeetingMetadata[];
  transcripts: StoredTranscript[];
}

export interface WebMigrationPackage extends WebMigrationSnapshot {
  formatVersion: typeof WEB_MIGRATION_FORMAT_VERSION;
  operationId: string;
  sourceIdentity: string;
  targetIdentity: string;
  sourceSchemaVersion: typeof WEB_MIGRATION_SCHEMA_VERSION;
  createdAt: string;
  payloadSha256: string;
}

/** Durable identity for an imported package, bound to its verified payload. */
export interface WebMigrationOperationIdentity {
  operationId: string;
  sourceIdentity: string;
  targetIdentity: string;
  payloadSha256: string;
}

export interface WebMigrationImportSink {
  /** The sink must check this marker in its own durable store. */
  hasImportedOperation(identity: WebMigrationOperationIdentity): Promise<boolean>;
  /** Commit all records and the operation marker atomically. */
  commit(
    snapshot: WebMigrationSnapshot,
    identity: WebMigrationOperationIdentity,
  ): Promise<void | WebMigrationImportResult>;
}

export interface WebMigrationIndexedDBSource {
  getMigrationSnapshot(): Promise<IndexedDBMigrationSnapshot>;
}

export interface WebMigrationIndexedDBTarget {
  hasImportedMigrationOperation(identity: WebMigrationOperationIdentity): Promise<boolean>;
  commitMigrationSnapshot(
    snapshot: IndexedDBMigrationSnapshot,
    identity: WebMigrationOperationIdentity,
  ): Promise<WebMigrationImportResult>;
}

export type WebMigrationImportResult = 'imported' | 'already_imported';

export type WebMigrationErrorCode =
  | 'invalid_package'
  | 'checksum_mismatch'
  | 'unsupported_version';

export class WebMigrationError extends Error {
  readonly code: WebMigrationErrorCode;

  constructor(code: WebMigrationErrorCode, message: string) {
    super(message);
    this.name = 'WebMigrationError';
    this.code = code;
  }
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (!value || typeof value !== 'object') return value;

  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, nested]) => [key, stableValue(nested)]),
  );
}

function stableJson(value: unknown): string {
  return JSON.stringify(stableValue(value));
}

async function sha256Hex(value: string): Promise<string> {
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) {
    throw new WebMigrationError(
      'invalid_package',
      'Web Crypto SHA-256 is unavailable; migration export cannot be verified',
    );
  }
  const digest = await subtle.digest('SHA-256', new TextEncoder().encode(value));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

function isSha256(value: unknown): value is string {
  return typeof value === 'string' && /^[0-9a-f]{64}$/i.test(value);
}

function isAllowedLocalStorageKey(key: string): boolean {
  return EXACT_LOCAL_STORAGE_KEYS.has(key)
    || LOCAL_STORAGE_PREFIXES.some((prefix) => key.startsWith(prefix) && key.length > prefix.length);
}

function isOperationId(value: unknown): value is string {
  return typeof value === 'string'
    && /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);
}

function isMeetingMetadata(value: unknown): value is MeetingMetadata {
  if (!value || typeof value !== 'object') return false;
  const meeting = value as Partial<MeetingMetadata>;
  return typeof meeting.meetingId === 'string'
    && meeting.meetingId.length > 0
    && typeof meeting.title === 'string'
    && typeof meeting.startTime === 'number'
    && Number.isFinite(meeting.startTime)
    && typeof meeting.lastUpdated === 'number'
    && Number.isFinite(meeting.lastUpdated)
    && typeof meeting.transcriptCount === 'number'
    && Number.isInteger(meeting.transcriptCount)
    && meeting.transcriptCount >= 0
    && typeof meeting.savedToSQLite === 'boolean'
    && (meeting.sqliteMeetingId === undefined
      || (typeof meeting.sqliteMeetingId === 'string' && meeting.sqliteMeetingId.length > 0))
    && (meeting.folderPath === undefined || typeof meeting.folderPath === 'string');
}

function isOptionalFiniteNumber(value: unknown): boolean {
  return value === undefined || (typeof value === 'number' && Number.isFinite(value));
}

function isStoredTranscript(value: unknown): value is StoredTranscript {
  if (!value || typeof value !== 'object') return false;
  const transcript = value as Partial<StoredTranscript>;
  return typeof transcript.meetingId === 'string'
    && transcript.meetingId.length > 0
    && typeof transcript.text === 'string'
    && typeof transcript.timestamp === 'string'
    && typeof transcript.confidence === 'number'
    && Number.isFinite(transcript.confidence)
    && typeof transcript.sequenceId === 'number'
    && Number.isInteger(transcript.sequenceId)
    && typeof transcript.storedAt === 'number'
    && Number.isFinite(transcript.storedAt)
    && (transcript.id === undefined
      || (typeof transcript.id === 'number' && Number.isInteger(transcript.id) && transcript.id >= 0))
    && isOptionalFiniteNumber(transcript.audioStartTime)
    && isOptionalFiniteNumber(transcript.audioEndTime)
    && isOptionalFiniteNumber(transcript.duration)
    && isOptionalFiniteNumber(transcript.chunkStartTime)
    && (transcript.isPartial === undefined || typeof transcript.isPartial === 'boolean');
}

function sortMeetings(meetings: readonly MeetingMetadata[]): MeetingMetadata[] {
  return meetings
    .map((meeting) => ({ ...meeting }))
    .sort((left, right) => left.meetingId.localeCompare(right.meetingId));
}

function sortTranscripts(transcripts: readonly StoredTranscript[]): StoredTranscript[] {
  return transcripts
    .map((transcript) => ({ ...transcript }))
    .sort((left, right) => {
      const meetingOrder = left.meetingId.localeCompare(right.meetingId);
      if (meetingOrder !== 0) return meetingOrder;
      const sequenceOrder = left.sequenceId - right.sequenceId;
      if (sequenceOrder !== 0) return sequenceOrder;
      return (left.id ?? Number.MAX_SAFE_INTEGER) - (right.id ?? Number.MAX_SAFE_INTEGER);
    });
}

function payloadWithoutChecksum(pkg: Omit<WebMigrationPackage, 'payloadSha256'>): string {
  return stableJson(pkg);
}

export function collectAllowedLocalStorage(
  storage: Pick<Storage, 'length' | 'key' | 'getItem'>,
): string[] {
  const keys: string[] = [];
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (key && isAllowedLocalStorageKey(key)) keys.push(key);
  }
  return keys.sort();
}

function readAllowedLocalStorage(
  storage: Pick<Storage, 'length' | 'key' | 'getItem'>,
): Record<string, string | null> {
  return Object.fromEntries(
    collectAllowedLocalStorage(storage).map((key) => [key, storage.getItem(key)]),
  );
}

export async function createWebMigrationPackageFromIndexedDB(input: {
  operationId: string;
  sourceIdentity: string;
  targetIdentity: string;
  createdAt?: string;
  storage: Pick<Storage, 'length' | 'key' | 'getItem'>;
  source?: WebMigrationIndexedDBSource;
}): Promise<WebMigrationPackage> {
  const source = input.source ?? indexedDBService;
  const { meetings, transcripts } = await source.getMigrationSnapshot();
  return createWebMigrationPackage({
    operationId: input.operationId,
    sourceIdentity: input.sourceIdentity,
    targetIdentity: input.targetIdentity,
    createdAt: input.createdAt,
    localStorage: readAllowedLocalStorage(input.storage),
    meetings,
    transcripts,
  });
}

export function createIndexedDBMigrationSink(
  storage: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>,
  target: WebMigrationIndexedDBTarget = indexedDBService,
): WebMigrationImportSink {
  return {
    hasImportedOperation: (identity) => target.hasImportedMigrationOperation(identity),
    async commit(snapshot, identity) {
      const previousValues = new Map<string, string | null>();
      try {
        for (const entry of snapshot.localStorage) {
          previousValues.set(entry.key, storage.getItem(entry.key));
          storage.setItem(entry.key, entry.value);
        }
        const result = await target.commitMigrationSnapshot({
          meetings: snapshot.meetings,
          transcripts: snapshot.transcripts,
        }, identity);
        return result;
      } catch (error) {
        for (const [key, value] of previousValues) {
          try {
            if (value === null) storage.removeItem(key);
            else storage.setItem(key, value);
          } catch {
            // Best-effort rollback; the durable marker is absent on failure.
          }
        }
        throw error;
      }
    },
  };
}

export async function createWebMigrationPackage(input: {
  operationId: string;
  sourceIdentity: string;
  targetIdentity: string;
  createdAt?: string;
  localStorage: Readonly<Record<string, string | null>>;
  meetings: readonly MeetingMetadata[];
  transcripts: readonly StoredTranscript[];
}): Promise<WebMigrationPackage> {
  if (!isOperationId(input.operationId)) {
    throw new WebMigrationError('invalid_package', 'operationId must be a canonical UUID');
  }
  if (!input.sourceIdentity.trim() || !input.targetIdentity.trim()
      || input.sourceIdentity === input.targetIdentity) {
    throw new WebMigrationError('invalid_package', 'source and target identities must differ');
  }
  const createdAt = input.createdAt ?? new Date().toISOString();
  if (Number.isNaN(Date.parse(createdAt))) {
    throw new WebMigrationError('invalid_package', 'createdAt must be an ISO timestamp');
  }

  if (!input.meetings.every(isMeetingMetadata) || !input.transcripts.every(isStoredTranscript)) {
    throw new WebMigrationError('invalid_package', 'snapshot contains malformed records');
  }

  const localStorage: WebMigrationLocalStorageEntry[] = [];
  for (const key of Object.keys(input.localStorage).sort()) {
    const value = input.localStorage[key];
    if (!isAllowedLocalStorageKey(key) || value === null) continue;
    localStorage.push({ key, value, sha256: await sha256Hex(value) });
  }

  const meetings = sortMeetings(input.meetings);
  const transcripts = sortTranscripts(input.transcripts);
  const meetingIds = new Set(meetings.map((meeting) => meeting.meetingId));
  if (transcripts.some((transcript) => !meetingIds.has(transcript.meetingId))) {
    throw new WebMigrationError('invalid_package', 'snapshot contains malformed records');
  }

  const payload: Omit<WebMigrationPackage, 'payloadSha256'> = {
    formatVersion: WEB_MIGRATION_FORMAT_VERSION,
    operationId: input.operationId,
    sourceIdentity: input.sourceIdentity,
    targetIdentity: input.targetIdentity,
    sourceSchemaVersion: WEB_MIGRATION_SCHEMA_VERSION,
    createdAt,
    localStorage,
    meetings,
    transcripts,
  };
  return { ...payload, payloadSha256: await sha256Hex(payloadWithoutChecksum(payload)) };
}

export function validateWebMigrationPackage(pkg: unknown): asserts pkg is WebMigrationPackage {
  if (!pkg || typeof pkg !== 'object') {
    throw new WebMigrationError('invalid_package', 'migration package must be an object');
  }
  const value = pkg as Partial<WebMigrationPackage>;
  if (value.formatVersion !== WEB_MIGRATION_FORMAT_VERSION
      || value.sourceSchemaVersion !== WEB_MIGRATION_SCHEMA_VERSION) {
    throw new WebMigrationError('unsupported_version', 'unsupported migration package version');
  }
  if (!isOperationId(value.operationId)) {
    throw new WebMigrationError('invalid_package', 'migration operation ID is invalid');
  }
  if (typeof value.sourceIdentity !== 'string' || !value.sourceIdentity.trim()
      || typeof value.targetIdentity !== 'string' || !value.targetIdentity.trim()
      || value.sourceIdentity === value.targetIdentity) {
    throw new WebMigrationError('invalid_package', 'migration identities are invalid');
  }
  if (typeof value.createdAt !== 'string' || Number.isNaN(Date.parse(value.createdAt))) {
    throw new WebMigrationError('invalid_package', 'migration timestamp is invalid');
  }
  if (!Array.isArray(value.localStorage) || !Array.isArray(value.meetings)
      || !Array.isArray(value.transcripts) || !isSha256(value.payloadSha256)) {
    throw new WebMigrationError('invalid_package', 'migration package payload is incomplete');
  }

  const localKeys = new Set<string>();
  for (const entry of value.localStorage) {
    if (!entry || typeof entry !== 'object') {
      throw new WebMigrationError('invalid_package', 'localStorage entry is malformed');
    }
    const typedEntry = entry as Partial<WebMigrationLocalStorageEntry>;
    if (typeof typedEntry.key !== 'string' || !isAllowedLocalStorageKey(typedEntry.key)
        || localKeys.has(typedEntry.key) || typeof typedEntry.value !== 'string'
        || !isSha256(typedEntry.sha256)) {
      throw new WebMigrationError('invalid_package', 'localStorage entry is not allowlisted');
    }
    localKeys.add(typedEntry.key);
  }
  const meetingIds = new Set(value.meetings.map((meeting) => meeting.meetingId));
  if (!value.meetings.every(isMeetingMetadata)
      || !value.transcripts.every(isStoredTranscript)
      || value.transcripts.some((transcript) => !meetingIds.has(transcript.meetingId))) {
    throw new WebMigrationError('invalid_package', 'snapshot contains malformed records');
  }
}

export async function verifyWebMigrationPackage(pkg: unknown): Promise<WebMigrationPackage> {
  validateWebMigrationPackage(pkg);
  for (const entry of pkg.localStorage) {
    if (await sha256Hex(entry.value) !== entry.sha256.toLowerCase()) {
      throw new WebMigrationError('checksum_mismatch', `localStorage checksum mismatch: ${entry.key}`);
    }
  }
  const payload: Omit<WebMigrationPackage, 'payloadSha256'> = {
    formatVersion: pkg.formatVersion,
    operationId: pkg.operationId,
    sourceIdentity: pkg.sourceIdentity,
    targetIdentity: pkg.targetIdentity,
    sourceSchemaVersion: pkg.sourceSchemaVersion,
    createdAt: pkg.createdAt,
    localStorage: pkg.localStorage,
    meetings: pkg.meetings,
    transcripts: pkg.transcripts,
  };
  if (await sha256Hex(payloadWithoutChecksum(payload)) !== pkg.payloadSha256.toLowerCase()) {
    throw new WebMigrationError('checksum_mismatch', 'migration package checksum mismatch');
  }
  return pkg;
}

export async function importWebMigrationPackage(
  pkg: unknown,
  sink: WebMigrationImportSink,
): Promise<WebMigrationImportResult> {
  const verified = await verifyWebMigrationPackage(pkg);
  const identity: WebMigrationOperationIdentity = {
    operationId: verified.operationId,
    sourceIdentity: verified.sourceIdentity,
    targetIdentity: verified.targetIdentity,
    payloadSha256: verified.payloadSha256,
  };
  if (await sink.hasImportedOperation(identity)) return 'already_imported';
  const commitResult = await sink.commit({
    localStorage: verified.localStorage.map((entry) => ({ ...entry })),
    meetings: verified.meetings.map((meeting) => ({ ...meeting })),
    transcripts: verified.transcripts.map((transcript) => ({ ...transcript })),
  }, identity);
  return commitResult === 'already_imported' ? 'already_imported' : 'imported';
}
