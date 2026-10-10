import { useSettingsStore } from '../../../modules/settings';
import { shortcutCatalog } from '../../../config/shortcutCatalog';
import { formatShortcut } from '../../../config/shortcuts';
import { Row } from '../Row';
import { Section } from '../Section';

export function ShortcutsSection() {
  const { config } = useSettingsStore();
  return (
    <>
      {shortcutCatalog(config).map((group) => (
        <Section key={group.title} title={group.title}>
          {group.items.map((item) => (
            <Row key={item.label} label={item.label}>
              <kbd className="q-settings-row__kbd">{formatShortcut(item.shortcut)}</kbd>
            </Row>
          ))}
        </Section>
      ))}
    </>
  );
}
