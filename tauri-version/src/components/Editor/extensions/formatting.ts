import { StateCommand, EditorSelection, Prec } from "@codemirror/state";
import { syntaxTree } from "@codemirror/language";
import { keymap } from "@codemirror/view";
import type { SyntaxNode } from "@lezer/common";
import { codeMirrorKey, SHORTCUTS } from "../../../config/shortcuts";
import { findAncestor } from "./syntaxAncestor";

function createToggleCommand(
    targetNodeName: string,
    wrapString: string,
    markerRegex: RegExp
): StateCommand {
    return ({ state, dispatch }) => {
        let hasChanges = false;

        const spec = state.changeByRange(range => {
            const tree = syntaxTree(state);

            const isTarget = (node: SyntaxNode) => node.name === targetNodeName;
            const targetNode = findAncestor(tree.resolveInner(range.head, 1), isTarget)
                ?? findAncestor(tree.resolveInner(range.head, -1), isTarget);

            if (targetNode) {
                const text = state.sliceDoc(targetNode.from, targetNode.to);
                const markMatch = text.match(markerRegex);

                if (markMatch) {
                    hasChanges = true;
                    const markLen = markMatch[1].length;

                    const getNewPos = (p: number) => {
                        if (p <= targetNode.from) return p;
                        if (p <= targetNode.from + markLen) return targetNode.from;
                        if (p <= targetNode.to - markLen) return p - markLen;
                        if (p <= targetNode.to) return targetNode.to - (markLen * 2);
                        return p - (markLen * 2);
                    };

                    return {
                        changes: [
                            { from: targetNode.from, to: targetNode.from + markLen, insert: "" },
                            { from: targetNode.to - markLen, to: targetNode.to, insert: "" }
                        ],
                        range: EditorSelection.range(getNewPos(range.anchor), getNewPos(range.head))
                    };
                }
            }

            if (range.empty) {
                return { range };
            } else {
                hasChanges = true;
                return {
                    changes: [
                        { from: range.from, insert: wrapString },
                        { from: range.to, insert: wrapString }
                    ],
                    range: EditorSelection.range(range.anchor + wrapString.length, range.head + wrapString.length)
                };
            }
        });

        if (hasChanges) {
            dispatch(state.update(spec, { scrollIntoView: true, userEvent: "input" }));
        }

        return true;
    };
}

const toggleBoldCommand = createToggleCommand("StrongEmphasis", "**", /^(\*\*|__)(.*)(\*\*|__)$/s);
const toggleItalicCommand = createToggleCommand("Emphasis", "*", /^(\*|_)(.*)(\*|_)$/s);
const toggleStrikeCommand = createToggleCommand("Strikethrough", "~~", /^(~~)(.*)(~~)$/s);

export const formattingKeymap = Prec.highest(keymap.of([
    { key: codeMirrorKey(SHORTCUTS.BOLD), run: toggleBoldCommand },
    { key: codeMirrorKey(SHORTCUTS.ITALIC), run: toggleItalicCommand },
    { key: codeMirrorKey(SHORTCUTS.STRIKETHROUGH), run: toggleStrikeCommand }
]));
