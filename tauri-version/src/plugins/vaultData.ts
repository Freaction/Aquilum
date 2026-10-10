import { useEffect, useMemo, useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTauriEvent } from '../hooks/useTauriEvent';
import { NOTES_RELOCATED_EVENT } from '../modules/documents/relocation';

export type ColorToken = 'red' | 'amber' | 'green' | 'teal' | 'blue' | 'purple' | 'pink' | 'gray';

export interface Filter {
  name: string;
  active: boolean;
  kind: 'path' | 'tag';
  target: 'files' | 'folders' | 'both';
  pattern: string;
  patternType: 'strict' | 'wildcard' | 'regex';
}

export interface VaultPluginData {
  version: number;
  colors: Record<string, ColorToken>;
  icons: Record<string, string>;
  recentIcons: string[];
  filters: { hide: Filter[]; pin: Filter[] };
}

// Mutators edit a draft and must be deterministic: pending edits are replayed after a failed save.
type Mutator = (data: VaultPluginData) => void;

function createStore(workspacePath: string) {
  let data: VaultPluginData | null = null;
  let confirmed: VaultPluginData | null = null;
  let queue = Promise.resolve();
  let loading: Promise<void> | null = null;
  const pending: Mutator[] = [];
  const listeners = new Set<() => void>();
  const publish = () => { for (const listener of listeners) listener(); };
  const replay = () => {
    data = confirmed ? structuredClone(confirmed) : null;
    if (data) for (const mutator of pending) mutator(data);
    publish();
  };

  const load = (refresh = false) => {
    if (loading && !refresh) return loading;
    // Reads follow pending writes so a relocation refresh cannot undo optimistic edits.
    const read = queue.catch(() => undefined).then(async () => {
      confirmed = await invoke<VaultPluginData>('plugin_vault_data_get', { workspacePath });
      replay();
    }).catch(error => {
      console.error('Failed to load plugin vault data:', error);
    }).finally(() => { if (loading === read) loading = null; });
    loading = read;
    queue = read;
    return read;
  };

  return {
    snapshot: () => data,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    load,
    update: (mutator: Mutator): Promise<void> => {
      if (!data) return Promise.reject(new Error('Plugin vault data is not loaded'));
      const next = structuredClone(data);
      mutator(next);
      pending.push(mutator);
      data = next;
      publish();
      const save = queue.catch(() => undefined).then(async () => {
        const next = structuredClone(confirmed!);
        mutator(next);
        await invoke('plugin_vault_data_set', { workspacePath, data: next });
        confirmed = next;
      }).finally(() => {
        pending.shift();
        replay();
      });
      queue = save;
      return save;
    },
  };
}

const stores = new Map<string, ReturnType<typeof createStore>>();
const emptyStore = {
  snapshot: () => null,
  subscribe: () => () => {},
  load: async () => {},
  update: async (_mutator: Mutator) => { throw new Error('No workspace is open'); },
};

export function reloadPluginVaultData(workspacePath: string): Promise<void> {
  let store = stores.get(workspacePath);
  if (!store) {
    store = createStore(workspacePath);
    stores.set(workspacePath, store);
  }
  return store.load(true);
}

export function usePluginVaultData(workspacePath: string | null) {
  const store = useMemo(() => {
    if (!workspacePath) return emptyStore;
    let store = stores.get(workspacePath);
    if (!store) {
      store = createStore(workspacePath);
      stores.set(workspacePath, store);
    }
    return store;
  }, [workspacePath]);
  const data = useSyncExternalStore(store.subscribe, store.snapshot);
  useEffect(() => { void store.load(); }, [store]);
  useTauriEvent(NOTES_RELOCATED_EVENT, () => { void store.load(true); }, !!workspacePath);
  return { data, update: store.update };
}
