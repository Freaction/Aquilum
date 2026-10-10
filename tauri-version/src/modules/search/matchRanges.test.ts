import { describe, expect, it } from 'vitest';
import { findMatchRanges, flattenMatchRanges } from './matchRanges';

describe('findMatchRanges', () => {
  it('находит совпадение в начале слова', () => {
    expect(findMatchRanges('Дмитрий Чаплинский', ['чапл'])).toEqual([{ start: 8, end: 12 }]);
  });

  it('по умолчанию пропускает совпадение внутри слова', () => {
    expect(findMatchRanges('Кто такой аудмитрий', ['дми'])).toEqual([]);
  });

  it('с wordStartOnly=false находит совпадение внутри слова', () => {
    expect(findMatchRanges('Кто такой аудмитрий', ['дми'], { wordStartOnly: false }))
      .toEqual([{ start: 12, end: 15 }]);
  });

  it('не различает регистр и диакритику', () => {
    expect(findMatchRanges('Café Réunion', ['cafe'])).toEqual([{ start: 0, end: 4 }]);
  });

  it('возвращает пусто без текста или без запроса', () => {
    expect(findMatchRanges('', ['дми'])).toEqual([]);
    expect(findMatchRanges('Дмитрий', [])).toEqual([]);
  });

  it('не выдаёт пересекающихся диапазонов', () => {
    const ranges = findMatchRanges('Дмитрий Дмитриев', ['дми', 'дмитри']);
    for (let index = 1; index < ranges.length; index += 1) {
      expect(ranges[index].start).toBeGreaterThanOrEqual(ranges[index - 1].end);
    }
  });
});

describe('flattenMatchRanges', () => {
  it('раскладывает диапазоны в пары чисел', () => {
    expect(flattenMatchRanges([{ start: 0, end: 3 }, { start: 8, end: 12 }])).toEqual([0, 3, 8, 12]);
  });

  it('из пустого списка делает пустой', () => {
    expect(flattenMatchRanges([])).toEqual([]);
  });
});
