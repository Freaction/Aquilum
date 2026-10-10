import {
    type Extension,
    type Range,
    type EditorState,
    StateField,
} from '@codemirror/state';
import { Decoration, type DecorationSet, EditorView } from '@codemirror/view';
import { findTablesInDoc } from './constructs';
import { TableWidget } from './widget';

function buildTableDecorations(state: EditorState): DecorationSet {
    const tables = findTablesInDoc(state.doc);
    if (tables.length === 0) return Decoration.none;

    const docLen = state.doc.length;
    const ranges: Range<Decoration>[] = [];

    for (const table of tables) {
        if (
            table.from < 0
            || table.to > docLen
            || table.from >= table.to
            || table.contentTo < table.from
            || table.contentTo > table.to
        ) {
            continue;
        }

        ranges.push(
            Decoration.replace({
                widget: new TableWidget({
                    from: table.from,
                    contentTo: table.contentTo,
                    blockTo: table.to,
                    model: table.model,
                    text: state.doc.sliceString(table.from, table.contentTo),
                }),
                block: true,
            }).range(table.from, table.to),
        );
    }

    return ranges.length ? Decoration.set(ranges, true) : Decoration.none;
}

const tableDecorationField = StateField.define<DecorationSet>({
    create: (state) => buildTableDecorations(state),
    update: (value, tr) => (tr.docChanged ? buildTableDecorations(tr.state) : value),
    provide: (field) => EditorView.decorations.from(field),
});

export const tablePreview: Extension = [
    tableDecorationField,
    EditorView.editorAttributes.of((view) => {
        const selection = view.state.selection;
        if (selection.ranges.length !== 1 || !selection.main.empty) return null;
        const head = selection.main.head;
        const tables = view.state.field(tableDecorationField).iter();
        while (tables.value) {
            if (tables.from === head || tables.to === head) {
                return { class: 'q-md-table-boundary-caret' };
            }
            tables.next();
        }
        return null;
    }),
    EditorView.atomicRanges.of((view) => view.state.field(tableDecorationField)),
];
