#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const publicRoot = join(repoRoot, 'release-repo-update');
const PUBLISHED_PATHS = ['aquilum-app', 'tauri-version', 'LICENSE', '.gitattributes', '.gitignore', '.gitmodules'];
const SNAPSHOT_OWNED_PATHS = [...PUBLISHED_PATHS, 'DEVELOPMENT.md'];
const SUBMODULE_PATH = 'tauri-version/vendor/foliate-js';
const NOTE_DIRECTORIES = ['knowledge base/', 'research_notes/', 'reports/', '.aquilum/'];
const PUBLISHED_MARKDOWN = [
  'aquilum-app/README.md',
  'aquilum-app/native/assets/fonts/LICENSE-iA-Writer.md',
  'tauri-version/README.md',
  'tauri-version/src/fonts/LICENSE-iA-Writer.md',
];
const VENDORED = 'aquilum-app/native/vendor/';
const PUBLIC_ONLY_MARKDOWN = ['README.md', 'README.ru.md', 'DEVELOPMENT.md'];
const publish = process.argv.includes('--publish');
const messageAt = process.argv.indexOf('--message');
const message = messageAt >= 0 ? process.argv[messageAt + 1] : undefined;

function git(cwd, args) {
  return execFileSync('git', args, { cwd, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }).trim();
}

function stop(message) {
  console.error(`publish-source: ${message}`);
  process.exit(1);
}

function lines(output) {
  return output ? output.split('\n') : [];
}

function notePaths(paths, allowedMarkdown) {
  const allowed = new Set(allowedMarkdown);
  return paths.filter((path) => (
    NOTE_DIRECTORIES.some((directory) => path.startsWith(directory) || path.includes(`/${directory}`))
    || (path.toLowerCase().endsWith('.md') && !allowed.has(path) && !path.startsWith(VENDORED))
  ));
}

function requireNoNotes(paths, allowedMarkdown, where, beforeStop = () => {}) {
  const found = notePaths(paths, allowedMarkdown);
  if (found.length === 0) return;
  beforeStop();
  stop(`${where} contains notes, nothing was published:\n  ${found.join('\n  ')}`);
}

function requireCleanPrivateTree() {
  if (git(repoRoot, ['status', '--porcelain', '--untracked-files=no'])) {
    stop('commit private changes first: the snapshot is taken from HEAD');
  }
}

function preparePublicCopy() {
  if (!existsSync(join(publicRoot, '.git'))) stop(`public working copy not found: ${publicRoot}`);
  if (git(publicRoot, ['status', '--porcelain'])) stop('public working copy has uncommitted changes');
  git(publicRoot, ['pull', '--ff-only', '--quiet']);
}

function extractSnapshot() {
  for (const path of SNAPSHOT_OWNED_PATHS) rmSync(join(publicRoot, path), { recursive: true, force: true });
  const archive = execFileSync('git', ['archive', 'HEAD', '--', ...PUBLISHED_PATHS], {
    cwd: repoRoot,
    maxBuffer: 512 * 1024 * 1024,
  });
  execFileSync('tar', ['-x', '-f', '-'], { cwd: publicRoot, input: archive });
  writeFileSync(join(publicRoot, 'DEVELOPMENT.md'), git(repoRoot, ['show', 'HEAD:README.md']) + '\n');
}

function stageSnapshot() {
  git(publicRoot, ['add', '-A']);
  const [mode, , sha] = git(repoRoot, ['ls-tree', 'HEAD', SUBMODULE_PATH]).split(/\s+/);
  if (mode !== '160000') throw new Error(`${SUBMODULE_PATH} is not a submodule in HEAD`);
  git(publicRoot, ['update-index', '--add', '--cacheinfo', `160000,${sha},${SUBMODULE_PATH}`]);
}

function discardSnapshot() {
  git(publicRoot, ['reset', '--hard', '--quiet']);
  git(publicRoot, ['clean', '-fdq']);
}

requireCleanPrivateTree();
requireNoNotes(
  lines(git(repoRoot, ['ls-tree', '-r', '--name-only', 'HEAD', '--', ...PUBLISHED_PATHS])),
  PUBLISHED_MARKDOWN,
  'private HEAD snapshot',
);
preparePublicCopy();
try {
  extractSnapshot();
  stageSnapshot();
} catch (error) {
  discardSnapshot();
  stop(`snapshot failed, public working copy restored: ${error.stderr?.toString().trim() || error.message}`);
}

if (!git(publicRoot, ['diff', '--cached', '--name-only'])) {
  discardSnapshot();
  console.log('publish-source: public repository is already up to date');
  process.exit(0);
}

requireNoNotes(
  lines(git(publicRoot, ['ls-files'])),
  [...PUBLISHED_MARKDOWN, ...PUBLIC_ONLY_MARKDOWN],
  'public repository',
  discardSnapshot,
);

console.log(git(publicRoot, ['diff', '--cached', '--stat']));

if (!publish) {
  discardSnapshot();
  console.log('publish-source: dry run, nothing committed. Run with --publish to commit and push.');
  process.exit(0);
}

const { version } = JSON.parse(readFileSync(join(repoRoot, 'aquilum-app', 'package.json'), 'utf8'));
const privateCommit = git(repoRoot, ['rev-parse', '--short', 'HEAD']);
git(publicRoot, ['commit', '--quiet', '-m', message ?? `исходники ${version} (${privateCommit})`]);
git(publicRoot, ['push', '--quiet', 'origin', 'HEAD']);
console.log(`publish-source: published ${version} from ${privateCommit}`);
