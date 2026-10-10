import { createTaskCheckbox, TASK_CHECKBOX_CLASS } from '../../../Common/taskCheckbox';
import type { DataviewCell, DataviewOutput } from '../../../../modules/docs/dataview';
import { DATAVIEW_CLASSES, DATAVIEW_MESSAGES, UNKNOWN_HEIGHT } from './constants';
import type { DataviewResult } from './store';

export function renderDataviewDom(
  result: DataviewResult | undefined,
  knownHeight = UNKNOWN_HEIGHT,
): HTMLElement {
  const root = document.createElement('div');
  root.className = DATAVIEW_CLASSES.root;
  root.contentEditable = 'false';

  const card = document.createElement('div');
  card.className = DATAVIEW_CLASSES.card;
  root.append(card);

  if (!result) {
    card.classList.add(DATAVIEW_CLASSES.pending);
    if (knownHeight > 0) card.style.minHeight = `${knownHeight}px`;
    return root;
  }
  if (result.status === 'failed') {
    card.classList.add(DATAVIEW_CLASSES.failed);
    card.append(message(result.message));
    return root;
  }
  const { output } = result;
  if (output.title) {
    card.append(heading(output.title, output.shape));
  }
  if (output.total === 0) {
    card.append(message(DATAVIEW_MESSAGES.empty));
    return root;
  }

  card.append(output.shape === 'table' ? table(output) : list(output, output.shape === 'tasks'));
  if (output.truncated) {
    card.append(message(DATAVIEW_MESSAGES.truncated(output.rows.length, output.total)));
  }
  return root;
}

function heading(text: string, shape: DataviewOutput['shape']): HTMLElement {
  const element = document.createElement('div');
  element.className = DATAVIEW_CLASSES.title;
  if (shape === 'table') element.classList.add(DATAVIEW_CLASSES.titleCells);
  element.textContent = text;
  return element;
}

function table(output: DataviewOutput): HTMLElement {
  const element = document.createElement('table');
  element.className = DATAVIEW_CLASSES.table;

  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  for (const title of output.columns) {
    const cell = document.createElement('th');
    cell.textContent = title;
    headRow.append(cell);
  }
  head.append(headRow);

  const body = document.createElement('tbody');
  const width = Math.max(output.width, 1);
  for (let start = 0; start < output.rows.length; start += width) {
    const row = document.createElement('tr');
    for (let column = 0; column < width; column += 1) {
      const cell = document.createElement('td');
      fillCell(cell, output.rows[start + column]);
      row.append(cell);
    }
    body.append(row);
  }

  element.append(head, body);
  return element;
}

function list(output: DataviewOutput, tasks = false): HTMLElement {
  const element = document.createElement('ul');
  element.className = tasks ? DATAVIEW_CLASSES.tasks : DATAVIEW_CLASSES.list;
  for (const cell of output.rows) {
    const item = document.createElement('li');
    fillCell(item, cell);
    element.append(item);
  }
  return element;
}

function fillCell(target: HTMLElement, cell: DataviewCell | undefined): void {
  if (!cell) return;
  for (const part of cell.parts) {
    if (part.kind === 'check') {
      target.append(checkbox(part.done, part.target, part.line));
      continue;
    }
    if (part.kind === 'progress') {
      target.classList.add(DATAVIEW_CLASSES.cellBar);
      target.append(progress(part.percent));
      continue;
    }
    if (part.kind === 'link') {
      const link = document.createElement('span');
      link.className = DATAVIEW_CLASSES.link;
      link.dataset.target = part.target;
      link.textContent = part.text;
      target.append(link);
      continue;
    }
    target.append(document.createTextNode(part.text));
  }
}

function checkbox(done: boolean, target: string, line: number): HTMLElement {
  const box = createTaskCheckbox(done);
  box.dataset.taskTarget = target;
  box.dataset.taskLine = String(line);
  return box;
}

export function dataviewTaskAt(node: EventTarget | null): { target: string; line: number } | null {
  const element = (node as HTMLElement | null)?.closest?.(`.${TASK_CHECKBOX_CLASS}`) as HTMLElement | null;
  const target = element?.dataset.taskTarget;
  const line = Number(element?.dataset.taskLine);
  return target && Number.isFinite(line) ? { target, line } : null;
}

function progress(percent: number): HTMLElement {
  const track = document.createElement('div');
  track.className = DATAVIEW_CLASSES.bar;
  const filled = document.createElement('div');
  filled.className = DATAVIEW_CLASSES.barFill;
  filled.style.width = `${Math.max(0, Math.min(100, percent))}%`;
  track.append(filled);
  return track;
}

function message(text: string): HTMLElement {
  const element = document.createElement('div');
  element.className = DATAVIEW_CLASSES.message;
  element.textContent = text;
  return element;
}

export function dataviewLinkTarget(node: EventTarget | null): string | null {
  const element = (node as HTMLElement | null)?.closest?.(`.${DATAVIEW_CLASSES.link}`);
  return (element as HTMLElement | null)?.dataset.target ?? null;
}
