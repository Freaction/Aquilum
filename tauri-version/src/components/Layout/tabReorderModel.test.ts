import { describe, expect, it } from 'vitest';
import { reorderLayout, restingOffset, type TabBox } from './tabReorderModel';

const BOXES: TabBox[] = [
  { left: 0, width: 100 },
  { left: 100, width: 100 },
  { left: 200, width: 100 },
];

describe('reorderLayout', () => {
  it('на месте — никто не двигается', () => {
    expect(reorderLayout(BOXES, 0, 0)).toEqual({ target: 0, shift: [0, 0, 0] });
  });

  it('сосед уступает, когда перевалили за его середину', () => {
    expect(reorderLayout(BOXES, 0, 40)).toEqual({ target: 0, shift: [40, 0, 0] });
    expect(reorderLayout(BOXES, 0, 60)).toEqual({ target: 1, shift: [60, -100, 0] });
  });

  it('перетаскивание влево сдвигает соседей вправо', () => {
    expect(reorderLayout(BOXES, 2, -110)).toEqual({ target: 1, shift: [0, 100, -110] });
    expect(reorderLayout(BOXES, 2, -210)).toEqual({ target: 0, shift: [100, 100, -210] });
  });

  it('за краем полосы цель не выходит за крайние вкладки', () => {
    expect(reorderLayout(BOXES, 1, 900).target).toBe(2);
    expect(reorderLayout(BOXES, 1, -900).target).toBe(0);
  });

  it('пустая полоса и выход за границы не ломают расчёт', () => {
    expect(reorderLayout([], 0, 50)).toEqual({ target: 0, shift: [] });
    expect(reorderLayout(BOXES, 7, 50).shift).toEqual([0, 0, 0]);
  });
});

describe('restingOffset', () => {
  it('без перестановки вкладка возвращается на место', () => {
    expect(restingOffset(BOXES, 1, 1)).toBe(0);
  });

  it('вправо — до правого края целевой вкладки', () => {
    expect(restingOffset(BOXES, 0, 2)).toBe(200);
  });

  it('влево — до левого края целевой вкладки', () => {
    expect(restingOffset(BOXES, 2, 0)).toBe(-200);
  });
});
