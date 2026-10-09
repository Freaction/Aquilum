export interface CodeInfo {
  lang: string;
  title?: string;
  ln?: boolean | number;
  hl: Set<number>;
}

export function parseInfo(info: string): CodeInfo {
  const result: CodeInfo = { lang: '', hl: new Set() };
  const tokens = info.match(/(?:[^\s"']+|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')+/g) ?? [];
  for (const [index, token] of tokens.entries()) {
    const colon = token.indexOf(':');
    if (colon < 0) {
      if (index === 0) result.lang = token;
      continue;
    }
    const key = token.slice(0, colon);
    let value = token.slice(colon + 1);
    if (/^(["']).*\1$/.test(value)) value = value.slice(1, -1).replace(/\\([\\"'])/g, '$1');
    if (key === 'title' && value) result.title = value;
    if (key === 'ln') {
      if (value === 'true' || value === 'false') result.ln = value === 'true';
      else if (/^[1-9]\d*$/.test(value) && Number.isSafeInteger(Number(value))) result.ln = Number(value);
    }
    if (key === 'hl') {
      for (const part of value.split(',')) {
        const match = /^([1-9]\d*)(?:-([1-9]\d*))?$/.exec(part);
        if (!match) continue;
        const start = Number(match[1]);
        const end = Number(match[2] ?? match[1]);
        // Не разворачиваем огромные диапазоны из текста заметки.
        if (!Number.isSafeInteger(end) || end < start || end - start > 10000) continue;
        for (let line = start; line <= end; line++) result.hl.add(line);
      }
    }
  }
  return result;
}
