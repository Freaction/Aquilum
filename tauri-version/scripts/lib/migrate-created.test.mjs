import { describe, expect, it } from 'vitest';
import { migrateNote } from './migrate-created.mjs';

const NOTE = [
  'Дата: 07-10-2024 | Время: (17:17)',
  'Тип: #term #дизайн',
  'Автор:: "Якоб Нильсен"',
  '',
  '# Эвристики',
  '',
].join('\n');

describe('migrateNote', () => {
  it('lifts the date and the time into a new frontmatter', () => {
    const result = migrateNote(NOTE);

    expect(result.changed).toBe(true);
    expect(result.text.split('\n').slice(0, 4)).toEqual([
      '---',
      'created: 07-10-2024 17:17',
      '---',
      '',
    ]);
  });

  it('leaves every other field where the author put it', () => {
    const result = migrateNote(NOTE);

    expect(result.text).toContain('Тип: #term #дизайн');
    expect(result.text).toContain('Автор:: "Якоб Нильсен"');
    expect(result.text).not.toContain('Дата:');
    expect(result.text).not.toContain('Время:');
    expect(result.text.endsWith('# Эвристики\n')).toBe(true);
  });

  it('keeps exactly one blank line under the frontmatter', () => {
    const result = migrateNote(`Дата: 07-10-2024\n\n\n\nТело\n`);

    expect(result.text).toBe('---\ncreated: 07-10-2024\n---\n\nТело\n');
  });

  it('adds the declaration to a frontmatter that already exists', () => {
    const note = '---\ntags: []\ntype: term\n---\n\nДата: 07-10-2024 | Время: (17:17)\nТип: #term\n';

    const result = migrateNote(note);

    expect(result.text).toBe(
      '---\ntags: []\ntype: term\ncreated: 07-10-2024 17:17\n---\n\nТип: #term\n',
    );
  });

  it('never touches a note that already declares its date', () => {
    const note = '---\ncreated: 2020-01-01\n---\n\nДата: 07-10-2024\n';

    const result = migrateNote(note);

    expect(result.changed).toBe(false);
    expect(result.reason).toBe('declared');
    expect(result.text).toBe(note);
  });

  it('leaves a note without a header date alone', () => {
    const note = '# Вторая мировая война\n\nНачалась 01-09-1939, закончилась 02-09-1945.\n';

    const result = migrateNote(note);

    expect(result.changed).toBe(false);
    expect(result.reason).toBe('no-date');
    expect(result.text).toBe(note);
  });

  it('ignores a date that is not a whole labelled line', () => {
    expect(migrateNote('Дата: 07-10-2024 плюс заметка\n').changed).toBe(false);
    expect(migrateNote('Смотри Дата: 07-10-2024\n').changed).toBe(false);
  });

  it('ignores a labelled date buried deep in the body', () => {
    const note = `${'строка\n'.repeat(9)}Дата: 07-10-2024\n`;

    expect(migrateNote(note).changed).toBe(false);
  });

  it('reads a time written with dots and normalizes it', () => {
    expect(migrateNote('Дата: 16-03-2021 | Время: (18.42)\n').value).toBe('16-03-2021 18:42');
    expect(migrateNote('Дата: 27-02-2021 | Время:(21.59)\n').value).toBe('27-02-2021 21:59');
  });

  it('drops a trailing block anchor together with the line', () => {
    const note = 'Дата: 07-04-2026 | Время: (14:24) {#l0zo5h}\nТип: #дизайн {#xtkv65}\n';

    const result = migrateNote(note);

    expect(result.value).toBe('07-04-2026 14:24');
    expect(result.text).toBe('---\ncreated: 07-04-2026 14:24\n---\n\nТип: #дизайн {#xtkv65}\n');
  });

  it('takes a slashed date only when the day cannot be a month', () => {
    expect(migrateNote('Дата: 13/03/2021 | Время: (22.24)\n').value).toBe('13-03-2021 22:24');
    expect(migrateNote('Дата: 03/12/2021\n')).toMatchObject({
      changed: false,
      reason: 'ambiguous',
    });
  });

  it('leaves a repeated date deeper in the body where it stands', () => {
    const note = `Дата: 07-04-2026\n${'строка\n'.repeat(20)}Дата: 07-04-2026\n`;

    const result = migrateNote(note);

    expect(result.text.match(/Дата:/g)).toHaveLength(1);
  });

  it('keeps windows line endings', () => {
    const result = migrateNote('Дата: 07-10-2024\r\nТип: #term\r\n');

    expect(result.text).toBe('---\r\ncreated: 07-10-2024\r\n---\r\n\r\nТип: #term\r\n');
  });

  it('takes an iso date and a date with dots too', () => {
    expect(migrateNote('Дата: 2024-10-07\n').value).toBe('2024-10-07');
    expect(migrateNote('Дата: 07.10.2024 | Время: 17:17:05\n').value).toBe('07.10.2024 17:17:05');
  });
});
