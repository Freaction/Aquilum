import type { Text } from '@codemirror/state';
import type { MarkdownConfig } from '@lezer/markdown';

const INDENT_COLUMNS = 4;
const OUTLINE_ITEM_PATTERN = /^([ \t]*)(?:(-)|(\d+)([.)])) /;

type OutlineMarker =
    | { kind: 'bullet'; text: '-'; number: null }
    | { kind: 'ordered'; text: string; number: number };

export interface OutlineItem {
    indent: string;
    marker: OutlineMarker;
    contentFrom: number;
}

interface AnalyzedOutlineItem {
    lineNumber: number;
    item: OutlineItem;
    level: number | null;
    hasParent: boolean;
    showBullet: boolean;
    isContinuation: boolean;
}

export function orderedSuffix(markerText: string): '.' | ')' {
    return markerText.endsWith(')') ? ')' : '.';
}

export function matchOutlineItem(text: string): OutlineItem | null {
    const match = OUTLINE_ITEM_PATTERN.exec(text);
    if (!match) return null;

    const marker: OutlineMarker = match[2]
        ? { kind: 'bullet', text: '-', number: null }
        : { kind: 'ordered', text: `${match[3]}${match[4]}`, number: Number(match[3]) };

    return {
        indent: match[1],
        marker,
        contentFrom: match[0].length,
    };
}

export function continuationPrefix(item: OutlineItem): string {
    if (item.marker.kind === 'bullet') return '- ';
    return `${item.marker.number + 1}${orderedSuffix(item.marker.text)} `;
}

export function markerPrefix(item: OutlineItem): string {
    return `${item.marker.text} `;
}

export function continuationIndent(item: OutlineItem): string {
    return `${item.indent}${' '.repeat(item.marker.text.length + 1)}`;
}

export function reduceOutlineIndent(indent: string): string | null {
    if (!indent) return null;
    if (indent.endsWith('\t')) return indent.slice(0, -1);
    const spaces = indent.match(/ {1,4}$/);
    if (spaces) return indent.slice(0, -spaces[0].length);
    return null;
}

export function getOutlineLevel(indent: string): number | null {
    let columns = 0;
    for (const character of indent) {
        columns += character === '\t' ? INDENT_COLUMNS - columns % INDENT_COLUMNS : 1;
    }
    return columns % INDENT_COLUMNS === 0 ? columns / INDENT_COLUMNS : null;
}

function leadingIndent(text: string): string {
    return /^[ \t]*/.exec(text)?.[0] ?? '';
}

export function openOutlineItem(doc: Text, lineNumber: number): OutlineItem | null {
    const indent = leadingIndent(doc.line(lineNumber).text);
    if (!indent) return null;

    for (let number = lineNumber - 1; number >= 1; number--) {
        const { text } = doc.line(number);
        if (!text.trim()) return null;
        const item = matchOutlineItem(text);
        if (item) return continuationIndent(item) === indent ? item : null;
        if (leadingIndent(text) !== indent) return null;
    }
    return null;
}

export function outlineContextStart(doc: Text, fromLine: number): number {
    for (let number = fromLine; number >= 1; number--) {
        const { text } = doc.line(number);
        if (text.trim() === '') continue;
        const item = matchOutlineItem(text);
        const indent = item?.indent ?? leadingIndent(text);
        if (getOutlineLevel(indent) === 0) return number;
    }
    return 1;
}

export function analyzeOutlineLines(lines: readonly string[]): AnalyzedOutlineItem[] {
    const result: AnalyzedOutlineItem[] = [];
    const structure = lines.map(text => {
        const item = matchOutlineItem(text);
        const indent = item?.indent ?? leadingIndent(text);
        return { item, level: getOutlineLevel(indent), blank: text.trim() === '' };
    });

    let open: AnalyzedOutlineItem | null = null;

    for (let index = 0; index < structure.length; index++) {
        const { item, level } = structure[index];
        if (!item) {
            if (open === null || !lines[index].startsWith(continuationIndent(open.item))) {
                open = null;
                continue;
            }
            result.push({ ...open, lineNumber: index + 1, showBullet: false, isContinuation: true });
            continue;
        }

        let hasParent = level === 0;
        if (level !== null && level > 0) {
            for (let parent = index - 1; parent >= 0; parent--) {
                const candidate = structure[parent];
                if (candidate.blank || candidate.level === null || candidate.level >= level) continue;
                hasParent = candidate.level === level - 1 && candidate.item !== null;
                break;
            }
        }

        const analyzed: AnalyzedOutlineItem = {
            lineNumber: index + 1,
            item,
            level,
            hasParent,
            showBullet: item.marker.kind === 'bullet' && hasParent,
            isContinuation: false,
        };
        result.push(analyzed);
        open = hasParent ? analyzed : null;
    }

    return result;
}

export const outlineMarkdownConfig: MarkdownConfig = {
    remove: ['SetextHeading'],
    parseBlock: [{
        name: 'AquilumOutline',
        before: 'IndentedCode',
        parse(_context, line) {
            if (matchOutlineItem(line.text.slice(line.pos))) {
                line.moveBase(line.pos);
            }
            return false;
        },
        endLeaf(_context, line) {
            return matchOutlineItem(line.text.slice(line.pos)) !== null;
        },
    }],
};
