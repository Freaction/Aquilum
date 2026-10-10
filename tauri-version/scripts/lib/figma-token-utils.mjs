export function figmaToCssVar(name) {
  const parts = name.split('/').map((part) =>
    part.trim().toLowerCase().replace(/\s+/g, '-'),
  );
  return `--q-${parts.join('-')}`;
}

export function figmaRef(name) {
  return `var(${figmaToCssVar(name)})`;
}

export function pxToTokenValue(px, { keepPx = false } = {}) {
  if (keepPx || Math.abs(px) >= 9999) {
    return `${px}px`;
  }
  if (px === 0) return '0';
  const rem = px / 16;
  const rounded = Math.round(rem * 10000) / 10000;
  return `${rounded}rem`;
}

export function cssBlock(selectors, lines) {
  return `${selectors} {\n${lines.map((l) => (l ? `  ${l}` : '')).join('\n')}\n}\n`;
}

export function alphaPctFromName(step) {
  if (step === 'main') return 100;
  const n = Number(step);
  return Number.isNaN(n) ? null : n;
}

export const ALPHA_STEPS = [
  '01', '02', '03', '04', '05', '06', '08', '10', '12', '14', '15', '16',
  '20', '25', '30', '32', '40', '46', '50', '60', '70', '80', '90', 'main',
];

export const COLOR_FAMILIES = [
  'gray', 'blue', 'purple', 'green', 'red', 'amber', 'teal', 'pink',
];

export const NEUTRAL_ALPHA_FAMILIES = ['black-alpha', 'white-alpha'];
