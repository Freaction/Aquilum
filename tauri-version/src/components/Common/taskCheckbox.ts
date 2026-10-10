export const TASK_CHECKBOX_CLASS = 'q-task-check';

const CHECK_PATH = 'M20 6 9 17l-5-5';

function checkIcon(): SVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '2.1');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('aria-hidden', 'true');
  const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  path.setAttribute('d', CHECK_PATH);
  svg.append(path);
  return svg;
}

export function createTaskCheckbox(done: boolean): HTMLElement {
  const box = document.createElement('span');
  box.className = done
    ? `${TASK_CHECKBOX_CLASS} ${TASK_CHECKBOX_CLASS}--done`
    : TASK_CHECKBOX_CLASS;
  box.setAttribute('role', 'checkbox');
  box.setAttribute('aria-checked', String(done));
  if (done) box.append(checkIcon());
  return box;
}
