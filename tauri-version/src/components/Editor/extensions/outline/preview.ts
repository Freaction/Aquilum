import { syntaxTree } from '@codemirror/language';
import { type Range, type Text } from '@codemirror/state';
import {
    Decoration,
    type DecorationSet,
    EditorView,
    ViewPlugin,
    type ViewUpdate,
    WidgetType,
} from '@codemirror/view';
import { createTaskCheckbox } from '../../../Common/taskCheckbox';
import { findTaskMark, markText, TASK_MARK_LENGTH } from '../../../../modules/documents/taskCheckbox';
import { previewCaret, revealAtCaretFacet } from '../livePreviewConfig';
import { shouldRevealSyntax } from '../livePreviewVisibility';
import { codeBlocks } from '../codeBlock/blocks';
import { ensureEditorTree } from '../ensureEditorTree';
import {
    analyzeOutlineLines,
    continuationIndent,
    getOutlineLevel,
    outlineContextStart,
    type OutlineItem,
} from './constructs';

export function getListIndentWidth(indent: string): string {
    const level = getOutlineLevel(indent);
    if (level === null || level === 0) return '0px';
    if (level === 1) return 'var(--q-editor-list-indent-width)';
    return `calc(${Array(level).fill('var(--q-editor-list-indent-width)').join(' + ')})`;
}

class OutlineBulletWidget extends WidgetType {
    toDOM(): HTMLElement {
        const bullet = document.createElement('span');
        bullet.className = 'q-md-outline-bullet';
        bullet.setAttribute('aria-hidden', 'true');
        return bullet;
    }
}

class TaskCheckboxWidget extends WidgetType {
    constructor(private readonly done: boolean) {
        super();
    }

    eq(other: WidgetType): boolean {
        return other instanceof TaskCheckboxWidget && other.done === this.done;
    }

    toDOM(view: EditorView): HTMLElement {
        const box = createTaskCheckbox(this.done);
        box.addEventListener('mousedown', (event) => {
            event.preventDefault();
            const position = view.posAtDOM(box);
            const line = view.state.doc.lineAt(position);
            const mark = findTaskMark(line.text);
            if (!mark) return;
            const from = line.from + mark.start;
            view.dispatch({
                changes: { from, to: from + TASK_MARK_LENGTH, insert: markText(!mark.done) },
            });
        });
        return box;
    }

    ignoreEvent(event: Event): boolean {
        return event.type === 'mousedown';
    }
}

const bulletDecoration = Decoration.replace({ widget: new OutlineBulletWidget() });
const indentationDecoration = Decoration.mark({ class: 'q-md-list-indent' });
const continuationIndentDecoration = Decoration.replace({});
const outlineMarkMuted = Decoration.mark({ class: 'q-md-outline-mark' });
const outlineMarkPlain = Decoration.mark({ class: 'q-md-outline-mark-plain' });

function createLineDecoration(item: OutlineItem, isContinuation = false): Decoration {
    const markerWidth = item.marker.text.length + 1;
    return Decoration.line({
        class: isContinuation ? 'q-md-list-item q-md-list-continuation' : 'q-md-list-item',
        attributes: {
            style: `--q-editor-list-level-indent: ${getListIndentWidth(item.indent)}; --q-editor-list-marker-width: ${markerWidth}ch`,
        },
    });
}

interface OutlineDecorations {
    decorations: DecorationSet;
    continuationIndents: DecorationSet;
}

const NO_DECORATIONS: OutlineDecorations = {
    decorations: Decoration.none,
    continuationIndents: Decoration.none,
};

