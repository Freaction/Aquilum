import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '../../components/Common/Button';
import { t } from '../../i18n';
import { useSettingsStore } from '../../modules/settings';
import { reloadPluginVaultData } from '../vaultData';

interface ImportReport {
  imported: string[];
  skipped: { id: string; reason: string }[];
  overwritten: string[];
}

export function ObsidianImport({ workspacePath }: { workspacePath: string | null }) {
  if (!workspacePath) return <Button disabled title={t('plugins.import.noObsidian')}>{t('plugins.import.button')}</Button>;
  return <VaultImport key={workspacePath} workspacePath={workspacePath} />;
}

function VaultImport({ workspacePath }: { workspacePath: string }) {
  const { loadConfig } = useSettingsStore();
  const [detected, setDetected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState('');
  const mounted = useRef(false);
  const locked = useRef(false);

  useEffect(() => {
    mounted.current = true;
    void invoke<boolean>('obsidian_detect', { workspacePath }).then(value => {
      if (mounted.current) setDetected(value);
    }).catch(failure => {
      if (mounted.current) setError(errorMessage(failure));
    });
    return () => { mounted.current = false; };
  }, [workspacePath]);

  const start = async () => {
    if (!detected || locked.current) return;
    locked.current = true;
    setBusy(true);
    setError('');
    try {
      const result = await invoke<ImportReport>('obsidian_import', { workspacePath });
      await Promise.all([loadConfig(), reloadPluginVaultData(workspacePath)]);
      if (mounted.current) setReport(result);
    } catch (failure) {
      if (mounted.current) setError(errorMessage(failure));
    } finally {
      locked.current = false;
      if (mounted.current) setBusy(false);
    }
  };

  return <div aria-busy={busy}>
    <Button disabled={!detected || busy} title={!detected ? t('plugins.import.noObsidian') : undefined} onClick={() => { void start(); }}>
      {t(busy ? 'plugins.import.importing' : 'plugins.import.button')}
    </Button>
    {!detected && <p>{t('plugins.import.noObsidian')}</p>}
    {error && <p role="alert">{error}</p>}
    {report && <div className="q-plugins__import-report" role="status">
      <h3>{t('plugins.import.report')}</h3>
      <h4>{t('plugins.import.imported')}</h4>
      {report.imported.length ? <ul>{report.imported.map(id => <li key={id}>{id}</li>)}</ul> : <p>{t('plugins.import.empty')}</p>}
      {!!report.overwritten?.length && <><h4>{t('plugins.import.overwritten')}</h4><ul>{report.overwritten.map(item => <li key={item}>{item}</li>)}</ul></>}
      {!!report.skipped.length && <><h4>{t('plugins.import.skipped')}</h4><ul>{report.skipped.map((row, index) => <li key={index}>{row.id}: {t(`plugins.import.reasons.${row.reason}`)}</li>)}</ul></>}
    </div>}
  </div>;
}

function errorMessage(error: unknown): string {
  return (error as { details?: { message?: string } } | null)?.details?.message ?? String(error);
}
