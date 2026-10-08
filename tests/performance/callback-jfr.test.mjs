// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { sanitizeEvent } from '../../tools/performance/contract.mjs';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { run } from './support/run.mjs';

test('real JFR campaign callback spans retain their thread and never become whole frames', async t => {
  const directory = await mkdtemp(join(tmpdir(), 'nf-callback-jfr-'));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const options = {cwd: fileURLToPath(new URL('../../', import.meta.url)), encoding: /** @type {const} */ ('utf8'), timeout: 30000, maxBuffer: 100000};
  const compiled = spawnSync('javac', ['--release', '17', '-Xlint:all', '-Werror', '-d', directory,
    'tools/performance/jfr/ExportRecording.java', 'tests/performance/support/JfrCallbackCapture.java'], options);
  assert.equal(compiled.status, 0, compiled.stderr);
  const traceId = '0123456789abcdef0123456789abcdef';
  const recording = join(directory, 'callback.jfr');
  const capture = spawnSync('java', ['-cp', directory, 'JfrCallbackCapture', recording, traceId], options);
  assert.equal(capture.status, 0, capture.stderr);
  const identity = JSON.parse(capture.stdout);
  assert.ok(Number.isSafeInteger(identity.threadId) && identity.threadId > 0);
  const exported = spawnSync('java', ['-cp', directory, 'ExportRecording', recording, traceId], options);
  assert.equal(exported.status, 0, exported.stderr);
  const raw = exported.stdout.trim() ? exported.stdout.trim().split('\n').map(line => JSON.parse(line)) : [];
  // First behavioral RED: the existing exporter omits these three actual callback events.
  assert.equal(raw.length, 3);
  const records = raw.map(sanitizeEvent);
  assert.ok(records.every(event => event.kind === 'campaign_callback' && event.source === 'jfr' &&
    event.traceId === traceId && event.threadId === identity.threadId &&
    Number.isSafeInteger(event.durationUs) && Number(event.durationUs) > 0));
  assert.ok(records.every(event => event.frameId === undefined));
  assert.ok(!exported.stdout.includes('private-callback-secret'));
  assert.ok(!exported.stdout.includes(directory));
  const start = Math.min(...records.map(event => event.atUs));
  const end = Math.max(...records.map(event => event.atUs + Number(event.durationUs)));
  const metadata = run({traceId, warmupUs: 0, startEpochUs: start, durationUs: end - start + 1,
    campaignThreadId: identity.threadId, instrumentation: 'callback+jfr'});
  const report = analyzeTrace(metadata, records, {bridgeP99Us: 2000, longFrameUs: 16667});
  const durations = records.map(event => Number(event.durationUs)).sort((a, b) => a - b);
  assert.deepEqual(report.phases.campaign_callback, {count: 3, p50Us: durations[1],
    p95Us: durations[2], p99Us: durations[2], p999Us: durations[2]});
  assert.deepEqual(report.frames, {count: 0, p50Us: null, p95Us: null, p99Us: null, p999Us: null, longFrames: 0});
  assert.equal(report.actualGameBaseline, 'unmeasured');
  assert.equal(report.bridge.status, 'unmeasured');
});
