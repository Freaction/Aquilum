import { PathQueue } from '../pathQueue';
import { absolutePath } from '../paths';
import { rewriteDocument } from './documentGateway';
import { EDIT_WRITE } from './fileGateway';
import { toggleTaskLine } from './taskCheckbox';

const toggles = new PathQueue<boolean>();

export async function toggleTaskInFile(
  workspacePath: string,
  relative: string,
  line: number,
): Promise<boolean> {
  const path = absolutePath(workspacePath, relative);
  return toggles.run(path, () => rewriteDocument(path, (text) => toggleTaskLine(text, line), EDIT_WRITE));
}
