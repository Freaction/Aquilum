import { describe, expect, it } from 'vitest';
import { titleWhenClipped } from './titleWhenClipped';

function span(scrollWidth: number, clientWidth: number, title = '') {
  const removed: string[] = [];
  return {
    scrollWidth,
    clientWidth,
    title,
    removeAttribute(name: string) {
      removed.push(name);
      this.title = '';
    },
    removed,
  };
}

describe('titleWhenClipped', () => {
  it('names the note the ellipsis cut', () => {
    const element = span(320, 180);

    titleWhenClipped(element, 'Нейросети на живых нейронах и принцип свободной энергии');

    expect(element.title).toBe('Нейросети на живых нейронах и принцип свободной энергии');
  });

  it('stays silent for a name that fits', () => {
    const element = span(120, 180);

    titleWhenClipped(element, 'Короткая');

    expect(element.title).toBe('');
    expect(element.removed).toEqual(['title']);
  });

  it('does not turn a rounded pixel into a tooltip', () => {
    const element = span(181, 180);

    titleWhenClipped(element, 'Почти влезает');

    expect(element.title).toBe('');
  });

  it('drops a tooltip left over from a narrower layout', () => {
    const element = span(120, 180, 'старое имя');

    titleWhenClipped(element, 'Короткая');

    expect(element.title).toBe('');
    expect(element.removed).toEqual(['title']);
  });
});
