import { useCallback, useState, type MouseEvent } from 'react';
import type { EditorView } from '@codemirror/view';
import { t } from '../../i18n';
import type { MenuItem } from './Menu';
import { useContextMenu } from './useContextMenu';

type ViewSource = () => EditorView | null | undefined;

function copyItem(getView: ViewSource, enabled: boolean): MenuItem {
  return {
    id: 'copy',
    label: t('common.copy'),
    disabled: !enabled,
    onSelect: () => {
      const view = getView();
      if (!view || view.state.selection.main.empty) return;
      const { from, to } = view.state.selection.main;
      void navigator.clipboard.writeText(view.state.sliceDoc(from, to)).catch((error) => {
        console.error('Failed to copy the selection', error);
      });
    },
  };
}

function selectAllItem(getView: ViewSource): MenuItem {
  return {
    id: 'select-all',
    label: t('common.selectAll'),
    onSelect: () => {
      const view = getView();
      if (!view) return;
      view.dispatch({ selection: { anchor: 0, head: view.state.doc.length } });
      view.focus();
    },
  };
}

export function useSelectionMenu(getView: ViewSource) {
  const { open, position, close, onContextMenu: openMenu } = useContextMenu();
  const [copyEnabled, setCopyEnabled] = useState(false);

  const onContextMenu = useCallback((event: MouseEvent<Element>) => {
    const view = getView();
    setCopyEnabled(Boolean(view && !view.state.selection.main.empty));
    openMenu(event);
  }, [getView, openMenu]);

  return {
    open,
    position,
    close,
    onContextMenu,
    copy: copyItem(getView, copyEnabled),
    selectAll: selectAllItem(getView),
  };
}
