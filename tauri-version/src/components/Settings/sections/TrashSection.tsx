import { useCallback, useEffect, useState } from 'react';
import {
  cleanupTrash,
  formatTrashCount,
  formatTrashSize,
  getTrashState,
  openTrash,
  trashRetentionOptions,
  type TrashState,
} from '../../../modules/trash';
import { t } from '../../../i18n';
import { TextButton } from '../../Common/TextButton';
import { Dropdown } from '../../Common/Dropdown';
import { Row } from '../Row';
import { Section } from '../Section';
import type { SettingsSectionProps } from '../types';
import { TrashedNotes } from './TrashedNotes';

interface TrashSectionProps extends SettingsSectionProps {
  workspacePath: string | null;
}

export function TrashSection({ config, workspacePath, onChange }: TrashSectionProps) {
  const [state, setState] = useState<TrashState | null>(null);
  const retentionDays = config.trash?.retentionDays ?? 30;

  const refresh = useCallback(() => {
    if (!workspacePath) return;
    void getTrashState(workspacePath).then(setState).catch((error) => {
      console.error('Failed to read trash state', error);
    });
  }, [workspacePath]);

  useEffect(refresh, [refresh]);

  const setRetention = (value: string) => {
    const days = Number(value);
    onChange({ ...config, trash: { retentionDays: days } });
    if (!workspacePath) return;
    void cleanupTrash(workspacePath, days).then(refresh).catch((error) => {
      console.error('Failed to clean up trash', error);
    });
  };

  const description = state
    ? `${formatTrashCount(state.count)}, ${formatTrashSize(state.bytes)}`
    : t('settings.trash.emptyHint');

  return (
    <>
      <Section title={t('settings.trash.section')}>
        <Row
          label={t('settings.trash.retention')}
          description={t('settings.trash.retentionHint')}
        >
          <Dropdown
            ariaLabel={t('settings.trash.retention')}
            value={String(retentionDays)}
            options={trashRetentionOptions()}
            onChange={setRetention}
          />
        </Row>
        <Row label={t('settings.trash.deleted')} description={description}>
          <TextButton
            disabled={!workspacePath}
            onClick={() => {
              if (workspacePath) void openTrash(workspacePath);
            }}
          >
            {t('settings.trash.open')}
          </TextButton>
        </Row>
      </Section>
      {workspacePath ? (
        <TrashedNotes key={workspacePath} workspacePath={workspacePath} onRestored={refresh} />
      ) : null}
    </>
  );
}
