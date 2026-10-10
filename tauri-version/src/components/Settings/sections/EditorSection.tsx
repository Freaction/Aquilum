import {
  DEFAULT_EDITOR_MAX_WIDTH_CH,
  DEFAULT_LINK_SUGGEST_MIN_CHARS,
  DEFAULT_LIVE_TABS,
  type AppConfig,
} from '../../../modules/settings';
import { Switch } from '../../Common/Switch';
import { t } from '../../../i18n';
import { FontSettingsRows } from '../FontSettingsRows';
import { Row } from '../Row';
import { Section } from '../Section';
import { ShortcutInput } from '../controls/ShortcutInput';
import { NumberControl } from '../controls/NumberControl';
import type { SettingsSectionProps } from '../types';

export function EditorSection({ config, onChange }: SettingsSectionProps) {
  const patchEditor = (partial: Partial<AppConfig['editor']>) => onChange({
    ...config,
    editor: { ...config.editor, ...partial },
  });

  return (
    <>
      <Section title={t('settings.font.section')}>
        <FontSettingsRows
          scope="editor"
          font={config.editor}
          onChange={patchEditor}
        />
        <Row label={t('settings.editor.lineHeight')}>
          <NumberControl
            ariaLabel={t('settings.editor.lineHeight')}
            value={config.editor.lineHeight}
            min={1.2}
            max={2}
            step={0.05}
            onChange={(lineHeight) => patchEditor({ lineHeight })}
          />
        </Row>
      </Section>
      <Section title={t('settings.editor.layout')}>
        <Row label={t('settings.editor.fullWidth')} description={t('settings.editor.fullWidthHint')}>
          <Switch
            checked={config.editor.fullWidth ?? false}
            label={t('settings.editor.fullWidth')}
            onChange={(fullWidth) => patchEditor({ fullWidth })}
          />
        </Row>
        <Row label={t('settings.editor.fullWidthShortcut')}>
          <ShortcutInput config={config} id="TOGGLE_FULL_WIDTH" value={config.editor.fullWidthShortcut} label={t('settings.editor.fullWidthShortcut')} onChange={fullWidthShortcut => patchEditor({ fullWidthShortcut })} />
        </Row>
        {!config.editor.fullWidth && (
          <Row label={t('settings.editor.maxWidth')} description={t('settings.editor.maxWidthHint')}>
            <NumberControl
              ariaLabel={t('settings.editor.maxWidthAria')}
              value={config.editor.maxWidthCh ?? DEFAULT_EDITOR_MAX_WIDTH_CH}
              min={40}
              max={120}
              step={1}
              onChange={(maxWidthCh) => patchEditor({ maxWidthCh })}
            />
          </Row>
        )}
      </Section>
      <Section title={t('settings.editor.tabs')}>
        <Row
          label={t('settings.editor.liveTabs')}
          description={t('settings.editor.liveTabsHint')}
        >
          <NumberControl
            ariaLabel={t('settings.editor.liveTabs')}
            value={config.editor.liveTabs ?? DEFAULT_LIVE_TABS}
            min={1}
            max={8}
            step={1}
            onChange={(liveTabs) => patchEditor({ liveTabs })}
          />
        </Row>
      </Section>
      <Section title={t('settings.editor.input')}>
        <Row label={t('settings.editor.smartDashes')} description={t('settings.editor.smartDashesHint')}>
          <Switch
            checked={config.editor.smartDashes}
            label={t('settings.editor.smartDashes')}
            onChange={(smartDashes) => patchEditor({ smartDashes })}
          />
        </Row>
        <Row
          label={t('settings.editor.listCallouts')}
          description={t('settings.editor.listCalloutsHint')}
        >
          <Switch
            checked={config.editor.listCallouts ?? true}
            label={t('settings.editor.listCallouts')}
            onChange={(listCallouts) => patchEditor({ listCallouts })}
          />
        </Row>
        <Row
          label={t('settings.editor.autoLinkTitle')}
          description={t('settings.editor.autoLinkTitleHint')}
        >
          <Switch
            checked={config.editor.autoLinkTitle ?? true}
            label={t('settings.editor.autoLinkTitle')}
            onChange={(autoLinkTitle) => patchEditor({ autoLinkTitle })}
          />
        </Row>
      </Section>
      <Section title={t('settings.editor.completion')}>
        <Row
          label={t('settings.editor.linkSuggest')}
          description={t('settings.editor.linkSuggestHint')}
        >
          <Switch
            checked={config.editor.linkSuggest ?? true}
            label={t('settings.editor.linkSuggest')}
            onChange={(linkSuggest) => patchEditor({ linkSuggest })}
          />
        </Row>
        <Row label={t('settings.editor.minChars')} description={t('settings.editor.minCharsHint')}>
          <NumberControl
            ariaLabel={t('settings.editor.minChars')}
            value={config.editor.linkSuggestMinChars ?? DEFAULT_LINK_SUGGEST_MIN_CHARS}
            min={1}
            max={5}
            step={1}
            onChange={(linkSuggestMinChars) => patchEditor({ linkSuggestMinChars })}
          />
        </Row>
      </Section>
      <Section title={t('settings.editor.saving')}>
        <Row label={t('settings.editor.saveDebounce')}>
          <NumberControl
            ariaLabel={t('settings.editor.saveDebounceAria')}
            value={config.editor.saveDebounceMs}
            min={250}
            max={10000}
            step={250}
            onChange={(saveDebounceMs) => patchEditor({ saveDebounceMs })}
          />
        </Row>
      </Section>
    </>
  );
}
