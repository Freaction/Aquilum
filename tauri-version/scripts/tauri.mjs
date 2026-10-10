#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setCargoTarget, CARGO_DEV, CARGO_RELEASE } from './lib/cargo-junction.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');

const args = process.argv.slice(2);
const command = args[0] || '';

if (command === 'build') {
  await setCargoTarget(CARGO_RELEASE);

  if (args.length === 1) {
    args.push('--features', 'updater');
  } else if (!args.includes('--features')) {
    args.splice(1, 0, '--features', 'updater');
  }
} else {
  await setCargoTarget(CARGO_DEV);
}

delete process.env.CARGO_TARGET_DIR;

const tauriCli = join(
  projectRoot,
  'node_modules',
  '@tauri-apps',
  'cli',
  'tauri.js',
);
execFileSync(process.execPath, [tauriCli, ...args], {
  cwd: projectRoot,
  stdio: 'inherit',
});
