interface CellClaim {
  cell: number;
  fresh: boolean;
}

export class AtlasCells {
  private readonly cellOf = new Map<string, number>();
  private readonly textOf: (string | null)[] = [];
  private readonly usedAt: number[] = [];
  private capacity = 0;
  private taken = 0;

  get size(): number {
    return this.capacity;
  }

  resize(capacity: number): void {
    this.capacity = Math.max(0, capacity);
    this.forget();
  }

  forget(): void {
    this.cellOf.clear();
    this.textOf.length = this.capacity;
    this.usedAt.length = this.capacity;
    this.textOf.fill(null);
    this.usedAt.fill(-1);
    this.taken = 0;
  }

  claim(text: string, frame: number): CellClaim | null {
    const known = this.cellOf.get(text);
    if (known !== undefined) {
      this.usedAt[known] = frame;
      return { cell: known, fresh: false };
    }
    if (this.capacity === 0) return null;
    if (this.taken < this.capacity) {
      const cell = this.taken;
      this.taken += 1;
      this.occupy(cell, text, frame);
      return { cell, fresh: true };
    }
    const stale = this.oldestBefore(frame);
    if (stale < 0) return null;
    const previous = this.textOf[stale];
    if (previous !== null) this.cellOf.delete(previous);
    this.occupy(stale, text, frame);
    return { cell: stale, fresh: true };
  }

  private occupy(cell: number, text: string, frame: number): void {
    this.textOf[cell] = text;
    this.usedAt[cell] = frame;
    this.cellOf.set(text, cell);
  }

  private oldestBefore(frame: number): number {
    let oldest = -1;
    let oldestFrame = frame;
    for (let cell = 0; cell < this.capacity; cell += 1) {
      const used = this.usedAt[cell];
      if (used >= frame) continue;
      if (oldest < 0 || used < oldestFrame) {
        oldest = cell;
        oldestFrame = used;
      }
    }
    return oldest;
  }
}
