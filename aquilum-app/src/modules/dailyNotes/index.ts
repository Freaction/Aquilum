export interface DailyNotesSettings {
  folder: string;
  format: string;
  template: string;
  openOnStartup: boolean;
  wordsPerDot: number;
  weekStart: 'locale' | 'monday' | 'sunday';
  confirmBeforeCreate: boolean;
}

export const DEFAULT_DAILY_NOTES_SETTINGS: DailyNotesSettings = {
  folder: '', format: 'YYYY-MM-DD', template: '', openOnStartup: false,
  wordsPerDot: 250, weekStart: 'locale', confirmBeforeCreate: true,
};
