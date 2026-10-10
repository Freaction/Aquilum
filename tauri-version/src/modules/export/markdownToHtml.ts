import { markdownLanguage } from '@codemirror/lang-markdown';
import type { MarkdownParser } from '@lezer/markdown';
import type { SyntaxNode, Tree } from '@lezer/common';
import { editorMarkdownExtensions } from '../../components/Editor/extensions/markdownConfig';
import { frontmatterRange } from '../docs/frontmatter';

interface MarkdownRenderOptions {
  resolveImageUrl?: (target: string) => string;
}

const HASHTAG_PATTERN = /(?:^|\s)(#[a-zA-Zа-яА-Я0-9_]+)(?=\s|$)/g;

const ESCAPED_CHARACTERS: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
};

const HEADING_LEVELS: Record<string, number> = {
  ATXHeading1: 1,
  ATXHeading2: 2,
  ATXHeading3: 3,
  ATXHeading4: 4,
  ATXHeading5: 5,
  ATXHeading6: 6,
  SetextHeading1: 1,
  SetextHeading2: 2,
};

const INLINE_WRAPPERS: Record<string, string> = {
  Emphasis: 'em',
  StrongEmphasis: 'strong',
  Strikethrough: 'del',
  Superscript: 'sup',
  Subscript: 'sub',
};

const DROPPED_MARKS = new Set([
  'HeaderMark',
  'QuoteMark',
  'ListMark',
  'EmphasisMark',
  'StrikethroughMark',
  'CodeMark',
  'CodeInfo',
  'LinkMark',
  'SuperscriptMark',
  'SubscriptMark',
  'TableDelimiter',
  'WikiLinkMark',
  'WikiLinkAliasMark',
]);

let cachedParser: MarkdownParser | null = null;

function editorParser(): MarkdownParser {
  if (!cachedParser) {
    const base = markdownLanguage.parser as unknown as MarkdownParser;
    cachedParser = base.configure([...editorMarkdownExtensions]);
  }
  return cachedParser;
}

