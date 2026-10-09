import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { FileText } from 'lucide';
import type { WorkspaceItem } from '../../modules/documents/fileGateway';
import { Button } from '../Common/Button';
import { DeleteNotesDialog } from '../Common/DeleteNotesDialog';
import { EmptyState } from '../Common/EmptyState';
import { FileTree } from './FileTree';
import { t } from '../../i18n';
import { SidebarFooter } from './SidebarFooter';
import { buildVisibleFileRows, unloadedExpandedFolders, type FileTreeActions } from './fileTreeModel';
import type { LinkDisposition } from '../../modules/links';
import { useExpandedFolders, useExplorerShowHidden } from '../../modules/workspace/uiPersist';
import { moveIntoFolder } from './moveIntoFolder';
import { useFileTreeDrag } from './useFileDragAndDrop';
import { useFileSelection } from './useFileSelection';
import { useFileTreeActions } from './useFileTreeActions';
import { useStableCallback } from '../../hooks/useStableCallback';
import './Sidebar.css';
// проводник.
import { useSettingsStore, DEFAULT_PLUGIN_SETTINGS } from '../../modules/settings';
import { usePluginVaultData, type VaultPluginData } from '../../plugins/vaultData';
import { useExplorerOverview } from '../../plugins/explorer/useExplorerOverview';
import { colorForPath } from '../../plugins/explorer/appearance';
import { ColorPopover } from '../../plugins/explorer/ColorPopover';
import { IconPickerDialog } from '../../plugins/explorer/IconPickerDialog';
import { relativePath } from '../../modules/paths';
import { GitStatusBar } from '../../plugins/git/GitStatusBar';

interface SidebarProps {
  activeFile: string | null;
  files: WorkspaceItem[];
  directories: ReadonlyMap<string, WorkspaceItem[]>;
  loadingDirectories: ReadonlySet<string>;
  workspacePath: string | null;
  workspaceName: string;
  onFileSelect: (path: string, options?: { disposition?: LinkDisposition }) => void;
  onCreateNote: () => void | Promise<void>;
  onLoadDirectory: (path: string) => Promise<boolean>;
  isOpen: boolean;
  onOpenSettings: () => void;
  onOpenWorkspaces: () => void;
  onPatchFileInTree?: (path: string, patch: { id?: string; name?: string }) => void;
}

