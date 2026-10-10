export type HeatmapAxis = 'none' | 'modified' | 'created';

export interface GraphPreferences {
  nodeSize: number;
  spread: number;
  highlightDepth: number;
  labels: boolean;
  heatmapAxis: HeatmapAxis;
}

export const DEFAULT_PREFERENCES: GraphPreferences = {
  nodeSize: 1,
  spread: 1,
  highlightDepth: 1,
  labels: true,
  heatmapAxis: 'none',
};

export interface GraphDisplay extends GraphPreferences {
  createdFrom: number;
}

export const DEFAULT_DISPLAY: GraphDisplay = {
  ...DEFAULT_PREFERENCES,
  createdFrom: Number.NEGATIVE_INFINITY,
};
