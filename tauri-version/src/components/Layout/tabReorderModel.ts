export interface TabBox {
  left: number;
  width: number;
}

interface ReorderLayout {
  target: number;
  shift: number[];
}

export function reorderLayout(boxes: TabBox[], from: number, offset: number): ReorderLayout {
  const shift = boxes.map(() => 0);
  if (from < 0 || from >= boxes.length) return { target: from, shift };

  const dragged = boxes[from]!;
  const width = dragged.width;
  const left = dragged.left + offset;
  const right = left + width;
  let target = from;

  for (let index = 0; index < boxes.length; index += 1) {
    if (index === from) continue;
    const box = boxes[index]!;
    const middle = box.left + box.width / 2;
    if (index < from && left < middle) {
      shift[index] = width;
      target = Math.min(target, index);
    } else if (index > from && right > middle) {
      shift[index] = -width;
      target = Math.max(target, index);
    }
  }

  shift[from] = offset;
  return { target, shift };
}

export function restingOffset(boxes: TabBox[], from: number, target: number): number {
  if (from === target) return 0;
  const dragged = boxes[from]!;
  if (target > from) {
    const last = boxes[target]!;
    return last.left + last.width - (dragged.left + dragged.width);
  }
  return boxes[target]!.left - dragged.left;
}
