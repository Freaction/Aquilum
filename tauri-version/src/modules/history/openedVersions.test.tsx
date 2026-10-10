// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { mountDom } from '../../testing/mountDom';
import { describe, expect, it } from 'vitest';
import {
  closeVersion,
  keepVersionsOf,
  openVersion,
  rememberScroll,
  useOpenedVersion,
  type OpenedVersion,
} from './openedVersions';
import type { NoteVersion } from './index';

const version: NoteVersion = {
  id: '1790000000000_aaaaaaaa_me.md',
  atMs: 1790000000000,
  device: 'aaaaaaaa',
  source: 'me',
  fromMs: null,
  name: null,
  isCurrent: false,
};

function last<T>(items: T[]): T {
  return items[items.length - 1];
}

function watch(tabId: string) {
  const seen: (OpenedVersion | null)[] = [];
  function Probe() {
    seen.push(useOpenedVersion(tabId));
    return null;
  }
  act(() => {
    mountDom(<Probe />);
  });
  return seen;
}

describe('opened versions', () => {
  it('belong to their tab and keep the scroll without re-rendering', () => {
    const seen = watch('tab-1');
    const other = watch('tab-2');
    act(() => openVersion('tab-1', 'C:/База/Идея.md', version));
    const renders = seen.length;

    rememberScroll('tab-1', 240);

    expect(seen.length).toBe(renders);
    expect(last(seen)?.scrollTop).toBe(240);
    expect(last(other)).toBeNull();
    act(() => closeVersion('tab-1'));
    expect(last(seen)).toBeNull();
  });

  it('are dropped together with closed tabs', () => {
    const open = watch('open');
    const closed = watch('closed');
    act(() => {
      openVersion('open', 'C:/База/А.md', version);
      openVersion('closed', 'C:/База/Б.md', version);
    });

    act(() => keepVersionsOf(['open']));

    expect(last(closed)).toBeNull();
    expect(last(open)?.path).toBe('C:/База/А.md');
    act(() => closeVersion('open'));
  });
});
