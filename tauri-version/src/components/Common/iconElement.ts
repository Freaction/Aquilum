import { createElement, type IconNode } from 'lucide';

export function createIconElement(icon: IconNode, strokeWidth = 1.2): SVGElement {
  return createElement(icon, { 'stroke-width': strokeWidth, 'aria-hidden': 'true' });
}
