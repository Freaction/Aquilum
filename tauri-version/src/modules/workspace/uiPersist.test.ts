import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { absolutePath, relativePath } from '../paths';
import {
  loadExpandedFolderPaths,
  loadNavigationHistory,
  saveNavigationHistory,
} from './uiPersist';

const WORKSPACE = 'C:\\notes';
const memory = new Map<string, string>();

function saveForTest(workspacePath: string, absolutePaths: string[]) {
  const key = `aquilum_expanded_folders:${workspacePath.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()}`;
  localStorage.setItem(
    key,
    JSON.stringify(absolutePaths.flatMap((path) => {
      try {
        return [relativePath(workspacePath, path)];
      } catch {
        return [];
      }
    })),
  );
}

beforeEach(() => {
  memory.clear();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => memory.get(key) ?? null,
    setItem: (key: string, value: string) => {
      memory.set(key, value);
    },
    removeItem: (key: string) => {
      memory.delete(key);
    },
    clear: () => memory.clear(),
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('expanded folder persistence', () => {
  it('round-trips expanded folders as relative paths', () => {
    saveForTest(WORKSPACE, [
      'C:\\notes\\Books',
      'C:\\notes\\Books\\Physics',
    ]);

    expect(loadExpandedFolderPaths(WORKSPACE)).toEqual([
      absolutePath(WORKSPACE, 'Books'),
      absolutePath(WORKSPACE, 'Books/Physics'),
    ]);
  });

  it('ignores paths outside the workspace', () => {
    saveForTest(WORKSPACE, [
      'C:\\notes\\Books',
      'C:\\other\\Secret',
    ]);

    expect(loadExpandedFolderPaths(WORKSPACE)).toEqual([
      absolutePath(WORKSPACE, 'Books'),
    ]);
  });
});

describe('navigation history persistence', () => {
  it('round-trips paths and index as relative entries', () => {
    saveNavigationHistory(WORKSPACE, {
      entries: [
        absolutePath(WORKSPACE, 'a.md'),
        absolutePath(WORKSPACE, 'b.md'),
      ],
      index: 1,
    });

    expect(loadNavigationHistory(WORKSPACE)).toEqual({
      entries: [
        absolutePath(WORKSPACE, 'a.md'),
        absolutePath(WORKSPACE, 'b.md'),
      ],
      index: 1,
    });
  });

  it('returns empty history for corrupt storage', () => {
    localStorage.setItem(
      `aquilum_nav_history:${WORKSPACE.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()}`,
      '{not-json',
    );
    expect(loadNavigationHistory(WORKSPACE)).toEqual({ entries: [], index: -1 });
  });
});
