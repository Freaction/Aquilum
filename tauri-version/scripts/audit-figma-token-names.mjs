#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const sourcePath = resolve(process.env.FIGMA_VARIABLES_PATH || join(root, 'variables.json'));
const strict = process.argv.includes('--strict');

if (!existsSync(sourcePath)) {
  console.error(`Missing ${sourcePath}`);
  process.exit(1);
}

const doc = JSON.parse(readFileSync(sourcePath, 'utf8'));
const records = [];
const supportedModes = new Map([
  ['Unit', ['Value']],
  ['Typography', ['Value']],
  ['Dimension Semantic', ['Value']],
  ['Color Primitives', ['Value']],
  ['Color Semantic', ['Light', 'Dark']],
]);

for (const collection of doc.collections ?? []) {
  for (const mode of collection.modes ?? []) {
    for (const variable of mode.variables ?? []) {
      records.push({ collection: collection.name, mode: mode.name, ...variable });
    }
  }
}

const logicalNames = new Map();
for (const record of records) {
  const key = `${record.collection}/${record.name}`;
  const entries = logicalNames.get(key) ?? [];
  entries.push(record);
  logicalNames.set(key, entries);
}

const groupBy = (items, keyFn) => {
  const groups = new Map();
  for (const item of items) {
    const key = keyFn(item);
    const group = groups.get(key) ?? [];
    group.push(item);
    groups.set(key, group);
  }
  return [...groups.values()].filter((group) => group.length > 1);
};

const duplicateNames = groupBy(records, (r) => `${r.collection}/${r.mode}/${r.name}`);
const duplicateValues = groupBy(
  records.filter((r) => !r.isAlias && r.value !== undefined),
  (r) => `${r.collection}/${r.mode}/${JSON.stringify(r.value)}`,
);
const duplicateAliases = groupBy(
  records.filter((r) => r.isAlias && r.value?.name),
  (r) => `${r.collection}/${r.mode}/${r.value.collection}/${r.value.name}`,
);

const namingIssues = [];
for (const [key, entries] of logicalNames) {
  const name = entries[0].name;
  const isStyle = entries.some((entry) => entry.mode === 'Style'
    || entry.type === 'typography'
    || entry.type === 'effect');
  const issues = [];
  if (!name.includes('/')) issues.push('missing slash hierarchy');
  if (/\s/.test(name)) issues.push('contains whitespace');
  if (!isStyle && /[A-Z]/.test(name)) issues.push('contains uppercase');
  if (name.includes('_')) issues.push('contains underscore');
  if (name.includes('//')) issues.push('contains empty path segment');
  if (issues.length) namingIssues.push({ key, name, issues });
}

const unsupportedModes = [];
for (const collection of doc.collections ?? []) {
  const allowed = supportedModes.get(collection.name) ?? [];
  for (const mode of collection.modes ?? []) {
    if (!allowed.includes(mode.name)) {
      unsupportedModes.push({
        collection: collection.name,
        mode: mode.name,
        variableCount: mode.variables?.length ?? 0,
      });
    }
  }
}

const counts = [...new Set(records.map((r) => r.collection))].map((collection) => {
  const names = [...new Set(records.filter((r) => r.collection === collection).map((r) => r.name))];
  return { collection, variables: names.length };
});

const printGroups = (title, groups, format) => {
  console.log(`\n${title}: ${groups.length}`);
  for (const group of groups) {
    console.log(`  - ${group.map(format).join(' = ')}`);
  }
};

console.log('=== Figma Token Naming Audit ===');
console.log(`Source: ${sourcePath}`);
console.log(`Logical variables: ${logicalNames.size}`);
console.log(`Raw mode entries: ${records.length}`);
console.log(`Naming issues: ${namingIssues.length}`);
console.log(`Unsupported modes: ${unsupportedModes.length}`);
console.log(`Duplicate definitions: ${duplicateNames.length}`);
console.log(`Duplicate raw values: ${duplicateValues.length}`);
console.log(`Duplicate aliases: ${duplicateAliases.length}`);
console.log('\nCounts by collection:');
for (const count of counts) console.log(`  ${count.collection}: ${count.variables}`);

if (namingIssues.length) {
  console.log('\n--- Naming issues ---');
  for (const issue of namingIssues) console.log(`  ${issue.key}: ${issue.issues.join(', ')}`);
}

if (unsupportedModes.length) {
  console.log('\n--- Not handled by tokens:sync ---');
  for (const mode of unsupportedModes) {
    console.log(`  ${mode.collection}/${mode.mode}: ${mode.variableCount} entries`);
  }
}

printGroups(
  '--- Exact duplicate definitions',
  duplicateNames,
  (r) => `${r.collection}/${r.mode}/${r.name}`,
);
printGroups(
  '--- Same raw value in one mode',
  duplicateValues,
  (r) => `${r.name} (${r.collection}/${r.mode})`,
);
printGroups(
  '--- Same alias target in one mode',
  duplicateAliases,
  (r) => `${r.name} → ${r.value.collection}/${r.value.name}`,
);

const hasBlockingIssues = namingIssues.length > 0 || duplicateNames.length > 0;
if (strict && hasBlockingIssues) process.exit(2);
