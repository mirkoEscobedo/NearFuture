// @ts-check
import { open } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { sanitizeEvent } from './contract.mjs';
/** @typedef {{read: (buffer: Buffer, offset: number, length: number, position: null) => Promise<{bytesRead: number}>, close: () => Promise<void>}} InputHandle */
/** @param {string} path @param {number} maxBytes @param {(path: string) => Promise<InputHandle>} [openInput] */
export async function readBounded(path, maxBytes, openInput = path => open(path, 'r')) {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > 67108864) throw new Error('INVALID_BUDGET: offline input limit is 64 MiB.');
  const handle = await openInput(path);
  try {
    const storage = Buffer.alloc(maxBytes + 1);
    let used = 0;
    while (used < storage.length) {
      const {bytesRead} = await handle.read(storage, used, storage.length - used, null);
      if (bytesRead === 0) break;
      used += bytesRead;
    }
    if (used > maxBytes) throw new Error('INPUT_TOO_LARGE: split recordings into bounded run files.');
    const bytes = storage.subarray(0, used);
    return {bytes, sha256: createHash('sha256').update(bytes).digest('hex')};
  } finally { await handle.close(); }
}
/** @param {string} path @param {number} [maxBytes] */
export async function readJson(path, maxBytes = 65536) {
  const input = await readBounded(path, maxBytes);
  try { return { value: JSON.parse(input.bytes.toString('utf8')), sha256: input.sha256 }; }
  catch { throw new Error('INVALID_JSON: repair the local run/configuration document.'); }
}
/** @param {string} path @param {{maxInputBytes: number, maxEventBytes: number, maxEvents: number}} limits */
export async function readEvents(path, limits) {
  const input = await readBounded(path, limits.maxInputBytes);
  /** @type {import('./contract.mjs').TraceEvent[]} */
  const events = [];
  for (const line of input.bytes.toString('utf8').split('\n')) {
    if (!line.trim()) continue;
    if (events.length >= limits.maxEvents || Buffer.byteLength(line) > limits.maxEventBytes)
      throw new Error('TRACE_TOO_LARGE: event count or event bytes exceed the run budget.');
    let event;
    try { event = JSON.parse(line); } catch { throw new Error('INVALID_EVENT_JSON: every trace line must contain one JSON object.'); }
    events.push(sanitizeEvent(event));
  }
  return events;
}
/** @param {unknown} value */
export function validateBudgets(value) {
  const budgets = /** @type {{schemaVersion: number, bridgeP99Us: number, longFrameUs: number, trace: {maxItems: number, maxBytes: number, maxEventBytes: number, maxBatchItems: number, maxInputBytes: number, maxEvents: number}}} */ (value);
  if (!budgets || budgets.schemaVersion !== 1 || !budgets.trace) throw new Error('INVALID_BUDGET: resource budget schemaVersion 1 is required.');
  for (const limit of [budgets.bridgeP99Us, budgets.longFrameUs, budgets.trace.maxItems, budgets.trace.maxBytes, budgets.trace.maxEventBytes, budgets.trace.maxBatchItems, budgets.trace.maxInputBytes, budgets.trace.maxEvents])
    if (!Number.isSafeInteger(limit) || limit < 1) throw new Error('INVALID_BUDGET: resource limits must be positive integers.');
  if (budgets.trace.maxItems > 100000 || budgets.trace.maxBytes > 16777216 || budgets.trace.maxEventBytes > budgets.trace.maxBytes || budgets.trace.maxInputBytes > 67108864 || budgets.trace.maxEvents > 100000 || budgets.trace.maxBatchItems > budgets.trace.maxItems)
    throw new Error('INVALID_BUDGET: offline trace and drain limits exceed the supported envelope.');
  return budgets;
}
