import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  type FocusEventHandler,
  type RefObject,
} from 'react';
import { EditorState, StateEffect, type EditorStateConfig, type Extension } from '@codemirror/state';
import { EditorView, type ViewUpdate } from '@codemirror/view';

const fillParentScroller = EditorView.theme({
  '& .cm-scroller': {
    height: '100% !important',
  },
});

interface CodeMirrorViewProps {
  viewRef: RefObject<EditorView | null>;
  doc: string;
  extensions: Extension[];
  selection?: EditorStateConfig['selection'];
  className?: string;
  autoFocus?: boolean;
  onCreate?: (view: EditorView) => void;
  onUpdate?: (update: ViewUpdate) => void;
  onFocus?: FocusEventHandler<HTMLDivElement>;
  onBlur?: FocusEventHandler<HTMLDivElement>;
}

export function CodeMirrorView({
  viewRef,
  doc,
  extensions,
  selection,
  className,
  autoFocus = false,
  onCreate,
  onUpdate,
  onFocus,
  onBlur,
}: CodeMirrorViewProps) {
  const parentRef = useRef<HTMLDivElement>(null);
  const onUpdateRef = useRef(onUpdate);
  onUpdateRef.current = onUpdate;

  const allExtensions = useMemo<Extension[]>(() => [
    fillParentScroller,
    EditorView.updateListener.of((update) => onUpdateRef.current?.(update)),
    extensions,
  ], [extensions]);

  const latest = useRef({ doc, selection, autoFocus, onCreate, allExtensions });
  latest.current = { doc, selection, autoFocus, onCreate, allExtensions };
  const appliedExtensions = useRef<Extension[] | null>(null);

  useLayoutEffect(() => {
    const initial = latest.current;
    appliedExtensions.current = initial.allExtensions;
    const view = new EditorView({
      state: EditorState.create({
        doc: initial.doc,
        selection: initial.selection,
        extensions: initial.allExtensions,
      }),
      parent: parentRef.current!,
    });
    viewRef.current = view;
    initial.onCreate?.(view);
    if (initial.autoFocus) view.focus();
    return () => {
      view.destroy();
      if (viewRef.current === view) viewRef.current = null;
    };
  }, [viewRef]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view || appliedExtensions.current === allExtensions) return;
    appliedExtensions.current = allExtensions;
    view.dispatch({ effects: StateEffect.reconfigure.of(allExtensions) });
  }, [allExtensions, viewRef]);

  return <div ref={parentRef} className={className} onFocus={onFocus} onBlur={onBlur} />;
}
