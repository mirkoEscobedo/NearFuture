// @ts-check
import { validateRun } from './contract.mjs';
/** @param {unknown[]} input */
export function compareRuns(input) {
  if (input.length < 2 || input.length > 300) throw new Error('INVALID_COMPARISON: supply 2–300 bounded run reports.');
  const reports = input.map(value => {
    if (!value || typeof value !== 'object') throw new Error('INVALID_REPORT: expected analyzed runs.');
    const report = /** @type {ReturnType<import('./analyze.mjs').analyzeTrace>} */ (value);
    const run = validateRun(report.run);
    if (run.droppedEvents || run.invalidEvents) throw new Error('TRACE_LOSS: repeat matched comparisons without dropped or invalid telemetry.');
    if (!report.frames || typeof report.frames.p99Us !== 'number' || !Number.isFinite(report.frames.p99Us) || report.frames.p99Us < 0)
      throw new Error('INVALID_REPORT: every comparison run needs measured frame observations.');
    return { run, p99Us: report.frames.p99Us };
  });
  const first = reports[0].run;
  const baselineRun = reports.find(report => report.run.scenario === 'baseline')?.run;
  if (!baselineRun) throw new Error('MISSING_BASELINE: include supported reference baseline runs.');
  const baselineInstrumentation = baselineRun.instrumentation;
  const equalFields = /** @type {const} */ (['budgetsSha256','checkpointSha256','warmupUs','durationUs','speed','paused','baseline','hardwareSha256']);
  for (const report of reports) {
    if (report.run.manifestSha256 !== baselineRun?.manifestSha256 && (report.run.scenario !== 'control' || report.run.referenceManifestSha256 !== baselineRun?.manifestSha256))
      throw new Error('COMPARISON_MISMATCH: manifestSha256; only a documented control may differ from its exact reference manifest.');
    for (const field of equalFields) if (report.run[field] !== first[field]) throw new Error(`COMPARISON_MISMATCH: ${field}; restore matching run inputs before comparing.`);
    if (report.run.liveJvmFlagsSha256 !== baselineRun.liveJvmFlagsSha256 && !['control','instrumentation-off'].includes(report.run.scenario)) throw new Error('COMPARISON_MISMATCH: live JVM flags changed without a named control/overhead experiment.');
    if (report.run.instrumentation !== baselineInstrumentation && report.run.scenario !== 'instrumentation-off')
      throw new Error('COMPARISON_MISMATCH: instrumentation; measure instrumentation overhead in its named paired scenario.');
  }
  /** @type {Map<string, {run: import('./contract.mjs').RunMetadata, p99Us: number}[]>} */
  const groups = new Map();
  for (const report of reports) {
    const group = groups.get(report.run.scenario) ?? [];
    if (group.some(previous => previous.run.repeat === report.run.repeat)) throw new Error('DUPLICATE_REPEAT: repeat IDs must be unique within each scenario.');
    if (group.length && group[0].run.manifestSha256 !== report.run.manifestSha256) throw new Error('COMPARISON_MISMATCH: manifest changed within a scenario.');
    if (group.length && group[0].run.instrumentation !== report.run.instrumentation) throw new Error('COMPARISON_MISMATCH: instrumentation changed within a scenario.');
    group.push(report); groups.set(report.run.scenario, group);
  }
  const scenarios = [...groups].sort(([a],[b]) => a.localeCompare(b)).map(([scenario, group]) => {
    const values = group.map(x => x.p99Us), mean = values.reduce((a,b) => a + b, 0) / values.length;
    const variance = values.length > 1 ? values.reduce((sum,value) => sum + (value - mean) ** 2, 0) / (values.length - 1) : null;
    return { scenario, manifestSha256: group[0].run.manifestSha256, controlEvidenceSha256: group[0].run.controlEvidenceSha256 ?? null, repeats: values.length, repeatIds: group.map(x => x.run.repeat).sort((a,b) => a-b),
      meanP99Us: mean, sampleStddevP99Us: variance === null ? null : Math.sqrt(variance),
      minP99Us: Math.min(...values), maxP99Us: Math.max(...values) };
  });
  const baseline = scenarios.find(group => group.scenario === 'baseline');
  if (!baseline) throw new Error('MISSING_BASELINE: include the supported reference baseline runs.');
  const comparisons = scenarios.filter(group => group !== baseline).map(group => {
    if (JSON.stringify(group.repeatIds) !== JSON.stringify(baseline.repeatIds)) throw new Error('UNPAIRED_REPEATS: baseline and comparison scenarios must use matching repeat IDs.');
    return { scenario: group.scenario, kind: group.scenario === 'instrumentation-off' ? 'instrumentation-overhead' : 'end-to-end-frame-observation',
      deltaMeanP99Us: group.meanP99Us - baseline.meanP99Us,
      percentChange: (group.meanP99Us - baseline.meanP99Us) / (baseline.meanP99Us || 1) * 100,
      observedRangesOverlap: group.minP99Us <= baseline.maxP99Us && baseline.minP99Us <= group.maxP99Us };
  });
  return { schemaVersion: 1, actualGameBaseline: first.baseline === 'game' ? 'recorded-not-certified' : 'unmeasured',
    accelerationClaim: 'none', scenarios, comparisons,
    warnings: ['Min/max overlap and sample standard deviation describe run variation, not statistical proof of a speedup.',
      scenarios.some(group => group.repeats < 3) ? 'At least three matched repetitions are required for a useful initial comparison.' : '',
      'Shadow duplicates work intentionally; instrumentation, capture, encoding, apply and contention all count in end-to-end frame observations.'].filter(Boolean) };
}
