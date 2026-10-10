import {
  forwardRef,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  type FocusEventHandler,
} from 'react';
import { EditorState, Prec, type Extension } from '@codemirror/state';
import { EditorView, placeholder as placeholderText, type ViewUpdate } from '@codemirror/view';
import { CodeMirrorView } from './CodeMirrorView';
import {
  aquilumCodeMirrorTheme,
  aquilumFieldSetup,
  aquilumHorizontalFieldTheme,
  aquilumSingleLineExtensions,
  normalizeSingleLineText,
  syncDocument,
} from './codeMirror';

const noExtensions: Extension[] = [];

export interface CodeMirrorFieldRef {
  readonly view: EditorView | null;
  focus: () => void;
  select: () => void;
  blur: () => void;
  setValue: (value: string) => void;
}

interface CodeMirrorFieldProps {
  value: string;
  onChange?: (value: string) => void;
  ariaLabel: string;
  mode?: 'single-line' | 'multiline';
  lineWrapping?: boolean;
  placeholder?: string;
  className?: string;
  autoFocus?: boolean;
  spellCheck?: boolean;
  disabled?: boolean;
  extensions?: Extension[];
  onKeyDown?: (event: KeyboardEvent, view: EditorView) => boolean | void;
  onFocus?: FocusEventHandler<HTMLDivElement>;
  onBlur?: FocusEventHandler<HTMLDivElement>;
  onCreate?: (view: EditorView) => void;
}

export const CodeMirrorField = forwardRef<CodeMirrorFieldRef, CodeMirrorFieldProps>(
  function CodeMirrorField({
    value,
    onChange,
    ariaLabel,
    mode = 'single-line',
    lineWrapping = mode === 'multiline',
    placeholder,
    className = '',
    autoFocus = false,
    spellCheck = false,
    disabled = false,
    extensions = noExtensions,
    onKeyDown,
    onFocus,
    onBlur,
    onCreate,
  }, forwardedRef) {
    const viewRef = useRef<EditorView>(null);
    const keyDownRef = useRef(onKeyDown);
    keyDownRef.current = onKeyDown;

    const text = mode === 'single-line' ? normalizeSingleLineText(value) : value;
    const textRef = useRef(text);
    textRef.current = text;

    useImperativeHandle(forwardedRef, () => ({
      get view() {
        return viewRef.current;
      },
      focus() {
        viewRef.current?.focus();
      },
      select() {
        const view = viewRef.current;
        if (!view) return;
        view.focus();
        view.dispatch({ selection: { anchor: 0, head: view.state.doc.length } });
      },
      blur() {
        viewRef.current?.contentDOM.blur();
      },
      setValue(next: string) {
        const view = viewRef.current;
        if (view) syncDocument(view, mode === 'single-line' ? normalizeSingleLineText(next) : next);
      },
    }), [mode]);

    useLayoutEffect(() => {
      const view = viewRef.current;
      if (view) syncDocument(view, text);
    }, [text]);

    const fieldExtensions = useMemo<Extension[]>(() => [
      placeholder ? placeholderText(placeholder) : [],
      aquilumFieldSetup,
      aquilumCodeMirrorTheme,
      EditorView.contentAttributes.of({
        'aria-label': ariaLabel,
        'aria-multiline': String(mode === 'multiline'),
        autocapitalize: 'off',
        autocomplete: 'off',
        spellcheck: String(spellCheck),
      }),
      EditorState.readOnly.of(disabled),
      EditorView.editable.of(!disabled),
      Prec.highest(EditorView.domEventHandlers({
        keydown(event, view) {
          const handled = keyDownRef.current?.(event, view) === true;
          if (handled || event.defaultPrevented) return true;
          if (mode === 'single-line' && event.key === 'Enter') {
            event.preventDefault();
            return true;
          }
          return false;
        },
      })),
      mode === 'single-line' ? aquilumSingleLineExtensions : [],
      mode === 'single-line' && !lineWrapping ? aquilumHorizontalFieldTheme : [],
      lineWrapping ? EditorView.lineWrapping : [],
      ...extensions,
    ], [ariaLabel, disabled, extensions, lineWrapping, mode, placeholder, spellCheck]);

    const handleUpdate = (update: ViewUpdate) => {
      if (!update.docChanged) return;
      const nextValue = update.state.doc.toString();
      const normalized = mode === 'single-line' ? normalizeSingleLineText(nextValue) : nextValue;
      if (normalized === textRef.current) return;
      onChange?.(normalized);
    };

    const classes = [
      'q-code-mirror-field',
      `q-code-mirror-field--${mode}`,
      className,
    ].filter(Boolean).join(' ');

    return (
      <CodeMirrorView
        viewRef={viewRef}
        className={classes}
        doc={text}
        autoFocus={autoFocus}
        extensions={fieldExtensions}
        onCreate={onCreate}
        onUpdate={handleUpdate}
        onFocus={onFocus}
        onBlur={onBlur}
      />
    );
  },
);
