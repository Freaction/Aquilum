export interface IndexRevisionEvent {
  workspacePath: string;
  generation: number;
  revision: number;
}

export const LINKS_CHANGED_EVENT = 'links-changed';
