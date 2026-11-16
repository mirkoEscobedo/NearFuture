// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { sanitizeEvent } from '../../tools/performance/contract.mjs';
import { analyzeTrace } from '../../tools/performance/analyze.mjs';
import { run } from '../performance/support/run.mjs';

test('explicit local observer registers, emits its own callbacks, and detaches before save', async t => {
  const game = process.env.STARSECTOR_HOME;
  assert.ok(game, 'licensed local STARSECTOR_HOME is required; setup failure is not behavioral RED');
  const directory = await mkdtemp(join(tmpdir(), 'nf-observer-host-'));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const libraries = ['starfarer.api.jar', 'xstream-1.4.10.jar', 'log4j-1.2.9.jar',
    'lwjgl.jar', 'lwjgl_util.jar', 'json.jar', 'fs.common_obf.jar', 'fs.sound_obf.jar'];
  const api = libraries.map(name => join(game, 'starsector-core', name)).join(delimiter);
  const options = {cwd: fileURLToPath(new URL('../../', import.meta.url)),
    encoding: /** @type {const} */ ('utf8'), timeout: 30000, maxBuffer: 100000};
  const compiled = spawnSync('javac', ['--release', '17', '-Xlint:all', '-Werror',
    '-cp', api, '-sourcepath', 'java/adapter/src/main/java', '-d', directory, 'java/contract/src/main/java/nf/contract/BuildIdentity.java',
    'java/adapter/src/main/java/nf/adapter/NearFutureModPlugin.java',
    'tools/performance/jfr/ExportRecording.java', 'tests/game/support/CampaignObserverCapture.java'], options);
  assert.equal(compiled.status, 0, compiled.stderr);
  const traceId = '0123456789abcdef0123456789abcdef';
  const recording = join(directory, 'observer.jfr');
  const capture = spawnSync('java', ['-cp', [directory, api].join(delimiter),
    'CampaignObserverCapture', recording, traceId], options);
  assert.equal(capture.status, 0, capture.stderr);
  const identity = JSON.parse(capture.stdout);
  assert.equal(identity.restoredSector, true);
  assert.equal(identity.restoredProperties, true);
  // First behavioral RED: the real inherited onGameLoad leaves this count at zero.
  assert.equal(identity.observerCount, 1);
  assert.equal(identity.remainingCount, 0);
  assert.equal(identity.removedOwned, true);
  assert.ok(Number.isSafeInteger(identity.threadId) && identity.threadId > 0);
  assert.ok((await stat(recording)).size <= 8 * 1024 * 1024);
  const exported = spawnSync('java', ['-cp', directory, 'ExportRecording', recording, traceId], options);
  assert.equal(exported.status, 0, exported.stderr);
  const raw = exported.stdout.trim().split('\n').filter(Boolean).map(line => JSON.parse(line));
  assert.equal(raw.length, 3);
  const records = raw.map(sanitizeEvent);
  for (const event of raw) {
    assert.deepEqual(Object.keys(event).sort(), ['atUs', 'durationUs', 'kind', 'source', 'threadId', 'traceId']);
    assert.equal(event.kind, 'campaign_callback');
    assert.equal(event.source, 'jfr');
    assert.equal(event.traceId, traceId);
    assert.equal(event.threadId, identity.threadId);
    assert.ok(Number.isSafeInteger(event.durationUs) && event.durationUs >= 0);
  }
  assert.ok(!exported.stdout.includes(directory));
  assert.ok(!exported.stdout.includes(game));
  const start = Math.min(...records.map(event => event.atUs));
  const end = Math.max(...records.map(event => event.atUs + Number(event.durationUs)));
  const report = analyzeTrace(run({traceId, warmupUs: 0, startEpochUs: start,
    durationUs: end - start + 1, campaignThreadId: identity.threadId, instrumentation: 'callback+jfr'}),
  records, {bridgeP99Us: 2000, longFrameUs: 16667});
  assert.equal(report.phases.campaign_callback.count, 3);
  assert.deepEqual(report.frames, {count: 0, p50Us: null, p95Us: null, p99Us: null, p999Us: null, longFrames: 0});
  assert.equal(report.actualGameBaseline, 'unmeasured');
  assert.equal(report.bridge.status, 'unmeasured');
});
