import { describe, expect, it } from 'vitest';
import { canMove } from './useFileDragAndDrop';

const VAULT = 'C:\\vault';
const NOTE = `${VAULT}\\Base\\заметка.md`;
const NEIGHBOUR = `${VAULT}\\Base\\соседняя.md`;
const OTHER_FOLDER = `${VAULT}\\Архив`;

describe('правила перетаскивания в дереве файлов', () => {
  it('файл переносится в другую папку', () => {
    expect(canMove(NOTE, OTHER_FOLDER, true)).toBe(true);
  });

  it('целью может быть соседний файл: заметка уедет в его папку', () => {
    expect(canMove(NOTE, `${OTHER_FOLDER}\\другая.md`, false)).toBe(true);
  });

  it('перенос в собственную папку ничего не меняет и запрещён', () => {
    expect(canMove(NOTE, `${VAULT}\\Base`, true)).toBe(false);
    expect(canMove(NOTE, NEIGHBOUR, false)).toBe(false);
  });

  it('заметку нельзя бросить саму на себя', () => {
    expect(canMove(NOTE, NOTE, false)).toBe(false);
  });

  it('папку нельзя утащить внутрь самой себя', () => {
    const folder = `${VAULT}\\Base`;
    expect(canMove(folder, `${folder}\\Вложенная`, true)).toBe(false);
    expect(canMove(folder, folder, true)).toBe(false);
  });

  it('работает и с косой чертой Unix', () => {
    expect(canMove('/vault/Base/заметка.md', '/vault/Архив', true)).toBe(true);
    expect(canMove('/vault/Base/заметка.md', '/vault/Base', true)).toBe(false);
    expect(canMove('/vault/Base', '/vault/Base/Вложенная', true)).toBe(false);
  });

  it('пустая цель или пустой источник не принимаются', () => {
    expect(canMove('', OTHER_FOLDER, true)).toBe(false);
    expect(canMove(NOTE, '', true)).toBe(false);
  });
});
