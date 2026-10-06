// @ts-check
/** @returns {import('../../../tools/performance/contract.mjs').RunMetadata} */
export function run(overrides = {}) {
  return { schemaVersion: 1, traceId: '0123456789abcdef0123456789abcdef',
    manifestSha256: 'a'.repeat(64), budgetsSha256: 'b'.repeat(64), checkpointSha256: 'c'.repeat(64),
    scenario: 'baseline', repeat: 1, warmupUs: 10, durationUs: 2000000000, speed: 1, paused: false,
    instrumentation: 'frame+jfr', campaignThreadId: 7, startEpochUs: 1000000,
    alignmentUncertaintyUs: 1000, baseline: 'synthetic', droppedEvents: 0, invalidEvents: 0, ...overrides };
}
