import { EditorView } from '@codemirror/view';
import { HighlightStyle } from '@codemirror/language';
import { tags as defaultTags } from '@lezer/highlight';

export const aquilumEditorTheme = EditorView.theme({
    "&": {
        color: "var(--q-editor-text)",
        backgroundColor: "transparent",
    },
    "& > .cm-scroller": {
        paddingTop: "var(--q-space-12)",
        paddingBottom: "var(--q-editor-scroll-past-end)",
        overflowX: "clip",
    },
    ".cm-content": {
        tabSize: "var(--q-editor-tab-size)"
    },
    ".cm-placeholder": {
        color: "var(--q-editor-placeholder-color)"
    },
    ".cm-content .q-cm-frontmatter, .cm-content .q-cm-frontmatter *": {
        fontFamily: "var(--q-editor-font-family)",
        fontSize: "var(--q-editor-font-size)",
        fontWeight: "var(--q-editor-font-weight)",
        color: "var(--q-text-secondary)",
        fontStyle: "normal"
    },
    ".cm-content .q-md-code:has(.q-cm-frontmatter)": {
        padding: "0",
        borderRadius: "0",
        backgroundColor: "transparent"
    },
    ".cm-content .q-cm-frontmatter-key, .cm-content .q-cm-frontmatter-key *": {
        color: "var(--q-text-tertiary)"
    },
    ".cm-content .q-cm-frontmatter-value, .cm-content .q-cm-frontmatter-value *": {
        color: "var(--q-text-accent)"
    },
    ".cm-content .q-cm-frontmatter .q-md-link, .cm-content .q-cm-frontmatter .q-md-link *": {
        color: "var(--q-text-link)",
        textDecorationColor: "var(--q-text-link)"
    },
    ".q-md-code": {
        padding: "var(--q-space-2) var(--q-space-4)",
        borderRadius: "var(--q-editor-code-radius)",
        backgroundColor: "var(--q-editor-code-bg)",
        color: "var(--q-editor-text)",
        fontFamily: "var(--q-editor-code-font)"
    },
    ".q-md-link": {
        cursor: "pointer",
        color: "var(--q-text-link)",
        textDecoration: "underline",
        textDecorationColor: "var(--q-text-link)",
    },
    ".q-md-link-flow": {
        wordBreak: "break-all",
        overflowWrap: "anywhere",
    },
    ".q-md-hidden-syntax, .q-md-hidden-syntax *": {
        fontSize: "0",
        letterSpacing: "0",
        padding: "0",
        opacity: "0",
    },
    ".q-md-transparent-syntax": {
        opacity: "0"
    },
    ".q-wiki-link--unresolved": {
        opacity: "var(--q-opacity-60)"
    },
    ".q-md-syntax-char": {
        color: "var(--q-text-tertiary)"
    },
    ".q-md-hr-line-cm": {
        position: "relative",
        cursor: "text"
    },
    ".q-md-hr-line-cm::after": {
        content: '""',
        position: "absolute",
        top: "50%",
        left: "0",
        width: "100%",
        height: "var(--q-space-2, 2px)",
        transform: "translateY(-50%)",
        backgroundColor: "var(--q-icon-quiet)",
        pointerEvents: "none"
    }
});

const headingTags = [
    defaultTags.heading1,
    defaultTags.heading2,
    defaultTags.heading3,
    defaultTags.heading4,
    defaultTags.heading5,
    defaultTags.heading6,
];

const headingStyles = headingTags.map((tag, index) => ({
    tag,
    fontFamily: "var(--q-editor-heading-font)",
    fontSize: `var(--q-editor-heading-${index + 1}-size)`,
    fontWeight: "var(--q-editor-heading-weight)",
    color: "var(--q-editor-text)",
}));

export const markdownStyles = HighlightStyle.define([
    ...headingStyles,
    { tag: defaultTags.strong, fontWeight: "var(--q-font-weight-bold)" },
    { tag: defaultTags.emphasis, fontStyle: "italic" },
    { tag: defaultTags.strikethrough, textDecoration: "line-through" },
    { tag: defaultTags.link, class: "q-md-link" },
    { tag: defaultTags.url, class: "q-md-link q-md-syntax-char" },
    { tag: defaultTags.monospace, class: "q-md-code" },
    {
        tag: [defaultTags.processingInstruction, defaultTags.meta, defaultTags.punctuation, defaultTags.contentSeparator],
        class: "q-md-syntax-char"
    }
]);
