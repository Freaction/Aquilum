import type { ViewState } from '../../../modules/ui-state';
import { clamp } from '../../../modules/math';

export interface ResolvedView {
  anchor: number;
  head: number;
  scroll: number;
}

export function fallbackView(initial: ViewState): ResolvedView {
  return { anchor: initial.fallbackAnchor, head: initial.fallbackHead, scroll: initial.fallbackScrollAnchor };
}

export function initialSelection(
  resolved: ResolvedView | null,
  length: number,
  preferredPosition?: number,
): { anchor: number; head: number } | undefined {
  if (preferredPosition !== undefined) {
    const position = clamp(preferredPosition, 0, length);
    return { anchor: position, head: position };
  }
  if (!resolved) return undefined;
  return { anchor: clamp(resolved.anchor, 0, length), head: clamp(resolved.head, 0, length) };
}
