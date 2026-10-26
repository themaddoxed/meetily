/**
 * IndexedDB Service for Transcript Recovery
 * Provides browser-based persistence for meeting transcripts and metadata
 * to enable recovery after app crashes or unexpected closures.
 */

import { TranscriptUpdate } from '@/types';
import type { WebMigrationOperationIdentity } from '@/services/webMigration';

// Database schema interfaces
export interface MeetingMetadata {
  meetingId: string;          // Primary key: "meeting-{timestamp}"
  title: string;              // Meeting title
  startTime: number;          // Unix timestamp (ms)
  lastUpdated: number;        // Unix timestamp (ms)
  transcriptCount: number;    // Number of transcript segments
  savedToSQLite: boolean;     // Flag: saved to backend DB
  sqliteMeetingId?: string;   // Durable SQLite ID while audio recovery is still pending
  folderPath?: string;        // Path to recording folder
}

export interface StoredTranscript {
  id?: number;                // Auto-increment primary key
  meetingId: string;          // Foreign key to meetings store
  text: string;               // Transcript text
  timestamp: string;          // ISO 8601 timestamp
  confidence: number;         // Whisper confidence score
  sequenceId: number;         // Sequence number for ordering
  storedAt: number;           // Unix timestamp when saved
  audioStartTime?: number;    // Recording-relative start time in seconds
  audioEndTime?: number;      // Recording-relative end time in seconds
  duration?: number;          // Duration in seconds
  chunkStartTime?: number;    // Legacy native transcript ordering field
  isPartial?: boolean;        // Whether the native update was partial
}

export interface IndexedDBMigrationSnapshot {
  meetings: MeetingMetadata[];
  transcripts: StoredTranscript[];
}

export type MigrationOperationCommitResult = 'imported' | 'already_imported';

type MigrationOperationMarker = WebMigrationOperationIdentity & {
  importedAt: number;
};

type StoredTranscriptRow = Partial<StoredTranscript> & {
  sequence_id?: unknown;
  audio_start_time?: unknown;
  audio_end_time?: unknown;
  chunk_start_time?: unknown;
  is_partial?: unknown;
};

function readFiniteNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function normalizeStoredTranscript(value: unknown): StoredTranscript | null {
  if (!value || typeof value !== 'object') return null;

  const row = value as StoredTranscriptRow;
  const sequenceId = readFiniteNumber(row.sequenceId) ?? readFiniteNumber(row.sequence_id);
  const confidence = readFiniteNumber(row.confidence);
  const storedAt = readFiniteNumber(row.storedAt);

  if (
    typeof row.meetingId !== 'string' ||
    typeof row.text !== 'string' ||
    typeof row.timestamp !== 'string' ||
    sequenceId === undefined ||
    confidence === undefined ||
    storedAt === undefined
  ) {
    return null;
  }

  const id = readFiniteNumber(row.id);
  const audioStartTime =
    readFiniteNumber(row.audioStartTime) ?? readFiniteNumber(row.audio_start_time);
  const audioEndTime =
    readFiniteNumber(row.audioEndTime) ?? readFiniteNumber(row.audio_end_time);
  const duration = readFiniteNumber(row.duration);
  const chunkStartTime =
    readFiniteNumber(row.chunkStartTime) ?? readFiniteNumber(row.chunk_start_time);
  const isPartial = typeof row.isPartial === 'boolean'
    ? row.isPartial
    : typeof row.is_partial === 'boolean'
      ? row.is_partial
      : undefined;

  return {
    ...(id === undefined ? {} : { id }),
    meetingId: row.meetingId,
    text: row.text,
    timestamp: row.timestamp,
    confidence,
    sequenceId,
    storedAt,
    ...(audioStartTime === undefined ? {} : { audioStartTime }),
    ...(audioEndTime === undefined ? {} : { audioEndTime }),
    ...(duration === undefined ? {} : { duration }),
    ...(chunkStartTime === undefined ? {} : { chunkStartTime }),
    ...(isPartial === undefined ? {} : { isPartial }),
  };
}

