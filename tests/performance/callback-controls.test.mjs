// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { sanitizeEvent } from '../../tools/performance/contract.mjs';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { run } from './support/run.mjs';

test('callback evidence rejects missing identity and whole-frame claims without disclosing private inputs', async t => {
  const traceId = '0123456789abcdef0123456789abcdef';
  const valid = { traceId, source: 'jfr', kind: 'campaign_callback', atUs: 2000000, durationUs: 12, threadId: 7 };
  const invalid = [
    {...valid, traceId: undefined}, {...valid, traceId: 'private-callback-secret'},
    {...valid, durationUs: undefined}, {...valid, durationUs: Infinity},
    {...valid, threadId: undefined}, {...valid, threadId: 0}, {...valid, threadId: -1},
    {...valid, threadId: 1.5}, {...valid, threadId: Infinity}, {...valid, threadId: '7'},
    {...valid, frameId: 0},
  ];
  for (const value of invalid) assert.throws(() => sanitizeEvent(value), error => {
    assert.ok(error instanceof Error);
    assert.match(error.message, /^INVALID_EVENT:/);
    assert.ok(!error.message.includes('private-callback-secret'));
    return true;
  });
  const cleaned = sanitizeEvent({...valid, privateText: 'private-callback-secret', savePath: '/private/save', threadName: 'private-thread'});
  assert.deepEqual(cleaned, valid);
  const report = analyzeTrace(run({instrumentation: 'callback+jfr'}), [cleaned], {bridgeP99Us: 2000, longFrameUs: 16667});
  assert.deepEqual(report.phases.campaign_callback, {count: 1, p50Us: 12, p95Us: 12, p99Us: 12, p999Us: 12});
  assert.deepEqual(report.frames, {count: 0, p50Us: null, p95Us: null, p99Us: null, p999Us: null, longFrames: 0});
  assert.equal(report.bridge.status, 'unmeasured');
  const directory = await mkdtemp(join(tmpdir(), 'nf-callback-controls-'));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const options = {cwd: fileURLToPath(new URL('../../', import.meta.url)), encoding: /** @type {const} */ ('utf8'), timeout: 30000, maxBuffer: 100000};
  const compiled = spawnSync('javac', ['--release', '17', '-Xlint:all', '-Werror', '-d', directory,
    'tools/performance/jfr/ExportRecording.java', 'tests/performance/support/JfrCallbackInvalidCapture.java'], options);
  assert.equal(compiled.status, 0, compiled.stderr);
  for (const mode of ['missing-trace', 'frame-claim', 'trace-mismatch']) {
    const recording = join(directory, `${mode}.jfr`);
    const captured = spawnSync('java', ['-cp', directory, 'JfrCallbackInvalidCapture', recording, traceId, mode], options);
    assert.equal(captured.status, 0, captured.stderr);
    const cliTrace = mode === 'trace-mismatch' ? 'f'.repeat(32) : traceId;
    const exported = spawnSync('java', ['-cp', directory, 'ExportRecording', recording, cliTrace], options);
    assert.equal(exported.status, 1);
    assert.equal(exported.stdout, '');
    assert.equal(exported.stderr.trim(), 'JFR_EXPORT_UNAVAILABLE: verify bounded recording, event fields and hexadecimal trace ID locally; discard any partial output.');
    assert.ok(!exported.stderr.includes(directory));
    assert.ok(!exported.stderr.includes('private-callback-secret'));
  }
});
