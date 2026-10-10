import { EditorState } from '@codemirror/state';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { rewriteDocument } from '../../modules/documents/documentGateway';
import { appendQuoteToDocument, applyTemplateToDocument } from './docMutations';
import { editorFor } from './openEditors';

vi.mock('../../modules/documents/documentGateway', () => ({ rewriteDocument: vi.fn(async () => true) }));
vi.mock('./openEditors', () => ({ editorFor: vi.fn() }));

function fakeView(text: string) {
  const view = {
    state: EditorState.create({ doc: text }),
    dispatch: vi.fn((spec: Parameters<EditorState['update']>[0]) => {
      view.state = view.state.update(spec).state;
    }),
  };
  return view;
}

describe('document mutations from the interface', () => {
  beforeEach(() => vi.clearAllMocks());

  it('applies a template through the open editor so that Ctrl+Z can undo it', async () => {
    const view = fakeView('текст');
    vi.mocked(editorFor).mockReturnValue(view as never);

    await applyTemplateToDocument('C:/База/Заметка.md', '## Шаблон\n');

    expect(view.state.doc.toString()).toBe('текст\n\n## Шаблон\n');
    expect(view.dispatch.mock.calls[0][0]).toMatchObject({ userEvent: 'input' });
    expect(rewriteDocument).not.toHaveBeenCalled();
  });

  it('fills an empty note with the template instead of appending to blank lines', async () => {
    const view = fakeView('\n\n');
    vi.mocked(editorFor).mockReturnValue(view as never);
    await applyTemplateToDocument('C:/База/Заметка.md', '## Шаблон\n');
    expect(view.state.doc.toString()).toBe('## Шаблон\n');
  });

  it('writes through the document core when the note is not open in an editor', async () => {
    vi.mocked(editorFor).mockReturnValue(null);
    await appendQuoteToDocument('C:/База/Книга.md', '> цитата');
    const [path, rewrite] = vi.mocked(rewriteDocument).mock.calls[0];
    expect(path).toBe('C:/База/Книга.md');
    expect(rewrite('текст\n')).toBe('текст\n\n> цитата');
    expect(rewrite('текст')).toBe('текст\n\n> цитата');
  });
});
