#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, rm, copyFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');
const repoRoot = resolve(projectRoot, '..');
const nativeRoot = join(projectRoot, 'native');
const artifactsRoot = join(repoRoot, '.artifacts');
const tauriCli = join(projectRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const releaseRepo = 'Freaction/Aquilum';

const packageJson = JSON.parse(await readFile(join(projectRoot, 'package.json'), 'utf8'));
const version = packageJson.version;
if (!version) throw new Error('package.json has no version field');

console.log(`\x1b[36mСборка нативного релиза Aquilum v${version}...\x1b[0m\n`);

const signingKeyPath = join(process.env.USERPROFILE || process.env.HOME, '.tauri', 'aquilum.key');
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  if (!existsSync(signingKeyPath)) {
    throw new Error(`Private signing key not found at: ${signingKeyPath}\nGenerate once: npx tauri signer generate -w "${signingKeyPath}"\nThen set password for this session:\n  set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=your-password`);
  }
  process.env.TAURI_SIGNING_PRIVATE_KEY = await readFile(signingKeyPath, 'utf8');
  process.env.TAURI_SIGNING_PRIVATE_KEY_PATH = signingKeyPath;
}
if (!process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
  throw new Error('TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not set in this terminal.\nRun:\n  set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=your-password');
}

execFileSync(process.execPath, ['scripts/sync-version.mjs'], { cwd: projectRoot, stdio: 'inherit' });
execFileSync(process.execPath, ['scripts/sync-icons.mjs'], { cwd: projectRoot, stdio: 'inherit' });

const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: projectRoot, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
const targetDir = process.env.CARGO_TARGET_DIR || metadata.target_directory;
const exe = process.platform === 'win32' ? '.exe' : '';

console.log(`\x1b[32mКомпиляция с использованием кэша: ${targetDir}\x1b[0m`);
execFileSync('cargo', ['build', '--release', '-p', 'aquilum-native', '--features', 'installed'], {
  cwd: projectRoot,
  stdio: 'inherit',
  env: { ...process.env, CARGO_TARGET_DIR: targetDir },
});

const release = join(targetDir, 'release');
await copyFile(join(release, `aquilum-native${exe}`), join(release, `aquilum-app${exe}`));

const bundleDir = join(release, 'bundle');
if (existsSync(bundleDir)) {
  await rm(bundleDir, { recursive: true, force: true });
}

console.log('\x1b[32mУпаковка установщика NSIS через Tauri CLI...\x1b[0m');
execFileSync(process.execPath, [tauriCli, 'bundle', '--bundles', 'nsis'], {
  cwd: nativeRoot,
  stdio: 'inherit',
  env: { ...process.env, CARGO_TARGET_DIR: targetDir },
});

const releaseRoot = join(artifactsRoot, 'releases', version);
if (existsSync(releaseRoot)) {
  await rm(releaseRoot, { recursive: true, force: true });
}
await mkdir(releaseRoot, { recursive: true });

const tauriConfig = JSON.parse(await readFile(join(nativeRoot, 'tauri.conf.json'), 'utf8'));
const productName = tauriConfig.productName || 'Aquilum';
const setupCandidates = [
  `${productName}_${version}_x64-setup.exe`,
  `aquilum-app_${version}_x64-setup.exe`,
];
const setupName = setupCandidates.find((name) => existsSync(join(release, 'bundle', 'nsis', name))) || setupCandidates[0];
const setupPath = join(release, 'bundle', 'nsis', setupName);
const setupSigPath = `${setupPath}.sig`;

if (!existsSync(setupPath)) {
  throw new Error(`Setup not found: ${setupPath}`);
}
if (!existsSync(setupSigPath)) {
  throw new Error(`Updater signature not found: ${setupSigPath}`);
}

await copyFile(setupPath, join(releaseRoot, setupName));
const signature = (await readFile(setupSigPath, 'utf8')).trim();
const manifest = {
  version,
  notes: '',
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature,
      url: `https://github.com/${releaseRepo}/releases/download/v${version}/${setupName}`
    }
  }
};
await writeFile(join(releaseRoot, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n', 'utf8');

console.log(`\n\x1b[32m✔ Установщик готов:\x1b[0m ${join(releaseRoot, setupName)}`);
console.log(`\x1b[32m✔ Манифест готов:\x1b[0m ${join(releaseRoot, 'latest.json')}`);
