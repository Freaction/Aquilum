import { describe, expect, it } from 'vitest';
import { findTaskMark, markText, toggleTaskLine } from './taskCheckbox';

describe('findTaskMark', () => {
  it('находит чекбокс при любом маркере списка', () => {
    expect(findTaskMark('- [ ] купить хлеб')).toEqual({ start: 2, done: false });
    expect(findTaskMark('3. [ ] нумерованная')).toEqual({ start: 3, done: false });
    expect(findTaskMark('1) [x] со скобкой')).toEqual({ start: 3, done: true });
  });

  it('звёздочка и плюс не начинают список, поэтому и задачи там нет', () => {
    expect(findTaskMark('* [ ] звёздочка')).toBeNull();
    expect(findTaskMark('+ [ ] плюс')).toBeNull();
  });

  it('учитывает отступ вложенного пункта', () => {
    expect(findTaskMark('    - [ ] вложенная')).toEqual({ start: 6, done: false });
  });

  it('любой знак кроме пробела значит «закрыта»', () => {
    expect(findTaskMark('- [/] в работе')?.done).toBe(true);
    expect(findTaskMark('- [-] отменена')?.done).toBe(true);
  });

  it('пустая задача — тоже задача', () => {
    expect(findTaskMark('- [ ]')).toEqual({ start: 2, done: false });
    expect(findTaskMark('- [ ]\r')).toEqual({ start: 2, done: false });
  });

  it('не видит чекбокса там, где его нет', () => {
    expect(findTaskMark('- обычный пункт')).toBeNull();
    expect(findTaskMark('[ ] без маркера списка')).toBeNull();
    expect(findTaskMark('- [] пустые скобки')).toBeNull();
    expect(findTaskMark('- [ ]текст без пробела')).toBeNull();
  });
});

describe('toggleTaskLine', () => {
  const text = '# Дела\n\n- [ ] написать\n- [x] отправить\n- обычный пункт';

  it('закрывает открытую задачу и открывает закрытую', () => {
    expect(toggleTaskLine(text, 3)).toContain('- [x] написать');
    expect(toggleTaskLine(text, 4)).toContain('- [ ] отправить');
  });

  it('не трогает остальные строки', () => {
    const next = toggleTaskLine(text, 3);
    expect(next?.split('\n')[3]).toBe('- [x] отправить');
    expect(next?.split('\n')[4]).toBe('- обычный пункт');
  });

  it('строка без чекбокса и строка за пределами текста оставляют заметку как есть', () => {
    expect(toggleTaskLine(text, 5)).toBeNull();
    expect(toggleTaskLine(text, 99)).toBeNull();
  });

  it('сохраняет отступ и маркер', () => {
    expect(toggleTaskLine('    1) [ ] дело', 1)).toBe('    1) [x] дело');
  });
});

describe('markText', () => {
  it('пишет крестик закрытой и пробел открытой', () => {
    expect(markText(true)).toBe('[x]');
    expect(markText(false)).toBe('[ ]');
  });
});
