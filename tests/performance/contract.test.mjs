// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { sanitizeEvent, validateRun } from '../../tools/performance/contract.mjs';
import { run } from './support/run.mjs';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { compareRuns } from '../../tools/performance/compare.mjs';
test('control comparisons require proof of a supported disabling seam', () => {
  assert.throws(() => validateRun(run({scenario: 'control'})), /control requires supported-seam evidence/);
});
test('nonfinite timings and mismatched trace IDs are rejected without exposing field values', () => {
  const metadata = run();
  assert.throws(() => sanitizeEvent({traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 1, durationUs: Infinity}), /durationUs must be a nonnegative safe integer/);
  assert.throws(() => analyzeTrace(metadata, [{traceId: '0'.repeat(32), source: 'adapter', kind: 'frame', atUs: 1, durationUs: 1}], {bridgeP99Us: 2000, longFrameUs: 16667}), /TRACE_ID_MISMATCH/);
});
test('different checkpoint inputs never become a matched comparison', () => {
  const metadata = run();
  const report = analyzeTrace(metadata, [{traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 2000000, durationUs: 100}], {bridgeP99Us: 2000, longFrameUs: 16667});
  assert.throws(() => compareRuns([report, {...report, run: {...metadata, scenario: 'shadow', checkpointSha256: 'd'.repeat(64)}}]), /COMPARISON_MISMATCH: checkpointSha256/);
});
test('known telemetry loss blocks comparative performance conclusions', () => {
  const metadata = {...run(), droppedEvents: 5, invalidEvents: 0};
  const degraded = analyzeTrace(metadata, [{traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 2000000, durationUs: 100}], {bridgeP99Us: 2000, longFrameUs: 16667});
  assert.throws(() => compareRuns([degraded, {...degraded, run: {...degraded.run, scenario: 'shadow'}}]), /TRACE_LOSS/);
});
test('actual game reports require explicit hardware and live JVM-flag identities', () => {
  assert.throws(() => validateRun(run({baseline: 'game'})), /MISSING_RUNTIME_EVIDENCE/);
});
