import { describe, expect, it } from 'vitest';
import { nextLiveTabs, sameTabOrder } from './liveTabs';

const all = () => true;

describe('nextLiveTabs', () => {
  it('ставит активную вкладку первой, не теряя остальных', () => {
    expect(nextLiveTabs(['b', 'c'], 'a', all, 3)).toEqual(['a', 'b', 'c']);
    expect(nextLiveTabs(['a', 'b'], 'b', all, 3)).toEqual(['b', 'a']);
  });

  it('держит не больше предела и вытесняет самую давнюю', () => {
    expect(nextLiveTabs(['b', 'c', 'd'], 'a', all, 3)).toEqual(['a', 'b', 'c']);
    expect(nextLiveTabs(['b', 'c', 'd'], 'a', all, 2)).toEqual(['a', 'b']);
  });

  it('оставляет активную вкладку живой при любом пределе из настроек', () => {
    expect(nextLiveTabs(['b'], 'a', all, 0)).toEqual(['a']);
    expect(nextLiveTabs(['b'], 'a', all, -5)).toEqual(['a']);
    expect(nextLiveTabs(['b', 'c'], 'a', all, 2.7)).toEqual(['a', 'b']);
  });

  it('выбрасывает вкладки, которые больше нельзя держать живыми', () => {
    const open = new Set(['a', 'c']);
    expect(nextLiveTabs(['b', 'c'], 'a', (tabId) => open.has(tabId), 3)).toEqual(['a', 'c']);
  });

  it('без активной вкладки сохраняет прежний порядок', () => {
    expect(nextLiveTabs(['a', 'b'], null, all, 3)).toEqual(['a', 'b']);
  });

  it('не плодит дублей, если активная уже в списке первой', () => {
    expect(nextLiveTabs(['a', 'b'], 'a', all, 3)).toEqual(['a', 'b']);
  });
});

describe('sameTabOrder', () => {
  it('различает порядок, а не только состав', () => {
    expect(sameTabOrder(['a', 'b'], ['a', 'b'])).toBe(true);
    expect(sameTabOrder(['a', 'b'], ['b', 'a'])).toBe(false);
    expect(sameTabOrder(['a'], ['a', 'b'])).toBe(false);
  });
});
