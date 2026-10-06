// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { run } from './support/run.mjs';
test('analysis excludes warm-up and reports independently known nearest-rank frame tails', () => {
  const metadata = run();
  const events = Array.from({length: 1000}, (_, i) => ({traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 2000000 + i * 1000000, durationUs: (i + 1) * 1000}));
  events.unshift({traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 1000000, durationUs: 999999999});
  const result = analyzeTrace(metadata, events, { bridgeP99Us: 2000, longFrameUs: 900000 });
  assert.deepEqual(result.frames, { count: 1000, p50Us: 500000, p95Us: 950000, p99Us: 990000, p999Us: 999000, longFrames: 100 });
  assert.equal(result.actualGameBaseline, 'unmeasured');
  assert.equal(result.bridge.status, 'unmeasured');
});
test('incomplete bridge groups stay unmeasured while CPU, blocking, I/O and GC retain separate evidence', () => {
  const metadata = run();
  /** @param {string} kind @param {Record<string,number>} fields */
  const event = (kind, fields = {}) => ({traceId: metadata.traceId, source: 'adapter', kind, atUs: 2000000, ...fields});
  const result = analyzeTrace(metadata, [
    event('capture', {frameId: 1, durationUs: 1500}), event('apply', {frameId: 1, durationUs: 800}),
    event('capture', {frameId: 2, durationUs: 50}),
    event('thread_running_sample', {threadId: 7}), event('thread_blocked', {threadId: 7, durationUs: 100}),
    event('thread_io', {threadId: 7, durationUs: 200}), event('gc_pause', {durationUs: 150}),
    event('thread_cpu', {threadId: 7, cpuUs: 100}), event('thread_cpu', {threadId: 7, atUs: 3000000, cpuUs: 600}),
  ], {bridgeP99Us: 2000, longFrameUs: 16667});
  assert.equal(result.bridge.incompleteFrames, 1);
  assert.equal(result.bridge.status, 'unmeasured');
  assert.equal(result.bridge.p99Us, 2300);
  assert.equal(result.bridge.exceedsBudget, true);
  assert.deepEqual(result.threads[0], {process: 'adapter', threadId: 7, executionSamples: 1, runningCpuUs: 500, blockedEventUs: 100, ioEventUs: 200, sleepEventUs: 0});
  assert.equal(result.phases.gc_pause.p99Us, 150);
});

test('duplicate identified frames from mixed capture sources cannot inflate percentile evidence', () => {
  const metadata = run();
  assert.throws(() => analyzeTrace(metadata, [
    {traceId: metadata.traceId, source: 'adapter', kind: 'frame', frameId: 4, atUs: 2000000, durationUs: 100},
    {traceId: metadata.traceId, source: 'jfr', kind: 'frame', frameId: 4, atUs: 2000000, durationUs: 100},
  ], {bridgeP99Us: 2000, longFrameUs: 16667}), /DUPLICATE_FRAME/);
});
test('equal numeric thread IDs in node and adapter remain separate CPU counters', () => {
  const metadata = run();
  const result = analyzeTrace(metadata, [
    {traceId: metadata.traceId, source: 'node', kind: 'thread_cpu', threadId: 7, cpuUs: 100, atUs: 2000000},
    {traceId: metadata.traceId, source: 'jfr', kind: 'thread_cpu', threadId: 7, cpuUs: 200, atUs: 2000000},
    {traceId: metadata.traceId, source: 'node', kind: 'thread_cpu', threadId: 7, cpuUs: 500, atUs: 3000000},
    {traceId: metadata.traceId, source: 'jfr', kind: 'thread_cpu', threadId: 7, cpuUs: 300, atUs: 3000000},
  ], {bridgeP99Us: 2000, longFrameUs: 16667});
  assert.equal(result.threads.length, 2);
  assert.equal(result.threads.find(thread => thread.process === 'node')?.runningCpuUs, 400);
  assert.equal(result.threads.find(thread => thread.process === 'adapter')?.runningCpuUs, 100);
});
