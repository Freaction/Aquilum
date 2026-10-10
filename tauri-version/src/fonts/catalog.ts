import iaMonoItalicUrl from './iAWriterMonoV-Italic.woff2?url';
import iaMonoUrl from './iAWriterMonoV.woff2?url';
import iaQuattroItalicUrl from './iAWriterQuattroV-Italic.woff2?url';
import iaQuattroUrl from './iAWriterQuattroV.woff2?url';
import interCyrillicItalicUrl from './inter-cyrillic-wght-italic.woff2?url';
import interCyrillicUrl from './inter-cyrillic-wght-normal.woff2?url';
import interLatinItalicUrl from './inter-latin-wght-italic.woff2?url';
import interLatinUrl from './inter-latin-wght-normal.woff2?url';

const IA_WRITER_MONO = 'iA Writer Mono';
export const IA_WRITER_QUATTRO = 'iA Writer Quattro';
const INTER = 'Inter';

export type FontFamily = typeof IA_WRITER_MONO | typeof IA_WRITER_QUATTRO | typeof INTER;
export type FontScope = 'ui' | 'editor' | 'reader';

interface FontFace {
  family: FontFamily;
  url: string;
  style: 'normal' | 'italic';
  weight: string;
  unicodeRange?: string;
}

const INTER_LATIN = 'U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD';
const INTER_CYRILLIC = 'U+0301,U+0400-045F,U+0490-0491,U+04B0-04B1,U+2116';

const FONT_FACES: FontFace[] = [
  { family: IA_WRITER_MONO, url: iaMonoUrl, style: 'normal', weight: '400 700' },
  { family: IA_WRITER_MONO, url: iaMonoItalicUrl, style: 'italic', weight: '400 700' },
  { family: IA_WRITER_QUATTRO, url: iaQuattroUrl, style: 'normal', weight: '400 700' },
  { family: IA_WRITER_QUATTRO, url: iaQuattroItalicUrl, style: 'italic', weight: '400 700' },
  { family: INTER, url: interLatinUrl, style: 'normal', weight: '100 900', unicodeRange: INTER_LATIN },
  { family: INTER, url: interLatinItalicUrl, style: 'italic', weight: '100 900', unicodeRange: INTER_LATIN },
  { family: INTER, url: interCyrillicUrl, style: 'normal', weight: '100 900', unicodeRange: INTER_CYRILLIC },
  { family: INTER, url: interCyrillicItalicUrl, style: 'italic', weight: '100 900', unicodeRange: INTER_CYRILLIC },
];

const GENERIC_FALLBACK: Record<FontFamily, string> = {
  [IA_WRITER_MONO]: 'monospace',
  [IA_WRITER_QUATTRO]: 'sans-serif',
  [INTER]: 'sans-serif',
};

export const SCOPE_FAMILIES: Record<FontScope, readonly FontFamily[]> = {
  ui: [INTER, IA_WRITER_QUATTRO],
  editor: [IA_WRITER_MONO, IA_WRITER_QUATTRO, INTER],
  reader: [IA_WRITER_QUATTRO, INTER, IA_WRITER_MONO],
};

export const BODY_WEIGHTS = [400, 450, 500, 550, 600] as const;
export const BOLD_WEIGHT = 700;

export function fontStack(family: FontFamily): string {
  return `"${family}", ${GENERIC_FALLBACK[family]}`;
}

function scopeFamily(scope: FontScope, family: string): FontFamily {
  const families = SCOPE_FAMILIES[scope];
  return families.find((candidate) => candidate === family) ?? families[0];
}

function bodyWeight(weight: number): number {
  return BODY_WEIGHTS.reduce((best, candidate) => (
    Math.abs(candidate - weight) < Math.abs(best - weight) ? candidate : best
  ));
}

export function knownFont<Font extends { fontFamily: string; fontWeight: number }>(
  scope: FontScope,
  font: Font,
): Font & { fontFamily: FontFamily } {
  return { ...font, fontFamily: scopeFamily(scope, font.fontFamily), fontWeight: bodyWeight(font.fontWeight) };
}

export function fontFaceCss(): string {
  return FONT_FACES.map((face) => [
    '@font-face {',
    `font-family: "${face.family}";`,
    `src: url("${new URL(face.url, import.meta.url).href}") format("woff2");`,
    `font-style: ${face.style};`,
    `font-weight: ${face.weight};`,
    'font-display: swap;',
    face.unicodeRange ? `unicode-range: ${face.unicodeRange};` : '',
    '}',
  ].join('')).join('\n');
}

export function installFontFaces(): void {
  const style = document.createElement('style');
  style.textContent = fontFaceCss();
  document.head.append(style);
}