export function buildOutlineDecorations(
    doc: Text,
    visibleRanges: readonly { from: number; to: number }[],
    head = 0,
): OutlineDecorations {
    if (visibleRanges.length === 0) return NO_DECORATIONS;

    const firstVisibleLine = doc.lineAt(visibleRanges[0].from);
    const lastVisibleLine = doc.lineAt(visibleRanges[visibleRanges.length - 1].to);

    const startLine = outlineContextStart(doc, firstVisibleLine.number);

    const lines: string[] = [];
    for (let lineNum = startLine; lineNum <= lastVisibleLine.number; lineNum++) {
        lines.push(doc.line(lineNum).text);
    }

    const ranges: Range<Decoration>[] = [];
    const indents: Range<Decoration>[] = [];
    for (const analysis of analyzeOutlineLines(lines)) {
        const actualLineNumber = startLine + analysis.lineNumber - 1;
        if (actualLineNumber < firstVisibleLine.number || actualLineNumber > lastVisibleLine.number) {
            continue;
        }

        const line = doc.line(actualLineNumber);

        if (analysis.isContinuation) {
            const indentTo = line.from + continuationIndent(analysis.item).length;
            ranges.push(createLineDecoration(analysis.item, true).range(line.from));
            ranges.push(continuationIndentDecoration.range(line.from, indentTo));
            indents.push(continuationIndentDecoration.range(line.from, indentTo));
            continue;
        }

        const markerFrom = line.from + analysis.item.indent.length;
        const markerTo = markerFrom + analysis.item.marker.text.length;
        const markerZoneTo = line.from + analysis.item.contentFrom;
        const markerActive = shouldRevealSyntax(doc, head, markerFrom, markerTo);

        if (!analysis.hasParent) {
            ranges.push(outlineMarkPlain.range(markerFrom, markerTo));
            continue;
        }

        ranges.push(createLineDecoration(analysis.item).range(line.from));

        if (analysis.item.indent.length > 0) {
            ranges.push(indentationDecoration.range(line.from, markerFrom));
        }

        const task = findTaskMark(line.text);
        const taskFrom = task ? line.from + task.start : 0;
        const taskTo = taskFrom + TASK_MARK_LENGTH;
        const taskActive = task ? shouldRevealSyntax(doc, head, taskFrom, taskTo) : false;

        if (markerActive) {
            ranges.push(outlineMarkPlain.range(markerFrom, markerTo));
        } else if (analysis.showBullet) {
            ranges.push(bulletDecoration.range(markerFrom, markerZoneTo));
        } else if (analysis.item.marker.kind === 'ordered') {
            ranges.push(outlineMarkMuted.range(markerFrom, markerZoneTo));
        }

        if (task && !taskActive) {
            ranges.push(
                Decoration.replace({ widget: new TaskCheckboxWidget(task.done) })
                    .range(taskFrom, taskTo),
            );
        }
    }

    return {
        decorations: Decoration.set(ranges, true),
        continuationIndents: Decoration.set(indents, true),
    };
}

export const outlinePreview = ViewPlugin.fromClass(class {
    decorations: DecorationSet = Decoration.none;
    continuationIndents: DecorationSet = Decoration.none;

    constructor(view: EditorView) {
        this.read(view);
    }

    update(update: ViewUpdate) {
        if (update.docChanged || update.selectionSet || update.viewportChanged || syntaxTree(update.state) !== syntaxTree(update.startState)
                || update.state.facet(revealAtCaretFacet) !== update.startState.facet(revealAtCaretFacet)) {
            this.read(update.view);
        }
    }

    read(view: EditorView) {
        const built = buildOutlineDecorations(
            view.state.doc,
            view.visibleRanges,
            previewCaret(view.state),
        );
        const blocks = codeBlocks(ensureEditorTree(view), view.state.doc, view.visibleRanges);
        const filter = (from: number) => !blocks.some(block => from >= block.from && from <= block.end);
        this.decorations = built.decorations.update({ filter });
        this.continuationIndents = built.continuationIndents.update({ filter });
    }
}, {
    decorations: plugin => plugin.decorations,
    provide: plugin => EditorView.atomicRanges.of(
        view => view.plugin(plugin)?.continuationIndents ?? Decoration.none,
    ),
});
