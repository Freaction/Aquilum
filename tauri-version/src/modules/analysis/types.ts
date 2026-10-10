export type AnalysisMethod = 'adamicAdar' | 'bm25f';

export type SidebarSourceMethod = AnalysisMethod | 'wixiv';

export interface AnalysisResult {
  path: string;
  title: string;
  rawScore: number;
  similarity?: number;
  confidence?: number;
  reasons: string[];
}

interface AnalysisMethodFlags {
  enableBm25f: boolean;
  enableAdamicAdar: boolean;
  enableWikixiv: boolean;
}

const SIDEBAR_METHOD_ORDER: SidebarSourceMethod[] = ['bm25f', 'adamicAdar', 'wixiv'];

export function enabledSidebarMethods(
  flags: AnalysisMethodFlags | null | undefined,
): SidebarSourceMethod[] {
  if (!flags) return SIDEBAR_METHOD_ORDER;
  const enabled: Record<SidebarSourceMethod, boolean> = {
    bm25f: flags.enableBm25f,
    adamicAdar: flags.enableAdamicAdar,
    wixiv: flags.enableWikixiv,
  };
  return SIDEBAR_METHOD_ORDER.filter((method) => enabled[method]);
}

export function isGraphAnalysisMethod(
  method: SidebarSourceMethod,
): method is AnalysisMethod {
  return method !== 'wixiv';
}