export const Sidebar = memo(function Sidebar({
  activeFile,
  files,
  directories,
  loadingDirectories,
  workspacePath,
  workspaceName,
  onFileSelect,
  onCreateNote,
  onLoadDirectory,
  isOpen,
  onOpenSettings,
  onOpenWorkspaces,
  onPatchFileInTree,
}: SidebarProps) {
  const { expandedFolders, setExpandedFolders, followFolder } = useExpandedFolders(workspacePath);
  const contentRef = useRef<HTMLDivElement>(null);
  const pendingScrollTopRef = useRef<number | null>(null);
  const treeReady = Boolean(workspacePath && directories.has(workspacePath));
  const { config } = useSettingsStore();
  const plugins = config?.plugins ?? DEFAULT_PLUGIN_SETTINGS;
  const vault = usePluginVaultData(workspacePath);
  const { overview, error: overviewError } = useExplorerOverview(workspacePath, vault.data, plugins);
  const [showHidden, setShowHidden] = useExplorerShowHidden(workspacePath);
  const hidden = useMemo(() => new Set(overview.hidden), [overview.hidden]);
  const pinned = useMemo(() => new Set(overview.pinned), [overview.pinned]);
  const [picker, setPicker] = useState<{ kind: 'color' | 'icon'; path: string; root: string } | null>(null);
  const [pluginError, setPluginError] = useState<string | null>(null);
  const [pluginPending, setPluginPending] = useState(false);
  const rootRef = useRef(workspacePath);
  rootRef.current = workspacePath;

  useEffect(() => { setPicker(null); setPluginError(null); }, [workspacePath, plugins.fileColors.enabled, plugins.fileIcons.enabled]);
  const rows = useMemo(() => {
    if (!treeReady) return [];
    const visible = buildVisibleFileRows(files, directories, expandedFolders, loadingDirectories,
      workspacePath && plugins.explorerFilters.enabled ? { workspacePath, hidden, pinned, showHidden } : undefined);
    return visible.map(row => {
      const path = relativePath(workspacePath!, row.item.id);
      const count = plugins.folderCounts.enabled && row.item.type === 'folder' ? overview.counts[path] : undefined;
      return {
        ...row,
        count: count === 0 && plugins.folderCounts.hideZero ? undefined : count,
        color: plugins.fileColors.enabled && vault.data ? colorForPath(path, vault.data.colors, plugins.fileColors.cascade) : undefined,
        colorBackground: plugins.fileColors.enabled && plugins.fileColors.background,
        iconName: plugins.fileIcons.enabled ? vault.data?.icons[path] : undefined,
      };
    });
  }, [directories, expandedFolders, files, loadingDirectories, treeReady, workspacePath, plugins, hidden, pinned, showHidden, overview.counts, vault.data]);

  const { selectedFiles, rowActions: selectionActions, targetsFor, clearSelection } = useFileSelection(
    activeFile,
    rows,
    onFileSelect,
  );
  const fileOps = useFileTreeActions({
    workspacePath,
    targetsFor,
    clearSelection,
    onPatchFileInTree,
    onPathMoved: followFolder,
  });

  const moveFiles = useStableCallback(async (sourceId: string, targetId: string) => {
    let anyMoved = false;
    for (const fileId of targetsFor(sourceId)) {
      const moved = await moveIntoFolder(fileId, targetId);
      if (!moved) continue;
      anyMoved = true;
      followFolder(fileId, moved);
    }

    if (!anyMoved) return;
    setExpandedFolders((prev) => (
      prev.has(targetId) ? prev : new Set(prev).add(targetId)
    ));
    void onLoadDirectory(targetId);
  });

  const toggleFolder = useStableCallback((id: string) => {
    pendingScrollTopRef.current = contentRef.current?.scrollTop ?? null;
    setExpandedFolders((current) => {
      const next = new Set(current);
      if (!current.has(id)) next.add(id);
      else next.delete(id);
      return next;
    });
  });

  const loadRevealedFolders = useStableCallback(() => {
    for (const id of unloadedExpandedFolders(rows, directories)) void onLoadDirectory(id);
  });

  useEffect(loadRevealedFolders, [directories, expandedFolders, rows, loadRevealedFolders]);

  const prefetchFolder = useStableCallback((id: string) => {
    if (!directories.has(id)) void onLoadDirectory(id);
  });

  useFileTreeDrag(contentRef, targetsFor, moveFiles);

  const focusAfterPicker = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (picker || !focusAfterPicker.current) return;
    const path = focusAfterPicker.current;
    focusAfterPicker.current = undefined;
    const row = Array.from(contentRef.current?.querySelectorAll<HTMLElement>('[data-file-id]') ?? [])
      .find(row => row.dataset.fileId === path);
    row?.querySelector<HTMLButtonElement>('.q-file-item')?.focus();
  }, [picker]);

  const closePicker = useStableCallback((expected?: typeof picker) => {
    if (expected && picker !== expected) return;
    focusAfterPicker.current = picker?.path;
    setPicker(null);
    setPluginError(null);
  });

  const savePluginData = useStableCallback(async (mutator: (data: VaultPluginData) => void) => {
    const root = workspacePath;
    if (pluginPending || !root) return false;
    setPluginPending(true);
    setPluginError(null);
    try {
      await vault.update(mutator);
      return rootRef.current === root;
    } catch {
      if (rootRef.current === root) setPluginError(t('plugins.explorerFilters.saveError'));
      return false;
    } finally {
      setPluginPending(false);
    }
  });

  const chooseColor = useStableCallback((path: string) => {
    if (workspacePath) { setPluginError(null); setPicker({ kind: 'color', path, root: workspacePath }); }
  });
  const chooseIcon = useStableCallback((path: string) => {
    if (workspacePath) { setPluginError(null); setPicker({ kind: 'icon', path, root: workspacePath }); }
  });
  const isPinned = useStableCallback((path: string) => !!workspacePath && pinned.has(relativePath(workspacePath, path)));
  const togglePin = useStableCallback(async (path: string) => {
    if (!workspacePath || !vault.data) return;
    const rel = relativePath(workspacePath, path);
    const strict = (filter: VaultPluginData['filters']['pin'][number]) => filter.kind === 'path' && filter.patternType === 'strict' && filter.pattern === rel && (filter.target === 'both' || filter.target === (rows.find(row => row.item.id === path)?.item.type === 'folder' ? 'folders' : 'files'));
    const unpin = isPinned(path);
    if (unpin && !vault.data.filters.pin.some(filter => filter.active && strict(filter))) {
      setPluginError(t('plugins.explorerFilters.pinnedByFilter'));
      return;
    }
    await savePluginData(data => {
      if (unpin) data.filters.pin = data.filters.pin.filter(filter => !strict(filter));
      else {
        const existing = data.filters.pin.find(strict);
        if (existing) existing.active = true;
        else data.filters.pin.push({ name: rel, active: true, kind: 'path', target: 'both', pattern: rel, patternType: 'strict' });
      }
    });
  });
  const isHidden = useStableCallback((path: string) => {
    if (!workspacePath) return false;
    const rel = relativePath(workspacePath, path);
    return [...hidden].some(parent => rel === parent || rel.startsWith(parent + '/'));
  });
  const hidePath = useStableCallback(async (path: string) => {
    if (!workspacePath || !vault.data) return;
    const rel = relativePath(workspacePath, path);
    const strict = (filter: VaultPluginData['filters']['hide'][number]) => filter.kind === 'path' && filter.patternType === 'strict' && filter.pattern === rel && (filter.target === 'both' || filter.target === (rows.find(row => row.item.id === path)?.item.type === 'folder' ? 'folders' : 'files'));
    const unhide = isHidden(path);
    if (unhide && ([...hidden].some(parent => rel.startsWith(parent + '/')) || !vault.data.filters.hide.some(filter => filter.active && strict(filter)))) {
      setPluginError(t('plugins.explorerFilters.hiddenByFilter'));
      return;
    }
    await savePluginData(data => {
      if (unhide) data.filters.hide = data.filters.hide.filter(filter => !filter.active || !strict(filter));
      else {
        const existing = data.filters.hide.find(strict);
        if (existing) existing.active = true;
        else data.filters.hide.push({ name: rel, active: true, kind: 'path', target: 'both', pattern: rel, patternType: 'strict' });
      }
    });
  });

  const copyText = useStableCallback(async (text: string) => {
    try { await navigator.clipboard.writeText(text); }
    catch { setPluginError(t('fileTree.copyFailed')); }
  });
  const copyVaultPath = useStableCallback((path: string) => {
    if (workspacePath) void copyText(relativePath(workspacePath, path));
  });
  // Windows-путь может прийти с префиксом \\?\ — пользователю он не нужен.
  const copySystemPath = useStableCallback((path: string) => { void copyText(path.replace(/^\\\\\?\\/, '')); });
  const revealInFileManager = useStableCallback((path: string) => {
    revealItemInDir(path).catch(() => setPluginError(t('fileTree.revealFailed')));
  });

  const treeActions = useMemo<FileTreeActions>(() => ({
    ...selectionActions,
    ...fileOps.rowActions,
    toggleFolder,
    prefetchFolder,
    // меню включённых плагинов.
    chooseColor: plugins.fileColors.enabled && vault.data ? chooseColor : undefined,
    chooseIcon: plugins.fileIcons.enabled && vault.data ? chooseIcon : undefined,
    togglePin: plugins.explorerFilters.enabled && vault.data ? togglePin : undefined,
    hidePath: plugins.explorerFilters.enabled && vault.data ? hidePath : undefined,
    isPinned,
    isHidden,
    copyVaultPath,
    copySystemPath,
    revealInFileManager,
  }), [fileOps.rowActions, prefetchFolder, selectionActions, toggleFolder, plugins, vault.data, chooseColor, chooseIcon, togglePin, hidePath, isPinned, isHidden, copyVaultPath, copySystemPath, revealInFileManager]);

  useLayoutEffect(() => {
    const content = contentRef.current;
    const item = content?.querySelector<HTMLElement>('[data-file-active="true"]');
    if (!item || !content) return;

    const contentRect = content.getBoundingClientRect();
    const itemRect = item.getBoundingClientRect();
    if (itemRect.top < contentRect.top) {
      content.scrollTop -= contentRect.top - itemRect.top;
    } else if (itemRect.bottom > contentRect.bottom) {
      content.scrollTop += itemRect.bottom - contentRect.bottom;
    }
  }, [activeFile]);

  useLayoutEffect(() => {
    const content = contentRef.current;
    const scrollTop = pendingScrollTopRef.current;
    if (!content || scrollTop === null) return;
    content.scrollTop = scrollTop;
    pendingScrollTopRef.current = null;
  }, [expandedFolders]);

  const { deleteTargets } = fileOps;

  return (
    <aside className={`q-collapsible-panel q-sidebar ${isOpen ? '' : 'is-collapsed'}`} aria-hidden={!isOpen}>
      <div className="q-panel-header" data-tauri-drag-region aria-hidden="true">
      </div>
      <div ref={contentRef} className="q-sidebar-content">
        {!treeReady ? null : rows.length === 0 ? (
          <EmptyState icon={FileText} title={t('fileTree.empty')} compact>
            <Button size="s" onClick={() => void onCreateNote()}>
              {t('editor.createNote')}
            </Button>
          </EmptyState>
        ) : (
          <FileTree
            rows={rows}
            activeFile={activeFile}
            selectedFiles={selectedFiles}
            renamingPath={fileOps.renamingPath}
            actions={treeActions}
          />
        )}
      </div>

      <DeleteNotesDialog
        targets={deleteTargets}
        pending={fileOps.isDeleting}
        onCancel={fileOps.dismissDelete}
        onConfirm={fileOps.confirmDelete}
      />

      {(overviewError || pluginError) && !picker && <p role="alert" className="q-explorer-error">{pluginError ?? overviewError}</p>}
      {plugins.explorerFilters.enabled && workspacePath && <label className="q-explorer-show-hidden">
        <input type="checkbox" checked={showHidden} onChange={event => setShowHidden(event.currentTarget.checked)} />
        {t('plugins.explorerFilters.showHidden')}
      </label>}
      {picker?.root === workspacePath && vault.data && <>{picker.kind === 'color' ? <ColorPopover
        open value={vault.data.colors[relativePath(picker.root, picker.path)]} pending={pluginPending} error={pluginError} onClose={closePicker}
        onSelect={value => { const rel = relativePath(picker.root, picker.path); void savePluginData(data => {
          if (value) data.colors[rel] = value; else delete data.colors[rel];
        }).then(saved => { if (saved) closePicker(picker); }); }}
      /> : <IconPickerDialog
        open value={vault.data.icons[relativePath(picker.root, picker.path)]} recentIcons={vault.data.recentIcons}
        pending={pluginPending} error={pluginError} onClose={closePicker}
        onSelect={value => { const rel = relativePath(picker.root, picker.path); void savePluginData(data => {
          if (value) { data.icons[rel] = value; data.recentIcons = [value, ...data.recentIcons.filter(icon => icon !== value)].slice(0, 5); }
          else delete data.icons[rel];
        }).then(saved => { if (saved) closePicker(picker); }); }}
      />}</>}
      <GitStatusBar workspacePath={workspacePath} />
      <SidebarFooter
        workspaceName={workspaceName}
        workspacePath={workspacePath}
        onOpenSettings={onOpenSettings}
        onOpenWorkspaces={onOpenWorkspaces}
      />
    </aside>
  );
});
