import { getLocale, t } from '../../i18n';
import { memo } from 'react';
import { RotateCcw } from 'lucide';
import { Icon } from '../Common/Icon';
import { IconButton } from '../Common/IconButton';
import { Slider } from '../Common/Slider';
import { SegmentedControl } from '../Common/SegmentedControl';
import { Switch } from '../Common/Switch';
import type { GraphPreferences, HeatmapAxis } from './graphDisplay';
import { MIN_SPREAD } from './nodeMetrics';
import type { GraphDateRange } from './renderer';
import './GraphSettings.css';

export interface GraphControls extends GraphPreferences {
  createdShare: number;
}

function depthHints() {
  return [t('graph.depth1'), t('graph.depth2'), t('graph.depth3')];
}

function heatmapAxes() {
  return [
    { value: 'none', label: t('graph.heatNone') },
    { value: 'modified', label: t('graph.heatModified') },
    { value: 'created', label: t('graph.heatCreated') },
  ];
}

interface GraphSettingsProps {
  controls: GraphControls;
  range: GraphDateRange;
  onChange: (patch: Partial<GraphControls>) => void;
  onRefresh: () => void;
}

export const GraphSettings = memo(function GraphSettings({
  controls,
  range,
  onChange,
  onRefresh,
}: GraphSettingsProps) {
  const threshold = range.oldest + (range.newest - range.oldest) * controls.createdShare;

  return (
    <aside className="q-graph-settings" aria-label={t('graph.settings')}>
      <div className="q-graph-settings-head">
        <span className="q-graph-settings-title">{t('graph.title')}</span>
        <IconButton label={t('graph.refresh')} size="small" onClick={onRefresh}>
          <Icon icon={RotateCcw} />
        </IconButton>
      </div>

      <Slider
        label={t('graph.nodeSize')}
        min={0.4}
        max={2.5}
        step={0.05}
        value={controls.nodeSize}
        hint={`${controls.nodeSize.toFixed(2)}×`}
        onChange={(nodeSize) => onChange({ nodeSize })}
      />

      <Slider
        label={t('graph.spread')}
        min={MIN_SPREAD}
        max={2.5}
        step={0.05}
        value={controls.spread}
        hint={`${controls.spread.toFixed(2)}×`}
        onChange={(spread) => onChange({ spread })}
      />

      <Slider
        label={t('graph.depth')}
        min={1}
        max={3}
        step={1}
        value={controls.highlightDepth}
        hint={depthHints()[controls.highlightDepth - 1]}
        onChange={(highlightDepth) => onChange({ highlightDepth })}
      />

      <Slider
        label={t('graph.createdAfter')}
        min={0}
        max={1}
        step={0.005}
        value={controls.createdShare}
        hint={controls.createdShare === 0 ? t('graph.all') : formatDay(threshold)}
        onChange={(createdShare) => onChange({ createdShare })}
      />

      <label className="q-graph-settings-toggle">
        <span>{t('graph.labels')}</span>
        <Switch
          checked={controls.labels}
          onChange={(labels) => onChange({ labels })}
        />
      </label>

      <div className="q-graph-settings-choice">
        <span>{t('graph.heatmap')}</span>
        <SegmentedControl
          stretch
          options={heatmapAxes()}
          value={controls.heatmapAxis}
          onChange={(value) => onChange({ heatmapAxis: value as HeatmapAxis })}
        />
      </div>
    </aside>
  );
});

function formatDay(day: number): string {
  if (!Number.isFinite(day) || day <= 0) return t('graph.all');
  return new Date(day * 86_400_000).toLocaleDateString(getLocale(), {
    day: '2-digit',
    month: '2-digit',
    year: '2-digit',
  });
}
