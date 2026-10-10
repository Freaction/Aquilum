#!/usr/bin/env node
import { mkdir, rm, symlink, readlink, lstat } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..', '..');
const repoRoot = resolve(projectRoot, '..');
const artifactsRoot = join(repoRoot, '.artifacts');

export const CARGO_DEV = join(artifactsRoot, 'cargo-dev');
export const CARGO_RELEASE = join(artifactsRoot, 'cargo-release');

const targetLink = join(projectRoot, 'src-tauri', 'target');

export async function setCargoTarget(destination) {
  await mkdir(destination, { recursive: true });
  const resolved = resolve(destination);

  const stats = await lstat(targetLink).catch(() => null);
  if (stats) {
    if (stats.isSymbolicLink()) {
      const current = await readlink(targetLink);
      if (resolve(current) === resolved) return;
      await rm(targetLink);
    } else if (stats.isDirectory()) {
      if (process.platform === 'win32') {
        execSync(`rmdir "${targetLink}"`, { stdio: 'ignore' });
        if (existsSync(targetLink)) {
          throw new Error('src-tauri/target exists and is a real folder. Move it aside, then retry.');
        }
      } else {
        throw new Error('src-tauri/target exists and is a real folder. Move it aside, then retry.');
      }
    }
  }

  await symlink(resolved, targetLink, process.platform === 'win32' ? 'junction' : 'dir');
}
