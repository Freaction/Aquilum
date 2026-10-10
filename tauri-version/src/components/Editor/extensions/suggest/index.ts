import {
  acceptCompletion,
  autocompletion,
  closeCompletion,
  completionStatus,
  moveCompletionSelection,
  startCompletion,
} from '@codemirror/autocomplete';
import { Prec } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { suggestContext } from './context';
import { noteCompletionSource } from './source';

const SUGGESTION_LIMIT = 12;
const TYPING_DELAY_MS = 80;

interface NoteSuggestSettings {
  enabled: boolean;
  workspacePath: string | null;
  minChars: number;
}

export function noteSuggestExtension({ enabled, workspacePath, minChars }: NoteSuggestSettings) {
  if (!enabled) return [];
  return [
    autocompletion({
      override: [noteCompletionSource({ workspacePath, minChars, limit: SUGGESTION_LIMIT })],
      activateOnTyping: true,
      activateOnTypingDelay: TYPING_DELAY_MS,
      defaultKeymap: false,
      icons: false,
      selectOnOpen: true,
      tooltipClass: () => 'q-suggest',
      optionClass: () => 'q-suggest__item',
    }),
    reopenAfterDelete(minChars),
    Prec.highest(keymap.of([
      { key: 'ArrowDown', run: moveCompletionSelection(true) },
      { key: 'ArrowUp', run: moveCompletionSelection(false) },
      { key: 'PageDown', run: moveCompletionSelection(true, 'page') },
      { key: 'PageUp', run: moveCompletionSelection(false, 'page') },
      { key: 'Escape', run: closeCompletion },
      { key: 'Enter', run: acceptCompletion },
    ])),
  ];
}

function reopenAfterDelete(minChars: number) {
  return EditorView.updateListener.of((update) => {
    if (!update.docChanged || completionStatus(update.state) !== null) return;
    if (!update.transactions.some((transaction) => transaction.isUserEvent('delete'))) return;
    const where = suggestContext(update.state, update.state.selection.main.head);
    if (where && where.query.length >= minChars) startCompletion(update.view);
  });
}
