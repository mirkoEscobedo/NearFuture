// @ts-check
/** @typedef {'adapter'|'node'|'jfr'} Source */
/** @typedef {{traceId: string, source: Source, kind: string, atUs: number, durationUs?: number, bytes?: number, threadId?: number, queueItems?: number, queueBytes?: number, rssBytes?: number, cpuUs?: number, sentBytes?: number, receivedBytes?: number, frameId?: number, requestId?: string}} TraceEvent */
const kinds = new Set(['frame', 'capture', 'encode', 'apply', 'queue', 'resource', 'stale', 'save', 'load', 'gc_pause', 'gc_cycle', 'allocation_sample', 'thread_running_sample', 'thread_cpu', 'thread_blocked', 'thread_io', 'thread_sleep']);
const durationKinds = new Set(['frame', 'capture', 'encode', 'apply', 'save', 'load', 'gc_pause', 'gc_cycle', 'thread_blocked', 'thread_io', 'thread_sleep']);
export const hashPattern = /^[a-f0-9]{64}$/;
export const tracePattern = /^[a-f0-9]{32}$/;
/** @param {unknown} value @returns {TraceEvent} */
export function sanitizeEvent(value) {
  if (!value || typeof value !== 'object') throw new Error('INVALID_EVENT: expected an event object.');
  const raw = /** @type {Record<string, unknown>} */ (value);
  if (typeof raw.traceId !== 'string' || !tracePattern.test(raw.traceId) ||
      !['adapter','node','jfr'].includes(String(raw.source)) || !kinds.has(String(raw.kind)))
    throw new Error('INVALID_EVENT: trace ID, source and kind must use the declared trace contract.');
  const fields = ['atUs', 'durationUs', 'bytes', 'threadId', 'queueItems', 'queueBytes', 'rssBytes', 'cpuUs', 'sentBytes', 'receivedBytes', 'frameId'];
  for (const key of fields) {
    if ((key === 'atUs' || raw[key] !== undefined) &&
      (typeof raw[key] !== 'number' || !Number.isSafeInteger(raw[key]) || Number(raw[key]) < 0))
      throw new Error(`INVALID_EVENT: ${key} must be a nonnegative safe integer.`);
  }
  if (durationKinds.has(String(raw.kind)) && raw.durationUs === undefined)
    throw new Error('INVALID_EVENT: this event kind requires durationUs.');
  if (raw.kind === 'queue' && (raw.queueItems === undefined || raw.queueBytes === undefined)) throw new Error('INVALID_EVENT: queue samples require item and byte counts.');
  if (raw.kind === 'allocation_sample' && raw.bytes === undefined)
    throw new Error('INVALID_EVENT: allocation samples require bytes.');
  if (raw.kind === 'thread_cpu' && (raw.threadId === undefined || raw.cpuUs === undefined))
    throw new Error('INVALID_EVENT: thread CPU records require threadId and cumulative cpuUs.');
  if (raw.requestId !== undefined && (typeof raw.requestId !== 'string' || !tracePattern.test(raw.requestId))) throw new Error('INVALID_EVENT: requestId must be hexadecimal correlation data.');
  const event = { traceId: raw.traceId, source: raw.source, kind: raw.kind };
  for (const key of fields) if (raw[key] !== undefined) Object.assign(event, { [key]: raw[key] });
  if (raw.requestId !== undefined) Object.assign(event, { requestId: raw.requestId });
  return /** @type {TraceEvent} */ (event);
}

/** @typedef {{schemaVersion: 1, traceId: string, manifestSha256: string, budgetsSha256: string, checkpointSha256: string, scenario: string, repeat: number, warmupUs: number, durationUs: number, speed: number, paused: boolean, instrumentation: string, campaignThreadId: number, startEpochUs: number, alignmentUncertaintyUs: number, baseline: 'synthetic'|'game', droppedEvents: number, invalidEvents: number, controlEvidenceSha256?: string, referenceManifestSha256?: string, hardwareSha256?: string, liveJvmFlagsSha256?: string}} RunMetadata */
/** @param {unknown} value @returns {RunMetadata} */
export function validateRun(value) {
  if (!value || typeof value !== 'object') throw new Error('INVALID_RUN: expected run metadata.');
  const raw = /** @type {Record<string, unknown>} */ (value);
  if (raw.schemaVersion !== 1 || typeof raw.traceId !== 'string' || !tracePattern.test(raw.traceId))
    throw new Error('INVALID_RUN: schemaVersion 1 and a 32-character hexadecimal traceId are required.');
  for (const key of ['manifestSha256','budgetsSha256','checkpointSha256'])
    if (typeof raw[key] !== 'string' || !hashPattern.test(String(raw[key]))) throw new Error(`INVALID_RUN: ${key} must identify exact inputs with SHA-256.`);
  for (const key of ['repeat','warmupUs','durationUs','campaignThreadId','startEpochUs','alignmentUncertaintyUs','droppedEvents','invalidEvents'])
    if (typeof raw[key] !== 'number' || !Number.isSafeInteger(raw[key]) || Number(raw[key]) < 0) throw new Error(`INVALID_RUN: ${key} must be a nonnegative safe integer.`);
  if (Number(raw.repeat) < 1 || Number(raw.repeat) > 50 || Number(raw.durationUs) < 1 ||
    typeof raw.speed !== 'number' || !Number.isFinite(raw.speed) || raw.speed <= 0 || raw.speed > 10 ||
    typeof raw.paused !== 'boolean' || !['none','frame','frame+jfr'].includes(String(raw.instrumentation)) ||
    !['baseline','control','shadow','offload','parallel','instrumentation-off'].includes(String(raw.scenario)) ||
    !['synthetic','game'].includes(String(raw.baseline))) throw new Error('INVALID_RUN: scenario, repetitions, duration, speed/pause and instrumentation must be explicit.');
  if (raw.scenario === 'control' && (typeof raw.controlEvidenceSha256 !== 'string' || !hashPattern.test(raw.controlEvidenceSha256)))
    throw new Error('INVALID_RUN: control requires supported-seam evidence; removing a mod is not a safe control.');
  for (const key of ['hardwareSha256','liveJvmFlagsSha256']) {
    if (raw[key] !== undefined && (typeof raw[key] !== 'string' || !hashPattern.test(String(raw[key])))) throw new Error('INVALID_RUN: hardware/live JVM flags need SHA-256 identities.');
    if (raw.baseline === 'game' && raw[key] === undefined) throw new Error('MISSING_RUNTIME_EVIDENCE: actual game runs require hardwareSha256 and liveJvmFlagsSha256; configured vmparams alone is insufficient.');
  }
  if (raw.referenceManifestSha256 !== undefined && (typeof raw.referenceManifestSha256 !== 'string' || !hashPattern.test(raw.referenceManifestSha256))) throw new Error('INVALID_RUN: referenceManifestSha256 must be a SHA-256 identity.');
  const keys = ['schemaVersion','traceId','manifestSha256','budgetsSha256','checkpointSha256','scenario','repeat','warmupUs','durationUs','speed','paused','instrumentation','campaignThreadId','startEpochUs','alignmentUncertaintyUs','baseline','droppedEvents','invalidEvents','controlEvidenceSha256','referenceManifestSha256','hardwareSha256','liveJvmFlagsSha256'];
  return /** @type {RunMetadata} */ (Object.fromEntries(keys.filter(key => raw[key] !== undefined).map(key => [key, raw[key]])));
}
