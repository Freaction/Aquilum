import { rewriteDocument } from '../../modules/documents/documentGateway';
import { editorFor } from './openEditors';

interface TextChange {
  from: number;
  to: number;
  insert: string;
}

function templateChange(current: string, template: string): TextChange {
  if (current.trim() === '') return { from: 0, to: current.length, insert: template };
  const gap = current.endsWith('\n') ? '\n' : '\n\n';
  return { from: current.length, to: current.length, insert: gap + template };
}

function quoteChange(current: string, quoteMd: string): TextChange {
  const gap = current.length > 0 && !current.endsWith('\n') ? '\n\n' : '\n';
  return { from: current.length, to: current.length, insert: gap + quoteMd };
}

function applied(current: string, change: TextChange): string {
  return current.slice(0, change.from) + change.insert + current.slice(change.to);
}

async function applyChange(path: string, change: (current: string) => TextChange): Promise<boolean> {
  const view = editorFor(path);
  if (!view) return rewriteDocument(path, (current) => applied(current, change(current)));
  view.dispatch({ changes: change(view.state.doc.toString()), userEvent: 'input' });
  return true;
}

export function applyTemplateToDocument(path: string, template: string): Promise<boolean> {
  return applyChange(path, (current) => templateChange(current, template));
}

export function appendQuoteToDocument(path: string, quoteMd: string): Promise<boolean> {
  return applyChange(path, (current) => quoteChange(current, quoteMd));
}
