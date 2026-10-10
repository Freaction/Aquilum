import { describe, expect, it } from 'vitest';
import { readerStyleCss } from './readerStyle';
import { DEFAULT_READER_SETTINGS } from '../../modules/settings';

const palette = { text: '#fff', background: '#000', muted: '#aaa' };

describe('readerStyleCss', () => {
  it('carries font, spacing and justification into the book page', () => {
    const css = readerStyleCss(
      { ...DEFAULT_READER_SETTINGS, fontSizeBase: 21, lineHeight: 1.9 },
      palette,
    );
    expect(css).toContain('font-size: 21px');
    expect(css).toContain('line-height: 1.9');
    expect(css).toContain('text-align: justify');
    expect(css).toContain('hyphens: auto');
  });

  it('drops justification and hyphenation when they are off', () => {
    const css = readerStyleCss(
      { ...DEFAULT_READER_SETTINGS, justify: false, hyphenate: false },
      palette,
    );
    expect(css).toContain('text-align: start');
    expect(css).toContain('hyphens: manual');
  });

  it('overrides the font the book brings with it and ships its faces into the book', () => {
    const css = readerStyleCss({ ...DEFAULT_READER_SETTINGS, fontFamily: 'Inter' }, palette);
    expect(css).toContain('font-family: "Inter", sans-serif !important');
    expect(css).toContain('@font-face {font-family: "Inter";');
    expect(css).toContain('@font-face {font-family: "iA Writer Quattro";');
  });

  it('keeps bold elements bold whatever the body weight is', () => {
    const css = readerStyleCss({ ...DEFAULT_READER_SETTINGS, fontWeight: 550 }, palette);
    expect(css).toMatch(/:is\(b, strong, h1, h2, h3, h4, h5, h6, th\) \*\) \{\s+font-weight: 550;/);
    expect(css).toContain(':is(b, strong, h1, h2, h3, h4, h5, h6, th), :is(b, strong, h1, h2, h3, h4, h5, h6, th) * { font-weight: 700; }');
  });
});
