import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTauriEvent } from '../../hooks/useTauriEvent';
import { t } from '../../i18n';
import type { PluginSettings } from '../../modules/settings';
import type { VaultPluginData } from '../vaultData';

export interface ExplorerOverview {
  counts: Record<string, number>;
  hidden: string[];
  pinned: string[];
  errors: { list: 'hide' | 'pin'; index: number; message: string }[];
}
const EMPTY: ExplorerOverview = { counts: {}, hidden: [], pinned: [], errors: [] };
export function useExplorerOverview(workspacePath: string | null, data: VaultPluginData | null, settings: PluginSettings) {
  const enabled = !!workspacePath && (settings.folderCounts.enabled || settings.explorerFilters.enabled);
  const [revision, setRevision] = useState(0);
  const [state, setState] = useState<{ root: string | null; overview: ExplorerOverview; error: string | null; loading: boolean }>({
    root: null, overview: EMPTY, error: null, loading: false,
  });
  const refresh = () => setRevision(value => value + 1);
  useTauriEvent('workspace-changed', refresh, enabled);
  useTauriEvent('links-changed', refresh, enabled);

  useEffect(() => {
    if (!enabled) return;
    let disposed = false;
    const timer = window.setTimeout(() => {
      setState(current => ({ ...current, root: workspacePath, loading: true, error: null,
        overview: current.root === workspacePath ? current.overview : EMPTY }));
      void invoke<ExplorerOverview>('explorer_overview', { workspacePath }).then(overview => {
        if (!disposed) setState({ root: workspacePath, overview, error: null, loading: false });
      }).catch(() => {
        if (!disposed) setState({ root: workspacePath, overview: EMPTY, error: t('plugins.explorerFilters.overviewError'), loading: false });
      });
    }, 150);
    return () => { disposed = true; window.clearTimeout(timer); };
  }, [workspacePath, data, settings.folderCounts, settings.explorerFilters, revision, enabled]);

  const current = enabled && state.root === workspacePath;
  const overview = current ? state.overview : EMPTY;
  return {
    overview: settings.explorerFilters.enabled ? overview : { ...overview, hidden: [], pinned: [], errors: [] },
    error: current ? state.error : null,
    loading: current && state.loading,
  };
}
