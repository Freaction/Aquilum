import { describe, expect, it } from 'vitest';
import { renderMarkdownToHtml } from './markdownToHtml';

describe('renderMarkdownToHtml', () => {
  it('drops the frontmatter block the reader never sees', () => {
    const html = renderMarkdownToHtml('---\ntype: book\n---\n\nТекст заметки');
    expect(html).not.toContain('type');
    expect(html).toContain('Текст заметки');
  });

  it('renders headings, emphasis and inline code without markdown syntax', () => {
    const html = renderMarkdownToHtml('## Заголовок\n\n**жирный** и *курсив* и `код`');
    expect(html).toContain('<h2 class="q-print-heading q-print-heading--2">Заголовок</h2>');
    expect(html).toContain('<strong>жирный</strong>');
    expect(html).toContain('<em>курсив</em>');
    expect(html).toContain('<code class="q-print-code-inline">код</code>');
    expect(html).not.toContain('**');
  });

  it('renders nested lists and task markers', () => {
    const html = renderMarkdownToHtml('- один\n    - вложенный\n- [x] сделано\n- [ ] нет');
    expect(html).toContain('<ul class="q-print-list">');
    expect(html.match(/<ul class="q-print-list">/g)?.length).toBe(2);
    expect(html).toContain('q-print-task--done');
    expect(html).toContain('<span class="q-print-task"></span>');
    expect(html).not.toContain('[x]');
  });

  it('renders a GFM table with a header row', () => {
    const html = renderMarkdownToHtml('| A | B |\n| - | - |\n| 1 | 2 |');
    expect(html).toContain('<thead><tr><th>A</th><th>B</th></tr></thead>');
    expect(html).toContain('<tbody><tr><td>1</td><td>2</td></tr></tbody>');
  });

  it('keeps fenced code verbatim and escapes markup inside it', () => {
    const html = renderMarkdownToHtml('```ts\nconst a = 1 < 2;\n```');
    expect(html).toContain('<pre class="q-print-code"><code>const a = 1 &lt; 2;</code></pre>');
  });

  it('renders links, wiki links and hashtags the way the editor shows them', () => {
    const html = renderMarkdownToHtml('[текст](https://a.example) [[Заметка|Алиас]] #тег');
    expect(html).toContain('<a class="q-print-link" href="https://a.example">текст</a>');
    expect(html).toContain('<span class="q-print-wikilink">Алиас</span>');
    expect(html).toContain('<span class="q-print-hashtag">#тег</span>');
    expect(html).not.toContain('[[');
  });

  it('resolves image sources through the vault resolver', () => {
    const html = renderMarkdownToHtml('![схема](assets/a.png)', {
      resolveImageUrl: (target) => `asset://${target}`,
    });
    expect(html).toContain('src="asset://assets/a.png"');
    expect(html).toContain('alt="схема"');
  });

  it('renders blockquotes and horizontal rules without their markers', () => {
    const html = renderMarkdownToHtml('> цитата\n\n---\n\nдальше');
    expect(html).toContain('<blockquote class="q-print-quote">');
    expect(html).toContain('цитата');
    expect(html).toContain('<hr class="q-print-rule" />');
    expect(html).not.toContain('&gt; цитата');
  });

  it('escapes html so a note cannot inject markup into the export', () => {
    const html = renderMarkdownToHtml('обычный <script>alert(1)</script> текст');
    expect(html).not.toContain('<script>');
    expect(html).toContain('&lt;script&gt;');
  });
});
