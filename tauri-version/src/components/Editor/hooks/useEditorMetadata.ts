import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { Transaction } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { frontmatterRange } from '../../../modules/docs/frontmatter';
import {
  isFrontmatterExpanded,
  setFrontmatterExpanded,
} from '../extensions/frontmatterUi';

const STORAGE_KEY = 'aquilum_metadata_expanded';

function storageKey(filePath: string): string {
  return `${STORAGE_KEY}:${filePath}`;
}

function readExpanded(filePath: string): boolean | null {
  try {
    const stored = localStorage.getItem(storageKey(filePath));
    return stored === null ? null : stored === '1';
  } catch {
    return null;
  }
}

function saveExpanded(filePath: string, expanded: boolean): void {
  try {
    localStorage.setItem(storageKey(filePath), expanded ? '1' : '0');
  } catch {}
}

function moveExpanded(oldPath: string, newPath: string, expanded: boolean): void {
  saveExpanded(newPath, expanded);
  try {
    localStorage.removeItem(storageKey(oldPath));
  } catch {}
}

function applyExpanded(view: EditorView, expanded: boolean): void {
  if (isFrontmatterExpanded(view.state) === expanded) return;
  view.dispatch({
    effects: setFrontmatterExpanded.of(expanded),
    annotations: Transaction.addToHistory.of(false),
  });
}

export function useEditorMetadata(options: {
  filePath: string;
  docContent: string;
  bodyRef: RefObject<EditorView | null>;
}) {
  const { filePath, docContent, bodyRef } = options;
  const hasFrontmatter = Boolean(frontmatterRange(docContent));
  const [expanded, setExpanded] = useState(() => readExpanded(filePath) ?? false);

  const knownPathRef = useRef(filePath);
  useEffect(() => {
    const previous = knownPathRef.current;
    if (previous === filePath) return;
    knownPathRef.current = filePath;
    const stored = readExpanded(filePath);
    if (stored === null) {
      moveExpanded(previous, filePath, expanded);
      return;
    }
    setExpanded(stored);
    const view = bodyRef.current;
    if (view) applyExpanded(view, stored);
  }, [bodyRef, expanded, filePath]);

  const syncFromView = useCallback((view: EditorView) => {
    const next = isFrontmatterExpanded(view.state);
    if (next === expanded) return;
    setExpanded(next);
    saveExpanded(filePath, next);
  }, [expanded, filePath]);

  const syncOnCreate = useCallback((view: EditorView) => {
    applyExpanded(view, expanded);
  }, [expanded]);

  const toggle = useCallback(() => {
    const view = bodyRef.current;
    const next = view
      ? !isFrontmatterExpanded(view.state)
      : !expanded;
    setExpanded(next);
    saveExpanded(filePath, next);
    if (view) applyExpanded(view, next);
  }, [bodyRef, expanded, filePath]);

  return {
    hasFrontmatter,
    metadataExpanded: expanded,
    toggleMetadata: toggle,
    syncOnCreate,
    syncFromView,
  };
}
