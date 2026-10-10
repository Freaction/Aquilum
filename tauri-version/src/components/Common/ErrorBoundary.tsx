import React from 'react';
import { t } from '../../i18n';
import { ErrorReport, type Recovery } from './ErrorReport';

export class RecoverableError extends Error {
    constructor(message: string, readonly title: string, readonly recovery: Recovery | null) {
        super(message);
    }
}

interface ErrorBoundaryProps {
    children: React.ReactNode;
    resetKey?: string | null;
    title?: string;
    onCatch?: () => void;
}

interface ErrorBoundaryState {
    error: Error | null;
    componentStack: string | null;
}

function errorReport(error: Error, componentStack: string | null = null): string {
    const parts = [error.toString()];
    if (error.stack) parts.push('', error.stack);
    if (componentStack) parts.push('', componentStack);
    return parts.join('\n');
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
    constructor(props: ErrorBoundaryProps) {
        super(props);
        this.state = { error: null, componentStack: null };
    }

    static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
        return { error };
    }

    componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
        console.error('Render crashed:', error, errorInfo);
        this.props.onCatch?.();
        this.setState({ componentStack: errorInfo.componentStack ?? null });
    }

    componentDidUpdate(previousProps: ErrorBoundaryProps) {
        if (this.state.error && previousProps.resetKey !== this.props.resetKey) {
            this.reset();
        }
    }

    private reset = (): void => {
        this.setState({ error: null, componentStack: null });
    };

    render() {
        const { error, componentStack } = this.state;
        if (!error) return this.props.children;
        const recoverable = error instanceof RecoverableError ? error : null;
        return (
            <ErrorReport
                title={recoverable?.title ?? this.props.title ?? t('error.title')}
                report={errorReport(error, componentStack)}
                recovery={recoverable?.recovery ?? null}
                onRetry={this.reset}
            />
        );
    }
}
