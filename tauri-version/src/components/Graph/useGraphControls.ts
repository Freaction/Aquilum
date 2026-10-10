import { useCallback, useEffect, useMemo, useState, type RefObject } from 'react';
import { DEFAULT_PREFERENCES, type GraphPreferences } from './graphDisplay';
import type { GraphControls } from './GraphSettings';
import type { GraphDateRange, GraphRenderer } from './renderer';
import type { GraphCounts } from './useGraphSnapshot';
import { useLocalState } from '../../modules/workspace/uiPersist';

export function useGraphControls(
  rendererRef: RefObject<GraphRenderer | null>,
  generation: number,
  counts: GraphCounts | null,
  range: GraphDateRange,
) {
  const [storedPreferences, setPreferences] = useLocalState<GraphPreferences>(
    'aquilum_graph_preferences',
    DEFAULT_PREFERENCES,
  );
  const preferences = useMemo(
    () => ({ ...DEFAULT_PREFERENCES, ...storedPreferences }),
    [storedPreferences],
  );
  const [createdShare, setCreatedShare] = useState(0);
  const controls = useMemo<GraphControls>(
    () => ({ ...preferences, createdShare }),
    [preferences, createdShare],
  );

  useEffect(() => {
    rendererRef.current?.applyDisplay({
      ...preferences,
      createdFrom: createdShare === 0
        ? Number.NEGATIVE_INFINITY
        : range.oldest + (range.newest - range.oldest) * createdShare,
    });
  }, [counts, createdShare, generation, preferences, range.newest, range.oldest, rendererRef]);

  const patchControls = useCallback((patch: Partial<GraphControls>) => {
    const { createdShare: nextShare, ...rest } = patch;
    if (nextShare !== undefined) setCreatedShare(nextShare);
    if (Object.keys(rest).length > 0) {
      setPreferences((current) => ({ ...current, ...rest }));
    }
  }, [setPreferences]);

  return { controls, createdShare, setCreatedShare, patchControls };
}
