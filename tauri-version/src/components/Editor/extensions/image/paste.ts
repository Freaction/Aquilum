import { EditorSelection } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { importIntoFiles } from '../../../../modules/docs/vaultFiles';
import {
  DEFAULT_IMAGE_ALIGN,
  formatImageEmbed,
  mediaExtensionForMime,
  mediaKindForMime,
} from '../../../../modules/docs/imageEmbeds';
import { livePreviewConfigFacet } from '../livePreviewConfig';
import { editorColumnWidth, lastImageArea } from './widgetDom';

function mediaFilesOf(transfer: DataTransfer | null): File[] {
  if (!transfer) return [];
  return Array.from(transfer.files).filter((file) => mediaKindForMime(file.type));
}

function fallbackNameOf(file: File): string {
  return mediaKindForMime(file.type) === 'video' ? 'video' : 'image';
}

function fileNameOf(file: File): string {
  if (file.name && file.name.includes('.')) return file.name;
  return `${fallbackNameOf(file)}.${mediaExtensionForMime(file.type)}`;
}

async function naturalWidthOf(file: File): Promise<number | null> {
  if (mediaKindForMime(file.type) !== 'image') return null;
  try {
    const bitmap = await createImageBitmap(file);
    const { width } = bitmap;
    bitmap.close();
    return width || null;
  } catch {
    return null;
  }
}

function insertImageLines(view: EditorView, at: number, lines: string[]): void {
  const line = view.state.doc.lineAt(at);
  const prefix = at === line.from ? '' : '\n';
  const restIsEmpty = view.state.doc.sliceString(at, line.to).trim() === '';
  const insert = `${prefix}${lines.join('\n\n')}\n${restIsEmpty ? '' : '\n'}`;

  view.dispatch({
    changes: { from: at, insert },
    selection: EditorSelection.cursor(at + insert.length),
    userEvent: 'input.paste',
    scrollIntoView: true,
  });
}

async function pasteMediaFiles(view: EditorView, files: File[], at: number): Promise<void> {
  const workspacePath = view.state.facet(livePreviewConfigFacet)?.workspacePath;
  if (!workspacePath) return;

  const maxWidth = lastImageArea() || editorColumnWidth(view);
  const lines: string[] = [];

  for (const file of files) {
    const natural = await naturalWidthOf(file);
    const width = natural && maxWidth ? Math.min(natural, maxWidth) : natural ?? maxWidth;
    const relative = await importIntoFiles(
      workspacePath,
      { kind: 'bytes', bytes: new Uint8Array(await file.arrayBuffer()), name: fileNameOf(file) },
      { fallbackName: fallbackNameOf(file), fallbackExt: mediaExtensionForMime(file.type) },
    );
    lines.push(formatImageEmbed({ src: relative, width, align: DEFAULT_IMAGE_ALIGN, crop: null }));
  }

  if (lines.length) insertImageLines(view, at, lines);
}

export const imageEmbedPaste = EditorView.domEventHandlers({
  paste(event, view) {
    if (view.state.readOnly) return false;
    const files = mediaFilesOf(event.clipboardData);
    if (!files.length) return false;

    event.preventDefault();
    pasteMediaFiles(view, files, view.state.selection.main.from).catch((error) => {
      console.error('Failed to import the pasted media', error);
    });
    return true;
  },
  drop(event, view) {
    if (view.state.readOnly) return false;
    const files = mediaFilesOf(event.dataTransfer);
    if (!files.length) return false;

    event.preventDefault();
    const at = view.posAtCoords({ x: event.clientX, y: event.clientY })
      ?? view.state.selection.main.from;
    pasteMediaFiles(view, files, at).catch((error) => {
      console.error('Failed to import the dropped media', error);
    });
    return true;
  },
});
