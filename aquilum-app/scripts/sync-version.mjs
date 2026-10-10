#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');
const packageJsonPath = join(projectRoot, 'package.json');
const tauriConfPath = join(projectRoot, 'native', 'tauri.conf.json');
const coreCargoTomlPath = join(projectRoot, 'core', 'Cargo.toml');
const nativeCargoTomlPath = join(projectRoot, 'native', 'Cargo.toml');

const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));
const version = packageJson.version;
if (!version) throw new Error('package.json has no version field');

async function setFilePattern(path, pattern, replacement) {
  const content = await readFile(path, 'utf8');
  const next = content.replace(pattern, replacement);
  if (content === next) return false;
  await writeFile(path, next, 'utf8');
  return true;
}

const tauriChanged = await setFilePattern(tauriConfPath, /"version"\s*:\s*"[^"]+"/, `"version": "${version}"`);
const coreChanged = await setFilePattern(coreCargoTomlPath, /^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
const nativeChanged = await setFilePattern(nativeCargoTomlPath, /^version\s*=\s*"[^"]+"/m, `version = "${version}"`);

if (tauriChanged || coreChanged || nativeChanged) {
  console.log(`Synced app version ${version} from package.json`);
}

console.log(version);
