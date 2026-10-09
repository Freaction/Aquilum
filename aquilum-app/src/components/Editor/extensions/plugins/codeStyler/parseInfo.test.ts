import { describe, expect, it } from 'vitest';
import { parseInfo } from './parseInfo';

describe('code fence info', () => {
  it('parses a language, quoted title, starting line and highlight ranges', () => {
    expect(parseInfo('js title:"a b" ln:5 hl:1,3-4')).toEqual({ lang: 'js', title: 'a b', ln: 5, hl: new Set([1, 3, 4]) });
  });
  it('handles escaped quotes, single quotes and boolean overrides', () => {
    expect(parseInfo(String.raw`ts title:"a \"b\"" ln:false hl:2,2`)).toEqual({ lang: 'ts', title: 'a "b"', ln: false, hl: new Set([2]) });
    expect(parseInfo("title:'hello world' ln:true")).toEqual({ lang: '', title: 'hello world', ln: true, hl: new Set() });
  });
  it('ignores malformed options and avoids expanding unbounded ranges', () => {
    expect(parseInfo('js ln:0 hl:0,-1,4-2,1-999999999 title:"unterminated')).toEqual({ lang: 'js', hl: new Set() });
    expect(parseInfo('')).toEqual({ lang: '', hl: new Set() });
    expect(parseInfo('js unknown:yes garbage ln:no')).toEqual({ lang: 'js', hl: new Set() });
  });
});
