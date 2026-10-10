import { getVersion } from '@tauri-apps/api/app';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useEffect, useRef, useState } from 'react';
import { Switch } from '../../Common/Switch';
import { t } from '../../../i18n';
import { openExternalUrl } from '../../../modules/openExternalUrl';
import type { AppConfig } from '../../../modules/settings';
import { checkForUpdate, installUpdate, UPDATES_UNAVAILABLE } from '../../../modules/updates';
import { TextButton } from '../../Common/TextButton';
import { Row } from '../Row';
import { Section } from '../Section';
import './SystemSection.css';

const AUTOSTART_AVAILABLE = !import.meta.env.DEV;

type UpdateCheck =
  | { state: 'idle' }
  | { state: 'checking' }
  | { state: 'latest' }
  | { state: 'available'; version: string }
  | { state: 'installing'; version: string }
  | { state: 'failed'; message: string };

function updateDescription(check: UpdateCheck): string {
  switch (check.state) {
    case 'idle': return t('settings.system.updatesIdle');
    case 'checking': return t('settings.system.updatesChecking');
    case 'latest': return t('settings.system.updatesLatest');
    case 'available': return t('settings.system.updatesAvailable', { version: check.version });
    case 'installing': return t('settings.system.updatesInstalling', { version: check.version });
    case 'failed':
      return check.message === UPDATES_UNAVAILABLE
        ? t('settings.system.updatesUnavailable')
        : t('settings.system.updatesFailed', { message: check.message });
  }
}

export function SystemSection({ config, onChange }: {
  config: AppConfig;
  onChange: (next: AppConfig) => void;
}) {
  const mounted = useRef(true);
  const [enabled, setEnabled] = useState(false);
  const [pending, setPending] = useState(AUTOSTART_AVAILABLE);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState('');
  const [update, setUpdate] = useState<UpdateCheck>({ state: 'idle' });

  useEffect(() => {
    mounted.current = true;
    if (AUTOSTART_AVAILABLE) {
      void isEnabled()
        .then((value) => {
          if (mounted.current) setEnabled(value);
        })
        .catch((reason) => {
          if (mounted.current) setError(String(reason));
        })
        .finally(() => {
          if (mounted.current) setPending(false);
        });
    }

    getVersion()
      .then((value) => {
        if (mounted.current) setVersion(value);
      })
      .catch((reason) => console.error('Failed to read the app version', reason));

    return () => {
      mounted.current = false;
    };
  }, []);

  const setAutostart = async (next: boolean) => {
    setPending(true);
    setError(null);
    try {
      if (next) await enable();
      else await disable();
      const actual = await isEnabled();
      if (mounted.current) setEnabled(actual);
    } catch (reason) {
      if (mounted.current) setError(String(reason));
      try {
        const actual = await isEnabled();
        if (mounted.current) setEnabled(actual);
      } catch (readError) {
        console.error('Failed to read the autostart state', readError);
      }
    } finally {
      if (mounted.current) setPending(false);
    }
  };

  const checkUpdates = async () => {
    setUpdate({ state: 'checking' });
    try {
      const available = await checkForUpdate();
      if (mounted.current) setUpdate(available ? { state: 'available', version: available } : { state: 'latest' });
    } catch (reason) {
      if (mounted.current) setUpdate({ state: 'failed', message: String(reason) });
    }
  };

  const installAvailable = async (target: string) => {
    setUpdate({ state: 'installing', version: target });
    try {
      await installUpdate();
    } catch (reason) {
      if (mounted.current) setUpdate({ state: 'failed', message: String(reason) });
    }
  };

  return (
    <>
      <Section title={t('settings.system.updates')}>
        <Row label={t('settings.system.autoUpdate')} description={t('settings.system.autoUpdateHint')}>
          <Switch
            checked={config.updates.auto}
            label={t('settings.system.autoUpdate')}
            onChange={(auto) => onChange({ ...config, updates: { auto } })}
          />
        </Row>
        {!config.updates.auto && (
          <Row label={t('settings.system.manualUpdate')} description={updateDescription(update)}>
            {update.state === 'available' ? (
              <TextButton onClick={() => void installAvailable(update.version)}>
                {t('settings.system.installUpdate')}
              </TextButton>
            ) : (
              <TextButton
                disabled={update.state === 'checking' || update.state === 'installing'}
                onClick={() => void checkUpdates()}
              >
                {t('settings.system.checkUpdates')}
              </TextButton>
            )}
          </Row>
        )}
      </Section>

      <Section title={t('settings.system.autostart')}>
        <Row
          label={t('settings.system.launch')}
          description={AUTOSTART_AVAILABLE ? error ?? t('settings.system.launchHint') : t('settings.system.launchUnavailable')}
        >
          <Switch
            checked={enabled}
            disabled={pending || !AUTOSTART_AVAILABLE}
            label={t('settings.system.launchAria')}
            onChange={setAutostart}
          />
        </Row>
      </Section>

      <Section title={t('settings.system.about')}>
        <Row label={t('settings.system.author')} description={t('settings.system.authorHint')}>
          <TextButton onClick={() => void openExternalUrl('https://t.me/dmitriy_yiu')}>
            https://t.me/dmitriy_yiu
          </TextButton>
        </Row>
        <Row label={t('settings.system.version')} description={t('settings.system.versionHint')}>
          <span className="q-settings-version">{version}</span>
        </Row>
      </Section>
    </>
  );
}
