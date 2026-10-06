// @ts-check
/** @typedef {{epochUs: number, monotonicNs: bigint, uncertaintyUs: number}} ClockAnchor */
/** @param {ClockAnchor} anchor @param {bigint} monotonicNs */
export function alignMonotonic(anchor, monotonicNs) {
  if (!Number.isSafeInteger(anchor.epochUs) || anchor.epochUs < 0 || !Number.isSafeInteger(anchor.uncertaintyUs) || anchor.uncertaintyUs < 0 ||
    typeof anchor.monotonicNs !== 'bigint' || typeof monotonicNs !== 'bigint' || monotonicNs < anchor.monotonicNs)
    throw new Error('INVALID_CLOCK: use an explicit same-process monotonic anchor and uncertainty.');
  const atUs = anchor.epochUs + Number((monotonicNs - anchor.monotonicNs) / 1000n);
  if (!Number.isSafeInteger(atUs)) throw new Error('INVALID_CLOCK: microsecond alignment exceeds safe numeric precision.');
  return {atUs, uncertaintyUs: anchor.uncertaintyUs};
}
/** @param {ClockAnchor} first @param {ClockAnchor} last */
export function clockDrift(first, last) {
  const predicted = alignMonotonic(first, last.monotonicNs);
  const driftUs = Math.abs(last.epochUs - predicted.atUs);
  return {driftUs, withinUncertainty: driftUs <= first.uncertaintyUs + last.uncertaintyUs};
}
/** Platform effects stay explicit; sample before/after each recording, outside game frame work. @returns {ClockAnchor} */
export function sampleClockAnchor() {
  const before = process.hrtime.bigint();
  const epochUs = Date.now() * 1000;
  const after = process.hrtime.bigint();
  return {epochUs, monotonicNs: (before + after) / 2n, uncertaintyUs: 1000 + Math.ceil(Number(after - before) / 1000)};
}
