import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { resolveVaultAssetUrl } from '../docs/vaultAssets';
import { escapeHtml, renderMarkdownToHtml } from './markdownToHtml';
import './print.css';

interface PdfExportRequest {
  title: string;
  markdown: string;
  workspacePath: string | null;
}

function mountPrintRoot(request: PdfExportRequest): HTMLElement {
  const root = document.createElement('article');
  root.className = 'q-print-root';
  root.innerHTML = `<h1 class="q-print-title">${escapeHtml(request.title)}</h1>`
    + renderMarkdownToHtml(request.markdown, {
      resolveImageUrl: (target) => resolveVaultAssetUrl(request.workspacePath, target, target),
    });
  document.body.append(root);
  return root;
}

function awaitImage(image: HTMLImageElement): Promise<void> {
  if (image.complete) return Promise.resolve();
  return new Promise((resolve) => {
    image.addEventListener('load', () => resolve(), { once: true });
    image.addEventListener('error', () => resolve(), { once: true });
  });
}

async function awaitLayout(root: HTMLElement): Promise<void> {
  await Promise.all(Array.from(root.querySelectorAll('img')).map(awaitImage));
  await document.fonts.ready.catch(() => undefined);
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
}

async function nativeExportPath(title: string): Promise<string | null> {
  const target = await save({
    defaultPath: `${title}.pdf`,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  });
  return target ?? null;
}

export async function exportNoteToPdf(request: PdfExportRequest): Promise<boolean> {
  const native = await invoke<boolean>('pdf_export_is_native').catch(() => false);
  if (!native) return printWithDialog(request);
  const target = await nativeExportPath(request.title);
  if (!target) return false;
  await exportNoteToPdfFile(request, target);
  return true;
}

export async function exportNoteToPdfFile(request: PdfExportRequest, target: string): Promise<void> {
  const root = mountPrintRoot(request);
  try {
    await awaitLayout(root);
    await invoke<void>('export_pdf', { path: target });
  } finally {
    root.remove();
  }
}

async function printWithDialog(request: PdfExportRequest): Promise<boolean> {
  const root = mountPrintRoot(request);
  try {
    await awaitLayout(root);
    window.print();
    return true;
  } finally {
    root.remove();
  }
}
