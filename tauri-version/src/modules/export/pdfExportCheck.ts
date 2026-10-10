import { exportNoteToPdfFile } from './exportPdf';

const SAMPLE = [
  'Проверка экспорта в PDF.',
  '',
  '## Раздел',
  '',
  '- первый пункт',
  '- второй пункт',
  '',
  'Последняя строка: AQUILUM-PDF-CHECK.',
].join('\n');

export function runPdfExportCheck(target: string): void {
  window.setTimeout(() => {
    exportNoteToPdfFile({ title: 'Экспорт', markdown: SAMPLE, workspacePath: null }, target)
      .then(() => console.info(`PDF export check written to ${target}`))
      .catch((error: unknown) => console.error('PDF export check failed', error));
  }, 3000);
}
