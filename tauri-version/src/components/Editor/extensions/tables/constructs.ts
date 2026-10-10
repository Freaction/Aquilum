import type { Text } from '@codemirror/state';
import { parseMarkdownTable, serializeMarkdownTable } from './markdown';
import type { TableModel } from './model';
import { isSeparatorRow, splitTableRow, tableCellParser } from './rows';

export interface TableRange {
    from: number;
    contentTo: number;
    to: number;
    model: TableModel;
}

export function sanitizeCellValue(value: string): string {
    const normalized = value.replace(/\r\n?/g, '\n').replace(/\n/g, ' ');
    const tree = tableCellParser.parse(normalized);
    return normalized.replace(/(\\*)\|/g, (match, slashes: string, offset: number) => {
        if (slashes.length % 2 === 1) return match;
        let node = tree.resolveInner(offset + slashes.length, 1);
        while (node.parent && node.name !== 'WikiLink' && node.name !== 'InlineCode') node = node.parent;
        return slashes + (node.name === 'WikiLink' || node.name === 'InlineCode' ? '\\|' : '∣');
    }).trimEnd();
}

export function serializeTable(model: TableModel): string {
    return serializeMarkdownTable({
        ...model,
        cells: model.cells.map((row) => row.map(sanitizeCellValue)),
    });
}

function lineLooksLikeTableRow(line: string): boolean {
    return line.includes('|') && splitTableRow(line) != null;
}

function findCompleteTablesInLines(doc: Text, fromLine: number, toLine: number): TableRange[] {
    const ranges: TableRange[] = [];
    let lineNum = Math.max(1, fromLine);

    while (lineNum <= toLine && lineNum <= doc.lines) {
        const startLine = doc.line(lineNum);
        if (!lineLooksLikeTableRow(startLine.text) || isSeparatorRow(startLine.text)) {
            lineNum += 1;
            continue;
        }
        if (lineNum + 1 > doc.lines) break;
        if (!isSeparatorRow(doc.line(lineNum + 1).text)) {
            lineNum += 1;
            continue;
        }

        let endLineNum = lineNum + 1;
        while (endLineNum + 1 <= toLine && endLineNum + 1 <= doc.lines) {
            const next = doc.line(endLineNum + 1);
            if (!lineLooksLikeTableRow(next.text) || isSeparatorRow(next.text)) break;
            if (
                endLineNum + 2 <= doc.lines
                && isSeparatorRow(doc.line(endLineNum + 2).text)
            ) {
                break;
            }
            endLineNum += 1;
        }

        if (endLineNum < lineNum + 2) {
            lineNum += 1;
            continue;
        }

        const from = startLine.from;
        const metadataLine = endLineNum < doc.lines ? doc.line(endLineNum + 1) : null;
        if (metadataLine?.text.trim().startsWith('<!--q-table:')) endLineNum += 1;
        const contentTo = doc.line(endLineNum).to;
        const to = endLineNum < doc.lines ? doc.line(endLineNum + 1).from : doc.length;
        const model = parseMarkdownTable(doc.sliceString(from, contentTo));
        if (model) {
            ranges.push({ from, contentTo, to, model });
            lineNum = endLineNum + 1;
            continue;
        }
        lineNum += 1;
    }

    return ranges;
}

export function findTablesInDoc(doc: Text): TableRange[] {
    if (doc.length === 0) return [];
    return findCompleteTablesInLines(doc, 1, doc.lines);
}

export function tableOwnsDocEnd(doc: Text, table: TableRange): boolean {
    if (table.contentTo <= table.from || doc.length === 0) return false;
    return doc.lineAt(table.contentTo - 1).number === doc.lines;
}
