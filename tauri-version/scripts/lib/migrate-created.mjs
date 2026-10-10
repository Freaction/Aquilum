const HEADER_REACH_LINES = 5;

const DATE_LINE =
  /^[ \t]*(?:Дата|Date)[ \t]*:[ \t]*(\d{1,2}[-./]\d{1,2}[-./]\d{4}|\d{4}-\d{1,2}-\d{1,2})(?:[ \t]*\|[ \t]*(?:Время|Time)[ \t]*:[ \t]*\(?(\d{1,2}[:.]\d{2}(?:[:.]\d{2})?)\)?)?[ \t]*(?:\{#[^}]*\})?[ \t]*$/m;

const CREATED_KEY = /^[ \t]*(created|created_at|date-created)[ \t]*:/im;

function frontmatterRange(text) {
  const opening = /^\uFEFF?---[ \t]*\r?\n/.exec(text);
  if (!opening) return null;
  const yamlStart = opening[0].length;
  const closing = /^---[ \t]*(\r?\n|$)/m.exec(text.slice(yamlStart));
  if (!closing) return null;
  const close = yamlStart + closing.index;
  return { yamlStart, close, bodyStart: close + closing[0].length };
}

function declaredLine(text, from) {
  const region = text.slice(from);
  const found = DATE_LINE.exec(region);
  if (!found) return null;
  const before = region.slice(0, found.index);
  if ((before.match(/\n/g) ?? []).length > HEADER_REACH_LINES) return null;
  const start = from + found.index;
  const end = start + found[0].length;
  return { start, end, date: found[1], time: found[2] };
}

function normalizedDate(text) {
  if (!text.includes('/')) return text;
  const fields = text.split('/');
  return Number(fields[0]) > 12 ? fields.join('-') : null;
}

function normalizedTime(text) {
  return text.split('.').join(':');
}

function eolOf(text) {
  return /\r\n/.test(text) ? '\r\n' : '\n';
}

function cut(text, start, end) {
  const trailing = /^\r?\n/.exec(text.slice(end));
  return text.slice(0, start) + text.slice(end + (trailing ? trailing[0].length : 0));
}

function normalizeGap(text, eol) {
  const range = frontmatterRange(text);
  if (!range) return text;
  const body = text.slice(range.bodyStart).replace(/^(?:[ \t]*\r?\n)+/, '');
  if (body.length === 0) return text.slice(0, range.bodyStart);
  return text.slice(0, range.bodyStart) + eol + body;
}

export function migrateNote(text) {
  const range = frontmatterRange(text);
  if (range && CREATED_KEY.test(text.slice(range.yamlStart, range.close))) {
    return { changed: false, reason: 'declared', text };
  }
  const line = declaredLine(text, range ? range.bodyStart : 0);
  if (!line) return { changed: false, reason: 'no-date', text };

  const date = normalizedDate(line.date);
  if (!date) return { changed: false, reason: 'ambiguous', text };
  const eol = eolOf(text);
  const value = line.time ? `${date} ${normalizedTime(line.time)}` : date;
  const trimmed = cut(text, line.start, line.end);
  const declaration = `created: ${value}`;

  if (range) {
    const shifted = frontmatterRange(trimmed);
    const merged =
      trimmed.slice(0, shifted.close) + declaration + eol + trimmed.slice(shifted.close);
    return { changed: true, reason: 'merged', value, text: normalizeGap(merged, eol) };
  }
  const created = `---${eol}${declaration}${eol}---${eol}${trimmed}`;
  return { changed: true, reason: 'created', value, text: normalizeGap(created, eol) };
}
