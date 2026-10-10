import { describe, expect, it } from 'vitest';
import { decodeGraphSnapshot } from './gateway';
import { graphBounds } from './types';

const HEADER_BYTES = 16;

function encode(epochLow = 7, epochHigh = 0): ArrayBuffer {
  const nodeCount = 2;
  const edgeCount = 1;
  const buffer = new ArrayBuffer(HEADER_BYTES + nodeCount * 20 + edgeCount * 8);
  const header = new DataView(buffer);
  header.setUint32(0, nodeCount, true);
  header.setUint32(4, edgeCount, true);
  header.setUint32(8, epochLow, true);
  header.setUint32(12, epochHigh, true);
  new Float32Array(buffer, HEADER_BYTES, 4).set([-1, -2, 3, 4]);
  new Float32Array(buffer, HEADER_BYTES + 16, 2).set([100, 200]);
  new Float32Array(buffer, HEADER_BYTES + 24, 2).set([300, 400]);
  new Uint32Array(buffer, HEADER_BYTES + 32, 2).set([1, 1]);
  new Uint32Array(buffer, HEADER_BYTES + 40, 2).set([0, 1]);
  return buffer;
}

describe('decodeGraphSnapshot', () => {
  it('reads the header and maps every section onto the same buffer', () => {
    const snapshot = decodeGraphSnapshot(encode());

    expect(snapshot.nodeCount).toBe(2);
    expect(snapshot.edgeCount).toBe(1);
    expect([...snapshot.positions]).toEqual([-1, -2, 3, 4]);
    expect([...snapshot.createdDays]).toEqual([100, 200]);
    expect([...snapshot.modifiedDays]).toEqual([300, 400]);
    expect([...snapshot.degrees]).toEqual([1, 1]);
    expect([...snapshot.edges]).toEqual([0, 1]);
  });

  it('carries the epoch that names the node numbering', () => {
    const snapshot = decodeGraphSnapshot(encode(0xdeadbeef, 7));

    expect(snapshot.epoch).toEqual({ low: 0xdeadbeef, high: 7 });
  });

  it('does not copy the payload out of the transferred buffer', () => {
    const buffer = encode();

    const snapshot = decodeGraphSnapshot(buffer);

    expect(snapshot.positions.buffer).toBe(buffer);
    expect(snapshot.edges.buffer).toBe(buffer);
  });

  it('bounds cover every node with padding', () => {
    const bounds = graphBounds(decodeGraphSnapshot(encode()));

    expect(bounds.minX).toBeLessThan(-1);
    expect(bounds.maxY).toBeGreaterThan(4);
  });

  it('an empty graph still yields usable bounds', () => {
    const buffer = new ArrayBuffer(HEADER_BYTES);

    const bounds = graphBounds(decodeGraphSnapshot(buffer));

    expect(bounds.maxX).toBeGreaterThan(bounds.minX);
    expect(bounds.maxY).toBeGreaterThan(bounds.minY);
  });
});
