import { readFileSync } from 'node:fs';
import { figmaToCssVar } from './figma-token-utils.mjs';
import { parseHex, rgbaToHex } from './color-normalize.mjs';

export function loadVariablesDoc(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function findCollection(doc, name) {
  return (doc.collections ?? []).find((c) => c.name === name);
}

function modeVars(collection, modeName) {
  if (!collection) return [];
  const mode = collection.modes.find((m) => m.name === modeName) ?? collection.modes[0];
  return mode?.variables ?? [];
}

function aliasName(v) {
  if (!v?.isAlias) return null;
  return typeof v.value === 'object' ? v.value.name : null;
}

function numericUnitAlias(name) {
  const match = name?.match(/^unit\/(-?\d+(?:\.\d+)?)$/);
  return match ? Number(match[1]) : null;
}

function colorEntry(v) {
  if (v.type !== 'color' || v.isAlias) return null;
  const c = parseHex(v.value);
  if (!c) return null;
  return { name: v.name, hex: rgbaToHex(c), r: c.r, g: c.g, b: c.b, a: c.a, raw: v.value };
}

export function parseVariablesJson(doc) {
  const unitCol = findCollection(doc, 'Unit');
  const dimCol = findCollection(doc, 'Dimension Semantic');
  const typoCol = findCollection(doc, 'Typography');
  const primCol = findCollection(doc, 'Color Primitives');
  const semCol = findCollection(doc, 'Color Semantic');

  const unit = modeVars(unitCol, 'Value')
    .filter((v) => v.type === 'number' && !v.isAlias)
    .map((v) => ({ name: v.name, value: v.value }))
    .sort((a, b) => a.name.localeCompare(b.name));

  const dimension = modeVars(dimCol, 'Value').map((v) => {
    if (v.isAlias) {
      const ref = aliasName(v);
      const unitToken = unit.find((u) => u.name === ref);
      return {
        name: v.name,
        value: unitToken?.value ?? numericUnitAlias(ref),
        alias: ref,
      };
    }
    return { name: v.name, value: v.value, alias: null };
  }).sort((a, b) => a.name.localeCompare(b.name));

  const typography = modeVars(typoCol, 'Value').map((v) => {
    if (v.isAlias) return { name: v.name, value: aliasName(v) };
    return { name: v.name, value: v.value };
  }).sort((a, b) => a.name.localeCompare(b.name));

  const colorPrimitives = modeVars(primCol, 'Value')
    .map(colorEntry)
    .filter(Boolean)
    .sort((a, b) => a.name.localeCompare(b.name));

  const primByName = new Map(colorPrimitives.map((p) => [p.name, p]));
  const lightVars = modeVars(semCol, 'Light');
  const darkByName = new Map(modeVars(semCol, 'Dark').map((v) => [v.name, v]));

  const colorSemantic = lightVars
    .filter((v) => v.type === 'color')
    .map((v) => {
      const light = aliasName(v) ?? (typeof v.value === 'string' ? v.value : null);
      const darkVar = darkByName.get(v.name);
      const dark = aliasName(darkVar)
        ?? (typeof darkVar?.value === 'string' ? darkVar.value : light);
      const lightPrim = light ? primByName.get(light) : null;
      const darkPrim = dark ? primByName.get(dark) : null;
      return {
        name: v.name,
        light,
        dark,
        lightHex: lightPrim?.hex ?? null,
        darkHex: darkPrim?.hex ?? null,
        css: figmaToCssVar(v.name),
      };
    })
    .sort((a, b) => a.name.localeCompare(b.name));

  return {
    unit,
    dimension,
    typography,
    colorPrimitives,
    colorSemantic,
    counts: {
      unit: unit.length,
      dimension: dimension.length,
      typography: typography.length,
      colorPrimitives: colorPrimitives.length,
      colorSemantic: colorSemantic.length,
    },
  };
}
