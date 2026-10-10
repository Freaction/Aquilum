const ROUNDING_SLACK = 1;

interface ClippableText {
  readonly scrollWidth: number;
  readonly clientWidth: number;
  title: string;
  removeAttribute: (name: string) => void;
}

export function titleWhenClipped(element: ClippableText, text: string): void {
  if (element.scrollWidth - element.clientWidth > ROUNDING_SLACK) element.title = text;
  else element.removeAttribute('title');
}
