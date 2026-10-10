import { describe, expect, it } from 'vitest';
import type { DataviewOutput } from '../../../../modules/docs/dataview';
import {
  cachedDataviewResult,
  rememberDataviewResult,
  toggleCachedDataviewTask,
} from './store';

function taskOutput(tasks: Array<{ done: boolean; target: string; line: number; text: string }>): DataviewOutput {
  return {
    shape: 'tasks',
    refreshSeconds: null,
    title: null,
    columns: [],
    rows: tasks.map((t) => ({
      parts: [
        { kind: 'check', done: t.done, target: t.target, line: t.line },
        { kind: 'text', text: t.text },
      ],
    })),
    width: 1,
    total: tasks.length,
    truncated: false,
  };
}

describe('dataview store', () => {
  it('excludes completed task when query filters for uncompleted tasks', () => {
    const notePath = 'Base/Home.md';
    const query = 'TASK TITLE "Задачи"\nWHERE !completed\nSORT file.mtime DESC';
    const output = taskOutput([
      { done: false, target: 'Tasks/Work.md', line: 10, text: 'Задача 1' },
      { done: false, target: 'Tasks/Work.md', line: 15, text: 'Задача 2' },
    ]);

    rememberDataviewResult(notePath, query, { status: 'ready', output });

    const changed = toggleCachedDataviewTask('Tasks/Work.md', 10);
    expect(changed).toBe(true);

    const result = cachedDataviewResult(notePath, query);
    expect(result?.status).toBe('ready');
    if (result?.status === 'ready') {
      expect(result.output.rows).toHaveLength(1);
      expect(result.output.total).toBe(1);
      expect(result.output.rows[0]?.parts[1]).toEqual({ kind: 'text', text: 'Задача 2' });
    }
  });

  it('keeps task and updates its done state when query does not filter completed', () => {
    const notePath = 'Base/All.md';
    const query = 'TASK\nSORT file.name ASC';
    const output = taskOutput([
      { done: false, target: 'Tasks/Work.md', line: 5, text: 'Купить молоко' },
    ]);

    rememberDataviewResult(notePath, query, { status: 'ready', output });

    const changed = toggleCachedDataviewTask('Tasks/Work.md', 5);
    expect(changed).toBe(true);

    const result = cachedDataviewResult(notePath, query);
    expect(result?.status).toBe('ready');
    if (result?.status === 'ready') {
      expect(result.output.rows).toHaveLength(1);
      expect(result.output.rows[0]?.parts[0]).toEqual({
        kind: 'check',
        done: true,
        target: 'Tasks/Work.md',
        line: 5,
      });
    }
  });

  it('matches paths with different separators and casing', () => {
    const notePath = 'Base/Sep.md';
    const query = 'TASK\nWHERE !completed';
    const output = taskOutput([
      { done: false, target: 'tasks/work.md', line: 20, text: 'Пункт' },
    ]);

    rememberDataviewResult(notePath, query, { status: 'ready', output });

    const changed = toggleCachedDataviewTask('Tasks\\Work.md', 20);
    expect(changed).toBe(true);

    const result = cachedDataviewResult(notePath, query);
    if (result?.status === 'ready') {
      expect(result.output.rows).toHaveLength(0);
      expect(result.output.total).toBe(0);
    }
  });

  it('excludes completed task with task.completed = false syntax', () => {
    const notePath = 'Base/TaskSyntax.md';
    const query = 'TASK WHERE task.completed = false';
    const output = taskOutput([
      { done: false, target: 'A.md', line: 1, text: 'Item' },
    ]);

    rememberDataviewResult(notePath, query, { status: 'ready', output });
    expect(toggleCachedDataviewTask('A.md', 1)).toBe(true);

    const result = cachedDataviewResult(notePath, query);
    if (result?.status === 'ready') {
      expect(result.output.rows).toHaveLength(0);
      expect(result.output.total).toBe(0);
    }
  });

  it('excludes task when query requires completed and task is unchecked', () => {
    const notePath = 'Base/CompletedOnly.md';
    const query = 'TASK WHERE completed = true';
    const output = taskOutput([
      { done: true, target: 'Done.md', line: 2, text: 'Finished item' },
    ]);

    rememberDataviewResult(notePath, query, { status: 'ready', output });
    expect(toggleCachedDataviewTask('Done.md', 2)).toBe(true);

    const result = cachedDataviewResult(notePath, query);
    if (result?.status === 'ready') {
      expect(result.output.rows).toHaveLength(0);
      expect(result.output.total).toBe(0);
    }
  });

  it('returns false when task is not in store', () => {
    const changed = toggleCachedDataviewTask('NonExistent.md', 99);
    expect(changed).toBe(false);
  });
});
