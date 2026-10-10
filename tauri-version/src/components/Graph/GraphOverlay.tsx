import { plural, t } from '../../i18n';
import { Network } from 'lucide';
import { Button } from '../Common/Button';
import { EmptyState } from '../Common/EmptyState';

interface GraphStatusProps {
  status: string;
  edgeCount: number | null;
  drawnEdges: number;
  onScaleReadout: (element: HTMLSpanElement | null) => void;
}

export function GraphStatus({
  status,
  edgeCount,
  drawnEdges,
  onScaleReadout,
}: GraphStatusProps) {
  return (
    <div className="q-graph-status">
      <span>{status}</span>
      {edgeCount !== null && <span>{plural('graph.edges', edgeCount)}</span>}
      {edgeCount !== null && edgeCount > drawnEdges && (
        <span>{t('graph.drawnEdges', { count: drawnEdges })}</span>
      )}
      <span ref={onScaleReadout} />
    </div>
  );
}

interface GraphNoticeProps {
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}

export function GraphNotice({ title, description, actionLabel, onAction }: GraphNoticeProps) {
  return (
    <div className="q-graph-message">
      <EmptyState icon={Network} title={title} description={description}>
        {actionLabel && onAction && (
          <Button size="s" onClick={onAction}>
            {actionLabel}
          </Button>
        )}
      </EmptyState>
    </div>
  );
}
