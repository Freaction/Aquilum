// @vitest-environment happy-dom
import { act } from 'preact/test-utils';
import { mountDom, type MountedDom } from '../../testing/mountDom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BacklinksPanel } from './BacklinksPanel';

const linkMocks = vi.hoisted(() => ({
  calendarEnabled: true,
  backlink: {
    path: 'C:\\notes\\Source.md',
    title: 'Source',
    offset: 3,
  },
  outgoing: {
    target: 'Target',
    title: 'Target',
    path: 'C:\\notes\\Target.md',
  },
  analysis: {
    path: 'C:\\notes\\Related.md',
    title: 'Related',
    rawScore: 0.255,
    reasons: ['related'],
  },
}));

vi.mock('../../i18n', () => ({ t: (key: string) => key, getLocale: () => 'en' }));
vi.mock('../../modules/settings', async importOriginal => ({
  ...await importOriginal<typeof import('../../modules/settings')>(),
  useSettingsStore: () => ({
    config: {
      plugins: { calendar: { enabled: linkMocks.calendarEnabled, showWeekNumbers: false, weekly: { enabled: false, folder: '', format: 'gggg-[W]ww', template: '' } } },
      analysis: {
        enableBm25f: true,
        enableAdamicAdar: true,
        enableWikixiv: true,
        bm25fParams: {
          k1: 1.2,
          k3: 8,
          bTitle: 0.3,
          bBody: 0.75,
          titleWeight: 2.5,
        },
      },
    },
  }),
}));
vi.mock('../../modules/links', () => ({
  useDocumentLinks: (mode: 'backlinks' | 'outgoing') => ({
    result: mode === 'backlinks'
      ? { mode, items: [linkMocks.backlink] }
      : { mode, items: [linkMocks.outgoing] },
    loading: false,
  }),
}));
vi.mock('../../modules/analysis', () => ({
  enabledSidebarMethods: () => ['bm25f', 'adamicAdar', 'wixiv'],
  isGraphAnalysisMethod: (method: string) => method !== 'wixiv',
  useDocumentAnalysis: () => ({
    items: [linkMocks.analysis],
    failed: false,
    loading: false,
  }),
}));
vi.mock('../../modules/wikixiv', () => ({
  useWikixivSources: () => ({
    hits: [],
    offline: false,
    insufficientText: false,
    failed: false,
    loading: false,
  }),
  setWikiHover: () => {},
  clearWikiHover: () => {},
}));
vi.mock('../../modules/openExternalUrl', () => ({
  openExternalUrl: vi.fn(),
}));

describe('BacklinksPanel', () => {
  let renderer: MountedDom | null = null;

  beforeEach(() => {
    linkMocks.calendarEnabled = true;
    try {
      localStorage.clear();
    } catch {}
  });

  afterEach(() => {
    if (renderer) act(() => renderer?.unmount());
    renderer = null;
  });

  it('switches the pressed mode and renders document-name rows', () => {
    const onOpenBacklink = vi.fn();
    const onOpenOutgoing = vi.fn();
    const onOpenAnalysis = vi.fn();
    act(() => {
      renderer = mountDom(
        <BacklinksPanel
          workspacePath="C:\\notes"
          documentPath="C:\\notes\\Current.md"
          activeTabId={null}
          indexReady
          indexRevision={1}
          isOpen
          onOpenBacklink={onOpenBacklink}
          onOpenOutgoing={onOpenOutgoing}
          onOpenAnalysis={onOpenAnalysis}
          onOpenNote={vi.fn()}
        />,
      );
    });

    const panel = renderer!.container;
    const modeButtons = [...panel.querySelectorAll<HTMLButtonElement>('.q-backlinks__mode-button')];
    const pressed = () => modeButtons.map((button) => button.getAttribute('aria-pressed'));
    expect(pressed()).toEqual(['true', 'false', 'false', 'false', 'false']);
    expect(panel.querySelector('h2')?.textContent).toBe('backlinks.mentionsTitle');
    expect(panel.querySelector('.q-sidebar-document-item__title')?.textContent).toBe('Source');

    act(() => modeButtons[1].click());

    expect(pressed()).toEqual(['false', 'true', 'false', 'false', 'false']);
    expect(panel.querySelector('h2')?.textContent).toBe('backlinks.outgoingTitle');
    const outgoingButton = panel.querySelector<HTMLElement>('.q-sidebar-document-item')!;
    expect(panel.querySelector('.q-sidebar-document-item__title')?.textContent).toBe('Target');

    act(() => {
      outgoingButton.dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
    });
    expect(onOpenOutgoing).toHaveBeenCalledWith(linkMocks.outgoing, 'new-tab');
    expect(onOpenBacklink).not.toHaveBeenCalled();
  });
  it('hides a disabled calendar and falls back from the stored calendar mode', () => {
    linkMocks.calendarEnabled = false;
    localStorage.setItem('aquilum_backlinks_mode', JSON.stringify('calendar'));
    act(() => {
      renderer = mountDom(<BacklinksPanel workspacePath="/vault" documentPath="/vault/Current.md" activeTabId={null} indexReady indexRevision={1} isOpen onOpenBacklink={vi.fn()} onOpenOutgoing={vi.fn()} onOpenAnalysis={vi.fn()} onOpenNote={vi.fn()} />);
    });
    const panel = renderer!.container;
    expect(panel.querySelector('[aria-label="calendar.title"]')).toBeNull();
    expect(panel.querySelector('h2')?.textContent).toBe('backlinks.mentionsTitle');
    expect(panel.querySelector('[aria-label="backlinks.backlinksTab"]')?.getAttribute('aria-pressed')).toBe('true');
  });

});
