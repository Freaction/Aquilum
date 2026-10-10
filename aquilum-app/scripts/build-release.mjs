#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, rm, copyFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');
const repoRoot = resolve(projectRoot, '..');
const artifactsRoot = join(repoRoot, '.artifacts');
const buildRoot = join(artifactsRoot, 'native-bundle');
const releaseRepo = 'Freaction/Aquilum';

const packageJson = JSON.parse(await readFile(join(projectRoot, 'package.json'), 'utf8'));
const version = packageJson.version;
if (!version) throw new Error('package.json has no version field');

const releaseRoot = join(artifactsRoot, 'releases', version);

const signingKeyPath = join(process.env.USERPROFILE || process.env.HOME, '.tauri', 'aquilum.key');
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  if (!existsSync(signingKeyPath)) {
    throw new Error(`Private signing key not found at: ${signingKeyPath}\nGenerate once: npx tauri signer generate -w "${signingKeyPath}"\nThen set password for this session:\n  set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=your-password`);
  }
  process.env.TAURI_SIGNING_PRIVATE_KEY = await readFile(signingKeyPath, 'utf8');
  process.env.TAURI_SIGNING_PRIVATE_KEY_PATH = signingKeyPath;
}
if (!process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
  throw new Error('TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not set in this terminal.\nRun (current session only):\n  set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=your-password\nOr permanently:\n  setx TAURI_SIGNING_PRIVATE_KEY_PASSWORD "your-password"\n  (then open a new terminal)');
}

execFileSync(process.execPath, ['scripts/sync-icons.mjs'], { cwd: projectRoot, stdio: 'inherit' });
if (existsSync(buildRoot)) {
  await rm(buildRoot, { recursive: true, force: true });
}
execFileSync(process.execPath, ['scripts/bundle-native.mjs', 'build', '--bundles', 'nsis'], {
  cwd: projectRoot,
  stdio: 'inherit',
  env: { ...process.env, CARGO_TARGET_DIR: buildRoot },
});

if (existsSync(releaseRoot)) {
  await rm(releaseRoot, { recursive: true, force: true });
}
await mkdir(releaseRoot, { recursive: true });

const tauriConfig = JSON.parse(await readFile(join(projectRoot, 'native', 'tauri.conf.json'), 'utf8'));
const productName = tauriConfig.productName || 'Aquilum';
const setupCandidates = [
  `${productName}_${version}_x64-setup.exe`,
  `aquilum-app_${version}_x64-setup.exe`,
];
const setupName = setupCandidates.find((name) => existsSync(join(buildRoot, 'release', 'bundle', 'nsis', name))) || setupCandidates[0];
const setupPath = join(buildRoot, 'release', 'bundle', 'nsis', setupName);
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

const notesPath = join(releaseRoot, 'UPLOAD.txt');
const uploadNotes = [
  `v${version} — залить в ${releaseRepo}`,
  '',
  'Файлы:',
  `  ${join(releaseRoot, setupName)}`,
  `  ${join(releaseRoot, 'latest.json')}`,
  '',
  'Вариант A — GitHub UI:',
  `  https://github.com/${releaseRepo}/releases/new?tag=v${version}`,
  `  Title: v${version}`,
  `  Attach: ${setupName} + latest.json  →  Publish release`,
  '',
  'Вариант B — gh CLI (из этой папки):',
  `  cd "${releaseRoot}"`,
  `  gh release create v${version} "${setupName}" latest.json --repo ${releaseRepo} --title "v${version}" --notes ""`,
  '',
  'Если release v${version} уже есть — догрузи/замени assets и обязательно обнови latest.json.',
  'В latest.json windows-x86_64.url должен указывать на *-setup.exe (NSIS), не на .msi.',
  '',
].join('\n');
await writeFile(notesPath, uploadNotes, 'utf8');

console.log(`\n\x1b[32mRelease artifacts: ${releaseRoot}\x1b[0m`);
console.log(`  ${setupName}`);
console.log('  latest.json  (windows-x86_64 → setup.exe)');
console.log(`\n\x1b[33mДальше руками — см. UPLOAD.txt или:\x1b[0m`);
console.log(`  gh release create v${version} "${setupName}" latest.json --repo ${releaseRepo} --title "v${version}" --notes ""`);
console.log(`  (cwd: ${releaseRoot})\n`);
