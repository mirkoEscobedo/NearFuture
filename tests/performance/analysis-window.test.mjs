// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { run } from './support/run.mjs';

/** @param {string} kind @param {number} atUs @param {number} durationUs @param {number} [frameId] */
const span = (kind, atUs, durationUs, frameId = 1) => ({traceId: run().traceId,
  source: 'adapter', kind, atUs, durationUs, frameId});
/** @param {unknown[]} events @param {number} [startEpochUs] */
const analyze = (events, startEpochUs = 0) => analyzeTrace(
  run({startEpochUs, warmupUs: 0, durationUs: 1000}), events,
  {bridgeP99Us: 2000, longFrameUs: 16667});

test('crossing apply cannot create a 10000us bridge sample while apply has no included samples', () => {
  const result = analyze([span('capture', 998, 1), span('apply', 999, 9999)]);
  assert.equal(result.phases.capture.count, 1);
  assert.equal(result.phases.apply.count, 0);
  assert.equal(result.bridge.count, 0);
  assert.equal(result.bridge.p99Us, null);
  assert.equal(result.bridge.exceedsBudget, null);
  assert.equal(result.bridge.incompleteFrames, 1);
  assert.equal(result.bridge.excludedSpans, 1);
  assert.equal(result.quality.excludedPhaseEvents, 1);
  assert.equal(result.bridge.status, 'unmeasured');
});

test('whole phases ending exactly at the window boundary produce a complete 2us bridge sample', () => {
  const result = analyze([span('capture', 998, 1), span('apply', 999, 1)]);
  assert.equal(result.phases.capture.count, 1);
  assert.equal(result.phases.apply.count, 1);
  assert.equal(result.bridge.count, 1);
  assert.equal(result.bridge.p99Us, 2);
  assert.equal(result.bridge.incompleteFrames, 0);
  assert.equal(result.bridge.excludedSpans, 0);
  assert.equal(result.quality.excludedPhaseEvents, 0);
});

test('a phase crossing the start boundary is excluded from both phase and bridge statistics', () => {
  const result = analyze([span('capture', 99, 2), span('apply', 101, 1)], 100);
  assert.equal(result.phases.capture.count, 0);
  assert.equal(result.phases.apply.count, 1);
  assert.equal(result.bridge.count, 0);
  assert.equal(result.bridge.incompleteFrames, 1);
  assert.equal(result.bridge.excludedSpans, 1);
});

test('an additional crossing required span prevents an otherwise complete frame from being retained', () => {
  const result = analyze([span('capture', 10, 1), span('apply', 11, 1), span('apply', 999, 2)]);
  assert.equal(result.phases.apply.count, 1);
  assert.equal(result.bridge.count, 0);
  assert.equal(result.bridge.incompleteFrames, 1);
  assert.equal(result.bridge.excludedSpans, 1);
});

test('phases entirely outside the window do not contaminate an independent complete frame', () => {
  const result = analyze([span('capture', 98, 1, 9), span('apply', 99, 1, 9),
    span('capture', 100, 2), span('apply', 102, 3)], 100);
  assert.equal(result.bridge.count, 1);
  assert.equal(result.bridge.p99Us, 5);
  assert.equal(result.bridge.incompleteFrames, 0);
  assert.equal(result.bridge.excludedSpans, 0);
});
