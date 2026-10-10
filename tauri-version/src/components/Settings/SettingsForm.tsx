import { useState } from 'react';
import { Button } from '../Common/Button';
import { t } from '../../i18n';
import type { AppConfig } from '../../modules/settings';
import type { SettingsSectionId } from './types';
import { AnalysisSection } from './sections/AnalysisSection';
import { EditorSection } from './sections/EditorSection';
import { DailyNotesSection } from './sections/DailyNotesSection';
import { McpSection } from './sections/McpSection';
import { ReaderSection } from './sections/ReaderSection';
import { HistorySection } from './sections/HistorySection';
import { TrashSection } from './sections/TrashSection';
import { SearchSection } from './sections/SearchSection';
import { ShortcutsSection } from './sections/ShortcutsSection';
import { SystemSection } from './sections/SystemSection';
import { UiSection } from './sections/UiSection';
import { TemplatesSection } from './sections/TemplatesSection';
import { FilesSection } from './sections/FilesSection';
import { PluginsSection } from './sections/PluginsSection';

interface SettingsFormProps {
  config: AppConfig | null;
  section: SettingsSectionId;
  workspacePath: string | null;
  homePage: string;
  onHomePageChange: (value: string) => void;
  onChange: (next: AppConfig) => void;
}

export function SettingsForm({
  config,
  section,
  workspacePath,
  homePage,
  onHomePageChange,
  onChange,
}: SettingsFormProps) {
  const [linkedSection, setLinkedSection] = useState<SettingsSectionId | null>(null);
  const [previousSection, setPreviousSection] = useState(section);
  if (previousSection !== section) {
    setPreviousSection(section);
    setLinkedSection(null);
  }
  if (section === 'plugins' && linkedSection && config) {
    return <>
      <Button variant="ghost" size="s" onClick={() => setLinkedSection(null)}>{t('plugins.back')}</Button>
      <SettingsForm config={config} section={linkedSection} workspacePath={workspacePath} homePage={homePage} onHomePageChange={onHomePageChange} onChange={onChange} />
    </>;
  }
  if (section === 'shortcuts') {
    return <ShortcutsSection />;
  }
  if (!config) return null;

  switch (section) {
    case 'ui':
      return (
        <UiSection
          config={config}
          workspacePath={workspacePath}
          homePage={homePage}
          onHomePageChange={onHomePageChange}
          onChange={onChange}
        />
      );
    case 'editor':
      return <EditorSection config={config} onChange={onChange} />;
    case 'reader':
      return <ReaderSection config={config} onChange={onChange} />;
    case 'search':
      return <SearchSection config={config} onChange={onChange} />;
    case 'dailyNotes':
      return <DailyNotesSection config={config} workspacePath={workspacePath} onChange={onChange} />;
    case 'templates':
      return <TemplatesSection config={config} workspacePath={workspacePath} onChange={onChange} />;
    case 'files':
      return <FilesSection config={config} onChange={onChange} />;
    case 'analysis':
      return <AnalysisSection config={config} onChange={onChange} />;
    case 'plugins':
      return <PluginsSection config={config} workspacePath={workspacePath} onChange={onChange} onNavigate={setLinkedSection} />;
    case 'mcp':
      return <McpSection config={config} />;
    case 'history':
      return <HistorySection config={config} onChange={onChange} />;
    case 'system':
      return <SystemSection config={config} onChange={onChange} />;
    case 'trash':
      return <TrashSection config={config} workspacePath={workspacePath} onChange={onChange} />;
  }
}
