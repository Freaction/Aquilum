import { t } from '../../i18n';
import { useMemo, type RefObject } from 'react';
import type { Extension } from '@codemirror/state';
import type { EditorView, ViewUpdate } from '@codemirror/view';
import { CodeMirrorField, type CodeMirrorFieldRef } from '../Common/CodeMirrorField';
import { CodeMirrorView } from '../Common/CodeMirrorView';
import { Menu } from '../Common/Menu';
import { collapseSelection } from './extensions/editorFocus';
import { wikiHoverHighlightTitle } from './extensions/wikiHoverHighlight';
import { useEditorBodyMenu } from './hooks/useEditorBodyMenu';
import { MetadataToggle } from './MetadataToggle';
import {
    bodyPositionUnderCaret,
    enterBodyFromTitle,
    focusBodyAt,
    hasNoModifiers,
    isOnLastVisualLine,
} from './titleCaret';

function collapseViewSelection(view: EditorView | null | undefined): void {
    if (view) collapseSelection(view);
}

interface EditorContentProps {
    titleRef: RefObject<CodeMirrorFieldRef | null>;
    bodyRef: RefObject<EditorView | null>;
    title: string;
    onCommitTitle: () => void;
    initialBody: string;
    selection: { anchor: number; head: number } | undefined;
    extensions: Extension[];
    autoLinkTitle: boolean;
    onCreateEditor: (view: EditorView) => void;
    onUpdate: (update: ViewUpdate) => void;
    hasFrontmatter?: boolean;
    metadataExpanded?: boolean;
    onToggleMetadata?: () => void;
}

export function EditorContent({
    titleRef,
    bodyRef,
    title,
    onCommitTitle,
    initialBody,
    selection,
    extensions,
    autoLinkTitle,
    onCreateEditor,
    onUpdate,
    hasFrontmatter = false,
    metadataExpanded = false,
    onToggleMetadata,
}: EditorContentProps) {
    const titleExtensions = useMemo(() => [wikiHoverHighlightTitle], []);
    const bodyMenu = useEditorBodyMenu(bodyRef, autoLinkTitle);

    return (
        <div className="q-editor-content" onContextMenu={bodyMenu.onContextMenu}>
            <CodeMirrorField
                ref={titleRef}
                className="q-editor-inline-title-cm"
                value={title}
                ariaLabel={t('editor.titleAria')}
                mode="single-line"
                lineWrapping
                placeholder={t('editor.untitled')}
                extensions={titleExtensions}
                onBlur={onCommitTitle}
                onFocus={() => collapseViewSelection(bodyRef.current)}
                onKeyDown={(event, view) => {
                    const body = bodyRef.current;
                    if (!body || !hasNoModifiers(event)) return false;
                    if (event.key === 'Enter') {
                        event.preventDefault();
                        enterBodyFromTitle(body);
                        return true;
                    }
                    if (event.key === 'ArrowDown' && isOnLastVisualLine(view)) {
                        event.preventDefault();
                        focusBodyAt(body, bodyPositionUnderCaret(view, body));
                        return true;
                    }
                    return false;
                }}
            />
            {hasFrontmatter && onToggleMetadata ? (
                <MetadataToggle
                    expanded={metadataExpanded}
                    onToggle={onToggleMetadata}
                />
            ) : null}
            <CodeMirrorView
                viewRef={bodyRef}
                doc={initialBody}
                selection={selection}
                extensions={extensions}
                onCreate={onCreateEditor}
                onUpdate={onUpdate}
                onFocus={() => collapseViewSelection(titleRef.current?.view)}
            />
            <Menu
                open={bodyMenu.open}
                position={bodyMenu.position}
                items={bodyMenu.items}
                onClose={bodyMenu.close}
                ariaLabel={t('editor.actions')}
            />
        </div>
    );
}

