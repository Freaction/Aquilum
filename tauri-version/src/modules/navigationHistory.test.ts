import { describe, expect, it } from 'vitest';
import {
  canNavigateBack,
  canNavigateForward,
  emptyNavigationHistory,
  pushNavigationEntry,
  stepNavigationHistory,
} from './navigationHistory';

describe('navigationHistory', () => {
  it('builds a linear stack and steps back and forward', () => {
    let state = pushNavigationEntry(emptyNavigationHistory, 'a.md');
    state = pushNavigationEntry(state, 'b.md');
    state = pushNavigationEntry(state, 'c.md');

    expect(canNavigateBack(state)).toBe(true);
    expect(canNavigateForward(state)).toBe(false);

    const back = stepNavigationHistory(state, -1);
    expect(back.path).toBe('b.md');
    expect(canNavigateForward(back.state)).toBe(true);

    const forward = stepNavigationHistory(back.state, 1);
    expect(forward.path).toBe('c.md');
  });

  it('drops forward branch after a new navigation', () => {
    let state = pushNavigationEntry(emptyNavigationHistory, 'a.md');
    state = pushNavigationEntry(state, 'b.md');
    const back = stepNavigationHistory(state, -1);
    state = pushNavigationEntry(back.state, 'c.md');

    expect(state.entries).toEqual(['a.md', 'c.md']);
    expect(canNavigateForward(state)).toBe(false);
  });

  it('collapses consecutive navigations to the same document', () => {
    let state = pushNavigationEntry(emptyNavigationHistory, 'a.md');
    state = pushNavigationEntry(state, 'a.md');

    expect(state.entries).toEqual(['a.md']);
    expect(state.index).toBe(0);
  });

  it('caps the stack size, dropping the oldest entries', () => {
    let state = emptyNavigationHistory;
    for (let i = 0; i < 130; i += 1) {
      state = pushNavigationEntry(state, `note-${i}.md`);
    }

    expect(state.entries.length).toBe(100);
    expect(state.entries[0]).toBe('note-30.md');
    expect(state.entries[state.entries.length - 1]).toBe('note-129.md');
    expect(state.index).toBe(99);
  });
});
