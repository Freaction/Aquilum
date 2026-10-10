import { expect, it } from 'vitest';
import { PLUGINS } from './registry';
import en from '../i18n/locales/en.json';
import ru from '../i18n/locales/ru.json';

it('provides translated names and descriptions for all eleven registered plugins', () => {
  const ids = ['calendar', 'folderCounts', 'fileColors', 'fileIcons', 'explorerFilters', 'coloredTags',
    'cursorTrail', 'codeStyler', 'readingMode', 'advancedTables', 'gitSync'];
  expect(PLUGINS.map(({ id }) => id)).toEqual(ids);
  const keys = (value: object): string[] => Object.entries(value).flatMap(([key, child]) =>
    typeof child === 'object' ? keys(child).map(nested => `${key}.${nested}`) : [key]).sort();
  expect(keys(en.plugins)).toEqual(keys(ru.plugins));
  expect(keys(en)).toEqual(keys(ru));
  for (const locale of [en, ru]) {
    expect(locale.settings.nav.plugins).toBeTruthy();
    expect(locale.plugins.title).toBeTruthy();
    expect(locale.plugins.builtinMcp.name).toBeTruthy();
    expect(locale.plugins.builtinMcp.description).toBeTruthy();
    for (const { id } of PLUGINS) {
      expect(locale.plugins[id].name).toBeTruthy();
      expect(locale.plugins[id].description).toBeTruthy();
    }
  }
});
