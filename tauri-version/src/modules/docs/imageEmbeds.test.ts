import { describe, expect, it } from 'vitest';
import {
  formatImageEmbed,
  nextImageAlign,
  parseImageEmbed,
} from './imageEmbeds';

describe('parseImageEmbed', () => {
  it('reads width from the alt slot', () => {
    expect(parseImageEmbed('![700](Files/image0.jpg)')).toEqual({
      src: 'Files/image0.jpg',
      width: 700,
      align: 'center',
      crop: null,
      kind: 'image',
      wiki: false,
    });
  });

  it('reads align and crop params', () => {
    expect(parseImageEmbed('![700|right|crop=10,0,90,100](Files/a.png)')).toEqual({
      src: 'Files/a.png',
      width: 700,
      align: 'right',
      crop: { left: 10, top: 0, right: 90, bottom: 100 },
      kind: 'image',
      wiki: false,
    });
  });

  it('keeps remote urls and empty alt', () => {
    expect(parseImageEmbed('![](https://example.com/a.jpg)')).toEqual({
      src: 'https://example.com/a.jpg',
      width: null,
      align: 'center',
      crop: null,
      kind: 'image',
      wiki: false,
    });
  });

  it('drops a crop with an empty area', () => {
    expect(parseImageEmbed('![700|crop=50,0,50,100](a.jpg)')?.crop).toBeNull();
  });

  it('ignores lines that are not a lone image', () => {
    expect(parseImageEmbed('текст ![700](a.jpg)')).toBeNull();
    expect(parseImageEmbed('![700](a.jpg) хвост')).toBeNull();
  });

  it('reads a path with spaces, plain or in angle brackets', () => {
    expect(parseImageEmbed('![560](Files/2026-08-09 08-08-49.mp4)')).toMatchObject({
      src: 'Files/2026-08-09 08-08-49.mp4',
      kind: 'video',
      width: 560,
      wiki: false,
    });
    expect(parseImageEmbed('![560](<Files/2026-08-09 08-08-49.mp4>)')).toMatchObject({
      src: 'Files/2026-08-09 08-08-49.mp4',
      kind: 'video',
      width: 560,
    });
  });

  it('reads a wiki embed with spaces in the name', () => {
    expect(parseImageEmbed('![[Pasted image 20260409131453.png]]')).toMatchObject({
      src: 'Pasted image 20260409131453.png',
      kind: 'image',
      wiki: true,
      width: null,
    });
  });

  it('reads wiki params after the name', () => {
    expect(parseImageEmbed('![[Files/clip.mp4|520|left]]')).toMatchObject({
      src: 'Files/clip.mp4',
      kind: 'video',
      wiki: true,
      width: 520,
      align: 'left',
    });
  });

  it('leaves note embeds alone', () => {
    expect(parseImageEmbed('![[Заметка]]')).toBeNull();
    expect(parseImageEmbed('![[Заметка.md]]')).toBeNull();
  });
});

describe('formatImageEmbed', () => {
  it('omits the default align', () => {
    expect(formatImageEmbed({
      src: 'Files/a.jpg', width: 700, align: 'center', crop: null,
    })).toBe('![700](Files/a.jpg)');
  });

  it('round-trips every param', () => {
    const line = '![520|left|crop=5,10,95,90](Files/a.jpg)';
    expect(formatImageEmbed(parseImageEmbed(line)!)).toBe(line);
  });

  it('wraps a path with spaces so markdown stays valid', () => {
    expect(formatImageEmbed(parseImageEmbed('![560](Files/a b.mp4)')!))
      .toBe('![560](<Files/a b.mp4>)');
  });

  it('round-trips a wiki embed in its own syntax', () => {
    const line = '![[Pasted image 1.png|520|left]]';
    expect(formatImageEmbed(parseImageEmbed(line)!)).toBe(line);
  });
});

describe('nextImageAlign', () => {
  it('cycles left → center → right', () => {
    expect(nextImageAlign('left')).toBe('center');
    expect(nextImageAlign('center')).toBe('right');
    expect(nextImageAlign('right')).toBe('left');
  });
});
