export function parseHex(hex) {
  if (!hex || typeof hex !== 'string') return null;
  let h = hex.trim().toLowerCase().replace(/^#/, '');
  if (h.length === 3) h = h.split('').map((c) => c + c).join('');
  if (h.length === 6) {
    return {
      r: parseInt(h.slice(0, 2), 16),
      g: parseInt(h.slice(2, 4), 16),
      b: parseInt(h.slice(4, 6), 16),
      a: 1,
    };
  }
  if (h.length === 8) {
    return {
      r: parseInt(h.slice(0, 2), 16),
      g: parseInt(h.slice(2, 4), 16),
      b: parseInt(h.slice(4, 6), 16),
      a: Math.round((parseInt(h.slice(6, 8), 16) / 255) * 1000) / 1000,
    };
  }
  return null;
}

export function parseRgb(text) {
  const m = String(text).trim().match(/^rgba?\(\s*([\d.]+)\s*[,\s]\s*([\d.]+)\s*[,\s]\s*([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/i);
  if (!m) return null;
  let a = 1;
  if (m[4] != null) {
    a = m[4].endsWith('%') ? parseFloat(m[4]) / 100 : parseFloat(m[4]);
  }
  return {
    r: Math.round(Number(m[1])),
    g: Math.round(Number(m[2])),
    b: Math.round(Number(m[3])),
    a: Math.round(a * 1000) / 1000,
  };
}

export function rgbaToHex({ r, g, b, a = 1 }) {
  const h = (n) => Math.round(n).toString(16).padStart(2, '0');
  const base = `#${h(r)}${h(g)}${h(b)}`;
  if (a >= 0.999) return base;
  return `${base}${h(a * 255)}`;
}

export function colorsEqual(a, b, { rgbTol = 1, alphaTol = 0.02 } = {}) {
  if (!a || !b) return false;
  return (
    Math.abs(a.r - b.r) <= rgbTol
    && Math.abs(a.g - b.g) <= rgbTol
    && Math.abs(a.b - b.b) <= rgbTol
    && Math.abs(a.a - b.a) <= alphaTol
  );
}

export function parseColorMix(text) {
  const m = String(text).trim().match(
    /^color-mix\(\s*in\s+srgb\s*,\s*var\(\s*(--q-[a-z0-9-]+)\s*\)\s+([\d.]+)%\s*,\s*transparent\s*\)$/i,
  );
  if (!m) return null;
  return { varName: m[1], pct: Number(m[2]) };
}

export function applyMix(base, pct) {
  if (!base) return null;
  return {
    r: base.r,
    g: base.g,
    b: base.b,
    a: Math.round((base.a * (pct / 100)) * 1000) / 1000,
  };
}

export function figmaNameToCss(name) {
  return `--q-${name.split('/').map((p) => p.trim().toLowerCase().replace(/\s+/g, '-')).join('-')}`;
}
