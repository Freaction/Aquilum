import { type ChangeSpec, type Text } from '@codemirror/state';
import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import { getOutlineLevel, matchOutlineItem, orderedSuffix } from './constructs';

export function collectOrderedRenumberChanges(doc: Text): ChangeSpec[] {
    const changes: ChangeSpec[] = [];
    const counters = new Map<number, number>();

    for (let number = 1; number <= doc.lines; number++) {
        const line = doc.line(number);
        const item = matchOutlineItem(line.text);
        const level = item ? getOutlineLevel(item.indent) : null;
        if (!item || level === null) {
            if (!item && line.text.trim() === '') continue;
            counters.clear();
            continue;
        }

        for (const activeLevel of counters.keys()) {
            if (activeLevel > level) counters.delete(activeLevel);
        }

        if (item.marker.kind === 'bullet') {
            counters.delete(level);
            continue;
        }

        const expected = (counters.get(level) ?? 0) + 1;
        counters.set(level, expected);
        if (item.marker.number !== expected) {
            const suffix = orderedSuffix(item.marker.text);
            changes.push({
                from: line.from + item.indent.length,
                to: line.from + item.indent.length + item.marker.text.length,
                insert: `${expected}${suffix}`,
            });
        }
    }

    return changes;
}

export const outlineRenumbering = ViewPlugin.fromClass(class {
    private scheduled = false;
    private destroyed = false;

    constructor(private readonly view: EditorView) {}

    update(update: ViewUpdate) {
        if (!update.docChanged || this.scheduled) return;
        this.scheduled = true;
        queueMicrotask(() => {
            this.scheduled = false;
            if (this.destroyed) return;
            const changes = collectOrderedRenumberChanges(this.view.state.doc);
            if (changes.length > 0) this.view.dispatch({ changes });
        });
    }

    destroy() {
        this.destroyed = true;
    }
});
