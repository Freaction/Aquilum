import { readDocument } from '../documents/documentGateway';
import { isEmptyTabPath } from '../ui-state';

export async function resolveDocumentText(documentPath: string | null): Promise<string> {
  if (!documentPath || isEmptyTabPath(documentPath)) return '';
  try {
    return (await readDocument(documentPath)).text;
  } catch {
    return '';
  }
}
