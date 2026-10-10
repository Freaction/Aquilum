import { invoke } from '@tauri-apps/api/core';
import type { GraphEpoch, GraphSnapshot } from './types';

const HEADER_BYTES = 16;

export async function loadGraphSnapshot(workspacePath: string): Promise<GraphSnapshot> {
  const response = await invoke<ArrayBuffer | Uint8Array>('get_graph_snapshot', {
    workspacePath,
  });
  return decodeGraphSnapshot(toArrayBuffer(response));
}

export function loadGraphPaths(epoch: GraphEpoch, indices: number[]): Promise<string[]> {
  return invoke<string[]>('get_graph_paths', {
    epochLow: epoch.low,
    epochHigh: epoch.high,
    indices,
  });
}

export function decodeGraphSnapshot(buffer: ArrayBuffer): GraphSnapshot {
  const header = new DataView(buffer);
  const nodeCount = header.getUint32(0, true);
  const edgeCount = header.getUint32(4, true);
  const epoch = {
    low: header.getUint32(8, true),
    high: header.getUint32(12, true),
  };

  let offset = HEADER_BYTES;
  const positions = new Float32Array(buffer, offset, nodeCount * 2);
  offset += nodeCount * 8;
  const createdDays = new Float32Array(buffer, offset, nodeCount);
  offset += nodeCount * 4;
  const modifiedDays = new Float32Array(buffer, offset, nodeCount);
  offset += nodeCount * 4;
  const degrees = new Uint32Array(buffer, offset, nodeCount);
  offset += nodeCount * 4;
  const edges = new Uint32Array(buffer, offset, edgeCount * 2);

  return { nodeCount, edgeCount, epoch, positions, createdDays, modifiedDays, degrees, edges };
}

function toArrayBuffer(response: ArrayBuffer | Uint8Array): ArrayBuffer {
  if (response instanceof ArrayBuffer) return response;
  if (response.byteOffset === 0 && response.byteLength === response.buffer.byteLength) {
    return response.buffer as ArrayBuffer;
  }
  return response.slice().buffer;
}
