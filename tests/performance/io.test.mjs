// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { readBounded } from '../../tools/performance/io.mjs';
test('opened input growth consumes and allocates at most limit plus one, closing on rejection', async () => {
  let consumed = 0;
  let closed = false;
  /** @type {number[][]} */
  const requests = [];
  /** @type {import('../../tools/performance/io.mjs').InputHandle} */
  const growing = {async read(buffer, offset, length) {
    requests.push([buffer.length, length]);
    const bytesRead = Math.min(3, length);
    buffer.fill(65, offset, offset + bytesRead);
    consumed += bytesRead;
    return {bytesRead};
  }, async close() {closed = true;}};
  await assert.rejects(readBounded('unused', 8, async () => growing), /INPUT_TOO_LARGE/);
  assert.equal(consumed, 9);
  assert.ok(requests.every(([allocated, requested]) => allocated === 9 && requested <= 9));
  assert.equal(closed, true);
});
