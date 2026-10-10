import { readNoteFields } from '../../../../modules/docs/noteFields';
import {
  getPages,
  hydrateFromFm,
  type BookPageRuntimeEntry,
} from '../../../../modules/docs/bookPageRuntime';
import { resolveVaultAbsolutePath, resolveVaultAssetUrl } from '../../../../modules/docs/vaultAssets';
import type { WikiLinkResolver } from '../../../../modules/links';
import type { BookCalloutModel } from './model';
import type { BookCalloutViewData } from './widgetDom';
import defaultBookCover from '../../../../assets/covers/default-book-cover.webp';

const hydrationCache = new Map<string, BookCalloutViewData>();

function hydrationCacheKey(workspacePath: string, wikiTarget: string): string {
  return `${workspacePath}\0${wikiTarget}`;
}

export function viewDataFromModel(
  model: BookCalloutModel,
  workspacePath: string | null,
): BookCalloutViewData {
  return {
    ...model,
    pages: null,
    coverUrl: resolveVaultAssetUrl(workspacePath, model.cover || undefined, defaultBookCover),
    linked: Boolean(model.wikiTarget),
    canRead: Boolean(model.filePath),
  };
}

export function resolveCalloutData(
  model: BookCalloutModel,
  workspacePath: string | null,
): BookCalloutViewData | null {
  if (!model.wikiTarget || !workspacePath) return viewDataFromModel(model, workspacePath);

  const cached = hydrationCache.get(hydrationCacheKey(workspacePath, model.wikiTarget));
  if (cached) {
    const pages = cached.bookPagePath
      ? getPages(cached.bookPagePath) ?? cached.pages
      : cached.pages;
    return { ...cached, pages };
  }
  return model.cover ? viewDataFromModel(model, workspacePath) : null;
}

export function viewDataFromEntry(
  base: BookCalloutViewData,
  absolute: string,
  entry: BookPageRuntimeEntry,
  workspacePath: string | null,
): BookCalloutViewData {
  const livePages = getPages(absolute) ?? entry.pages;
  return {
    ...base,
    title: entry.title || base.title,
    author: entry.author || base.author,
    pages: livePages,
    coverUrl: resolveVaultAssetUrl(workspacePath, entry.cover || undefined, defaultBookCover) || base.coverUrl,
    filePath: entry.bookFile || base.filePath,
    bookPagePath: absolute,
    canRead: Boolean(entry.bookFile || base.filePath),
    linked: true,
  };
}

export function bookCalloutSignature(data: BookCalloutViewData | null): string {
  if (!data) return '';
  return [data.title, data.author, data.coverUrl, data.filePath, data.bookPagePath ?? ''].join('\u0000');
}

type PrefetchResult = {
  changed: boolean;
  covers: string[];
};

export async function prefetchLinkedCallouts(
  models: BookCalloutModel[],
  workspacePath: string,
  resolveWikiLinks: WikiLinkResolver,
  refresh = false,
): Promise<PrefetchResult> {
  const byTarget = new Map<string, BookCalloutModel>();
  for (const model of models) {
    const target = model.wikiTarget;
    if (!target || byTarget.has(target)) continue;
    const cached = hydrationCache.get(hydrationCacheKey(workspacePath, target));
    if (!refresh && cached?.bookPagePath) continue;
    byTarget.set(target, model);
  }
  const targets = [...byTarget.keys()];
  if (targets.length === 0) return { changed: false, covers: [] };

  const resolved = await resolveWikiLinks(targets);
  const absolute = targets.map((_, index) => {
    const path = resolved.paths[index];
    return path ? resolveVaultAbsolutePath(workspacePath, path) : null;
  });
  const known = absolute.filter((path): path is string => Boolean(path));
  const fields = await readNoteFields(workspacePath, known);
  const fieldsByPath = new Map(known.map((path, index) => [path, fields[index]]));

  const covers: string[] = [];
  let changed = false;
  targets.forEach((target, index) => {
    const base = viewDataFromModel(byTarget.get(target)!, workspacePath);
    const path = absolute[index];
    const data = path
      ? viewDataFromEntry(base, path, hydrateFromFm(path, fieldsByPath.get(path)), workspacePath)
      : base;
    const key = hydrationCacheKey(workspacePath, target);
    const previous = hydrationCache.get(key);
    hydrationCache.set(key, data);
    if (previous && bookCalloutSignature(previous) === bookCalloutSignature(data)) return;
    changed = true;
    if (data.coverUrl) covers.push(data.coverUrl);
  });

  return { changed, covers };
}
