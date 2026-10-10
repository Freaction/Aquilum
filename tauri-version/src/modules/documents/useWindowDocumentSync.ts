import { useEffect } from 'react';
import { flushDocuments, reconcileDocuments } from './documentGateway';

function report(action: string): (error: unknown) => void {
  return (error) => console.error(`Failed to ${action} open documents`, error);
}

export function useWindowDocumentSync(): void {
  useEffect(() => {
    const reconcile = () => {
      void reconcileDocuments().catch(report('reconcile'));
    };
    const flush = () => {
      void flushDocuments().catch(report('flush'));
    };
    const onVisibility = () => {
      if (document.visibilityState === 'visible') reconcile();
    };
    window.addEventListener('focus', reconcile);
    window.addEventListener('blur', flush);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      window.removeEventListener('focus', reconcile);
      window.removeEventListener('blur', flush);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, []);
}
