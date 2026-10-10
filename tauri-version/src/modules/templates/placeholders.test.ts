import { describe, expect, it } from 'vitest';
import { applyPlaceholders, formatMoment } from './placeholders';

const MOMENT = new Date(2024, 9, 7, 17, 17, 5);

describe('formatMoment', () => {
  it('tells the month from the minutes by case, as moment.js does', () => {
    expect(formatMoment('DD-MM-YYYY', MOMENT)).toBe('07-10-2024');
    expect(formatMoment('HH:mm', MOMENT)).toBe('17:17');
  });

  it('keeps single-letter tokens unpadded', () => {
    expect(formatMoment('D.M.YY', MOMENT)).toBe('7.10.24');
  });

  it('writes seconds and a two-digit year', () => {
    expect(formatMoment('YY-MM-DD HH:mm:ss', MOMENT)).toBe('24-10-07 17:17:05');
  });

  it('passes anything it does not know straight through', () => {
    expect(formatMoment('DDMM', MOMENT)).toBe('0710');
    expect(formatMoment('неделя DD', MOMENT)).toBe('неделя 07');
  });

  it('leaves text in square brackets alone', () => {
    expect(formatMoment('[Дата] DD-MM-YYYY', MOMENT)).toBe('Дата 07-10-2024');
  });
});

describe('applyPlaceholders', () => {
  it('substitutes the template the owner writes', () => {
    const template = 'created: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})';

    expect(applyPlaceholders(template, MOMENT)).toBe('created: 07-10-2024 | (17:17)');
  });

  it('falls back to iso date and short time without a format', () => {
    expect(applyPlaceholders('{{date}} {{time}}', MOMENT)).toBe('2024-10-07 17:17');
  });

  it('tolerates spacing and casing inside the braces', () => {
    expect(applyPlaceholders('{{ DATE : DD.MM.YYYY }}', MOMENT)).toBe('07.10.2024');
  });

  it('leaves a text without placeholders untouched', () => {
    const text = '---\ntags: []\n---\n\nБез подстановок {{title}}';

    expect(applyPlaceholders(text, MOMENT)).toBe(text);
  });
});
