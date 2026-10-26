import { describe, expect, it } from 'vitest';

import {
  collectAllowedLocalStorage,
  createIndexedDBMigrationSink,
  createWebMigrationPackage,
  createWebMigrationPackageFromIndexedDB,
  importWebMigrationPackage,
  validateWebMigrationPackage,
  verifyWebMigrationPackage,
  WebMigrationError,
} from '@/services/webMigration';

const operationId = '11111111-1111-4111-8111-111111111111';

const meeting = {
  meetingId: 'meeting-1',
  title: 'Migration test',
  startTime: 1,
  lastUpdated: 2,
  transcriptCount: 1,
  savedToSQLite: false,
} as const;

const transcript = {
  id: 4,
  meetingId: 'meeting-1',
  text: 'Keep this row',
  timestamp: '2026-09-21T00:00:00.000Z',
  confidence: 0.9,
  sequenceId: 1,
  storedAt: 2,
} as const;

function snapshot() {
  return {
    operationId,
    sourceIdentity: 'com.legacy.meetily',
    targetIdentity: 'com.new.meetily',
    createdAt: '2026-09-21T00:00:00.000Z',
    localStorage: {
      themeMode: 'dark',
      'summaryLanguageFallback:meeting-1': 'ru',
      'unknown-private-key': 'must stay out',
    },
    meetings: [meeting],
    transcripts: [transcript],
  };
}

function storageFrom(values: Record<string, string | null>) {
  const entries = new Map(Object.entries(values));
  return {
    get length() { return entries.size; },
    key(index: number) { return [...entries.keys()][index] ?? null; },
    getItem(key: string) { return entries.get(key) ?? null; },
    setItem(key: string, value: string) { entries.set(key, value); },
    removeItem(key: string) { entries.delete(key); },
    values: entries,
  };
}

