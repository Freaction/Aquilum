#!/usr/bin/env node
import { existsSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadVariablesDoc, parseVariablesJson } from './lib/parse-variables-json.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const sourcePath = resolve(process.env.FIGMA_VARIABLES_PATH || join(root, 'variables.json'));
const outPath = join(__dirname, 'figma-color-snapshot.json');

if (!existsSync(sourcePath)) {
  console.error(`Missing variables file: ${sourcePath}`);
  process.exit(1);
}

const parsed = parseVariablesJson(loadVariablesDoc(sourcePath));
const snapshot = {
  fileKey: 'ZWJ2vXmE06uiadJPo1HPVB',
  exportedAt: new Date().toISOString(),
  source: sourcePath,
  collections: {
    primitives: parsed.colorPrimitives.length,
    semantic: parsed.colorSemantic.length,
  },
  primitives: parsed.colorPrimitives,
  semantic: parsed.colorSemantic,
};

writeFileSync(outPath, JSON.stringify(snapshot, null, 2));
console.log(`Source: ${sourcePath}`);
console.log(`Wrote:  ${outPath}`);
console.log(`  primitives: ${snapshot.collections.primitives}`);
console.log(`  semantic:   ${snapshot.collections.semantic}`);
