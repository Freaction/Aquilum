import { StateEffect, StateField } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, type DecorationSet } from '@codemirror/view';

const TICK_MS = 30;
const DURATION_MS = 900;
const MAX_ANIMATED_CHARS = 2_000;

interface Reveal {
  from: number;
  to: number;
  shown: number;
  step: number;
}

const startReveal = StateEffect.define<{ from: number; to: number }>();
const advanceReveal = StateEffect.define<null>();

function advance(reveal: Reveal): Reveal {
  return { ...reveal, shown: Math.min(reveal.shown + reveal.step, reveal.to - reveal.from) };
}

function pending(reveal: Reveal): boolean {
  return reveal.from + reveal.shown < reveal.to;
}

const revealField = StateField.define<Reveal[]>({
  create: () => [],
  update(reveals, transaction) {
    let next = reveals;
    if (transaction.docChanged) {
      next = next.map((reveal) => ({
        ...reveal,
        from: transaction.changes.mapPos(reveal.from, -1),
        to: transaction.changes.mapPos(reveal.to, 1),
      }));
    }
    for (const effect of transaction.effects) {
      if (effect.is(startReveal)) {
        const length = effect.value.to - effect.value.from;
        next = [
          ...next,
          {
            ...effect.value,
            shown: 0,
            step: Math.max(1, Math.ceil(length / (DURATION_MS / TICK_MS))),
          },
        ];
      }
      if (effect.is(advanceReveal)) {
        next = next.map(advance);
      }
    }
    return next.filter(pending);
  },
  provide: (field) => EditorView.decorations.from(field, (reveals): DecorationSet => Decoration.set(
    reveals.map((reveal) => Decoration.replace({}).range(reveal.from + reveal.shown, reveal.to)),
    true,
  )),
});

const revealTicker = ViewPlugin.fromClass(class {
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(private readonly view: EditorView) {
    this.sync();
  }

  update() {
    this.sync();
  }

  destroy() {
    this.stop();
  }

  private sync() {
    const active = this.view.state.field(revealField).length > 0;
    if (active && this.timer === null) {
      this.timer = setInterval(() => tickExternalReveal(this.view), TICK_MS);
    } else if (!active) {
      this.stop();
    }
  }

  private stop() {
    if (this.timer === null) return;
    clearInterval(this.timer);
    this.timer = null;
  }
});

export const externalRevealExtension = [revealField, revealTicker];

export function tickExternalReveal(view: EditorView): void {
  view.dispatch({ effects: advanceReveal.of(null) });
}

export function revealExternalInsert(
  view: EditorView,
  edit: { from: number; to: number; insert: string },
): void {
  if (edit.to !== edit.from || !edit.insert) return;
  if (edit.insert.length > MAX_ANIMATED_CHARS) return;
  const to = edit.from + edit.insert.length;
  if (to > view.state.doc.length) return;
  view.dispatch({ effects: startReveal.of({ from: edit.from, to }) });
}