describe('web migration package', () => {
  it('collects only explicitly allowlisted localStorage keys and prefixes', () => {
    const values = new Map([
      ['themeMode', 'dark'],
      ['summaryLanguageFallback:meeting-1', 'ru'],
      ['meetily.selected-summary-template', 'custom-template'],
      ['unknown-private-key', 'secret'],
    ]);
    const storage = {
      get length() { return values.size; },
      key(index: number) { return [...values.keys()][index] ?? null; },
      getItem(key: string) { return values.get(key) ?? null; },
    };

    expect(collectAllowedLocalStorage(storage)).toEqual([
      'summaryLanguageFallback:meeting-1',
      'themeMode',
    ]);
  });

  it('creates a deterministic package with per-entry and payload checksums', async () => {
    const first = await createWebMigrationPackage(snapshot());
    const second = await createWebMigrationPackage(snapshot());

    expect(first).toEqual(second);
    expect(first.localStorage.map((entry) => entry.key)).toEqual([
      'summaryLanguageFallback:meeting-1',
      'themeMode',
    ]);
    expect(first.payloadSha256).toMatch(/^[0-9a-f]{64}$/);
    await expect(verifyWebMigrationPackage(first)).resolves.toEqual(first);
  });

  it('rejects a changed value before import', async () => {
    const pkg = await createWebMigrationPackage(snapshot());
    const changed = {
      ...pkg,
      localStorage: pkg.localStorage.map((entry) =>
        entry.key === 'themeMode' ? { ...entry, value: 'light' } : entry,
      ),
    };

    await expect(verifyWebMigrationPackage(changed)).rejects.toMatchObject({
      code: 'checksum_mismatch',
    } satisfies Partial<WebMigrationError>);
  });

  it('does not import the same operation twice', async () => {
    const pkg = await createWebMigrationPackage(snapshot());
    const imported = new Map<string, string>();
    const commits: unknown[] = [];
    const sink = {
      async hasImportedOperation(identity: { operationId: string; payloadSha256: string }) {
        const existing = imported.get(identity.operationId);
        if (existing && existing !== identity.payloadSha256) {
          throw new Error('Migration operation ID is already bound to a different verified package');
        }
        return Boolean(existing);
      },
      async commit(value: unknown, identity: { operationId: string; payloadSha256: string }) {
        commits.push(value);
        imported.set(identity.operationId, identity.payloadSha256);
      },
    };

    await expect(importWebMigrationPackage(pkg, sink)).resolves.toBe('imported');
    await expect(importWebMigrationPackage(pkg, sink)).resolves.toBe('already_imported');
    expect(commits).toHaveLength(1);

    const conflicting = await createWebMigrationPackage({
      ...snapshot(),
      targetIdentity: 'com.other.meetily',
    });
    await expect(importWebMigrationPackage(conflicting, sink))
      .rejects.toThrow('different verified package');
  });

  it('never exports a private localStorage key', async () => {
    const pkg = await createWebMigrationPackage({
      ...snapshot(),
      localStorage: { 'summaryLanguageFallback:': 'bad' },
    });
    expect(pkg.localStorage).toEqual([]);
  });

  it('rejects equal source and target identities', async () => {
    await expect(createWebMigrationPackage({
      ...snapshot(),
      targetIdentity: snapshot().sourceIdentity,
    })).rejects.toMatchObject({ code: 'invalid_package' });
  });

  it('validates package structure before checksum work', async () => {
    expect(() => validateWebMigrationPackage({})).toThrow(WebMigrationError);
  });

  it('rejects malformed optional transcript timing fields', async () => {
    const pkg = await createWebMigrationPackage(snapshot());
    const malformed = {
      ...pkg,
      transcripts: pkg.transcripts.map((row) => ({ ...row, audioStartTime: 'not-a-number' })),
    };

    await expect(verifyWebMigrationPackage(malformed)).rejects.toMatchObject({
      code: 'invalid_package',
    } satisfies Partial<WebMigrationError>);
  });

  it('rejects transcripts that do not belong to an exported meeting', async () => {
    await expect(createWebMigrationPackage({
      ...snapshot(),
      transcripts: [{ ...transcript, meetingId: 'missing-meeting' }],
    })).rejects.toMatchObject({ code: 'invalid_package' });
  });

  it('builds a package from the full IndexedDB source and localStorage allowlist', async () => {
    const storage = storageFrom({ themeMode: 'dark', 'private-key': 'omit' });
    const pkg = await createWebMigrationPackageFromIndexedDB({
      operationId,
      sourceIdentity: 'com.legacy.meetily',
      targetIdentity: 'com.new.meetily',
      createdAt: '2026-09-21T00:00:00.000Z',
      storage,
      source: {
        async getMigrationSnapshot() {
          return { meetings: [meeting], transcripts: [transcript] };
        },
      },
    });

    expect(pkg.meetings).toEqual([meeting]);
    expect(pkg.transcripts).toEqual([transcript]);
    expect(pkg.localStorage.map((entry) => entry.key)).toEqual(['themeMode']);
  });

  it('commits localStorage and IndexedDB through an idempotent sink', async () => {
    const storage = storageFrom({ themeMode: 'light' });
    const snapshots: unknown[] = [];
    const imported = new Map<string, string>();
    const sink = createIndexedDBMigrationSink(storage, {
      async hasImportedMigrationOperation(identity) {
        const existing = imported.get(identity.operationId);
        if (existing && existing !== identity.payloadSha256) {
          throw new Error('Migration operation ID is already bound to a different verified package');
        }
        return Boolean(existing);
      },
      async commitMigrationSnapshot(value, identity) {
        if (imported.has(identity.operationId)) return 'already_imported';
        snapshots.push(value);
        imported.set(identity.operationId, identity.payloadSha256);
        return 'imported';
      },
    });
    const pkg = await createWebMigrationPackage(snapshot());

    await expect(importWebMigrationPackage(pkg, sink)).resolves.toBe('imported');
    await expect(importWebMigrationPackage(pkg, sink)).resolves.toBe('already_imported');
    expect(storage.getItem('themeMode')).toBe('dark');
    expect(snapshots).toHaveLength(1);
  });

  it('rolls localStorage back when the IndexedDB commit fails', async () => {
    const storage = storageFrom({ themeMode: 'light' });
    const sink = createIndexedDBMigrationSink(storage, {
      async hasImportedMigrationOperation() { return false; },
      async commitMigrationSnapshot() { throw new Error('target unavailable'); },
    });
    const pkg = await createWebMigrationPackage(snapshot());

    await expect(importWebMigrationPackage(pkg, sink)).rejects.toThrow('target unavailable');
    expect(storage.getItem('themeMode')).toBe('light');
  });
});
