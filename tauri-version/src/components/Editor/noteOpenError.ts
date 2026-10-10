import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { t } from '../../i18n';
import { RecoverableError } from '../Common/ErrorBoundary';
import type { Recovery } from '../Common/ErrorReport';

function errorCode(error: unknown): string | null {
  return typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : null;
}

export function failureReason(error: unknown): string {
  const code = errorCode(error);
  if (code === 'invalid_utf8') return t('editor.openFailedEncoding');
  if (code !== null) return `${code}: ${JSON.stringify((error as { details?: unknown }).details)}`;
  return error instanceof Error ? error.stack ?? error.message : String(error);
}

function recoveryFor(filePath: string, error: unknown): Recovery | null {
  if (errorCode(error) === 'invalid_utf8') {
    return { label: t('editor.revealFile'), retriesAfter: false, run: () => revealItemInDir(filePath) };
  }
  return null;
}

export function noteOpenError(filePath: string, error: unknown): RecoverableError {
  return new RecoverableError(
    t('editor.openFailed', { path: filePath, reason: failureReason(error) }),
    t('editor.openFailedTitle'),
    recoveryFor(filePath, error),
  );
}
