#!/usr/bin/env node
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  ALPHA_STEPS,
  COLOR_FAMILIES,
  NEUTRAL_ALPHA_FAMILIES,
  alphaPctFromName,
  cssBlock,
  figmaRef,
  figmaToCssVar,
  pxToTokenValue,
} from './lib/figma-token-utils.mjs';
import { loadVariablesDoc, parseVariablesJson } from './lib/parse-variables-json.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const tokensDir = join(root, 'src/styles/tokens');
const themesDir = join(root, 'src/styles/themes');
const variablesPath = resolve(process.env.FIGMA_VARIABLES_PATH || join(root, 'variables.json'));

if (!existsSync(variablesPath)) {
  console.error(`Missing ${variablesPath}`);
  process.exit(1);
}

const manifest = parseVariablesJson(loadVariablesDoc(variablesPath));
const GENERATED = `/* Generated from variables.json — do not edit. Run: npm run tokens:sync */\n`;

function writeGenerated(path, body) {
  writeFileSync(path, GENERATED + body, 'utf8');
  console.log('wrote', path.slice(root.length + 1).replace(/\\/g, '/'));
}

function writeAlphaRamp(lines, family, steps = ALPHA_STEPS) {
  const mainRef = figmaRef(`${family}/main`);
  const existing = new Set(
    manifest.colorPrimitives
      .filter((p) => p.name.startsWith(`${family}/`))
      .map((p) => p.name.slice(family.length + 1)),
  );
  for (const step of steps) {
    if (step === 'main') continue;
    if (!existing.has(step)) continue;
    const pct = alphaPctFromName(step);
    if (pct == null) continue;
    lines.push(
      `  ${figmaToCssVar(`${family}/${step}`)}: color-mix(in srgb, ${mainRef} ${pct}%, transparent);`,
    );
  }
}

function generateColors() {
  const lines = [':root {'];
  const opaque = manifest.colorPrimitives.filter((t) => t.a >= 0.999);
  for (const t of opaque.sort((a, b) => a.name.localeCompare(b.name))) {
    const six = `#${t.hex.replace(/^#/, '').slice(0, 6).toLowerCase()}`;
    lines.push(`  ${figmaToCssVar(t.name)}: ${six};`);
  }

  for (const family of COLOR_FAMILIES) {
    const hasMain = opaque.some((t) => t.name === `${family}-alpha/main`);
    const has500 = opaque.some((t) => t.name === `${family}/500`);
    if (!hasMain && has500) {
      lines.push(`  ${figmaToCssVar(`${family}-alpha/main`)}: ${figmaRef(`${family}/500`)};`);
    }
  }
  if (!opaque.some((t) => t.name === 'gray-alpha/main') && opaque.some((t) => t.name === 'gray/500')) {
    lines.push(`  ${figmaToCssVar('gray-alpha/main')}: ${figmaRef('gray/500')};`);
  }
  if (!opaque.some((t) => t.name === 'yellow-alpha/main')
    && manifest.colorPrimitives.some((t) => t.name === 'yellow-alpha/60')) {
    lines.push(`  ${figmaToCssVar('yellow-alpha/main')}: #ffea00;`);
  }

  lines.push('');
  lines.push('  /* Alpha ramps — only steps that exist in variables.json */');
  for (const family of NEUTRAL_ALPHA_FAMILIES) writeAlphaRamp(lines, family);
  for (const family of COLOR_FAMILIES) writeAlphaRamp(lines, `${family}-alpha`);
  writeAlphaRamp(lines, 'gray-alpha');
  writeAlphaRamp(lines, 'yellow-alpha', ['60']);

  lines.push('}');
  writeGenerated(join(tokensDir, 'colors.css'), `${lines.join('\n')}\n`);
}

function generateUnit() {
  const lines = manifest.unit.map((t) => {
    const keepPx = Math.abs(t.value) >= 9999;
    return `  ${figmaToCssVar(t.name)}: ${pxToTokenValue(t.value, { keepPx })};`;
  });
  writeGenerated(join(tokensDir, 'unit.css'), cssBlock(':root', lines));
}

function generateDimension() {
  const lines = manifest.dimension.map((t) => {
    if (t.alias && manifest.unit.some((u) => u.name === t.alias)) {
      return `  ${figmaToCssVar(t.name)}: ${figmaRef(t.alias)};`;
    }
    if (t.value == null) return null;
    const keepPx = t.name === 'radius/full';
    return `  ${figmaToCssVar(t.name)}: ${pxToTokenValue(t.value, { keepPx })};`;
  }).filter(Boolean);
  writeGenerated(join(tokensDir, 'dimension.css'), cssBlock(':root', lines));
}

