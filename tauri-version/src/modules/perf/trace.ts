const ENABLED = import.meta.env.DEV;

const QUIET_MS = 700;
const LABEL_WIDTH = 32;

interface Tracer<Stage extends string> {
  begin(label: string): void;
  mark(stage: Stage): void;
}

interface Options<Stage extends string> {
  name: string;
  labels: Record<Stage, string>;
  order: readonly Stage[];
  origin: 'load' | 'now';
}

interface Mark<Stage extends string> {
  stage: Stage;
  at: number;
}

export function createTracer<Stage extends string>(options: Options<Stage>): Tracer<Stage> {
  let current: { label: string; started: number; marks: Mark<Stage>[] } | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const totals = new Map<string, number[]>();

  const print = (): void => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    const trace = current;
    current = null;
    if (!trace || trace.marks.length === 0) return;

    const total = Math.max(...trace.marks.map((mark) => mark.at - trace.started));
    const seen = totals.get(trace.label) ?? [];
    seen.push(total);
    totals.set(trace.label, seen);

    const lines = [`[aquilum:${options.name}] ${trace.label} — ${round(total)} мс${repeats(seen)}`];
    for (const stage of options.order) {
      const marks = trace.marks.filter((mark) => mark.stage === stage);
      if (marks.length === 0) continue;
      const first = round(marks[0].at - trace.started);
      const last = round(marks[marks.length - 1].at - trace.started);
      const tail = marks.length > 1 ? `  →  ${last}  (${marks.length} раз)` : '';
      lines.push(`  ${options.labels[stage].padEnd(LABEL_WIDTH, '.')} ${first}${tail}`);
    }
    console.log(lines.join('\n'));
  };

  const schedule = (): void => {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(print, QUIET_MS);
  };

  return {
    begin(label: string): void {
      if (!ENABLED) return;
      print();
      current = {
        label,
        started: options.origin === 'load' ? 0 : performance.now(),
        marks: [],
      };
      schedule();
    },
    mark(stage: Stage): void {
      if (!ENABLED || !current) return;
      current.marks.push({ stage, at: performance.now() });
      schedule();
    },
  };
}

function repeats(totals: number[]): string {
  if (totals.length < 3) return '';
  const sorted = [...totals].sort((left, right) => left - right);
  const median = sorted[Math.floor(sorted.length / 2)];
  return `  (медиана ${round(median)} из ${totals.length}: ${round(sorted[0])}…${round(sorted[sorted.length - 1])})`;
}

function round(value: number): number {
  return Math.round(value * 10) / 10;
}
