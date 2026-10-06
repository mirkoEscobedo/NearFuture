// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { compareRuns } from '../../tools/performance/compare.mjs';
import { run } from './support/run.mjs';
/** @param {string} scenario @param {number} repeat @param {number} durationUs */
function report(scenario, repeat, durationUs) {
  const metadata = run({scenario, repeat});
  return analyzeTrace(metadata, [{traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 2000000, durationUs}], {bridgeP99Us: 2000, longFrameUs: 16667});
}
test('matched repeated runs report variance and keep synthetic improvement separate from game evidence', () => {
  const results = compareRuns([report('baseline', 1, 10000), report('baseline', 2, 12000), report('baseline', 3, 14000),
    report('shadow', 1, 15000), report('shadow', 2, 16000), report('shadow', 3, 17000)]);
  assert.equal(results.scenarios[0].meanP99Us, 12000);
  assert.equal(results.scenarios[0].sampleStddevP99Us, 2000);
  assert.equal(results.comparisons[0].deltaMeanP99Us, 4000);
  assert.equal(results.actualGameBaseline, 'unmeasured');
  assert.equal(results.accelerationClaim, 'none');
});
test('instrumentation-off input order cannot hide mismatched shadow instrumentation', () => {
  const off = report('instrumentation-off', 1, 10000);
  off.run.instrumentation = 'frame';
  const baseline = report('baseline', 1, 12000);
  const shadow = report('shadow', 1, 9000);
  shadow.run.instrumentation = 'frame';
  assert.throws(() => compareRuns([off, baseline, shadow]), /COMPARISON_MISMATCH: instrumentation/);
});
test('a supported control records its exact differing manifest and explicit reference identity', () => {
  const baseline = report('baseline', 1, 12000);
  const metadata = run({scenario: 'control', manifestSha256: 'd'.repeat(64), controlEvidenceSha256: 'e'.repeat(64)});
  const control = analyzeTrace({...metadata, referenceManifestSha256: baseline.run.manifestSha256}, [{traceId: metadata.traceId, source: 'adapter', kind: 'frame', atUs: 2000000, durationUs: 10000}], {bridgeP99Us: 2000, longFrameUs: 16667});
  const compared = compareRuns([baseline, control]);
  assert.equal(compared.comparisons[0].scenario, 'control');
});
