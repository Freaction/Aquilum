// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { EditorView } from '@codemirror/view';
import { actAndSettle, mountDom, type MountedDom } from '../../testing/mountDom';
import { ColorPopover } from './ColorPopover';
import { IconPickerDialog } from './IconPickerDialog';
import { ICON_NAMES } from './icons';
import { setLanguage, t } from '../../i18n';

let mounted: MountedDom;
afterEach(() => { mounted?.unmount(); setLanguage('en'); });

describe.each(['color', 'icon'])('фокус диалога %s', picker => {
  it('удерживает Tab и Shift+Tab внутри и возвращает фокус после закрытия', async () => {
    const trigger = document.createElement('button');
    document.body.appendChild(trigger); trigger.focus();
    const render = (open: boolean) => picker === 'color'
      ? <ColorPopover open={open} onSelect={() => {}} onClose={() => {}} />
      : <IconPickerDialog open={open} recentIcons={[]} onSelect={() => {}} onClose={() => {}} />;
    try {
      await actAndSettle(() => { mounted = mountDom(render(true)); });
      const buttons = mounted.container.querySelectorAll<HTMLButtonElement>('button:not([disabled])');
      const first = buttons[0]; const last = buttons[buttons.length - 1];
      last.focus();
      last.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
      expect(document.activeElement).toBe(first);
      first.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true }));
      expect(document.activeElement).toBe(last);
      await actAndSettle(() => mounted.update(render(false)));
      expect(document.activeElement).toBe(trigger);
    } finally { trigger.remove(); }
  });
});

describe('палитра проводника', () => {
  it('выбирает один из восьми токенов и убирает цвет', async () => {
    const select = vi.fn();
    await actAndSettle(() => { mounted = mountDom(<ColorPopover open value="blue" onSelect={select} onClose={() => {}} />); });
    const buttons = mounted.container.querySelectorAll<HTMLButtonElement>('.q-color-picker__option');
    expect(buttons).toHaveLength(8);
    expect(buttons[4].getAttribute('aria-pressed')).toBe('true');
    buttons[0].click();
    expect(select).toHaveBeenLastCalledWith('red');
    mounted.container.querySelector<HTMLButtonElement>('.q-color-picker__remove')!.click();
    expect(select).toHaveBeenLastCalledWith(null);
  });

  it('показывает ошибку, блокирует выбор при записи и закрывается по Escape', async () => {
    const select = vi.fn(); const close = vi.fn();
    await actAndSettle(() => { mounted = mountDom(<ColorPopover open pending error="Ошибка записи" onSelect={select} onClose={close} />); });
    expect(mounted.container.querySelector('[role="alert"]')?.textContent).toBe('Ошибка записи');
    mounted.container.querySelector<HTMLButtonElement>('.q-color-picker__option')!.click();
    expect(select).not.toHaveBeenCalled();
    mounted.container.querySelector('[role="dialog"]')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(close).toHaveBeenCalledOnce();
  });
});

describe('выбор иконки проводника', () => {
  it.each([['en', 'Icon'], ['ru', 'Иконка']])('uses a neutral icon title in %s for files and folders', async (locale, title) => {
    setLanguage(locale);
    await actAndSettle(() => { mounted = mountDom(<IconPickerDialog open recentIcons={[]} onSelect={() => {}} onClose={() => {}} />); });
    expect(mounted.container.querySelector('.q-dialog__title')?.textContent).toBe(title);
  });
  it('фокусирует поиск и закрывается по Escape из поля', async () => {
    const close = vi.fn();
    await actAndSettle(() => { mounted = mountDom(<IconPickerDialog open recentIcons={[]} onSelect={() => {}} onClose={close} />); });
    await vi.waitFor(() => expect(document.activeElement).toBe(mounted.container.querySelector('.cm-content')));
    document.activeElement!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(close).toHaveBeenCalledOnce();
  });
  it('показывает первые 200 совпадений и пять уникальных валидных недавних', async () => {
    const select = vi.fn();
    await actAndSettle(() => { mounted = mountDom(<IconPickerDialog open value="calendar-days" recentIcons={['bad', 'calendar-days', 'calendar-days', 'star', 'folder', 'file', 'heart', 'home']} onSelect={select} onClose={() => {}} />); });
    expect(mounted.container.querySelectorAll('.q-icon-picker__results button')).toHaveLength(200);
    expect(Array.from(mounted.container.querySelectorAll('.q-icon-picker__recent button')).map(button => button.getAttribute('aria-label'))).toEqual(['calendar-days', 'star', 'folder', 'file', 'heart']);
    mounted.container.querySelector<HTMLButtonElement>('.q-icon-picker__recent button')!.click();
    expect(select).toHaveBeenLastCalledWith('calendar-days');
    mounted.container.querySelector<HTMLButtonElement>('.q-icon-picker__remove')!.click();
    expect(select).toHaveBeenLastCalledWith(null);
  });

  it('ищет без учёта регистра, показывает пустой результат и сбрасывает поиск после открытия', async () => {
    const props = { recentIcons: [], onSelect: vi.fn(), onClose: vi.fn() };
    await actAndSettle(() => { mounted = mountDom(<IconPickerDialog open {...props} />); });
    const search = async (value: string) => {
      const view = EditorView.findFromDOM(mounted.container.querySelector('.cm-content')!)!;
      await actAndSettle(() => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } }));
    };
    await search('CALENDAR');
    expect(mounted.container.querySelectorAll('.q-icon-picker__results button')).toHaveLength(ICON_NAMES.filter(name => name.includes('calendar')).length);
    await search('no-such-icon');
    expect(mounted.container.querySelector('[role="status"]')?.textContent).toBe(t('plugins.fileIcons.empty'));
    await actAndSettle(() => mounted.update(<IconPickerDialog open={false} {...props} />));
    await actAndSettle(() => mounted.update(<IconPickerDialog open {...props} />));
    expect(mounted.container.querySelectorAll('.q-icon-picker__results button')).toHaveLength(200);
  });

  it('блокирует выбор при записи и показывает ошибку', async () => {
    const select = vi.fn();
    await actAndSettle(() => { mounted = mountDom(<IconPickerDialog open pending error="Ошибка записи" recentIcons={['star']} onSelect={select} onClose={() => {}} />); });
    expect(mounted.container.querySelector('[role="alert"]')?.textContent).toBe('Ошибка записи');
    expect(Array.from(mounted.container.querySelectorAll<HTMLButtonElement>('.q-icon-picker__option, .q-icon-picker__remove')).every(button => button.disabled)).toBe(true);
  });
});
