#!/usr/bin/env node
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  applyMix,
  colorsEqual,
  figmaNameToCss,
  parseColorMix,
  parseHex,
  parseRgb,
  rgbaToHex,
} from './lib/color-normalize.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const snapshotPath = join(__dirname, 'figma-color-snapshot.json');
const reportPath = join(__dirname, 'figma-color-diff.json');
const jsonOut = process.argv.includes('--json');
const strict = process.argv.includes('--strict');

if (!existsSync(snapshotPath)) {
  console.error('Missing scripts/figma-color-snapshot.json');
  console.error('Run: npm run tokens:diff');
  process.exit(1);
}

const snapshot = JSON.parse(readFileSync(snapshotPath, 'utf8'));

function parseCssVars(filePath) {
  const text = readFileSync(filePath, 'utf8');
  const map = new Map();
  const re = /(--q-[a-z0-9-]+)\s*:\s*([^;]+);/gi;
  for (const m of text.matchAll(re)) {
    map.set(m[1], m[2].trim());
  }
  return map;
}

const colors = parseCssVars(join(root, 'src/styles/tokens/colors.css'));
const semanticLight = parseCssVars(join(root, 'src/styles/tokens/semantic.css'));
const semanticDark = parseCssVars(join(root, 'src/styles/themes/dark.css'));

function unwrapVar(value) {
  const m = String(value).trim().match(/^var\(\s*(--q-[a-z0-9-]+)\s*\)$/i);
  return m ? m[1] : null;
}

function resolveColor(varName, tables, hops = 0) {
  if (!varName || hops > 30) return null;
  let raw = null;
  for (const table of tables) {
    if (table.has(varName)) {
      raw = table.get(varName);
      break;
    }
  }
  if (raw == null) return null;

  const asHex = parseHex(raw);
  if (asHex) return asHex;
  const asRgb = parseRgb(raw);
  if (asRgb) return asRgb;

  const mix = parseColorMix(raw);
  if (mix) {
    const base = resolveColor(mix.varName, tables, hops + 1);
    return applyMix(base, mix.pct);
  }

  const ref = unwrapVar(raw);
  if (ref) return resolveColor(ref, tables, hops + 1);

  return null;
}

function resolveAliasName(varName, tables, hops = 0) {
  if (!varName || hops > 30) return null;
  let raw = null;
  for (const table of tables) {
    if (table.has(varName)) {
      raw = table.get(varName);
      break;
    }
  }
  if (raw == null) return null;
  const ref = unwrapVar(raw);
  if (!ref) return null;
  const nested = unwrapVar(
    [...tables].map((t) => t.get(ref)).find(Boolean) ?? '',
  );
  if (nested && colors.has(ref) === false && colors.has(nested)) {
    return nested;
  }
  return ref;
}

function cssToFigmaName(cssVar) {
  const body = cssVar.replace(/^--q-/, '');
  const prefixes = [
    'black-alpha', 'white-alpha', 'blue-alpha', 'green-alpha', 'red-alpha',
    'amber-alpha', 'teal-alpha', 'purple-alpha', 'pink-alpha', 'gray-alpha',
    'yellow-alpha',
  ];
  for (const p of prefixes) {
    if (body === p || body.startsWith(`${p}-`)) {
      const rest = body.slice(p.length + 1);
      return rest ? `${p}/${rest}` : p;
    }
  }
  if (body === 'text-on-color') return 'text/on/color';
  const i = body.indexOf('-');
  if (i < 0) return body;
  return `${body.slice(0, i)}/${body.slice(i + 1)}`;
}

const primByName = new Map(snapshot.primitives.map((p) => [p.name, p]));
const semByName = new Map(snapshot.semantic.map((s) => [s.name, s]));

const report = {
  snapshotAt: snapshot.exportedAt,
  summary: {},
  primitiveMissingInProd: [],
  primitiveValueMismatch: [],
  semanticMissingInProd: [],
  semanticAliasMismatch: [],
  semanticDarkMismatch: [],
  prodOnlyFigmaShaped: [],
};

for (const p of snapshot.primitives) {
  const css = figmaNameToCss(p.name);
  if (!colors.has(css) && !semanticLight.has(css)) {
    report.primitiveMissingInProd.push({ figma: p.name, css, figmaHex: p.hex });
    continue;
  }
  const resolved = resolveColor(css, [colors, semanticLight]);
  const expected = { r: p.r, g: p.g, b: p.b, a: p.a };
  if (!colorsEqual(resolved, expected)) {
    report.primitiveValueMismatch.push({
      figma: p.name,
      css,
      figmaHex: p.hex,
      prodHex: resolved ? rgbaToHex(resolved) : null,
      prodRaw: colors.get(css) ?? semanticLight.get(css) ?? null,
    });
  }
}

