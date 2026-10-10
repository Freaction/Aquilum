import { Facet } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import type { PluginSettings } from '../../../../modules/settings';

export const TAG_COLORS = ['red', 'amber', 'green', 'teal', 'blue', 'purple', 'pink', 'gray'] as const;
export const coloredTagsFacet = Facet.define<PluginSettings['coloredTags'], PluginSettings['coloredTags'] | null>({
  combine: values => values[values.length - 1] ?? null,
});

export function tagHash(tag: string): number {
  let hash = 2166136261;
  for (const byte of new TextEncoder().encode(tag)) hash = Math.imul(hash ^ byte, 16777619);
  return hash >>> 0;
}

export function tagColor(tag: string, settings: PluginSettings['coloredTags']): string {
  const token = (name: string) => {
    const pinned = settings.tagColors[name];
    return TAG_COLORS.find(color => color === pinned) ?? TAG_COLORS[tagHash(name) % TAG_COLORS.length];
  };
  const css = (name: string) => `var(--q-${token(name)}-500)`;
  const pinned = settings.tagColors[tag];
  if (TAG_COLORS.some(color => color === pinned)) return css(tag);
  const root = tag.split('/')[0];
  if (root === tag || !settings.mixNested) return css(root);
  return `color-mix(in srgb, ${css(root)} 60%, ${css(tag)})`;
}

export function coloredTagsExtension(settings: PluginSettings['coloredTags']) {
  return [
    coloredTagsFacet.of(settings),
    // Цвет тега подмешан к основному тексту, чтобы контраст не падал ниже обычного тега.
    EditorView.theme({ '.q-cm-hashtag.q-cm-hashtag--colored': {
      background: 'color-mix(in srgb, var(--q-tag-color) 14%, transparent)',
      color: 'color-mix(in srgb, var(--q-tag-color) 70%, var(--q-text-primary))',
    } }),
  ];
}
