import { memo } from 'react';
import { FileTreeRow } from './FileTreeRow';
import { comparablePath } from '../../modules/paths';
import type { FileTreeActions, VisibleFileRow } from './fileTreeModel';

interface FileTreeProps {
  rows: readonly VisibleFileRow[];
  activeFile: string | null;
  selectedFiles: ReadonlySet<string>;
  renamingPath: string | null;
  actions: FileTreeActions;
}

export const FileTree = memo(function FileTree({
  rows,
  activeFile,
  selectedFiles,
  renamingPath,
  actions,
}: FileTreeProps) {
  const activePath = comparablePath(activeFile);

  return (
    <>
      {rows.map((row) => {
        const selected = selectedFiles.has(row.item.id);
        return (
          <FileTreeRow
            key={row.item.id}
            item={row.item}
            depth={row.depth}
            expanded={row.expanded}
            loading={row.loading}
            active={row.comparableId === activePath}
            selected={selected}
            guideDepths={row.guideDepths}
            renaming={renamingPath === row.item.id}
            actions={actions}
            isHidden={row.isHidden}
            color={row.color}
            colorBackground={row.colorBackground}
            iconName={row.iconName}
            count={row.count}
          />
        );
      })}
    </>
  );
});