function migrationMarkerMatches(
  value: unknown,
  identity: WebMigrationOperationIdentity,
): value is MigrationOperationMarker {
  if (!value || typeof value !== 'object') return false;
  const marker = value as Partial<MigrationOperationMarker>;
  return marker.operationId === identity.operationId
    && marker.sourceIdentity === identity.sourceIdentity
    && marker.targetIdentity === identity.targetIdentity
    && marker.payloadSha256 === identity.payloadSha256
    && typeof marker.importedAt === 'number'
    && Number.isFinite(marker.importedAt);
}

function isValidMeetingMetadata(value: unknown): value is MeetingMetadata {
  if (!value || typeof value !== 'object') return false;

  const meeting = value as Partial<MeetingMetadata>;
  return (
    typeof meeting.meetingId === 'string' &&
    meeting.meetingId.length > 0 &&
    typeof meeting.title === 'string' &&
    typeof meeting.startTime === 'number' &&
    Number.isFinite(meeting.startTime) &&
    typeof meeting.lastUpdated === 'number' &&
    Number.isFinite(meeting.lastUpdated) &&
    typeof meeting.transcriptCount === 'number' &&
    Number.isInteger(meeting.transcriptCount) &&
    meeting.transcriptCount >= 0 &&
    typeof meeting.savedToSQLite === 'boolean' &&
    (meeting.sqliteMeetingId === undefined ||
      (typeof meeting.sqliteMeetingId === 'string' && meeting.sqliteMeetingId.length > 0)) &&
    (meeting.folderPath === undefined || typeof meeting.folderPath === 'string')
  );
}

class IndexedDBService {
  private db: IDBDatabase | null = null;
  private readonly DB_NAME = 'MeetilyRecoveryDB';
  private readonly DB_VERSION = 3;
  private initPromise: Promise<void> | null = null;

  /**
   * Initialize database connection
   */
  async init(): Promise<void> {
    // Return existing promise if initialization is in progress
    if (this.initPromise) {
      return this.initPromise;
    }

    // Return immediately if already initialized
    if (this.db) {
      return Promise.resolve();
    }

    this.initPromise = new Promise((resolve, reject) => {
      try {
        const request = indexedDB.open(this.DB_NAME, this.DB_VERSION);

        request.onerror = () => {
          console.error('Failed to open IndexedDB:', request.error);
          reject(request.error);
        };

        request.onsuccess = () => {
          this.db = request.result;
          resolve();
        };

        request.onupgradeneeded = (event) => {
          const db = (event.target as IDBOpenDBRequest).result;

          // Create meetings store
          if (!db.objectStoreNames.contains('meetings')) {
            const meetingsStore = db.createObjectStore('meetings', { keyPath: 'meetingId' });
            meetingsStore.createIndex('lastUpdated', 'lastUpdated', { unique: false });
            meetingsStore.createIndex('savedToSQLite', 'savedToSQLite', { unique: false });
          }

          // Create transcripts store
          if (!db.objectStoreNames.contains('transcripts')) {
            const transcriptsStore = db.createObjectStore('transcripts', {
              keyPath: 'id',
              autoIncrement: true
            });
            transcriptsStore.createIndex('meetingId', 'meetingId', { unique: false });
            transcriptsStore.createIndex('storedAt', 'storedAt', { unique: false });
          }

          if (!db.objectStoreNames.contains('migrationOperations')) {
            db.createObjectStore('migrationOperations', { keyPath: 'operationId' });
          }
        };
      } catch (error) {
        console.error('Exception during IndexedDB initialization:', error);
        reject(error);
      }
    });

    return this.initPromise;
  }

  // Meeting operations

