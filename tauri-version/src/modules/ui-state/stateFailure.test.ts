import { describe, expect, it } from 'vitest';
import { stateFailure } from './types';

describe('stateFailure', () => {
  it('treats a failed session and an unusable database as broken', () => {
    expect(stateFailure(new Error('locked'), true).databaseBroken).toBe(true);
    expect(stateFailure({ code: 'unsupported_schema', details: { version: 99 } }, false).databaseBroken).toBe(true);
    expect(stateFailure({ code: 'unavailable', details: { message: 'x' } }, false).databaseBroken).toBe(true);
  });

  it('does not offer the full reset for a single failed request', () => {
    expect(stateFailure({ code: 'database', details: { message: 'database is locked' } }, false).databaseBroken).toBe(false);
  });
});
