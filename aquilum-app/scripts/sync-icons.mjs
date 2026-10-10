#!/usr/bin/env node
import { execFileSync, execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptsDir, '..');
const source = join(projectRoot, 'Aquilum-logo.png');
const output = join(projectRoot, 'native', 'icons');
// `tauri icon` does not produce byte-identical files (icon.icns differs on every run), so icons are
// regenerated only when the logo itself changed; the hash of the logo they were built from is kept.
const sourceHashPath = join(output, '.source-hash');
const tauriBin = join(
  projectRoot,
  'node_modules',
  '.bin',
  process.platform === 'win32' ? 'tauri.cmd' : 'tauri',
);

if (!existsSync(source)) {
  throw new Error(`Icon source not found: ${source}`);
}
if (!existsSync(tauriBin)) {
  throw new Error('Tauri CLI is not installed. Run npm install first.');
}

const sourceHash = createHash('sha256').update(readFileSync(source)).digest('hex');
const builtFrom = existsSync(sourceHashPath) ? readFileSync(sourceHashPath, 'utf8').trim() : '';
if (builtFrom === sourceHash && existsSync(join(output, 'icon.icns'))) {
  process.exit(0);
}

console.log(`Updating Tauri icons from ${source}`);
if (process.platform === 'win32') {
  execSync(`"${tauriBin}" icon "${source}" --output "${output}"`, {
    cwd: projectRoot,
    stdio: 'inherit',
  });
} else {
  execFileSync(tauriBin, ['icon', source, '--output', output], {
    cwd: projectRoot,
    stdio: 'inherit',
  });
}
writeFileSync(sourceHashPath, `${sourceHash}
`);
console.log('Tauri icons updated. Restart the running dev app to see the new icon.');
