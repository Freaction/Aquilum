import { useLayoutEffect, useRef } from 'react';
import type { EditorView } from '@codemirror/view';
import { CodeMirrorField, type CodeMirrorFieldRef } from './CodeMirrorField';
import './InlineRenameField.css';

function selectAll(view: EditorView) {
  view.focus();
  view.dispatch({ selection: { anchor: 0, head: view.state.doc.length } });
}

interface InlineRenameFieldProps {
  name: string;
  ariaLabel: string;
  className?: string;
  onCommit: (nextName: string) => void;
  onCancel: () => void;
}

export function InlineRenameField({
  name,
  ariaLabel,
  className = '',
  onCommit,
  onCancel,
}: InlineRenameFieldProps) {
  const fieldRef = useRef<CodeMirrorFieldRef>(null);
  const skipBlurCommit = useRef(false);

  useLayoutEffect(() => {
    const view = fieldRef.current?.view;
    if (view) selectAll(view);
  }, [name]);

  const draft = () => fieldRef.current?.view?.state.doc.toString() ?? name;

  return (
    <CodeMirrorField
      ref={fieldRef}
      className={`q-inline-rename ${className}`.trim()}
      value={name}
      ariaLabel={ariaLabel}
      mode="single-line"
      onCreate={selectAll}
      onBlur={() => {
        if (skipBlurCommit.current) {
          skipBlurCommit.current = false;
          return;
        }
        onCommit(draft());
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          skipBlurCommit.current = true;
          onCommit(draft());
          return true;
        }
        if (event.key === 'Escape') {
          event.preventDefault();
          skipBlurCommit.current = true;
          onCancel();
          return true;
        }
        return false;
      }}
    />
  );
}
