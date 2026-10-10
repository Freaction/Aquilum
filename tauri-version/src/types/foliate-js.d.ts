declare module 'foliate-js/view.js';

declare module 'foliate-js/overlayer.js' {
  export class Overlayer {
    static highlight(
      rects: DOMRectList | Iterable<DOMRectReadOnly>,
      options?: { color?: string },
    ): SVGElement;
  }
}
