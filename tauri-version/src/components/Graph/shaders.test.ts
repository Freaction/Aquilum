import { describe, expect, it } from 'vitest';
import { EDGE_FRAGMENT, EDGE_VERTEX, NODE_FRAGMENT, NODE_VERTEX } from './shaders';

const RESERVED_WORDS = [
  'active', 'asm', 'atomic_uint', 'attribute', 'cast', 'class', 'coherent', 'common', 'double',
  'dvec2', 'dvec3', 'dvec4', 'enum', 'extern', 'external', 'filter', 'fixed', 'fvec2', 'fvec3',
  'fvec4', 'goto', 'half', 'hvec2', 'hvec3', 'hvec4', 'iimage2D', 'image2D', 'inline', 'input',
  'interface', 'long', 'namespace', 'noinline', 'noperspective', 'output', 'partition', 'patch',
  'public', 'readonly', 'resource', 'restrict', 'sample', 'sampler1D', 'sampler3DRect', 'short',
  'sizeof', 'static', 'subroutine', 'superp', 'template', 'this', 'typedef', 'union', 'unsigned',
  'using', 'varying', 'volatile', 'writeonly',
];

const SHADERS = {
  NODE_VERTEX,
  NODE_FRAGMENT,
  EDGE_VERTEX,
  EDGE_FRAGMENT,
};

describe('graph shaders', () => {
  it('never name anything with a word GLSL ES 3.00 keeps for itself', () => {
    const clashes = Object.entries(SHADERS).flatMap(([shader, source]) =>
      RESERVED_WORDS
        .filter((word) => new RegExp(`\\b${word}\\b`).test(source))
        .map((word) => `${shader}: ${word}`));

    expect(clashes).toEqual([]);
  });

  it('open with the version the renderer compiles against', () => {
    for (const source of Object.values(SHADERS)) {
      expect(source.split('\n')[0]).toBe('#version 300 es');
    }
  });

  it('give every fragment shader a float precision', () => {
    expect(NODE_FRAGMENT).toContain('precision highp float;');
    expect(EDGE_FRAGMENT).toContain('precision highp float;');
  });
});
