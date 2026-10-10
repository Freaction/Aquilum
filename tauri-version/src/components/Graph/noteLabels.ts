import type { GraphEpoch } from '../../modules/graph';
import { fileStem } from '../../modules/paths';

const BATCH_LIMIT = 512;

interface NoteLabel {
  path: string;
  title: string;
}

type Waiter = (label: NoteLabel | null) => void;

export class NoteLabels {
  private readonly known = new Map<number, NoteLabel>();
  private readonly absent = new Set<number>();
  private readonly waiters = new Map<number, Waiter[]>();
  private readonly queue = new Set<number>();
  private epoch: GraphEpoch | null = null;
  private flushHandle = 0;
  private loading = false;
  onLoaded: (() => void) | null = null;

  constructor(
    private readonly fetchPaths: (epoch: GraphEpoch, nodes: number[]) => Promise<string[]>,
  ) {}

  get(node: number): NoteLabel | null {
    return this.known.get(node) ?? null;
  }

  request(node: number): void {
    if (this.epoch === null) return;
    if (this.known.has(node) || this.absent.has(node) || this.queue.has(node)) return;
    this.queue.add(node);
    this.scheduleFlush();
  }

  resolve(node: number): Promise<NoteLabel | null> {
    const ready = this.known.get(node);
    if (ready) return Promise.resolve(ready);
    if (this.absent.has(node) || this.epoch === null) return Promise.resolve(null);
    return new Promise((settle) => {
      const waiting = this.waiters.get(node);
      if (waiting) waiting.push(settle);
      else this.waiters.set(node, [settle]);
      this.request(node);
    });
  }

  adopt(epoch: GraphEpoch): void {
    this.epoch = epoch;
    this.forget();
  }

  detach(): void {
    this.epoch = null;
    this.forget();
  }

  private forget(): void {
    this.known.clear();
    this.absent.clear();
    this.queue.clear();
    this.releaseWaiters();
    if (this.flushHandle) {
      window.clearTimeout(this.flushHandle);
      this.flushHandle = 0;
    }
  }

  private scheduleFlush(): void {
    if (this.flushHandle || this.loading) return;
    this.flushHandle = window.setTimeout(() => {
      this.flushHandle = 0;
      void this.flush();
    }, 0);
  }

  private async flush(): Promise<void> {
    const epoch = this.epoch;
    if (!epoch) return;
    const batch = [...this.queue].slice(0, BATCH_LIMIT);
    if (batch.length === 0) return;
    batch.forEach((node) => this.queue.delete(node));
    this.loading = true;
    try {
      const paths = await this.fetchPaths(epoch, batch);
      if (this.sameEpoch(epoch)) {
        batch.forEach((node, slot) => this.settle(node, paths[slot] ?? ''));
        this.onLoaded?.();
      }
    } catch (error) {
      console.error('Failed to load graph note labels', error);
      batch.forEach((node) => this.release(node));
    } finally {
      this.loading = false;
    }
    if (this.queue.size > 0) this.scheduleFlush();
  }

  private sameEpoch(epoch: GraphEpoch): boolean {
    return this.epoch !== null
      && this.epoch.low === epoch.low
      && this.epoch.high === epoch.high;
  }

  private settle(node: number, path: string): void {
    const label = path ? { path, title: fileStem(path) } : null;
    if (label) this.known.set(node, label);
    else this.absent.add(node);
    this.release(node, label);
  }

  private release(node: number, label: NoteLabel | null = null): void {
    this.waiters.get(node)?.forEach((accept) => accept(label));
    this.waiters.delete(node);
  }

  private releaseWaiters(): void {
    this.waiters.forEach((waiting) => waiting.forEach((accept) => accept(null)));
    this.waiters.clear();
  }
}
