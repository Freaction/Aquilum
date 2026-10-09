import { useState } from 'react';
import { t } from '../../i18n';
import type { PluginSettingsProps } from '../registry';
import { TAG_COLORS } from '../../components/Editor/extensions/plugins/coloredTags';
import { Row } from '../../components/Settings/Row';
import { Input } from '../../components/Common/Input';
import { Dropdown } from '../../components/Common/Dropdown';
import { Button } from '../../components/Common/Button';
import './ColoredTagsSettings.css';

export function ColoredTagsSettings({ config, onChange }: PluginSettingsProps) {
  const settings = config.plugins.coloredTags;
  const [tag, setTag] = useState('');
  const [color, setColor] = useState<string>('blue');
  const name = tag.trim().replace(/^#/, '');
  const valid = /^[a-zA-Zа-яА-Я0-9_]+(?:\/[a-zA-Zа-яА-Я0-9_]+)*$/.test(name);
  const options = TAG_COLORS.map(value => ({ value, label: t(`plugins.coloredTags.colors.${value}`) }));
  const patch = (partial: Partial<typeof settings>) => onChange({
    ...config, plugins: { ...config.plugins, coloredTags: { ...settings, ...partial } },
  });
  const pin = (name: string, token: string) => patch({ tagColors: { ...settings.tagColors, [name]: token } });

  return <div className="q-colored-tags-settings">
    <Row label={t('plugins.coloredTags.mixNested')}>
      <Dropdown
        ariaLabel={t('plugins.coloredTags.mixNested')}
        value={String(settings.mixNested)}
        options={[
          { value: 'true', label: t('plugins.coloredTags.mixOn') },
          { value: 'false', label: t('plugins.coloredTags.mixOff') },
        ]}
        onChange={value => patch({ mixNested: value === 'true' })}
      />
    </Row>
    {Object.entries(settings.tagColors).map(([name, token]) => <Row key={name} label={`#${name}`}>
      <div className="q-colored-tags-settings__controls">
        <Dropdown value={token} options={options} ariaLabel={`${t('plugins.coloredTags.color')} #${name}`} onChange={value => pin(name, value)} />
        <Button variant="ghost" size="s" aria-label={`${t('plugins.coloredTags.remove')} #${name}`} onClick={() => {
          const tagColors = { ...settings.tagColors };
          delete tagColors[name];
          patch({ tagColors });
        }}>{t('plugins.coloredTags.remove')}</Button>
      </div>
    </Row>)}
    <Row label={t('plugins.coloredTags.tag')}>
      <div className="q-colored-tags-settings__controls">
        <Input value={tag} onChange={setTag} ariaLabel={t('plugins.coloredTags.tag')} placeholder="#project/alpha" />
        <Dropdown value={color} options={options} ariaLabel={t('plugins.coloredTags.color')} onChange={setColor} />
        <Button size="s" disabled={!valid} onClick={() => { pin(name, color); setTag(''); }}>{t('plugins.coloredTags.add')}</Button>
      </div>
    </Row>
  </div>;
}
