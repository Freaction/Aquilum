import { invoke } from '@tauri-apps/api/core';
import { comparablePath } from '../paths';

const REPLICA_PREFIX = 'aquilum-sync-';
const UPDATES_STORE = 'updates';
const RECORDS_DATABASE = 'aquilum-file-sync';
const RECORDS_STORE = 'records';

interface LegacyPoint {
  fileHash: string;
  textHash: string;
}

interface LegacyReplica {
  path: string;
  updates: number[][];
  point: LegacyPoint | null;
}

let importing: Promise<void> | null = null;

function openExisting(name: string): Promise<IDBDatabase | null> {
  return new Promise((resolve) => {
    const request = indexedDB.open(name);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => resolve(null);
    request.onblocked = () => resolve(null);
  });
}

function readAll<T>(database: IDBDatabase, store: string, read: (store: IDBObjectStore) => IDBRequest<T>): Promise<T | null> {
  if (!database.objectStoreNames.contains(store)) return Promise.resolve(null);
  return new Promise((resolve) => {
    const request = read(database.transaction(store, 'readonly').objectStore(store));
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => resolve(null);
  });
}

async function readReplica(name: string, records: IDBDatabase | null): Promise<LegacyReplica | null> {
  const database = await openExisting(name);
  if (!database) return null;
  try {
    const updates = await readAll<Uint8Array[]>(database, UPDATES_STORE, (store) => store.getAll());
    if (!updates || updates.length === 0) return null;
    const path = name.slice(REPLICA_PREFIX.length);
    const point = records
      ? await readAll<LegacyPoint | undefined>(records, RECORDS_STORE, (store) => store.get(comparablePath(path)))
      : null;
    return { path, updates: updates.map((update) => Array.from(update)), point: point ?? null };
  } finally {
    database.close();
  }
}

function deleteDatabase(name: string): Promise<void> {
  return new Promise((resolve) => {
    const request = indexedDB.deleteDatabase(name);
    request.onsuccess = () => resolve();
    request.onerror = () => {
      console.error(`Failed to delete the old local copy ${name}`, request.error);
      resolve();
    };
    request.onblocked = () => resolve();
  });
}

async function importNow(): Promise<void> {
  if (typeof indexedDB === 'undefined' || !indexedDB.databases) return;
  const names = (await indexedDB.databases()).map((info) => info.name ?? '');
  const legacyNames = names.filter((name) => name.startsWith(REPLICA_PREFIX) || name === RECORDS_DATABASE);
  if (legacyNames.length === 0) return;
  const records = names.includes(RECORDS_DATABASE) ? await openExisting(RECORDS_DATABASE) : null;
  try {
    const replicaNames = legacyNames.filter((name) => name.startsWith(REPLICA_PREFIX));
    const replicas = (await Promise.all(replicaNames.map((name) => readReplica(name, records))))
      .filter((replica): replica is LegacyReplica => replica !== null);
    if (replicas.length > 0) await invoke<number>('document_import_legacy', { replicas });
  } finally {
    records?.close();
  }
  await Promise.all(legacyNames.map(deleteDatabase));
}

export function legacyReplicasImported(): Promise<void> {
  if (!importing) {
    importing = importNow().catch((error: unknown) => {
      console.error('Failed to move local copies from the previous version', error);
    });
  }
  return importing;
}
