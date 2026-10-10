import { SidebarNavItem } from '../Layout/SidebarNavItem';
import { t } from '../../i18n';
import { settingsSections, SETTINGS_SECTION_ICONS } from './constants';
import type { SettingsSectionId } from './types';
import './SettingsNav.css';

interface SettingsNavProps {
  section: SettingsSectionId;
  onSelect: (section: SettingsSectionId) => void;
}

export function SettingsNav({ section, onSelect }: SettingsNavProps) {
  return (
    <nav className="q-settings-nav" aria-label={t('settings.navAria')}>
      {settingsSections().map((item) => (
        <SidebarNavItem
          key={item.id}
          label={item.label}
          icon={SETTINGS_SECTION_ICONS[item.id]}
          active={section === item.id}
          onClick={() => onSelect(item.id)}
        />
      ))}
    </nav>
  );
}
