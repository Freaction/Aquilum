import { indentLess, indentMore } from '@codemirror/commands';
import { insertNewlineContinueMarkup } from '@codemirror/lang-markdown';
import { syntaxTree } from '@codemirror/language';
import { EditorSelection, Prec, type EditorState, type StateCommand } from '@codemirror/state';
import { keymap, type KeyBinding } from '@codemirror/view';
import { codeMirrorKey, SHORTCUTS } from '../../../../config/shortcuts';
import {
    continuationIndent,
    continuationPrefix,
    markerPrefix,
    matchOutlineItem,
    openOutlineItem,
    reduceOutlineIndent,
} from './constructs';
import { hasAncestorNamed } from '../syntaxAncestor';

function inBlockquote(state: EditorState, pos: number): boolean {
    return hasAncestorNamed(syntaxTree(state).resolveInner(pos, -1), 'Blockquote');
}

export const exitEmptyBlockquote: StateCommand = ({ state, dispatch }) => {
    const sel = state.selection.main;
    if (!sel.empty) return false;
    const line = state.doc.lineAt(sel.head);
    if (!/^(>\s*)$/.test(line.text) || !inBlockquote(state, sel.head)) return false;
    dispatch(state.update({
        changes: { from: line.from, to: line.to, insert: state.lineBreak },
        selection: EditorSelection.cursor(line.from + 1),
        scrollIntoView: true,
        userEvent: 'input',
    }));
    return true;
};

export const continueOutlineItem: StateCommand = ({ state, dispatch }) => {
    let applicable = true;
    const transaction = state.changeByRange(range => {
        if (!range.empty) {
            applicable = false;
            return { range };
        }

        const line = state.doc.lineAt(range.head);
        const item = matchOutlineItem(line.text);

        if (!item) {
            const openItem = openOutlineItem(state.doc, line.number);
            if (!openItem) {
                applicable = false;
                return { range };
            }

            const nextItem = `${state.lineBreak}${openItem.indent}${continuationPrefix(openItem)}`;
            return {
                changes: { from: range.head, insert: nextItem },
                range: EditorSelection.cursor(range.head + nextItem.length),
            };
        }

        if (range.head < line.from + item.contentFrom) {
            applicable = false;
            return { range };
        }

        if (!line.text.slice(item.contentFrom).trim()) {
            const reducedIndent = reduceOutlineIndent(item.indent);
            if (reducedIndent === null) {
                return {
                    changes: { from: line.from, to: line.to, insert: '' },
                    range: EditorSelection.cursor(line.from),
                };
            }

            const prefix = `${reducedIndent}${markerPrefix(item)}`;
            return {
                changes: { from: line.from, to: line.to, insert: prefix },
                range: EditorSelection.cursor(line.from + prefix.length),
            };
        }

        const continuation = `${state.lineBreak}${item.indent}${continuationPrefix(item)}`;
        return {
            changes: { from: range.head, insert: continuation },
            range: EditorSelection.cursor(range.head + continuation.length),
        };
    });

    if (!applicable) return false;
    dispatch(state.update(transaction, { scrollIntoView: true, userEvent: 'input' }));
    return true;
};

export const breakOutlineLine: StateCommand = ({ state, dispatch }) => {
    const range = state.selection.main;
    if (!range.empty || state.selection.ranges.length > 1) return false;

    const line = state.doc.lineAt(range.head);
    const item = matchOutlineItem(line.text);
    if (item && range.head < line.from + item.contentFrom) return false;

    const openItem = item ?? openOutlineItem(state.doc, line.number);
    if (!openItem) return false;

    const insert = `${state.lineBreak}${continuationIndent(openItem)}`;
    dispatch(state.update({
        changes: { from: range.head, insert },
        selection: EditorSelection.cursor(range.head + insert.length),
        scrollIntoView: true,
        userEvent: 'input',
    }));
    return true;
};

export const outlineKeyBindings: readonly KeyBinding[] = [
    { key: 'Enter', run: continueOutlineItem },
    { key: 'Enter', run: exitEmptyBlockquote },
    { key: 'Enter', run: insertNewlineContinueMarkup },
    { key: codeMirrorKey(SHORTCUTS.LINE_BREAK), run: breakOutlineLine },
    { key: codeMirrorKey(SHORTCUTS.INDENT), run: indentMore },
    { key: codeMirrorKey(SHORTCUTS.OUTDENT), run: indentLess },
];

export const outlineKeymap = Prec.highest(keymap.of(outlineKeyBindings));
