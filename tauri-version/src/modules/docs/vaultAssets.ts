import { convertFileSrc } from '@tauri-apps/api/core';
import { absolutePath } from '../paths';

function isAbsoluteFsPath(path: string): boolean {
  return /^[a-zA-Z]:[\\/]/.test(path) || path.startsWith('/') || path.startsWith('\\\\');
}

function isRemoteOrAssetUrl(url: string): boolean {
  return (
    url.startsWith('http')
    || url.startsWith('data:')
    || url.startsWith('asset:')
    || url.startsWith('blob:')
  );
}

function decodedPath(path: string): string {
  if (!path.includes('%')) return path;
  try {
    return decodeURIComponent(path);
  } catch {
    return path;
  }
}

function fromFileUrl(value: string): string {
  if (!/^file:\/\//i.test(value)) return value;
  const path = value.replace(/^file:\/\/\/?/i, '');
  return /^[a-zA-Z]:/.test(path) ? path : `/${path}`;
}

export function normalizeVaultRelativePath(path: string): string {
  const segments = path.replace(/\\/g, '/').split('/');
  const kept: string[] = [];
  for (const segment of segments) {
    if (!segment || segment === '.') continue;
    if (segment === '..') {
      kept.pop();
      continue;
    }
    kept.push(segment);
  }
  return kept.join('/');
}

export function resolveVaultAbsolutePath(
  workspacePath: string | null | undefined,
  vaultRelativePath: string,
): string {
  const path = decodedPath(fromFileUrl(vaultRelativePath.trim()));
  if (workspacePath && !isAbsoluteFsPath(path)) {
    return absolutePath(workspacePath, normalizeVaultRelativePath(path));
  }
  return path;
}

export function resolveVaultAssetUrl(
  workspacePath: string | null | undefined,
  vaultRelativePath: string | undefined,
  fallback: string,
): string {
  if (!vaultRelativePath) return fallback;
  if (isRemoteOrAssetUrl(vaultRelativePath)) return vaultRelativePath;
  try {
    return convertFileSrc(resolveVaultAbsolutePath(workspacePath, vaultRelativePath));
  } catch {
    return fallback;
  }
}