function generateTypography() {
  const lines = [];
  const fontStacks = {
    'font/family/ui': "'Inter', sans-serif",
    'font/family/editor': "'iA Writer Mono', monospace",
    'font/family/mono': "'iA Writer Mono', monospace",
    'font/family/heading': 'var(--q-font-family-editor)',
  };
  for (const t of manifest.typography) {
    const value = typeof t.value === 'number'
      ? Math.round(t.value * 10000) / 10000
      : t.value;
    if (fontStacks[t.name]) {
      lines.push(`  ${figmaToCssVar(t.name)}: ${fontStacks[t.name]};`);
    } else if (typeof value === 'string' && value.includes('/')) {
      lines.push(`  ${figmaToCssVar(t.name)}: ${figmaRef(value)};`);
    } else if (typeof value === 'string') {
      lines.push(`  ${figmaToCssVar(t.name)}: '${value}';`);
    } else if (t.name.startsWith('font/weight/')) {
      lines.push(`  ${figmaToCssVar(t.name)}: ${value};`);
    } else if (t.name.startsWith('font/letter-spacing/')) {
      lines.push(`  ${figmaToCssVar(t.name)}: ${value}em;`);
    } else if (t.name.startsWith('font/line-height/')) {
      lines.push(`  ${figmaToCssVar(t.name)}: ${value};`);
    } else {
      lines.push(`  ${figmaToCssVar(t.name)}: ${pxToTokenValue(value)};`);
    }
  }
  writeGenerated(join(tokensDir, 'typography.css'), cssBlock(':root', lines));
}

function generateSemantic() {
  const lightLines = [];
  const darkLines = ['  color-scheme: dark;'];

  for (const t of manifest.colorSemantic) {
    lightLines.push(`  ${figmaToCssVar(t.name)}: ${figmaRef(t.light)};`);
    if (t.light !== t.dark) {
      darkLines.push(`  ${figmaToCssVar(t.name)}: ${figmaRef(t.dark)};`);
    }
  }

  const compat = [
    '',
    '  /* Legacy aliases — migrate usages to Figma names */',
    '  --q-white: var(--q-white-alpha-main);',
    '  --q-bg-primary: var(--q-bg-canvas);',
    '  --q-bg-elevated: var(--q-bg-canvas);',
    '  --q-bg-sunken: var(--q-bg-surface);',
    '  --q-text-link: var(--q-text-accent);',
    '  --q-text-link-hover: var(--q-blue-600);',
    '  --q-rounded-sm: var(--q-radius-sm);',
    '  --q-rounded-md: var(--q-radius-md);',
    '  --q-rounded-lg: var(--q-radius-lg);',
    '  --q-rounded-xl: var(--q-radius-xl);',
    '  --q-rounded-2xl: var(--q-radius-2xl);',
    '  --q-rounded-full: var(--q-radius-full);',
    '',
    '  /* App tokens (not in Figma collections) */',
    '  --q-caret-color: var(--q-bg-accent);',
    '  --q-caret-width: var(--q-border-3);',
    '  --q-caret-radius: var(--q-radius-sm);',
    '  --q-caret-scale-y: 1.4;',
    '  --q-icon-tertiary: var(--q-black-alpha-60);',
    '  --q-scrollbar-track: transparent;',
    '  --q-scrollbar-thumb: var(--q-black-alpha-10);',
    '  --q-scrollbar-thumb-hover: var(--q-black-alpha-20);',
    '  --q-scrollbar-width: var(--q-size-16);',
    '  --q-selection-bg: color-mix(in srgb, var(--q-caret-color) 25%, transparent);',
    '  --q-selection-match-bg: var(--q-purple-alpha-30);',
    '  --q-selection-text: inherit;',
  ];

  for (const u of manifest.unit) {
    const short = u.name.replace('unit/', '');
    compat.push(`  --q-space-${short}: var(${figmaToCssVar(u.name)});`);
  }

  darkLines.push(
    '',
    '  /* App dark overrides (component chrome) */',
    '  --q-icon-tertiary: var(--q-white-alpha-60);',
    '  --q-scrollbar-thumb: var(--q-white-alpha-10);',
    '  --q-scrollbar-thumb-hover: var(--q-white-alpha-20);',
    '  --q-selection-bg: color-mix(in srgb, var(--q-caret-color) 25%, transparent);',
    '  --q-selection-match-bg: var(--q-purple-alpha-60);',
    '  --q-sidebar-item-hover-bg: var(--q-bg-surface-hover);',
    '  --q-sidebar-item-active-bg: var(--q-bg-surface-hover);',
    '  --q-search-result-bg: var(--q-white-alpha-03);',
    '  --q-tab-bg-hover: var(--q-white-alpha-06);',
    '  --q-editor-bg: var(--q-gray-900);',
    '  --q-tab-bg-active: var(--q-editor-bg);',
  );

  writeGenerated(join(tokensDir, 'semantic.css'), cssBlock(':root', [...lightLines, ...compat]));
  writeGenerated(join(themesDir, 'dark.css'), cssBlock('[data-theme="dark"]', darkLines));
}

console.log(`Source: ${variablesPath}`);
console.log('Counts:', manifest.counts);
generateColors();
generateUnit();
generateDimension();
generateTypography();
generateSemantic();
console.log('Done. Diff: npm run tokens:diff');
