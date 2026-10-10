import { useEffect } from 'react';
import { cleanupHistory } from '../modules/history';
import type { AppConfig } from '../modules/settings';
import { cleanupTrash } from '../modules/trash';
import { cleanupUiState } from '../modules/ui-state';

function useCleanup(
  workspacePath: string | null,
  retentionDays: number | undefined,
  cleanup: (workspacePath: string, retentionDays: number) => Promise<number>,
  what: string,
): void {
  useEffect(() => {
    if (!workspacePath || retentionDays === undefined) return;
    void cleanup(workspacePath, retentionDays).catch((error) => {
      console.error(`Failed to clean up ${what}`, error);
    });
  }, [cleanup, retentionDays, what, workspacePath]);
}

export function useRetentionCleanup(workspacePath: string | null, config: AppConfig | null): void {
  useCleanup(workspacePath, config?.trash.retentionDays, cleanupTrash, 'trash');
  useCleanup(workspacePath, config?.history.retentionDays, cleanupHistory, 'history');
  useMissingDocumentsCleanup(config?.trash.retentionDays);
}

function useMissingDocumentsCleanup(retentionDays: number | undefined): void {
  useEffect(() => {
    if (retentionDays === undefined) return;
    void cleanupUiState(retentionDays, Date.now()).catch((error) => {
      console.error('Failed to clean up missing documents', error);
    });
  }, [retentionDays]);
}
