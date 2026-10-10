import { invoke } from '@tauri-apps/api/core';
import type { Backlink, OutgoingLink, WikiLinkResolution } from './types';

export function getBacklinks(workspacePath: string, documentPath: string): Promise<Backlink[]> {
  return invoke<Backlink[]>('get_backlinks', { workspacePath, documentPath });
}

export function getOutgoingLinks(
  workspacePath: string,
  documentPath: string,
): Promise<OutgoingLink[]> {
  return invoke<OutgoingLink[]>('get_outgoing_links', { workspacePath, documentPath });
}

export function resolveWikiLinks(
  workspacePath: string,
  sourcePath: string,
  targets: string[],
): Promise<WikiLinkResolution> {
  return invoke<WikiLinkResolution>('resolve_wiki_links', { workspacePath, sourcePath, targets });
}
