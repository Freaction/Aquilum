import { syntaxTree } from '@codemirror/language';
import { type Extension, type Range, type Text } from '@codemirror/state';
import {
    Decoration,
    type DecorationSet,
    EditorView,
    ViewPlugin,
    type ViewUpdate,
    WidgetType,
} from '@codemirror/view';
import { previewCaret, revealAtCaretFacet } from '../livePreviewConfig';
import { shouldRevealSyntax } from '../livePreviewVisibility';
import { codeBlocks } from '../codeBlock/blocks';
import { ensureEditorTree } from '../ensureEditorTree';
import { matchOutlineItem } from './constructs';

const DEFAULT_LIST_CALLOUTS = [
    { char: '&', tone: 'highlight' },
    { char: '?', tone: 'question' },
    { char: '!', tone: 'important' },
    { char: '~', tone: 'idea' },
    { char: '@', tone: 'info' },
    { char: '$', tone: 'success' },
    { char: '%', tone: 'muted' },
] as const;

type ListCalloutDef = (typeof DEFAULT_LIST_CALLOUTS)[number];

const CALLOUT_BY_CHAR = new Map<string, ListCalloutDef>(
    DEFAULT_LIST_CALLOUTS.map((entry) => [entry.char, entry]),
);

interface MatchedListCallout {
    charFrom: number;
    charTo: number;
    callout: ListCalloutDef;
}

export function matchListCallout(lineText: string): MatchedListCallout | null {
    const item = matchOutlineItem(lineText);
    if (!item) return null;
    const rest = lineText.slice(item.contentFrom);
    if (rest.length < 2 || rest[1] !== ' ') return null;
    const callout = CALLOUT_BY_CHAR.get(rest[0]);
    if (!callout) return null;
    const charFrom = item.contentFrom;
    return { charFrom, charTo: charFrom + 1, callout };
}

class CalloutMarkerWidget extends WidgetType {
    constructor(readonly char: string) {
        super();
    }

    toDOM(): HTMLElement {
        const el = document.createElement('span');
        el.className = 'q-md-list-callout-marker';
        el.textContent = this.char;
        el.setAttribute('aria-hidden', 'true');
        return el;
    }

    eq(other: CalloutMarkerWidget): boolean {
        return other.char === this.char;
    }
}

export function buildListCalloutDecorations(
    doc: Text,
    visibleRanges: readonly { from: number; to: number }[],
    head = 0,
): DecorationSet {
    if (visibleRanges.length === 0) return Decoration.none;

    const ranges: Range<Decoration>[] = [];
    const seenLines = new Set<number>();

    for (const { from, to } of visibleRanges) {
        let pos = from;
        while (pos <= to) {
            const line = doc.lineAt(pos);
            if (!seenLines.has(line.number)) {
                seenLines.add(line.number);
                const matched = matchListCallout(line.text);
                if (matched) {
                    const absFrom = line.from + matched.charFrom;
                    const absTo = line.from + matched.charTo;
                    ranges.push(Decoration.line({
                        class: 'q-md-list-callout',
                        attributes: {
                            style: `--q-list-callout-color: var(--q-list-callout-${matched.callout.tone})`,
                        },
                    }).range(line.from));

                    if (!shouldRevealSyntax(doc, head, absFrom, absTo)) {
                        ranges.push(Decoration.replace({
                            widget: new CalloutMarkerWidget(matched.callout.char),
                        }).range(absFrom, absTo));
                    }
                }
            }
            if (line.to >= doc.length) break;
            pos = line.to + 1;
        }
    }

    return Decoration.set(ranges, true);
}

function listCalloutPreview(): Extension {
    return ViewPlugin.fromClass(class {
        decorations: DecorationSet = Decoration.none;

        constructor(view: EditorView) {
            this.read(view);
        }

        read(view: EditorView) {
            const blocks = codeBlocks(ensureEditorTree(view), view.state.doc, view.visibleRanges);
            this.decorations = buildListCalloutDecorations(
                view.state.doc,
                view.visibleRanges,
                previewCaret(view.state),
            ).update({ filter: from => !blocks.some(block => from >= block.from && from <= block.end) });
        }

        update(update: ViewUpdate) {
            if (update.docChanged || update.selectionSet || update.viewportChanged || syntaxTree(update.state) !== syntaxTree(update.startState)
                || update.state.facet(revealAtCaretFacet) !== update.startState.facet(revealAtCaretFacet)) {
                this.read(update.view);
            }
        }
    }, {
        decorations: (plugin) => plugin.decorations,
    });
}

const listCalloutTheme = EditorView.theme({
    '.q-md-list-callout': {
        position: 'relative',
        borderRadius: 'var(--q-rounded-md)',
    },
    '.q-md-list-item.q-md-list-callout::before': {
        content: '""',
        position: 'absolute',
        insetBlock: '0',
        insetInlineEnd: '0',
        insetInlineStart: 'var(--q-editor-list-level-indent, 0px)',
        borderRadius: 'inherit',
        backgroundColor: 'color-mix(in srgb, var(--q-list-callout-color) 12%, transparent)',
        pointerEvents: 'none',
        zIndex: '-1',
    },
    '.q-md-list-item.q-md-list-callout .q-md-list-callout-marker': {
        color: 'var(--q-list-callout-color)',
        fontWeight: 'var(--q-font-weight-bold)',
    },
});

export function listCalloutsExtension(enabled: boolean): Extension {
    if (!enabled) return [];
    return [listCalloutPreview(), listCalloutTheme];
}
