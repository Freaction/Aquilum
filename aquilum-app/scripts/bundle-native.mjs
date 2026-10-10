#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { copyFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const nativeRoot = join(projectRoot, 'native');
const tauriCli = join(projectRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const args = process.argv.slice(2);

const valueOf = (flag) => {
  const index = args.indexOf(flag);
  return index >= 0 ? args[index + 1] : undefined;
};

if (args[0] !== 'build') {
  execFileSync(process.execPath, [tauriCli, ...args], { cwd: nativeRoot, stdio: 'inherit' });
  process.exit(0);
}

const target = valueOf('--target');
const bundles = valueOf('--bundles');
const exe = process.platform === 'win32' ? '.exe' : '';

execFileSync(process.execPath, ['scripts/sync-version.mjs'], { cwd: projectRoot, stdio: 'inherit' });

const cargo = ['build', '--release', '-p', 'aquilum-native', '--features', 'installed', ...(target ? ['--target', target] : [])];
execFileSync('cargo', cargo, { cwd: projectRoot, stdio: 'inherit' });

const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: projectRoot, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
const release = join(metadata.target_directory, target ?? '', 'release');
await copyFile(join(release, `aquilum-native${exe}`), join(release, `aquilum-app${exe}`));

const bundle = ['bundle', ...(target ? ['--target', target] : []), ...(bundles ? ['--bundles', bundles] : [])];
execFileSync(process.execPath, [tauriCli, ...bundle], {
  cwd: nativeRoot,
  stdio: 'inherit',
  env: { ...process.env, CARGO_TARGET_DIR: metadata.target_directory },
});
