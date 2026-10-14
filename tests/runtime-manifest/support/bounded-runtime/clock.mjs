// @ts-check
import { performance } from 'node:perf_hooks';

/** One case clock, including imports, setup, observation and cleanup. */
export class CaseClock {
  start = performance.now();
  deadline = this.start + 20_000;
  /** @param {boolean} [cleanup] */
  check(cleanup = false) {
    if (performance.now() >= this.deadline - (cleanup ? 0 : 2_000))
      throw new Error('CASE_DEADLINE: no further case work permitted');
  }
  /** @template T @param {() => T | Promise<T>} action @param {boolean} [cleanup] */
  async guard(action, cleanup = false) {
    this.check(cleanup);
    try { return await action(); }
    finally { this.check(cleanup); }
  }
  /** Mandatory resource cleanup never refreshes the original deadline.
   * @param {() => Promise<void>} action */
  async close(action) {
    const overdue = performance.now() >= this.deadline;
    await action();
    if (overdue || performance.now() >= this.deadline)
      throw new Error('CASE_DEADLINE: close returned after deadline');
  }
}
