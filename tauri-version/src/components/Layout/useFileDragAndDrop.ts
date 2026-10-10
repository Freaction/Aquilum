import { useEffect, type RefObject } from 'react';
import { useStableCallback } from '../../hooks/useStableCallback';
import { plural } from '../../i18n';
import { isInsidePath, parentDirectory } from '../../modules/paths';

const DRAG_THRESHOLD = 5;
const GHOST_OFFSET = 12;
const GHOST_LABEL_LIMIT = 25;

let ghost: HTMLDivElement | null = null;

function showGhost(label: string, x: number, y: number) {
  if (!ghost) {
    ghost = document.createElement('div');
    ghost.className = 'q-drag-ghost';
    document.body.appendChild(ghost);
  }
  ghost.textContent = label.length > GHOST_LABEL_LIMIT
    ? `${label.slice(0, GHOST_LABEL_LIMIT)}…`
    : label;
  moveGhost(x, y);
  ghost.style.display = 'block';
}

function moveGhost(x: number, y: number) {
  if (!ghost) return;
  ghost.style.left = `${x}px`;
  ghost.style.top = `${y}px`;
}

function hideGhost() {
  if (ghost) ghost.style.display = 'none';
}

function rowUnder(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>('[data-file-id]') : null;
}

function dropDirOf(row: HTMLElement): string {
  const id = row.dataset.fileId ?? '';
  return row.dataset.fileType === 'folder' ? id : parentDirectory(id);
}

export function canMove(sourceId: string, targetId: string, targetIsFolder: boolean): boolean {
  if (!sourceId || !targetId || targetId === sourceId) return false;
  const target = targetIsFolder ? targetId : parentDirectory(targetId);
  if (!target || target === parentDirectory(sourceId)) return false;
  return target !== sourceId && !isInsidePath(target, sourceId);
}

function accepts(row: HTMLElement, sourceId: string): boolean {
  return canMove(sourceId, row.dataset.fileId ?? '', row.dataset.fileType === 'folder');
}

export function useFileTreeDrag(
  container: RefObject<HTMLElement | null>,
  targetsFor: (id: string) => readonly string[],
  onDrop: (sourceId: string, targetDir: string) => void,
) {
  const targets = useStableCallback(targetsFor);
  const drop = useStableCallback(onDrop);

  useEffect(() => {
    const element = container.current;
    if (!element) return;

    let sourceId = '';
    let sourceName = '';
    let startX = 0;
    let startY = 0;
    let dragging = false;
    let hovered: HTMLElement | null = null;

    function highlight(row: HTMLElement | null) {
      if (hovered === row) return;
      hovered?.removeAttribute('data-drop-over');
      hovered = row;
      hovered?.setAttribute('data-drop-over', '');
    }

    function stop() {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.removeEventListener('mouseover', onOver);
      highlight(null);
      hideGhost();
      document.body.style.cursor = '';
      sourceId = '';
      dragging = false;
    }

    function onMove(event: MouseEvent) {
      if (!dragging) {
        const far = Math.abs(event.clientX - startX) >= DRAG_THRESHOLD
          || Math.abs(event.clientY - startY) >= DRAG_THRESHOLD;
        if (!far) return;
        dragging = true;
        const count = targets(sourceId).length;
        showGhost(
          count > 1 ? plural('fileTree.dragCount', count) : sourceName,
          event.clientX + GHOST_OFFSET,
          event.clientY + GHOST_OFFSET,
        );
        document.body.style.cursor = 'grabbing';
        document.addEventListener('mouseover', onOver);
      }
      moveGhost(event.clientX + GHOST_OFFSET, event.clientY + GHOST_OFFSET);
    }

    function onOver(event: MouseEvent) {
      const row = rowUnder(event.target);
      highlight(row && accepts(row, sourceId) ? row : null);
    }

    function onUp() {
      const target = dragging && hovered ? dropDirOf(hovered) : null;
      const moved = sourceId;
      stop();
      if (target) drop(moved, target);
    }

    function onDown(event: MouseEvent) {
      if (event.button !== 0) return;
      const row = rowUnder(event.target);
      if (!row || row.dataset.renaming !== undefined) return;
      sourceId = row.dataset.fileId ?? '';
      sourceName = row.dataset.fileName ?? '';
      startX = event.clientX;
      startY = event.clientY;
      dragging = false;
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    }

    element.addEventListener('mousedown', onDown);
    return () => {
      element.removeEventListener('mousedown', onDown);
      stop();
    };
  }, [container, drop, targets]);
}
