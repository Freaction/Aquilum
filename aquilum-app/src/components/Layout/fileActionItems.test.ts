import { describe, expect, it, vi } from 'vitest';
import { fileActionItems } from './fileActionItems';

const base = { startRename: vi.fn(), duplicateFile: vi.fn(), requestDelete: vi.fn() };

describe('explorer file menu', () => {
  it('keeps plugin actions absent when their plugins are disabled', () => {
    expect(fileActionItems(base, '/vault/n.md').map(item => item.id)).toEqual(['rename', 'duplicate', 'delete']);
  });

  it('adds only enabled plugin actions and invokes each for its target', () => {
    const chooseColor = vi.fn();
    const chooseIcon = vi.fn();
    const togglePin = vi.fn();
    const hidePath = vi.fn();
    const items = fileActionItems({ ...base, chooseColor, chooseIcon, togglePin, hidePath, isPinned: () => false }, '/vault/n.md');
    expect(items.map(item => item.id)).toEqual(['rename', 'duplicate', 'delete', 'color', 'icon', 'pin', 'hide']);
    for (const id of ['color', 'icon', 'pin', 'hide']) items.find(item => item.id === id)!.onSelect();
    for (const action of [chooseColor, chooseIcon, togglePin, hidePath]) expect(action).toHaveBeenCalledWith('/vault/n.md');
    expect(fileActionItems({ ...base, chooseIcon }, '/vault/n.md').map(item => item.id)).toEqual(['rename', 'duplicate', 'delete', 'icon']);
  });

  it('offers unpin for pinned items and keeps attachment menus limited to plugin actions', () => {
    const togglePin = vi.fn();
    expect(fileActionItems({ ...base, togglePin, isPinned: () => true }, '/vault/n.md').map(item => item.id)).toContain('unpin');
    expect(fileActionItems({ ...base, chooseColor: vi.fn() }, '/vault/photo.png', false).map(item => item.id)).toEqual(['color']);
  });

  it('offers path copying and reveal for every item, attachments included', () => {
    const copyVaultPath = vi.fn();
    const copySystemPath = vi.fn();
    const revealInFileManager = vi.fn();
    const items = fileActionItems({ ...base, copyVaultPath, copySystemPath, revealInFileManager }, '/vault/photo.png', false);
    expect(items.map(item => item.id)).toEqual(['copy-path', 'reveal']);
    expect(items[0].children!.map(item => item.id)).toEqual(['copy-vault-path', 'copy-system-path']);
    for (const item of [...items[0].children!, items[1]]) item.onSelect();
    for (const action of [copyVaultPath, copySystemPath, revealInFileManager]) expect(action).toHaveBeenCalledWith('/vault/photo.png');
  });
});
