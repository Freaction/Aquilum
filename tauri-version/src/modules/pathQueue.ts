import { comparablePath } from './paths';

export class PathQueue<T> {
  private readonly tails = new Map<string, Promise<T>>();

  run(path: string, operation: () => Promise<T>): Promise<T> {
    const key = comparablePath(path);
    const previous = this.tails.get(key);
    const next = previous ? previous.then(operation, operation) : operation();
    this.tails.set(key, next);
    const forget = () => {
      if (this.tails.get(key) === next) this.tails.delete(key);
    };
    next.then(forget, forget);
    return next;
  }

  current(path: string): Promise<T> | undefined {
    return this.tails.get(comparablePath(path));
  }
}
