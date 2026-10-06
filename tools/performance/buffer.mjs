// @ts-check
import { sanitizeEvent } from './contract.mjs';
/** Bounded telemetry only: never use this lossy queue for commands or receipts. */
export class TraceBuffer {
  /** @param {{maxItems: number, maxBytes: number, maxEventBytes: number}} limits */
  constructor(limits) {
    if (!limits || typeof limits !== 'object') throw new Error('INVALID_BUDGET: explicit trace limits are required.');
    for (const value of [limits.maxItems, limits.maxBytes, limits.maxEventBytes]) if (!Number.isSafeInteger(value) || value <= 0) throw new Error('INVALID_BUDGET: trace limits must be positive integers.');
    if (limits.maxItems > 100000 || limits.maxBytes > 16777216 || limits.maxEventBytes > limits.maxBytes)
      throw new Error('INVALID_BUDGET: trace queue exceeds the supported bounded envelope.');
    this.limits = { ...limits };
    /** @type {({event: import('./contract.mjs').TraceEvent, bytes: number}|undefined)[]} */
    this.slots = new Array(limits.maxItems);
    this.head = 0; this.count = 0; this.bytes = 0;
    this.offered = 0; this.accepted = 0; this.dropped = 0; this.invalid = 0;
  }
  /** Nonblocking in-memory enqueue; no filesystem/socket access. @param {unknown} value */
  offer(value) {
    this.offered++;
    let event;
    try { event = sanitizeEvent(value); } catch { this.invalid++; return false; }
    const bytes = Buffer.byteLength(JSON.stringify(event));
    if (bytes > this.limits.maxEventBytes || this.count === this.limits.maxItems || this.bytes + bytes > this.limits.maxBytes) {
      this.dropped++; return false;
    }
    this.slots[(this.head + this.count) % this.limits.maxItems] = { event, bytes };
    this.count++; this.bytes += bytes; this.accepted++;
    return true;
  }
  /** Called by the consumer, never a game-thread file writer. @param {number} maxItems */
  drain(maxItems) {
    if (!Number.isSafeInteger(maxItems) || maxItems <= 0 || maxItems > this.limits.maxItems) throw new Error('INVALID_DRAIN: use a bounded positive batch size.');
    const records = [];
    while (this.count && records.length < maxItems) {
      const item = this.slots[this.head];
      if (!item) throw new Error('TRACE_CORRUPTION: occupied queue slot missing.');
      records.push(item.event); this.bytes -= item.bytes;
      this.slots[this.head] = undefined;
      this.head = (this.head + 1) % this.limits.maxItems; this.count--;
    }
    return records;
  }
  stats() { return { offered: this.offered, accepted: this.accepted, dropped: this.dropped, invalid: this.invalid, queuedItems: this.count, queuedBytes: this.bytes }; }
}
