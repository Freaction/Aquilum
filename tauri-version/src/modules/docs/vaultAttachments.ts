import { resolveAttachments } from '../documents/fileGateway';
import { resolveVaultAssetUrl } from './vaultAssets';

type Request = {
  workspacePath: string;
  target: string;
  settle: (url: string) => void;
};

const MAX_CACHED_URLS = 512;

const urlByKey = new Map<string, string>();
const pendingByKey = new Map<string, Promise<string>>();
let queued: Request[] = [];

function keyOf(workspacePath: string, target: string): string {
  return `${workspacePath}\u0000${target.toLowerCase()}`;
}

export function cachedAttachmentUrl(
  workspacePath: string | null | undefined,
  target: string,
): string | undefined {
  if (!workspacePath) return undefined;
  return urlByKey.get(keyOf(workspacePath, target));
}

function rememberUrl(key: string, url: string): void {
  urlByKey.delete(key);
  urlByKey.set(key, url);
  if (urlByKey.size <= MAX_CACHED_URLS) return;
  const oldest = urlByKey.keys().next().value;
  if (oldest !== undefined) urlByKey.delete(oldest);
}

export function forgetAttachmentUrls(): void {
  urlByKey.clear();
  pendingByKey.clear();
}

async function flushQueue(): Promise<void> {
  const batch = queued;
  queued = [];

  const byWorkspace = new Map<string, Request[]>();
  for (const request of batch) {
    const group = byWorkspace.get(request.workspacePath);
    if (group) group.push(request);
    else byWorkspace.set(request.workspacePath, [request]);
  }

  for (const [workspacePath, requests] of byWorkspace) {
    const names = requests.map((request) => request.target);
    let relatives: (string | null)[] | null = null;
    try {
      relatives = await resolveAttachments(workspacePath, names);
    } catch (error) {
      console.error('Failed to resolve attachments', error);
    }
    requests.forEach((request, index) => {
      const relative = relatives?.[index];
      const url = relative ? resolveVaultAssetUrl(workspacePath, relative, '') : '';
      const key = keyOf(workspacePath, request.target);
      if (relatives) rememberUrl(key, url);
      pendingByKey.delete(key);
      request.settle(url);
    });
  }
}

export function attachmentUrl(
  workspacePath: string | null | undefined,
  target: string,
): Promise<string> {
  if (!workspacePath) return Promise.resolve('');
  const key = keyOf(workspacePath, target);

  const known = urlByKey.get(key);
  if (known !== undefined) return Promise.resolve(known);

  const pending = pendingByKey.get(key);
  if (pending) return pending;

  const request = new Promise<string>((settle) => {
    if (!queued.length) void Promise.resolve().then(flushQueue);
    queued.push({ workspacePath, target, settle });
  });
  pendingByKey.set(key, request);
  return request;
}
