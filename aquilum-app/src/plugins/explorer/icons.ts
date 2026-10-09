import { icons, type IconNode } from 'lucide';

const iconsByName = new Map<string, IconNode>(Object.entries(icons).map(([name, icon]) => [
  name.replace(/(Arrow(?:Up|Down))([01])([01])$/, '$1$2-$3')
    .replace(/(\d)[Xx](\d)/g, '$1x$2').replace(/(\d)D$/, '$1d')
    .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2').replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .replace(/(?<!\d)([A-Za-z])(\d)/g, '$1-$2').toLowerCase(),
  icon,
]));

export const ICON_NAMES: string[] = [...iconsByName.keys()].sort();

export function iconByName(name: string): IconNode | undefined {
  return iconsByName.get(name);
}
