function toPosixPath(path: string): string {
  return path.replace(/\\/g, '/');
}

function separatorOf(path: string): string {
  return path.includes('\\') ? '\\' : '/';
}

function withoutVerbatimPrefix(posixPath: string): string {
  if (posixPath.toLowerCase().startsWith('//?/unc/')) return `//${posixPath.slice('//?/UNC/'.length)}`;
  return posixPath.startsWith('//?/') ? posixPath.slice('//?/'.length) : posixPath;
}

export function comparablePath(path: string | null | undefined): string {
  return path ? toPosixPath(path).replace(/\/+$/, '').toLowerCase() : '';
}

export function samePath(left: string | null | undefined, right: string | null | undefined): boolean {
  if (!left || !right) return false;
  return comparablePath(left) === comparablePath(right);
}

export function isInsidePath(path: string, folder: string): boolean {
  return comparablePath(path).startsWith(`${comparablePath(folder)}/`);
}

export function rebasedPath(path: string, from: string, to: string): string | null {
  if (samePath(path, from)) return to;
  return isInsidePath(path, from) ? `${to}${path.slice(from.length)}` : null;
}

export function fileName(path: string): string {
  const normalized = toPosixPath(path).replace(/\/+$/, '');
  const slash = normalized.lastIndexOf('/');
  return slash === -1 ? normalized : normalized.slice(slash + 1);
}

export function fileStem(path: string): string {
  return fileName(path).replace(/\.md$/i, '');
}

export function parentDirectory(path: string): string {
  const cut = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return cut > 0 ? path.slice(0, cut) : '';
}

export function childPath(directory: string, name: string): string {
  return `${directory.replace(/[\\/]+$/, '')}${separatorOf(directory)}${name}`;
}

export function siblingPath(path: string, name: string): string {
  const cut = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return `${path.slice(0, cut + 1)}${name}`;
}

export function absolutePath(workspacePath: string, relative: string): string {
  const separator = separatorOf(workspacePath);
  return childPath(workspacePath, relative.replace(/[\\/]/g, separator));
}

export function relativePath(workspacePath: string, filePath: string): string {
  const root = withoutVerbatimPrefix(toPosixPath(workspacePath)).replace(/\/+$/, '');
  const file = withoutVerbatimPrefix(toPosixPath(filePath));
  if (!isInsidePath(file, root)) throw new Error(`File is outside workspace: ${filePath}`);
  return file.slice(root.length + 1);
}

const NUMBERED_PATH_ATTEMPTS = 1000;

function numberedPath(path: string, index: number): string {
  if (index === 0) return path;
  const extension = fileName(path).match(/\.[^.]+$/)?.[0] ?? '';
  return `${path.slice(0, path.length - extension.length)} (${index})${extension}`;
}

export async function claimNumberedPath(
  path: string,
  claim: (candidate: string) => Promise<void>,
  isConflict: (error: unknown) => boolean,
  firstIndex = 0,
): Promise<string> {
  for (let index = firstIndex; index < firstIndex + NUMBERED_PATH_ATTEMPTS; index += 1) {
    const candidate = numberedPath(path, index);
    try {
      await claim(candidate);
      return candidate;
    } catch (error) {
      if (!isConflict(error)) throw error;
    }
  }
  throw new Error(`No free numbered name for ${path}`);
}
