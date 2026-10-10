import { describe, expect, it } from 'vitest';
import { AtlasCells } from './atlasCells';

describe('AtlasCells', () => {
  it('hands the same cell back for a text it already holds', () => {
    const cells = new AtlasCells();
    cells.resize(4);

    const first = cells.claim('Заметка', 1);
    const again = cells.claim('Заметка', 2);

    expect(first).toEqual({ cell: 0, fresh: true });
    expect(again).toEqual({ cell: 0, fresh: false });
  });

  it('fills empty cells before it evicts anything', () => {
    const cells = new AtlasCells();
    cells.resize(3);

    const claims = ['a', 'b', 'c'].map((text) => cells.claim(text, 1));

    expect(claims.map((claim) => claim?.cell)).toEqual([0, 1, 2]);
    expect(claims.every((claim) => claim?.fresh)).toBe(true);
  });

  it('evicts the text untouched for the longest time', () => {
    const cells = new AtlasCells();
    cells.resize(2);
    cells.claim('old', 1);
    cells.claim('recent', 1);
    cells.claim('recent', 2);

    const arrival = cells.claim('new', 3);

    expect(arrival).toEqual({ cell: 0, fresh: true });
    expect(cells.claim('recent', 3)).toEqual({ cell: 1, fresh: false });
    expect(cells.claim('old', 3)).not.toEqual({ cell: 0, fresh: false });
  });

  it('refuses to evict a text already used on this frame', () => {
    const cells = new AtlasCells();
    cells.resize(2);
    cells.claim('first', 5);
    cells.claim('second', 5);

    expect(cells.claim('third', 5)).toBeNull();
  });

  it('has nowhere to put anything before it is sized', () => {
    const cells = new AtlasCells();

    expect(cells.claim('anything', 1)).toBeNull();
  });

  it('drops every cell when it is resized', () => {
    const cells = new AtlasCells();
    cells.resize(2);
    cells.claim('kept', 1);

    cells.resize(4);

    expect(cells.claim('kept', 2)).toEqual({ cell: 0, fresh: true });
    expect(cells.size).toBe(4);
  });
});
