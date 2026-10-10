#!/usr/bin/env node
import { existsSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));

const exportResult = spawnSync(process.execPath, [join(__dirname, 'export-figma-colors.mjs')], {
  stdio: 'inherit',
  cwd: join(__dirname, '..'),
});
if (exportResult.status !== 0) process.exit(exportResult.status ?? 1);

const compareResult = spawnSync(
  process.execPath,
  [join(__dirname, 'compare-figma-colors.mjs'), ...process.argv.slice(2)],
  { stdio: 'inherit', cwd: join(__dirname, '..') },
);
process.exit(compareResult.status ?? 0);
