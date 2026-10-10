import "./UpdateSplash.css";
import { t } from '../../i18n';
import { useUpdateProgress, type UpdateProgress } from '../../modules/updates';

function progressPercent(progress: UpdateProgress): number | null {
  if (progress.phase === 'installing') return 100;
  if (!progress.total) return null;
  return Math.min(100, Math.round((progress.downloaded / progress.total) * 100));
}

export function UpdateSplash() {
  const progress = useUpdateProgress();
  if (!progress) return null;

  const installing = progress.phase === 'installing';
  const percent = progressPercent(progress);

  return (
    <div className="q-update-splash" role="status" aria-live="polite">
      <div className="q-update-splash__panel">
        <strong>{installing ? t('update.installing') : t('update.downloading')}</strong>
        <span>
          {installing
            ? t('update.willOpen')
            : percent === null
              ? t('update.preparing')
              : `${percent}%`}
        </span>
        {percent !== null && (
          <div className="q-update-splash__track" aria-hidden="true">
            <div className="q-update-splash__value" style={{ width: `${percent}%` }} />
          </div>
        )}
      </div>
    </div>
  );
}
