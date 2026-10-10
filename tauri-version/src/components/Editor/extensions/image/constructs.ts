import type { Text } from '@codemirror/state';
import { parseImageEmbed, type ImageEmbed } from '../../../../modules/docs/imageEmbeds';

type ImageEmbedSpan = ImageEmbed & { from: number; to: number };

export function findImageEmbeds(doc: Text): ImageEmbedSpan[] {
  const spans: ImageEmbedSpan[] = [];

  for (let lineNo = 1; lineNo <= doc.lines; lineNo += 1) {
    const line = doc.line(lineNo);
    if (!line.text.includes('![')) continue;
    const embed = parseImageEmbed(line.text);
    if (embed) spans.push({ ...embed, from: line.from, to: line.to });
  }

  return spans;
}
