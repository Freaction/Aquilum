// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { mountDom, type MountedDom } from '../../testing/mountDom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { t } from '../../i18n';
import { NewTab } from './NewTab';

describe('NewTab', () => {
  let renderer: MountedDom | null = null;

  afterEach(() => {
    if (renderer) act(() => renderer?.unmount());
    renderer = null;
  });

  it('renders and invokes the three new-tab actions', () => {
    const onCreate = vi.fn();
    const onOpen = vi.fn();
    const onClose = vi.fn();
    act(() => {
      renderer = mountDom(<NewTab onCreate={onCreate} onOpen={onOpen} onClose={onClose} />);
    });

    const buttons = [...renderer!.container.querySelectorAll('button')];
    expect(buttons.map((button) => button.textContent)).toEqual([
      t('editor.createNote'),
      t('editor.openFile'),
      t('editor.close'),
    ]);

    act(() => buttons.forEach((button) => button.click()));
    expect(onCreate).toHaveBeenCalledOnce();
    expect(onOpen).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });
});
