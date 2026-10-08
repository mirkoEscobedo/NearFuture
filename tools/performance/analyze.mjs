// @ts-check
import { sanitizeEvent, validateRun } from './contract.mjs';
/** Nearest-rank percentiles; null preserves unavailable evidence. @param {number[]} values */
export function percentiles(values) {
  const sorted = [...values].sort((a,b) => a - b);
  /** @param {number} q */
  const rank = q => sorted.length ? sorted[Math.ceil(q * sorted.length) - 1] : null;
  return { count: sorted.length, p50Us: rank(.5), p95Us: rank(.95), p99Us: rank(.99), p999Us: rank(.999) };
}
/** @param {unknown} metadata @param {unknown[]} records @param {{bridgeP99Us: number, longFrameUs: number, maxEvents?: number}} budgets */
export function analyzeTrace(metadata, records, budgets) {
  const run = validateRun(metadata);
  for (const value of [budgets.bridgeP99Us, budgets.longFrameUs])
    if (!Number.isSafeInteger(value) || value <= 0) throw new Error('INVALID_BUDGET: latency budgets must be positive integer microseconds.');
  if (records.length > (budgets.maxEvents ?? 100000)) throw new Error('TRACE_TOO_LARGE: split the trace into bounded run files.');
  const start = run.startEpochUs + run.warmupUs, end = start + run.durationUs;
  if (!Number.isSafeInteger(end)) throw new Error('INVALID_RUN: time window exceeds safe microsecond precision.');
  const events = records.map(sanitizeEvent);
  if (events.some(event => event.traceId !== run.traceId)) throw new Error('TRACE_ID_MISMATCH: do not combine recordings from different runs.');
  const measured = events.filter(event => event.atUs >= start && event.atUs < end).sort((a,b) => a.atUs - b.atUs);
  /** @param {string} kind */
  const durations = kind => measured.filter(event => event.kind === kind && event.atUs + (event.durationUs ?? 0) <= end).map(event => event.durationUs ?? 0);
  const frames = durations('frame');
  const frameIds = new Set();
  for (const event of measured.filter(event => event.kind === 'frame' && event.frameId !== undefined)) {
    if (frameIds.has(event.frameId)) throw new Error('DUPLICATE_FRAME: select one authoritative frame capture source per run.');
    frameIds.add(event.frameId);
  }
  /** @type {Map<number, {capture: boolean, apply: boolean, durationUs: number}>} */
  const bridgeByFrame = new Map();
  let ungroupedBridgeSpans = 0;
  for (const event of measured.filter(event => ['capture','apply'].includes(event.kind))) {
    if (event.frameId === undefined) ungroupedBridgeSpans++;
    else {
      const spans = bridgeByFrame.get(event.frameId) ?? { capture: false, apply: false, durationUs: 0 };
      if (event.kind === 'capture') spans.capture = true;
      if (event.kind === 'apply') spans.apply = true;
      spans.durationUs += event.durationUs ?? 0;
      bridgeByFrame.set(event.frameId, spans);
    }
  }
  const groups = [...bridgeByFrame.values()];
  const incompleteFrames = groups.filter(spans => !spans.capture || !spans.apply).length;
  const bridge = percentiles(groups.filter(spans => spans.capture && spans.apply).map(spans => spans.durationUs));
  /** @type {Map<string, {process: 'node'|'adapter', threadId: number, executionSamples: number, runningCpuUs: number|null, blockedEventUs: number, ioEventUs: number, sleepEventUs: number, cpuFirst?: number, cpuLast?: number, cpuSamples?: number}>} */
  const threadData = new Map();
  for (const event of measured) {
    if (event.threadId === undefined) continue;
    const process = event.source === 'node' ? 'node' : 'adapter';
    const key = process + ':' + event.threadId;
    const thread = threadData.get(key) ?? { process, threadId: event.threadId, executionSamples: 0, runningCpuUs: null, blockedEventUs: 0, ioEventUs: 0, sleepEventUs: 0 };
    if (event.kind === 'thread_running_sample') thread.executionSamples++;
    if (event.kind === 'thread_blocked') thread.blockedEventUs += Math.min(event.durationUs ?? 0, end - event.atUs);
    if (event.kind === 'thread_io') thread.ioEventUs += Math.min(event.durationUs ?? 0, end - event.atUs);
    if (event.kind === 'thread_sleep') thread.sleepEventUs += Math.min(event.durationUs ?? 0, end - event.atUs);
    if (event.kind === 'thread_cpu') {
      if (event.cpuUs === undefined || (thread.cpuLast !== undefined && event.cpuUs < thread.cpuLast)) throw new Error('COUNTER_RESET: use a new traceId after runtime/counter restart.');
      thread.cpuFirst ??= event.cpuUs; thread.cpuLast = event.cpuUs;
      thread.cpuSamples = (thread.cpuSamples ?? 0) + 1;
      thread.runningCpuUs = thread.cpuSamples > 1 ? thread.cpuLast - thread.cpuFirst : null;
    }
    threadData.set(key, thread);
  }
  const threads = [...threadData.values()].map(({cpuFirst, cpuLast, cpuSamples, ...thread}) => thread).sort((a,b) => a.process.localeCompare(b.process) || a.threadId - b.threadId);
  /** @param {string} source */
  function resources(source) {
    const samples = measured.filter(event => event.kind === 'resource' && event.source === source);
    /** @param {'cpuUs'|'sentBytes'|'receivedBytes'} field */
    function delta(field) {
      const values = samples.map(event => event[field]).filter(value => value !== undefined);
      if (values.some((value,i) => i > 0 && value < values[i-1])) throw new Error('COUNTER_RESET: use a new traceId after runtime/counter restart.');
      return values.length >= 2 ? values[values.length-1] - values[0] : null;
    }
    const rss = samples.flatMap(event => event.rssBytes === undefined ? [] : [event.rssBytes]);
    return { samples: samples.length, peakRssBytes: rss.length ? rss.reduce((peak, value) => Math.max(peak, value), 0) : null,
      cpuUs: delta('cpuUs'), sentBytes: delta('sentBytes'), receivedBytes: delta('receivedBytes') };
  }
  const allocations = measured.filter(event => event.kind === 'allocation_sample');
  const allocationBytes = allocations.reduce((sum,event) => sum + (event.bytes ?? 0), 0);
  const queueSamples = measured.filter(event => event.kind === 'queue');
  return { schemaVersion: 1, run, actualGameBaseline: run.baseline === 'game' ? 'recorded-not-certified' : 'unmeasured',
    frames: { ...percentiles(frames), longFrames: frames.filter(value => value > budgets.longFrameUs).length },
    bridge: { ...bridge, p99BudgetUs: budgets.bridgeP99Us, ungroupedSpans: ungroupedBridgeSpans, incompleteFrames,
      status: bridge.count && !ungroupedBridgeSpans && !incompleteFrames && !run.droppedEvents && !run.invalidEvents ? 'observed-sample-only' : 'unmeasured',
      exceedsBudget: bridge.count ? Number(bridge.p99Us) > budgets.bridgeP99Us : null },
    phases: Object.fromEntries(['capture','encode','apply','save','load','gc_pause','gc_cycle','campaign_callback'].map(kind => [kind, percentiles(durations(kind))])),
    allocations: { samples: allocations.length, sampledWeightedBytes: allocations.length ? allocationBytes : null,
      estimatedBytesPerSecond: allocations.length ? allocationBytes / (run.durationUs / 1000000) : null },
    threads, resources: { node: resources('node'), adapter: resources('adapter') },
    queues: { samples: queueSamples.length, peakItems: queueSamples.length ? queueSamples.reduce((peak, x) => Math.max(peak, x.queueItems ?? 0), 0) : null,
      peakBytes: queueSamples.length ? queueSamples.reduce((peak, x) => Math.max(peak, x.queueBytes ?? 0), 0) : null },
    staleRejections: measured.filter(event => event.kind === 'stale').length,
    quality: { droppedEvents: run.droppedEvents, invalidEvents: run.invalidEvents },
    warnings: [run.droppedEvents || run.invalidEvents ? 'Telemetry loss: frame tails and resource observations cover an incomplete trace; comparisons are unavailable.' : '',
      frames.length < 10000 ? 'Low tail sample count: p99.9 is descriptive and needs more repetitions.' : '',
      'Execution samples are not running time; CPU counters and wait/I/O events may overlap and must not be added as a partition.',
      'JFR allocation weights estimate allocation; missing event kinds are unavailable evidence.',
      'No acceleration or guaranteed FPS claim follows from these observations.'].filter(Boolean) };
}
