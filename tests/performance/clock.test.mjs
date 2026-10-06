// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { alignMonotonic, clockDrift } from '../../tools/performance/clock.mjs';
test('monotonic alignment preserves declared uncertainty and detects wall-clock jumps', () => {
  const anchor = {epochUs: 1000000, monotonicNs: 1000000000n, uncertaintyUs: 1000};
  assert.deepEqual(alignMonotonic(anchor, 1002000000n), {atUs: 1002000, uncertaintyUs: 1000});
  assert.deepEqual(clockDrift(anchor, {epochUs: 1006000, monotonicNs: 1002000000n, uncertaintyUs: 1000}), {driftUs: 4000, withinUncertainty: false});
});
