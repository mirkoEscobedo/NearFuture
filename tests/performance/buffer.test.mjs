// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { TraceBuffer } from '../../tools/performance/buffer.mjs';
const traceId = '0123456789abcdef0123456789abcdef';
test('telemetry saturation stays bounded and sheds overflow without retaining private fields', () => {
  const buffer = new TraceBuffer({ maxItems: 2, maxBytes: 1000, maxEventBytes: 512 });
  assert.equal(buffer.offer({ traceId, source: 'adapter', kind: 'frame', atUs: 1, durationUs: 100, privateSave: 'secret' }), true);
  assert.equal(buffer.offer({ traceId, source: 'node', kind: 'queue', atUs: 2, queueItems: 3, queueBytes: 42 }), true);
  for (let i = 0; i < 10000; i++) assert.equal(buffer.offer({ traceId, source: 'adapter', kind: 'frame', atUs: i + 3, durationUs: 20 }), false);
  const { queuedBytes, ...counts } = buffer.stats();
  assert.deepEqual(counts, { offered: 10002, accepted: 2, dropped: 10000, invalid: 0, queuedItems: 2 });
  assert.ok(queuedBytes <= 1000);
  const records = buffer.drain(1);
  assert.equal(records.length, 1);
  assert.ok(!JSON.stringify(records).includes('secret'));
  assert.equal(buffer.stats().queuedItems, 1);
  assert.equal(buffer.offer({ traceId, source: 'adapter', kind: 'frame', atUs: 20, durationUs: 1 }), true);
});

test('incomplete queue budget cannot silently disable capacity enforcement', () => {
  assert.throws(() => Reflect.construct(TraceBuffer, [{maxBytes: 1000, maxEventBytes: 512}]), /INVALID_BUDGET/);
});
