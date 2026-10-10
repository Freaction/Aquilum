import { collab, getClientID, getSyncedVersion, receiveUpdates, sendableUpdates } from '@codemirror/collab';
import { ChangeSet, type Extension } from '@codemirror/state';
import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import {
  onDocumentChanged,
  pullChanges,
  pushChanges,
  type LoggedChange,
  type OpenedDocument,
} from '../../../modules/documents/documentGateway';
import { caretAtSameLine } from './caretAtSameLine';
import { revealExternalInsert } from './externalReveal';

interface DocumentSyncOptions {
  path: () => string;
  startVersion: number;
  clientID: string;
  onResync: (document: OpenedDocument) => void;
  onFailure: (error: unknown) => void;
}

const EXTERNAL_CLIENTS = new Set(['disk']);

function caretInsideReplacement(changes: ChangeSet, caret: number): boolean {
  let inside = false;
  changes.iterChangedRanges((fromA, toA) => {
    if (toA > fromA && caret > fromA && caret <= toA) inside = true;
  });
  return inside;
}

function applyIncoming(view: EditorView, incoming: LoggedChange[]): void {
  const updates = incoming.map((change) => ({
    changes: ChangeSet.fromJSON(change.changes),
    clientID: change.client,
  }));
  const external = updates.filter((update) => EXTERNAL_CLIENTS.has(update.clientID));
  const caret = view.state.selection.main.head;
  const restoreCaret = view.hasFocus && external.some((update) => caretInsideReplacement(update.changes, caret));
  const before = restoreCaret ? view.state.doc.toString() : '';

  view.dispatch(receiveUpdates(view.state, updates));

  if (restoreCaret) {
    view.dispatch({ selection: { anchor: caretAtSameLine(before, view.state.doc.toString(), caret) } });
  }
  for (const update of external) {
    update.changes.iterChanges((fromA, toA, fromB, _toB, inserted) => {
      if (fromA === toA) revealExternalInsert(view, { from: fromB, to: fromB, insert: inserted.toString() });
    });
  }
}

export function documentSync(options: DocumentSyncOptions): Extension {
  const plugin = ViewPlugin.fromClass(class {
    private pushing = false;
    private pulling = false;
    private pullAgain = false;
    private done = false;
    private readonly unlisten: () => void;

    constructor(private readonly view: EditorView) {
      this.unlisten = onDocumentChanged(options.path, () => {
        void this.pull();
      });
      void this.pull();
    }

    update(update: ViewUpdate): void {
      if (update.docChanged) void this.push();
    }

    private async push(): Promise<void> {
      const updates = sendableUpdates(this.view.state);
      if (this.pushing || this.done || updates.length === 0) return;
      this.pushing = true;
      try {
        const version = getSyncedVersion(this.view.state);
        const accepted = await pushChanges(
          options.path(),
          version,
          getClientID(this.view.state),
          updates.map((update) => update.changes.toJSON()),
        );
        if (!accepted) this.pullAgain = true;
      } catch (error) {
        this.pushing = false;
        options.onFailure(error);
        return;
      }
      this.pushing = false;
      await this.pull();
    }

    private async pull(): Promise<void> {
      if (this.done) return;
      if (this.pulling) {
        this.pullAgain = true;
        return;
      }
      this.pulling = true;
      const outcome = await this.drain().catch((error: unknown) => {
        options.onFailure(error);
        return 'failed' as const;
      });
      this.pulling = false;
      if (outcome === 'synced' && sendableUpdates(this.view.state).length > 0) void this.push();
    }

    private async drain(): Promise<'synced' | 'stopped' | 'failed'> {
      do {
        this.pullAgain = false;
        const result = await pullChanges(options.path(), getSyncedVersion(this.view.state));
        if (this.done) return 'stopped';
        if (result.kind === 'resync') {
          options.onResync({ text: result.text, version: result.version });
          return 'stopped';
        }
        if (result.changes.length > 0) applyIncoming(this.view, result.changes);
      } while (this.pullAgain && !this.done);
      return 'synced';
    }

    destroy(): void {
      this.done = true;
      this.unlisten();
    }
  });

  return [collab({ startVersion: options.startVersion, clientID: options.clientID }), plugin];
}