  /**
   * Save or update meeting metadata
   */
  async saveMeetingMetadata(metadata: MeetingMetadata): Promise<void> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['meetings'], 'readwrite');
      const store = transaction.objectStore('meetings');

      await new Promise<void>((resolve, reject) => {
        const request = store.put(metadata);
        request.onsuccess = () => resolve();
        request.onerror = () => reject(request.error);
      });
    } catch (error) {
      console.warn('Failed to save meeting metadata to IndexedDB:', error);
      // Fail silently - don't interrupt recording
    }
  }

  /**
   * Get meeting metadata by ID
   */
  async getMeetingMetadata(meetingId: string): Promise<MeetingMetadata | null> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['meetings'], 'readonly');
      const store = transaction.objectStore('meetings');

      return new Promise((resolve, reject) => {
        const request = store.get(meetingId);
        request.onsuccess = () => resolve(request.result || null);
        request.onerror = () => reject(request.error);
      });
    } catch (error) {
      console.error('Failed to get meeting metadata from IndexedDB:', error);
      return null;
    }
  }

  /**
   * Get all unsaved meetings (savedToSQLite = false)
   */
  async getAllMeetings(): Promise<MeetingMetadata[]> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['meetings'], 'readonly');
      const store = transaction.objectStore('meetings');

      return new Promise((resolve, reject) => {
        const request = store.getAll();
        request.onsuccess = () => {
          const rows = request.result as unknown[];
          const allMeetings = rows.filter(isValidMeetingMetadata);
          if (allMeetings.length !== rows.length) {
            console.warn(
              `Skipped ${rows.length - allMeetings.length} malformed IndexedDB recovery row(s)`,
            );
          }
          // Filter for unsaved meetings (savedToSQLite = false)
          const unsavedMeetings = allMeetings.filter(m => m.savedToSQLite === false);

          // Sort by most recent first
          unsavedMeetings.sort((a, b) => b.lastUpdated - a.lastUpdated);
          resolve(unsavedMeetings);
        };
        request.onerror = () => reject(request.error);
      });
    } catch (error) {
      console.error('Failed to get meetings from IndexedDB:', error);
      return [];
    }
  }

  /**
   * Read every valid meeting row for a bundle-ID migration. Unlike recovery
   * listing, this deliberately includes meetings already saved to SQLite.
   */
  async getAllMeetingsForMigration(): Promise<MeetingMetadata[]> {
    if (!this.db) await this.init();

    const transaction = this.db!.transaction(['meetings'], 'readonly');
    const store = transaction.objectStore('meetings');
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => {
        const rows = request.result as unknown[];
        const meetings = rows.filter(isValidMeetingMetadata);
        if (meetings.length !== rows.length) {
          reject(new Error(
            `Found ${rows.length - meetings.length} malformed IndexedDB meeting row(s)`,
          ));
          return;
        }
        meetings.sort((left, right) => left.meetingId.localeCompare(right.meetingId));
        resolve(meetings);
      };
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Read meetings and transcripts from one readonly transaction so the
   * browser package cannot combine rows from two different database states.
   */
  async getMigrationSnapshot(): Promise<IndexedDBMigrationSnapshot> {
    if (!this.db) await this.init();

    const transaction = this.db!.transaction(['meetings', 'transcripts'], 'readonly');
    const meetingsStore = transaction.objectStore('meetings');
    const transcriptsStore = transaction.objectStore('transcripts');

    return new Promise((resolve, reject) => {
      let requestFailure: unknown;
      let meetings: MeetingMetadata[] | undefined;
      let transcripts: StoredTranscript[] | undefined;

      const fail = (error: unknown) => {
        requestFailure = error;
        try {
          transaction.abort();
        } catch {
          // The transaction may already be aborting after a request error.
        }
      };

      transaction.oncomplete = () => {
        if (!meetings || !transcripts) {
          reject(new Error('Migration snapshot transaction completed without both stores'));
          return;
        }
        resolve({ meetings, transcripts });
      };
      transaction.onabort = () => reject(
        requestFailure ?? transaction.error ?? new Error('Migration snapshot was aborted'),
      );
      transaction.onerror = () => reject(
        requestFailure ?? transaction.error ?? new Error('Migration snapshot failed'),
      );

      const meetingsRequest = meetingsStore.getAll();
      meetingsRequest.onsuccess = () => {
        const rows = meetingsRequest.result as unknown[];
        const validMeetings = rows.filter(isValidMeetingMetadata);
        if (validMeetings.length !== rows.length) {
          fail(new Error(
            `Found ${rows.length - validMeetings.length} malformed IndexedDB meeting row(s)`,
          ));
          return;
        }
        validMeetings.sort((left, right) => left.meetingId.localeCompare(right.meetingId));
        meetings = validMeetings;
      };
      meetingsRequest.onerror = () => fail(
        meetingsRequest.error ?? new Error('Failed to read IndexedDB meetings'),
      );

      const transcriptsRequest = transcriptsStore.getAll();
      transcriptsRequest.onsuccess = () => {
        const rows = transcriptsRequest.result as unknown[];
        const validTranscripts = rows
          .map(normalizeStoredTranscript)
          .filter((row): row is StoredTranscript => row !== null);
        if (validTranscripts.length !== rows.length) {
          fail(new Error(
            `Found ${rows.length - validTranscripts.length} malformed IndexedDB transcript row(s)`,
          ));
          return;
        }
        validTranscripts.sort((left, right) => {
          const meetingOrder = left.meetingId.localeCompare(right.meetingId);
          if (meetingOrder !== 0) return meetingOrder;
          const sequenceOrder = left.sequenceId - right.sequenceId;
          if (sequenceOrder !== 0) return sequenceOrder;
          return (left.id ?? Number.MAX_SAFE_INTEGER) - (right.id ?? Number.MAX_SAFE_INTEGER);
        });
        transcripts = validTranscripts;
      };
      transcriptsRequest.onerror = () => fail(
        transcriptsRequest.error ?? new Error('Failed to read IndexedDB transcripts'),
      );
    });
  }

  /**
   * Mark meeting as saved to SQLite
   */
  async markMeetingSaved(meetingId: string): Promise<void> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['meetings'], 'readwrite');
      const store = transaction.objectStore('meetings');

      await new Promise<void>((resolve, reject) => {
        let requestFailure: unknown;
        const transactionFailure = (fallback: string) =>
          transaction.error ?? requestFailure ?? new Error(fallback);
        const abortWith = (error: unknown) => {
          requestFailure = error;
          reject(error);
          try {
            transaction.abort();
          } catch {
            // The transaction may already be aborting after a request error.
          }
        };

        transaction.oncomplete = () => resolve();
        transaction.onabort = () => {
          reject(transactionFailure('Mark-saved transaction was aborted'));
        };
        transaction.onerror = () => {
          reject(transactionFailure('Mark-saved transaction failed'));
        };

        const getRequest = store.get(meetingId);
        getRequest.onsuccess = () => {
          const meeting = getRequest.result as MeetingMetadata | undefined;
          if (meeting) {
            meeting.savedToSQLite = true;
            meeting.lastUpdated = Date.now();
            const putRequest = store.put(meeting);
            putRequest.onerror = () => {
              abortWith(putRequest.error ?? new Error('Failed to mark meeting as saved'));
            };
          } else {
            abortWith(new Error(`Recovery metadata not found for meeting ${meetingId}`));
          }
        };
        getRequest.onerror = () => {
          abortWith(getRequest.error ?? new Error('Failed to read recovery metadata'));
        };
      });
    } catch (error) {
      console.error('Failed to mark meeting as saved:', error);
      throw error;
    }
  }

  /**
   * Atomically return the existing SQLite ID or persist the proposed one.
   * The ID is allocated before the SQLite save so retries can be idempotent.
   */
  async getOrCreateSQLiteMeetingId(meetingId: string, proposedId: string): Promise<string> {
    try {
      if (!this.db) await this.init();
      if (!proposedId) throw new Error('Proposed SQLite meeting ID cannot be empty');

      const transaction = this.db!.transaction(['meetings'], 'readwrite');
      const store = transaction.objectStore('meetings');

      return await new Promise<string>((resolve, reject) => {
        let selectedId: string | undefined;
        let requestFailure: unknown;

        const transactionFailure = (fallback: string) =>
          transaction.error ?? requestFailure ?? new Error(fallback);
        const abortWith = (error: unknown) => {
          requestFailure = error;
          reject(error);
          try {
            transaction.abort();
          } catch {
            // The transaction may already be aborting after a request error.
          }
        };

        transaction.oncomplete = () => {
          if (selectedId) {
            resolve(selectedId);
          } else {
            reject(new Error('SQLite meeting ID transaction completed without an ID'));
          }
        };
        transaction.onabort = () => {
          reject(transactionFailure('SQLite meeting ID transaction was aborted'));
        };
        transaction.onerror = () => {
          reject(transactionFailure('SQLite meeting ID transaction failed'));
        };

        const getRequest = store.get(meetingId);
        getRequest.onsuccess = () => {
          const meeting = getRequest.result as MeetingMetadata | undefined;
          if (!meeting) {
            abortWith(new Error(`Recovery metadata not found for meeting ${meetingId}`));
            return;
          }

          if (meeting.sqliteMeetingId) {
            selectedId = meeting.sqliteMeetingId;
            return;
          }

          meeting.sqliteMeetingId = proposedId;
          meeting.lastUpdated = Date.now();
          const putRequest = store.put(meeting);
          putRequest.onsuccess = () => {
            selectedId = proposedId;
          };
          putRequest.onerror = () => {
            abortWith(putRequest.error ?? new Error('Failed to persist SQLite meeting ID'));
          };
        };
        getRequest.onerror = () => {
          abortWith(getRequest.error ?? new Error('Failed to read recovery metadata'));
        };
      });
    } catch (error) {
      console.error('Failed to allocate SQLite meeting ID:', error);
      throw error;
    }
  }

  /**
   * Delete meeting and all its transcripts
   */
  async deleteMeeting(meetingId: string): Promise<void> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['meetings', 'transcripts'], 'readwrite');
      const meetingsStore = transaction.objectStore('meetings');
      const transcriptsStore = transaction.objectStore('transcripts');

      // Delete transcripts
      await this.deleteTranscriptsForMeetingInternal(transcriptsStore, meetingId);

      // Delete meeting
      await new Promise<void>((resolve, reject) => {
        const request = meetingsStore.delete(meetingId);
        request.onsuccess = () => resolve();
        request.onerror = () => reject(request.error);
      });
    } catch (error) {
      console.error('Failed to delete meeting from IndexedDB:', error);
      throw error;
    }
  }

  // Transcript operations

  /**
   * Save a transcript segment
   */
  async saveTranscript(meetingId: string, transcript: TranscriptUpdate): Promise<void> {
    try {
      if (!this.db) await this.init();

      const storedTranscript: Omit<StoredTranscript, 'id'> = {
        meetingId,
        text: transcript.text,
        timestamp: transcript.timestamp,
        confidence: transcript.confidence,
        sequenceId: transcript.sequence_id,
        storedAt: Date.now(),
        audioStartTime: transcript.audio_start_time,
        audioEndTime: transcript.audio_end_time,
        duration: transcript.duration,
        chunkStartTime: transcript.chunk_start_time,
        isPartial: transcript.is_partial,
      };

      const transaction = this.db!.transaction(['transcripts', 'meetings'], 'readwrite');
      const transcriptsStore = transaction.objectStore('transcripts');
      const meetingsStore = transaction.objectStore('meetings');

      // Save transcript
      await new Promise<void>((resolve, reject) => {
        const request = transcriptsStore.add(storedTranscript);
        request.onsuccess = () => resolve();
        request.onerror = () => reject(request.error);
      });

      // Update meeting metadata
      const meeting = await new Promise<MeetingMetadata | null>((resolve, reject) => {
        const request = meetingsStore.get(meetingId);
        request.onsuccess = () => resolve(request.result || null);
        request.onerror = () => reject(request.error);
      });

      if (meeting) {
        meeting.lastUpdated = Date.now();
        meeting.transcriptCount += 1;
        await new Promise<void>((resolve, reject) => {
          const request = meetingsStore.put(meeting);
          request.onsuccess = () => resolve();
          request.onerror = () => reject(request.error);
        });
      }
    } catch (error) {
      console.warn('Failed to save transcript to IndexedDB:', error);
      // Fail silently - don't interrupt recording
    }
  }

  /**
   * Get all transcripts for a meeting
   */
  async getTranscripts(meetingId: string): Promise<StoredTranscript[]> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['transcripts'], 'readonly');
      const store = transaction.objectStore('transcripts');
      const index = store.index('meetingId');

      return await new Promise((resolve, reject) => {
        const request = index.getAll(meetingId);
        request.onsuccess = () => {
          const rows = request.result as unknown[];
          const transcripts = rows
            .map(normalizeStoredTranscript)
            .filter((row): row is StoredTranscript => row !== null);
          if (transcripts.length !== rows.length) {
            reject(new Error(
              `Found ${rows.length - transcripts.length} malformed IndexedDB transcript row(s)`,
            ));
            return;
          }
          // Sort by sequence ID
          transcripts.sort((a, b) => a.sequenceId - b.sequenceId);
          resolve(transcripts);
        };
        request.onerror = () => reject(request.error);
      });
    } catch (error) {
      console.error('Failed to get transcripts from IndexedDB:', error);
      throw error;
    }
  }

  /**
   * Read every transcript row for a bundle-ID migration, preserving optional
   * timing and partial-segment fields after the same validation used by
   * recovery reads.
   */
  async getAllTranscriptsForMigration(): Promise<StoredTranscript[]> {
    if (!this.db) await this.init();

    const transaction = this.db!.transaction(['transcripts'], 'readonly');
    const store = transaction.objectStore('transcripts');
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => {
        const rows = request.result as unknown[];
        const transcripts = rows
          .map(normalizeStoredTranscript)
          .filter((row): row is StoredTranscript => row !== null);
        if (transcripts.length !== rows.length) {
          reject(new Error(
            `Found ${rows.length - transcripts.length} malformed IndexedDB transcript row(s)`,
          ));
          return;
        }
        transcripts.sort((left, right) => {
          const meetingOrder = left.meetingId.localeCompare(right.meetingId);
          if (meetingOrder !== 0) return meetingOrder;
          const sequenceOrder = left.sequenceId - right.sequenceId;
          if (sequenceOrder !== 0) return sequenceOrder;
          return (left.id ?? Number.MAX_SAFE_INTEGER) - (right.id ?? Number.MAX_SAFE_INTEGER);
        });
        resolve(transcripts);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async hasImportedMigrationOperation(identity: WebMigrationOperationIdentity): Promise<boolean> {
    if (!this.db) await this.init();

    const transaction = this.db!.transaction(['migrationOperations'], 'readonly');
    const store = transaction.objectStore('migrationOperations');
    return new Promise((resolve, reject) => {
      const request = store.get(identity.operationId);
      request.onsuccess = () => {
        if (!request.result) {
          resolve(false);
          return;
        }
        if (!migrationMarkerMatches(request.result, identity)) {
          reject(new Error(
            'Migration operation ID is already bound to a different verified package',
          ));
          return;
        }
        resolve(true);
      };
      request.onerror = () => reject(request.error);
    });
  }

  /**
   * Add a migration snapshot and its durable operation marker in one
   * read-write transaction. `add`, rather than `put`, makes a destination
   * collision abort the whole import instead of overwriting user data.
   */
  async commitMigrationSnapshot(
    snapshot: IndexedDBMigrationSnapshot,
    identity: WebMigrationOperationIdentity,
  ): Promise<MigrationOperationCommitResult> {
    if (!this.db) await this.init();

    const transaction = this.db!.transaction(
      ['meetings', 'transcripts', 'migrationOperations'],
      'readwrite',
    );
    const meetingsStore = transaction.objectStore('meetings');
    const transcriptsStore = transaction.objectStore('transcripts');
    const operationsStore = transaction.objectStore('migrationOperations');

    return new Promise((resolve, reject) => {
      let requestFailure: unknown;
      let result: MigrationOperationCommitResult = 'imported';
      let writesQueued = false;

      const fail = (error: unknown) => {
        requestFailure = error;
        try {
          transaction.abort();
        } catch {
          // The transaction may already be aborting after a request error.
        }
      };

      transaction.oncomplete = () => resolve(result);
      transaction.onabort = () => reject(
        requestFailure ?? transaction.error ?? new Error('Migration transaction was aborted'),
      );
      transaction.onerror = () => reject(
        requestFailure ?? transaction.error ?? new Error('Migration transaction failed'),
      );

      const markerRequest = operationsStore.get(identity.operationId);
      markerRequest.onsuccess = () => {
        if (markerRequest.result) {
          if (!migrationMarkerMatches(markerRequest.result, identity)) {
            fail(new Error(
              'Migration operation ID is already bound to a different verified package',
            ));
            return;
          }
          result = 'already_imported';
          return;
        }
        if (writesQueued) return;
        writesQueued = true;

        for (const meeting of snapshot.meetings) {
          const request = meetingsStore.add({ ...meeting });
          request.onerror = () => fail(request.error ?? new Error('Failed to import meeting'));
        }
        for (const transcript of snapshot.transcripts) {
          const request = transcriptsStore.add({ ...transcript });
          request.onerror = () => fail(request.error ?? new Error('Failed to import transcript'));
        }
        const operationRequest = operationsStore.add({
          ...identity,
          importedAt: Date.now(),
        });
        operationRequest.onerror = () => {
          fail(operationRequest.error ?? new Error('Failed to persist migration marker'));
        };
      };
      markerRequest.onerror = () => fail(markerRequest.error ?? new Error('Failed to read migration marker'));
    });
  }

  /**
   * Get transcript count for a meeting
   */
  async getTranscriptCount(meetingId: string): Promise<number> {
    try {
      if (!this.db) await this.init();

      const transaction = this.db!.transaction(['transcripts'], 'readonly');
      const store = transaction.objectStore('transcripts');
      const index = store.index('meetingId');

      return new Promise((resolve, reject) => {
        const request = index.count(meetingId);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });
    } catch (error) {
      console.error('Failed to get transcript count from IndexedDB:', error);
      return 0;
    }
  }

  // Cleanup operations

  /**
   * Delete meetings older than specified days
   * @param daysOld Number of days threshold
   * @returns Number of meetings deleted
   */
  async deleteOldMeetings(daysOld: number): Promise<number> {
    try {
      if (!this.db) await this.init();

      const cutoffTime = Date.now() - (daysOld * 24 * 60 * 60 * 1000);
      const transaction = this.db!.transaction(['meetings', 'transcripts'], 'readwrite');
      const meetingsStore = transaction.objectStore('meetings');
      const transcriptsStore = transaction.objectStore('transcripts');

      // Get all meetings
      const allMeetings = await new Promise<MeetingMetadata[]>((resolve, reject) => {
        const request = meetingsStore.getAll();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });

      let deletedCount = 0;

      for (const meeting of allMeetings) {
        if (meeting.lastUpdated < cutoffTime) {
          // Delete transcripts
          await this.deleteTranscriptsForMeetingInternal(transcriptsStore, meeting.meetingId);

          // Delete meeting
          await new Promise<void>((resolve, reject) => {
            const request = meetingsStore.delete(meeting.meetingId);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
          });

          deletedCount++;
        }
      }

      console.log(`Cleaned up ${deletedCount} old meetings`);
      return deletedCount;
    } catch (error) {
      console.error('Failed to delete old meetings:', error);
      return 0;
    }
  }

  /**
   * Delete saved meetings older than specified hours
   * @param hoursOld Number of hours threshold after save
   * @returns Number of meetings deleted
   */
  async deleteSavedMeetings(hoursOld: number): Promise<number> {
    try {
      if (!this.db) await this.init();

      const cutoffTime = Date.now() - (hoursOld * 60 * 60 * 1000);
      const transaction = this.db!.transaction(['meetings', 'transcripts'], 'readwrite');
      const meetingsStore = transaction.objectStore('meetings');
      const transcriptsStore = transaction.objectStore('transcripts');

      // Get all meetings and filter for saved ones
      const allMeetings = await new Promise<MeetingMetadata[]>((resolve, reject) => {
        const request = meetingsStore.getAll();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });

      // Filter for saved meetings (savedToSQLite = true)
      const savedMeetings = allMeetings.filter(m => m.savedToSQLite === true);

      let deletedCount = 0;

      for (const meeting of savedMeetings) {
        if (meeting.lastUpdated < cutoffTime) {
          // Delete transcripts
          await this.deleteTranscriptsForMeetingInternal(transcriptsStore, meeting.meetingId);

          // Delete meeting
          await new Promise<void>((resolve, reject) => {
            const request = meetingsStore.delete(meeting.meetingId);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
          });

          deletedCount++;
        }
      }

      console.log(`Cleaned up ${deletedCount} saved meetings`);
      return deletedCount;
    } catch (error) {
      console.error('Failed to delete saved meetings:', error);
      return 0;
    }
  }

  /**
   * Helper to delete all transcripts for a meeting
   */
  private async deleteTranscriptsForMeetingInternal(
    transcriptsStore: IDBObjectStore,
    meetingId: string
  ): Promise<void> {
    const index = transcriptsStore.index('meetingId');

    return new Promise((resolve, reject) => {
      const request = index.openCursor(IDBKeyRange.only(meetingId));

      request.onsuccess = (event) => {
        const cursor = (event.target as IDBRequest<IDBCursorWithValue>).result;
        if (cursor) {
          cursor.delete();
          cursor.continue();
        } else {
          resolve();
        }
      };

      request.onerror = () => reject(request.error);
    });
  }
}

// Export singleton instance
export const indexedDBService = new IndexedDBService();
