// @ts-check
import { parseArgs } from 'node:util';
import { writeFile } from 'node:fs/promises';
import { TraceBuffer } from './buffer.mjs';
import { analyzeTrace } from './analyze.mjs';
import { compareRuns } from './compare.mjs';
import { readJson, readEvents, readBounded, validateBudgets } from './io.mjs';
try {
  const { values, positionals } = parseArgs({ allowPositionals: true, options: {
    metadata: {type: 'string'}, events: {type: 'string'}, manifest: {type: 'string'},
    budgets: {type: 'string', default: 'config/resource-budgets.json'}, output: {type: 'string'}, offers: {type: 'string', default: '1000000'}, help: {type: 'boolean'},
  }});
  let result;
  if (values.help) {
    process.stdout.write('Usage: cli.mjs analyze --metadata <json> --events <ndjson> --manifest <json> [--budgets <json>] [--output <json>]\n       cli.mjs compare <report-json> <report-json> ... [--output <json>]\n       cli.mjs saturate [--offers <1..1000000>] [--budgets <json>] [--output <json>]\nOffline and synthetic tooling only; no live game attachment or acceleration certificate.\n');
  } else if (positionals[0] === 'compare') {
    if (positionals.length < 3 || positionals.length > 301) throw new Error('INVALID_COMPARISON: supply 2–300 bounded report inputs.');
    const inputs = await Promise.all(positionals.slice(1).map(path => readJson(path)));
    result = compareRuns(inputs.map(input => input.value));
  } else {
    const budgetInput = await readJson(values.budgets ?? 'config/resource-budgets.json');
    const budgets = validateBudgets(budgetInput.value);
    if (positionals[0] === 'analyze') {
      if (!values.metadata || !values.events || !values.manifest) throw new Error('MISSING_INPUT: analysis requires metadata, events and exact manifest input.');
      const [metadata, manifest, events] = await Promise.all([readJson(values.metadata), readBounded(values.manifest, 1048576), readEvents(values.events, budgets.trace)]);
      if (metadata.value?.manifestSha256 !== manifest.sha256 || metadata.value?.budgetsSha256 !== budgetInput.sha256)
        throw new Error('INPUT_IDENTITY_MISMATCH: run metadata must match the exact manifest and budget file bytes.');
      result = analyzeTrace(metadata.value, events, {...budgets, maxEvents: budgets.trace.maxEvents});
    } else if (positionals[0] === 'saturate') {
      const offers = Number(values.offers);
      if (!Number.isSafeInteger(offers) || offers < 1 || offers > 1000000) throw new Error('INVALID_OFFERS: use 1–1000000 synthetic telemetry offers.');
      const buffer = new TraceBuffer(budgets.trace);
      const cpuStart = process.cpuUsage();
      for (let i = 0; i < offers; i++) buffer.offer({traceId: '0123456789abcdef0123456789abcdef', source: 'adapter', kind: 'frame', atUs: i, durationUs: 1000, privateText: 'redacted'});
      const saturated = buffer.stats();
      while (buffer.stats().queuedItems) buffer.drain(budgets.trace.maxBatchItems);
      result = {schemaVersion: 1, baseline: 'synthetic', actualGameBaseline: 'unmeasured', budgetsSha256: budgetInput.sha256,
        saturated, drained: buffer.stats(), nodeCpuUs: process.cpuUsage(cpuStart), nodeRssBytes: process.memoryUsage().rss,
        bridgeP99Us: {budget: budgets.bridgeP99Us, measured: false},
        scope: 'Bounded telemetry payload and slots; process RSS includes runtime overhead, not a configured guarantee. No game/IPC saturation claim.'};
    } else throw new Error('INVALID_COMMAND: run --help for analyze, compare or saturate.');
  }
  if (result) {
    const json = JSON.stringify(result, null, 2) + '\n';
    if (values.output) await writeFile(values.output, json); else process.stdout.write(json);
  }
} catch (error) {
  const message = error instanceof Error && /^[A-Z_]+: /.test(error.message) ? error.message : 'PERFORMANCE_UNAVAILABLE: check local inputs, permissions and arguments; run --help.';
  process.stderr.write(JSON.stringify({schemaVersion: 1, error: message}) + '\n'); process.exitCode = 1;
}
