// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { sanitizeEvent } from '../../tools/performance/contract.mjs';
test('real synthetic JFR capture exports allowlisted numeric evidence without private fields', async t => {
  const directory = await mkdtemp(join(tmpdir(), 'nf-jfr-'));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const compiled = spawnSync('javac', ['--release', '17', '-Xlint:all', '-Werror', '-d', directory,
    'tools/performance/jfr/ExportRecording.java', 'tests/performance/support/JfrCapture.java'], {encoding: 'utf8', timeout: 30000});
  assert.equal(compiled.status, 0, compiled.stderr);
  const traceId = '0123456789abcdef0123456789abcdef';
  const recording = join(directory, 'synthetic.jfr');
  const capture = spawnSync('java', ['-cp', directory, 'JfrCapture', recording, traceId], {encoding: 'utf8', timeout: 30000});
  assert.equal(capture.status, 0, capture.stderr);
  const exported = spawnSync('java', ['-cp', directory, 'ExportRecording', recording, traceId], {encoding: 'utf8', timeout: 30000});
  assert.equal(exported.status, 0, exported.stderr);
  const records = exported.stdout.trim().split('\n').map(line => sanitizeEvent(JSON.parse(line)));
  assert.equal(records.filter(event => event.kind === 'frame').length, 3);
  assert.ok(records.some(event => event.kind === 'thread_sleep' && Number(event.durationUs) > 0));
  assert.ok(records.every(event => event.traceId === traceId && event.source === 'jfr'));
  assert.ok(!exported.stdout.includes('private-secret'));
  assert.ok(!exported.stdout.includes(directory));
});
