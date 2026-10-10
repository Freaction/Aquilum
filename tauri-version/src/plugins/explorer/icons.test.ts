import { describe, expect, it } from 'vitest';
import { icons } from 'lucide';
import { ICON_NAMES, iconByName } from './icons';

describe('имена иконок проводника', () => {
  it('берёт имена из установленного Lucide и находит kebab-case', () => {
    expect(ICON_NAMES.length).toBeGreaterThan(1900);
    expect(ICON_NAMES.length).toBeLessThanOrEqual(Object.keys(icons).length);
    expect(ICON_NAMES).toContain('calendar-days');
    expect(ICON_NAMES).toContain('a-arrow-down');
    expect(ICON_NAMES).toEqual([...new Set(ICON_NAMES)].sort());
    expect(iconByName('calendar-days')).toBe(icons.CalendarDays);
    expect(iconByName('gamepad-2')).toBe(icons.Gamepad2);
    expect(iconByName('grid-2x2')).toBe(icons.Grid2x2);
    expect(iconByName('axis-3d')).toBe(icons.Axis3d);
    expect(iconByName('arrow-down-0-1')).toBe(icons.ArrowDown01);
    expect(iconByName('clock-12')).toBe(icons.Clock12);
    expect(iconByName('grid-2x2-x')).toBe(icons.Grid2x2X);
    expect(iconByName('gamepad2')).toBeUndefined();
    expect(iconByName('circle-1')).toBeUndefined();
    expect(iconByName('missing-icon')).toBeUndefined();
    expect(iconByName('toString')).toBeUndefined();
  });
});
