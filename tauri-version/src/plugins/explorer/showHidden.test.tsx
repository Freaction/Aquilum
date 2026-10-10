// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { afterEach, expect, it } from 'vitest';
import { useExplorerShowHidden } from '../../modules/workspace/uiPersist';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';

let mounted: MountedDom | undefined;
afterEach(() => { act(() => mounted?.unmount()); mounted = undefined; localStorage.clear(); });
function Preference({ root }: { root: string }) {
  const [shown, setShown] = useExplorerShowHidden(root);
  return <button aria-pressed={shown} onClick={() => setShown(!shown)}>hidden</button>;
}
it('persists show hidden independently for each workspace', async () => {
  localStorage.clear();
  await actAndSettle(() => { mounted = mountDom(<Preference root="/a" />); });
  const button = () => mounted!.container.querySelector('button')!;
  expect(button().getAttribute('aria-pressed')).toBe('false');
  await actAndSettle(() => button().click());
  expect(localStorage.getItem('aquilum_explorer_show_hidden:/a')).toBe('true');
  await actAndSettle(() => mounted!.update(<Preference root="/b" />));
  expect(button().getAttribute('aria-pressed')).toBe('false');
  await actAndSettle(() => mounted!.update(<Preference root="/a" />));
  expect(button().getAttribute('aria-pressed')).toBe('true');
});
