import { matchOutlineItem } from '../../components/Editor/extensions/outline/constructs';

interface TaskMark {
  start: number;
  done: boolean;
}

const MARK_LENGTH = 3;

export function markText(done: boolean): string {
  return done ? '[x]' : '[ ]';
}

export function findTaskMark(line: string): TaskMark | null {
  const cleanLine = line.replace(/\r$/, '');
  const item = matchOutlineItem(cleanLine);
  if (!item) return null;

  const afterMarker = cleanLine.slice(item.contentFrom);
  const spaces = afterMarker.length - afterMarker.trimStart().length;
  const box = afterMarker.slice(spaces);
  if (box.length < MARK_LENGTH || box[0] !== '[' || box[2] !== ']') return null;
  const state = box[1]!;
  const tail = box.slice(MARK_LENGTH);
  if (tail.length > 0 && tail[0] !== ' ' && tail[0] !== '\t') return null;

  return { start: item.contentFrom + spaces, done: !/\s/.test(state) };
}

export function toggleTaskLine(text: string, line: number): string | null {
  const lines = text.split('\n');
  const index = line - 1;
  const found = lines[index];
  if (found === undefined) return null;
  const mark = findTaskMark(found);
  if (!mark) return null;
  lines[index] = found.slice(0, mark.start)
    + markText(!mark.done)
    + found.slice(mark.start + MARK_LENGTH);
  return lines.join('\n');
}

export const TASK_MARK_LENGTH = MARK_LENGTH;
