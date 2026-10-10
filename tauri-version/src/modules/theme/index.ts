export type Theme = 'light' | 'dark' | 'system';

type ThemeMode = 'light' | 'dark';

let currentTheme: Theme = 'system';
let currentMode: ThemeMode = 'light';
const modeListeners = new Set<() => void>();

function resolveTheme(theme: Theme): ThemeMode {
  return theme === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : theme;
}

function applyTheme(theme: Theme): void {
  const mode = resolveTheme(theme);
  document.documentElement.setAttribute('data-theme', mode);
  if (mode === currentMode) return;
  currentMode = mode;
  modeListeners.forEach((notify) => notify());
}

export function themeMode(): ThemeMode {
  return currentMode;
}

export function subscribeThemeMode(listener: () => void): () => void {
  modeListeners.add(listener);
  return () => modeListeners.delete(listener);
}

export function setTheme(theme: Theme): void {
  currentTheme = theme;
  applyTheme(theme);
}

export function initTheme(): () => void {
  applyTheme(currentTheme);

  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  const handleChange = () => {
    if (currentTheme === 'system') applyTheme(currentTheme);
  };

  mediaQuery.addEventListener('change', handleChange);
  return () => mediaQuery.removeEventListener('change', handleChange);
}
