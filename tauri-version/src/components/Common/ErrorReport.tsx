import { useState } from 'react';
import { Button } from './Button';
import './ErrorBoundary.css';
import { t } from '../../i18n';

export interface Recovery {
    label: string;
    run: () => Promise<void>;
    retriesAfter: boolean;
}

interface ErrorReportProps {
    title: string;
    report: string;
    recovery: Recovery | null;
    onRetry: () => void;
}

export function ErrorReport({ title, report, recovery, onRetry }: ErrorReportProps) {
    const [recoveryFailure, setRecoveryFailure] = useState<string | null>(null);
    const copyReport = () => {
        navigator.clipboard.writeText(report).catch((error) => {
            console.error('Failed to copy the error report', error);
        });
    };
    const recover = (action: Recovery) => {
        setRecoveryFailure(null);
        action.run().then(() => {
            if (action.retriesAfter) onRetry();
        }).catch((error) => {
            console.error('Recovery action failed', error);
            setRecoveryFailure(error instanceof Error ? error.message : JSON.stringify(error));
        });
    };

    return (
        <div className="q-error-boundary q-selectable">
            <h2>{title}</h2>
            <p className="q-error-boundary-hint">{t('error.hint')}</p>
            <pre className="q-error-boundary-report">{report}</pre>
            {recoveryFailure !== null && (
                <p className="q-error-boundary-hint" role="alert">
                    {t('error.recoveryFailed', { reason: recoveryFailure })}
                </p>
            )}
            <div className="q-error-boundary-actions">
                <Button variant="ghost" onClick={copyReport}>{t('error.copy')}</Button>
                {recovery && <Button onClick={() => recover(recovery)}>{recovery.label}</Button>}
                {!recovery?.retriesAfter && (
                    <Button variant={recovery ? 'ghost' : undefined} onClick={onRetry}>{t('error.retry')}</Button>
                )}
            </div>
        </div>
    );
}