for (const s of snapshot.semantic) {
  const css = figmaNameToCss(s.name);
  if (!semanticLight.has(css)) {
    report.semanticMissingInProd.push({ figma: s.name, css, light: s.light, dark: s.dark });
    continue;
  }

  const lightRaw = semanticLight.get(css);
  const lightRef = unwrapVar(lightRaw);
  const expectedLight = s.light ? figmaNameToCss(s.light) : null;
  if (expectedLight && lightRef !== expectedLight) {
    const resolvedLeaf = resolveAliasName(css, [semanticLight, colors]);
    if (resolvedLeaf !== expectedLight) {
      report.semanticAliasMismatch.push({
        figma: s.name,
        css,
        mode: 'light',
        figmaAlias: s.light,
        expectedCss: expectedLight,
        prodRaw: lightRaw,
        prodResolved: resolvedLeaf,
      });
    }
  }

  const expectedDark = s.dark ? figmaNameToCss(s.dark) : null;
  const darkRaw = semanticDark.get(css);
  if (s.light !== s.dark) {
    if (!darkRaw) {
      report.semanticDarkMismatch.push({
        figma: s.name,
        css,
        issue: 'missing-dark-override',
        figmaDark: s.dark,
        expectedCss: expectedDark,
      });
    } else {
      const darkRef = unwrapVar(darkRaw);
      if (expectedDark && darkRef !== expectedDark) {
        report.semanticDarkMismatch.push({
          figma: s.name,
          css,
          issue: 'wrong-dark-alias',
          figmaDark: s.dark,
          expectedCss: expectedDark,
          prodRaw: darkRaw,
        });
      }
    }
  } else if (darkRaw) {
    const darkRef = unwrapVar(darkRaw);
    if (expectedDark && darkRef !== expectedDark) {
      report.semanticDarkMismatch.push({
        figma: s.name,
        css,
        issue: 'unexpected-dark-diff',
        figmaDark: s.dark,
        expectedCss: expectedDark,
        prodRaw: darkRaw,
      });
    }
  }
}

for (const [css] of semanticLight) {
  if (!css.startsWith('--q-bg-') && !css.startsWith('--q-text-')
    && !css.startsWith('--q-icon-') && !css.startsWith('--q-border-')
    && !css.startsWith('--q-shadow-')) {
    continue;
  }
  const legacy = new Set([
    '--q-bg-primary', '--q-bg-elevated', '--q-bg-sunken', '--q-bg-hover', '--q-bg-active',
    '--q-bg-selected', '--q-bg-accent-hover', '--q-bg-tooltip', '--q-text-link',
    '--q-text-link-hover', '--q-text-on-accent', '--q-text-tooltip', '--q-border-subtle',
    '--q-border-overlay', '--q-border-hover',
  ]);
  if (legacy.has(css)) continue;

  const figmaGuess = cssToFigmaName(css);
  const hasMatch = [...semByName.keys()].some(
    (n) => n.toLowerCase() === figmaGuess.toLowerCase()
      || n.toLowerCase().replace(/\s+/g, '-') === figmaGuess.toLowerCase(),
  );
  if (!hasMatch && !primByName.has(figmaGuess)) {
    report.prodOnlyFigmaShaped.push({ css, guessedFigma: figmaGuess });
  }
}

report.summary = {
  primitivesFigma: snapshot.primitives.length,
  semanticFigma: snapshot.semantic.length,
  primitiveMissingInProd: report.primitiveMissingInProd.length,
  primitiveValueMismatch: report.primitiveValueMismatch.length,
  semanticMissingInProd: report.semanticMissingInProd.length,
  semanticAliasMismatch: report.semanticAliasMismatch.length,
  semanticDarkMismatch: report.semanticDarkMismatch.length,
  prodOnlyFigmaShaped: report.prodOnlyFigmaShaped.length,
};

writeFileSync(reportPath, JSON.stringify(report, null, 2));

if (jsonOut) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log('=== Figma ↔ Prod Color Diff ===\n');
  console.log(`Snapshot: ${snapshot.exportedAt}`);
  console.log(`Primitives Figma: ${report.summary.primitivesFigma}`);
  console.log(`Semantic Figma:   ${report.summary.semanticFigma}\n`);
  console.log(`Missing primitives in prod: ${report.summary.primitiveMissingInProd}`);
  console.log(`Primitive value mismatches: ${report.summary.primitiveValueMismatch}`);
  console.log(`Missing semantic in prod:   ${report.summary.semanticMissingInProd}`);
  console.log(`Semantic alias mismatches:  ${report.summary.semanticAliasMismatch}`);
  console.log(`Dark override mismatches:   ${report.summary.semanticDarkMismatch}`);
  console.log(`Prod-only Figma-shaped:     ${report.summary.prodOnlyFigmaShaped}`);
  console.log(`\nFull report: scripts/figma-color-diff.json`);

  const show = (title, items, fmt) => {
    if (!items.length) return;
    console.log(`\n--- ${title} (first 25) ---`);
    for (const item of items.slice(0, 25)) console.log(fmt(item));
    if (items.length > 25) console.log(`  … +${items.length - 25} more`);
  };

  show('Missing primitives', report.primitiveMissingInProd, (i) => `  ${i.figma} → ${i.css}`);
  show('Value mismatches', report.primitiveValueMismatch, (i) => `  ${i.figma}: figma=${i.figmaHex} prod=${i.prodHex} raw=${i.prodRaw}`);
  show('Missing semantic', report.semanticMissingInProd, (i) => `  ${i.figma} → ${i.css}`);
  show('Alias mismatches', report.semanticAliasMismatch, (i) => `  ${i.figma} [${i.mode}]: want ${i.expectedCss}, got ${i.prodRaw}`);
  show('Dark mismatches', report.semanticDarkMismatch, (i) => `  ${i.figma}: ${i.issue} want ${i.expectedCss} got ${i.prodRaw ?? '—'}`);
}

const actionable = report.summary.primitiveMissingInProd
  + report.summary.primitiveValueMismatch
  + report.summary.semanticMissingInProd
  + report.summary.semanticAliasMismatch
  + report.summary.semanticDarkMismatch;

process.exit(strict && actionable > 0 ? 1 : 0);
