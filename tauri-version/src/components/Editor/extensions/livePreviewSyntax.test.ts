import { describe, expect, it } from 'vitest';
import { syntaxMarkerPresentation } from './livePreviewSyntax';

describe('live preview syntax registry', () => {
  it('keeps link chrome atomic and short marks revealable', () => {
    expect(syntaxMarkerPresentation('WikiLinkMark')).toBe('collapsed');
    expect(syntaxMarkerPresentation('URL')).toBe('collapsed');
    expect(syntaxMarkerPresentation('StrongMark')).toBe('hidden');
  });

  it('leaves constructs owned by specialized extensions alone', () => {
    expect(syntaxMarkerPresentation('ListMark')).toBeNull();
    expect(syntaxMarkerPresentation('Table')).toBeNull();
  });
});
