// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { compareRuns } from '../../tools/performance/compare.mjs';
import { run } from './support/run.mjs';

/** @param {string} scenario @param {number} repeat @param {string} flags @param {string} [certificate] */
function report(scenario, repeat, flags, certificate = 'e'.repeat(64)) {
  const metadata = run({scenario, repeat, liveJvmFlagsSha256: flags,
    ...(scenario === 'control' ? {manifestSha256: 'd'.repeat(64),
      referenceManifestSha256: 'a'.repeat(64), controlEvidenceSha256: certificate} : {})});
  return analyzeTrace(metadata, [{traceId: metadata.traceId, source: 'adapter',
    kind: 'frame', atUs: 2000000, durationUs: 100}], {bridgeP99Us: 2000, longFrameUs: 16667});
}

test('control B/C JVM flag mismatch refuses within one scenario despite a named baseline difference', () => {
  assert.throws(() => compareRuns([
    report('baseline', 1, 'a'.repeat(64)), report('baseline', 2, 'a'.repeat(64)),
    report('control', 1, 'b'.repeat(64)), report('control', 2, 'c'.repeat(64)),
  ]), /COMPARISON_MISMATCH: live JVM flags changed within a scenario/);
});

test('instrumentation-off repetitions also require one intended JVM flag identity', () => {
  assert.throws(() => compareRuns([
    report('baseline', 1, 'a'.repeat(64)), report('baseline', 2, 'a'.repeat(64)),
    report('instrumentation-off', 1, 'b'.repeat(64)), report('instrumentation-off', 2, 'c'.repeat(64)),
  ]), /COMPARISON_MISMATCH: live JVM flags changed within a scenario/);
});

test('a stable control method certificate cannot silently discard a differing repeat certificate', () => {
  assert.throws(() => compareRuns([
    report('baseline', 1, 'a'.repeat(64)), report('baseline', 2, 'a'.repeat(64)),
    report('control', 1, 'b'.repeat(64)), report('control', 2, 'b'.repeat(64), 'f'.repeat(64)),
  ]), /COMPARISON_MISMATCH: control method certificate changed within a scenario/);
});

test('explicit baseline/control flag differences retain the shared method certificate and every repeat', () => {
  const compared = compareRuns([
    report('control', 2, 'b'.repeat(64)), report('baseline', 1, 'a'.repeat(64)),
    report('control', 1, 'b'.repeat(64)), report('baseline', 2, 'a'.repeat(64)),
  ]);
  const control = compared.scenarios.find(group => group.scenario === 'control');
  assert.ok(control);
  assert.equal(control.controlEvidenceSha256, 'e'.repeat(64));
  assert.equal(control.liveJvmFlagsSha256, 'b'.repeat(64));
  assert.equal(compared.scenarios.find(group => group.scenario === 'baseline')?.liveJvmFlagsSha256, 'a'.repeat(64));
  assert.deepEqual(control.repeatIds, [1, 2]);
  assert.equal(control.repeats, 2);
  assert.equal(compared.comparisons[0].deltaMeanP99Us, 0);
  assert.equal(compared.actualGameBaseline, 'unmeasured');
  assert.equal(compared.accelerationClaim, 'none');
});