export function escapeHtml(value: string): string {
  return value.replace(/[&<>"]/g, (character) => ESCAPED_CHARACTERS[character]);
}

function withHashtags(raw: string): string {
  let rendered = '';
  let consumed = 0;
  HASHTAG_PATTERN.lastIndex = 0;
  for (let match = HASHTAG_PATTERN.exec(raw); match; match = HASHTAG_PATTERN.exec(raw)) {
    const label = match[1];
    const start = match.index + match[0].length - label.length;
    rendered += escapeHtml(raw.slice(consumed, start));
    rendered += `<span class="q-print-hashtag">${escapeHtml(label)}</span>`;
    consumed = start + label.length;
  }
  return rendered + escapeHtml(raw.slice(consumed));
}

interface RenderContext {
  source: string;
  resolveImageUrl: (target: string) => string;
}

function slice(context: RenderContext, from: number, to: number): string {
  return context.source.slice(from, to);
}

function nodeText(context: RenderContext, node: SyntaxNode): string {
  return slice(context, node.from, node.to);
}

function childNodes(node: SyntaxNode): SyntaxNode[] {
  const children: SyntaxNode[] = [];
  for (let child = node.firstChild; child; child = child.nextSibling) children.push(child);
  return children;
}

function findChild(node: SyntaxNode, name: string): SyntaxNode | null {
  for (const child of childNodes(node)) if (child.name === name) return child;
  return null;
}

function renderInlineRange(
  parent: SyntaxNode,
  from: number,
  to: number,
  context: RenderContext,
): string {
  let rendered = '';
  let cursor = from;
  for (const child of childNodes(parent)) {
    if (child.to <= from || child.from >= to) continue;
    if (child.from > cursor) rendered += withHashtags(slice(context, cursor, child.from));
    rendered += renderInline(child, context);
    cursor = child.to;
  }
  if (cursor < to) rendered += withHashtags(slice(context, cursor, to));
  return rendered;
}

function renderInlineChildren(node: SyntaxNode, context: RenderContext): string {
  return renderInlineRange(node, node.from, node.to, context);
}

function labelRange(node: SyntaxNode): { from: number; to: number } | null {
  const marks = childNodes(node).filter((child) => child.name === 'LinkMark');
  if (marks.length < 2) return null;
  return { from: marks[0].to, to: marks[1].from };
}

function renderLink(node: SyntaxNode, context: RenderContext): string {
  const url = findChild(node, 'URL');
  const range = labelRange(node);
  const label = range
    ? renderInlineRange(node, range.from, range.to, context)
    : escapeHtml(nodeText(context, node));
  if (!url) return label;
  return `<a class="q-print-link" href="${escapeHtml(nodeText(context, url))}">${label}</a>`;
}

function renderImage(node: SyntaxNode, context: RenderContext): string {
  const url = findChild(node, 'URL');
  if (!url) return escapeHtml(nodeText(context, node));
  const range = labelRange(node);
  const alt = range ? slice(context, range.from, range.to) : '';
  const source = context.resolveImageUrl(nodeText(context, url));
  return `<img class="q-print-image" src="${escapeHtml(source)}" alt="${escapeHtml(alt)}" />`;
}

function renderWikiLink(node: SyntaxNode, context: RenderContext): string {
  const alias = findChild(node, 'WikiLinkAlias');
  const target = findChild(node, 'WikiLinkTarget');
  const label = alias ?? target;
  if (!label) return '';
  return `<span class="q-print-wikilink">${escapeHtml(nodeText(context, label))}</span>`;
}

function renderInline(node: SyntaxNode, context: RenderContext): string {
  const wrapper = INLINE_WRAPPERS[node.name];
  if (wrapper) return `<${wrapper}>${renderInlineChildren(node, context)}</${wrapper}>`;

  switch (node.name) {
    case 'InlineCode': {
      const marks = childNodes(node).filter((child) => child.name === 'CodeMark');
      const from = marks.length ? marks[0].to : node.from;
      const to = marks.length > 1 ? marks[marks.length - 1].from : node.to;
      return `<code class="q-print-code-inline">${escapeHtml(slice(context, from, to))}</code>`;
    }
    case 'Link':
      return renderLink(node, context);
    case 'Image':
      return renderImage(node, context);
    case 'WikiLink':
      return renderWikiLink(node, context);
    case 'Autolink':
    case 'URL': {
      const url = nodeText(context, node).replace(/^<|>$/g, '');
      return `<a class="q-print-link" href="${escapeHtml(url)}">${escapeHtml(url)}</a>`;
    }
    case 'HardBreak':
      return '<br />';
    case 'Escape':
      return escapeHtml(nodeText(context, node).slice(1));
    case 'Entity':
      return nodeText(context, node);
    case 'Emoji':
      return escapeHtml(nodeText(context, node));
    default:
      break;
  }

  if (DROPPED_MARKS.has(node.name)) return '';
  if (node.firstChild) return renderInlineChildren(node, context);
  return withHashtags(nodeText(context, node));
}

function fencedCodeText(node: SyntaxNode, context: RenderContext): string {
  const body = findChild(node, 'CodeText');
  if (body) return nodeText(context, body);
  const marks = childNodes(node).filter((child) => child.name === 'CodeMark');
  if (marks.length < 2) return '';
  return slice(context, marks[0].to, marks[marks.length - 1].from).replace(/^[^\n]*\n?/, '');
}

function indentedCodeText(node: SyntaxNode, context: RenderContext): string {
  return nodeText(context, node)
    .split('\n')
    .map((line) => line.replace(/^ {1,4}|^\t/, ''))
    .join('\n');
}

function renderTableRow(node: SyntaxNode, cell: 'th' | 'td', context: RenderContext): string {
  const cells = childNodes(node)
    .filter((child) => child.name === 'TableCell')
    .map((child) => `<${cell}>${renderInlineChildren(child, context)}</${cell}>`)
    .join('');
  return `<tr>${cells}</tr>`;
}

function renderTable(node: SyntaxNode, context: RenderContext): string {
  const header = findChild(node, 'TableHeader');
  const rows = childNodes(node)
    .filter((child) => child.name === 'TableRow')
    .map((child) => renderTableRow(child, 'td', context))
    .join('');
  const head = header ? `<thead>${renderTableRow(header, 'th', context)}</thead>` : '';
  return `<table class="q-print-table">${head}<tbody>${rows}</tbody></table>`;
}

function renderTask(node: SyntaxNode, context: RenderContext): string {
  const marker = findChild(node, 'TaskMarker');
  const done = marker ? /\[[xX]\]/.test(nodeText(context, marker)) : false;
  const from = marker ? marker.to : node.from;
  const box = `<span class="q-print-task${done ? ' q-print-task--done' : ''}"></span>`;
  return `${box}${renderInlineRange(node, from, node.to, context)}`;
}

function renderListItem(node: SyntaxNode, context: RenderContext): string {
  const blocks = childNodes(node).filter((child) => !DROPPED_MARKS.has(child.name));
  const body = blocks
    .map((child) => (child.name === 'Paragraph'
      ? renderInlineChildren(child, context)
      : renderBlock(child, context)))
    .join('');
  return `<li>${body}</li>`;
}

function renderList(node: SyntaxNode, context: RenderContext): string {
  const items = childNodes(node)
    .filter((child) => child.name === 'ListItem')
    .map((child) => renderListItem(child, context))
    .join('');
  if (node.name === 'BulletList') return `<ul class="q-print-list">${items}</ul>`;
  const first = node.firstChild ? nodeText(context, node).match(/^\s*(\d+)/) : null;
  const start = first && first[1] !== '1' ? ` start="${Number(first[1])}"` : '';
  return `<ol class="q-print-list"${start}>${items}</ol>`;
}

function renderBlockChildren(node: SyntaxNode, context: RenderContext): string {
  return childNodes(node)
    .filter((child) => !DROPPED_MARKS.has(child.name))
    .map((child) => renderBlock(child, context))
    .join('');
}

function renderBlock(node: SyntaxNode, context: RenderContext): string {
  const heading = HEADING_LEVELS[node.name];
  if (heading) {
    return `<h${heading} class="q-print-heading q-print-heading--${heading}">`
      + `${renderInlineChildren(node, context).trim()}</h${heading}>`;
  }

  switch (node.name) {
    case 'Paragraph':
      return `<p class="q-print-paragraph">${renderInlineChildren(node, context)}</p>`;
    case 'Blockquote':
      return `<blockquote class="q-print-quote">${renderBlockChildren(node, context)}</blockquote>`;
    case 'BulletList':
    case 'OrderedList':
      return renderList(node, context);
    case 'Task':
      return renderTask(node, context);
    case 'FencedCode':
      return `<pre class="q-print-code"><code>${escapeHtml(fencedCodeText(node, context))}</code></pre>`;
    case 'CodeBlock':
      return `<pre class="q-print-code"><code>${escapeHtml(indentedCodeText(node, context))}</code></pre>`;
    case 'HorizontalRule':
      return '<hr class="q-print-rule" />';
    case 'Table':
      return renderTable(node, context);
    case 'HTMLBlock':
    case 'CommentBlock':
      return `<p class="q-print-paragraph">${escapeHtml(nodeText(context, node))}</p>`;
    default:
      break;
  }

  if (node.firstChild) return renderBlockChildren(node, context);
  return `<p class="q-print-paragraph">${withHashtags(nodeText(context, node))}</p>`;
}

function documentBody(markdown: string): string {
  const frontmatter = frontmatterRange(markdown);
  return frontmatter ? markdown.slice(frontmatter.to).replace(/^\n+/, '') : markdown;
}

export function renderMarkdownToHtml(
  markdown: string,
  options: MarkdownRenderOptions = {},
): string {
  const source = documentBody(markdown).replace(/\r\n?/g, '\n');
  const tree: Tree = editorParser().parse(source);
  const context: RenderContext = {
    source,
    resolveImageUrl: options.resolveImageUrl ?? ((target) => target),
  };
  return renderBlockChildren(tree.topNode, context);
}
