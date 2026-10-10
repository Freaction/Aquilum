import { useMemo } from 'react';
import { EditorView, placeholder } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { syntaxHighlighting, indentUnit } from '@codemirror/language';

import { livePreviewExtension } from './livePreviewPlugin';
import { listCalloutsExtension, outlineExtension } from './outline';
import { tablesExtension } from './tables';
import { bookCalloutTheme, type BookCalloutReadRequest } from './bookCallout';
import { dataviewTheme } from './dataview';
import { blockquoteTheme } from './blockquotePreview';
import { readerQuoteTheme } from './readerQuote';
import { imageEmbedExtension } from './image';
import { aquilumEditorTheme, markdownStyles } from './theme';
import { formattingKeymap } from './formatting';
import { aquilumCodeMirrorTheme } from '../../Common/codeMirror';
import { editorLinkExtension } from './links';
import { blockWidthExtension } from './blockWidth';
import { editorMarkdownSupport } from './markdownConfig';
import { codeBlockExtension } from './codeBlock';
import { hashtagExtension } from './hashtags';
import { externalRevealExtension } from './externalReveal';
import { frontmatterBlock } from './frontmatterBlock';
import { frontmatterPaste } from './frontmatterPaste';
import { wikiHoverHighlight } from './wikiHoverHighlight';
import { smartDashExtension } from './smartDash';
import { pageSearchExtension } from './pageSearch';
import { noteSuggestExtension } from './suggest';
import { bodySetup } from './bodySetup';
import { DEFAULT_PLUGIN_SETTINGS, type PluginSettings } from '../../../modules/settings';
import { coloredTagsExtension } from './plugins/coloredTags';
import { cursorTrailExtension } from './plugins/cursorTrail';
import { readingModeExtension } from './plugins/readingMode';
import { codeStylerExtension } from './plugins/codeStyler';
import { advancedTablesConfig } from './tables/advancedTables';
import type { LinkDisposition, WikiLinkResolver } from '../../../modules/links';

export function useEditorExtensions(
    resolveWikiLinks: WikiLinkResolver,
    onOpenWikiLink: (target: string, disposition: LinkDisposition) => void,
    onOpenExternalUrl: (url: string) => void,
    smartDashes = true,
    listCallouts = true,
    autoLinkTitle = true,
    workspacePath: string | null = null,
    onReadBookCallout?: (request: BookCalloutReadRequest) => void,
    notePath: () => string = () => '',
    linkSuggest = true,
    linkSuggestMinChars = 2,
    plugins: PluginSettings = DEFAULT_PLUGIN_SETTINGS,
) {
    return useMemo(() => {
        return [
            bodySetup,
            EditorState.tabSize.of(4),
            noteSuggestExtension({
                enabled: linkSuggest,
                workspacePath,
                minChars: linkSuggestMinChars,
            }),
            outlineExtension,
            listCalloutsExtension(listCallouts),
            tablesExtension,
            blockWidthExtension(),
            bookCalloutTheme,
            dataviewTheme,
            blockquoteTheme,
            readerQuoteTheme,
            imageEmbedExtension,
            formattingKeymap,
            indentUnit.of("\t"),
            editorMarkdownSupport,
            syntaxHighlighting(markdownStyles),
            frontmatterBlock,
            frontmatterPaste,
            livePreviewExtension({
                resolveWikiLinks,
                workspacePath,
                notePath,
                onOpenWikiLink,
                onOpenExternalUrl,
                onReadBook: onReadBookCallout ?? (() => {}),
            }),
            editorLinkExtension(onOpenWikiLink, onOpenExternalUrl, autoLinkTitle),
            codeBlockExtension,
            hashtagExtension,
            externalRevealExtension,
            wikiHoverHighlight,
            smartDashExtension(smartDashes),
            aquilumCodeMirrorTheme,
            aquilumEditorTheme,
            pageSearchExtension,
            EditorView.lineWrapping,
            placeholder("Начните писать текст..."),
            // выключенные плагины не добавляют расширения.
            ...(plugins.coloredTags.enabled ? coloredTagsExtension(plugins.coloredTags) : []),
            ...(plugins.cursorTrail.enabled ? [cursorTrailExtension] : []),
            ...(plugins.readingMode.enabled ? readingModeExtension() : []),
            ...(plugins.codeStyler.enabled ? [codeStylerExtension(plugins.codeStyler)] : []),
            ...(plugins.advancedTables.enabled ? [advancedTablesConfig.of(plugins.advancedTables)] : []),
        ];
    }, [plugins, autoLinkTitle, linkSuggest, linkSuggestMinChars, listCallouts, onOpenExternalUrl, onOpenWikiLink, onReadBookCallout, resolveWikiLinks, smartDashes, workspacePath]);
}
