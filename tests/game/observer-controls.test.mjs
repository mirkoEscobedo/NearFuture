// @ts-check
import test, {before, after} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {delimiter, join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

let directory = '';
let api = '';
const options = {cwd: fileURLToPath(new URL('../../', import.meta.url)),
  encoding: /** @type {const} */ ('utf8'), timeout: 30000, maxBuffer: 100000};
before(async () => {
  const game = process.env.STARSECTOR_HOME;
  assert.ok(game, 'licensed local STARSECTOR_HOME is required; compilation failure is not a control result');
  directory = await mkdtemp(join(tmpdir(), 'nf-observer-controls-'));
  const libraries = ['starfarer.api.jar', 'xstream-1.4.10.jar', 'log4j-1.2.9.jar',
    'lwjgl.jar', 'lwjgl_util.jar', 'json.jar', 'fs.common_obf.jar', 'fs.sound_obf.jar'];
  api = libraries.map(name => join(game, 'starsector-core', name)).join(delimiter);
  const compiled = spawnSync('javac', ['--release', '17', '-Xlint:all', '-Werror',
    '-cp', api, '-sourcepath', 'java/adapter/src/main/java', '-d', directory,
    'java/contract/src/main/java/nf/contract/BuildIdentity.java',
    'java/adapter/src/main/java/nf/adapter/NearFutureModPlugin.java',
    'tests/game/support/CampaignObserverLifecycleControls.java'], options);
  assert.equal(compiled.status, 0, compiled.stderr);
});
after(async () => { if (directory) await rm(directory, {recursive: true, force: true}); });
/** @param {string} mode @param {string[]} properties */
function capture(mode, properties) {
  const captured = spawnSync('java', [...properties, '-cp', [directory, api].join(delimiter),
    'CampaignObserverLifecycleControls', mode, join(directory, `${mode}.jfr`)], options);
  assert.equal(captured.status, 0, captured.stderr);
  return JSON.parse(captured.stdout);
}
test('invalid or absent observer opt-in and null sector create no owned script and restore nullable caller state', () => {
  const actual = capture('optIn', []);
  assert.deepEqual(actual.ownedCounts, Array(11).fill(0));
  assert.equal(actual.sentinelIntactEveryCut, true);
  assert.equal(actual.finalOwnedCount, 0);
  assert.equal(actual.sentinelFinal, true);
  assert.equal(actual.previousEnableUnset, true);
  assert.equal(actual.previousTraceUnset, true);
  assert.equal(actual.restoredSector, true);
  assert.equal(actual.restoredProperties, true);
});
test('repeated load replaces only the owned script on the same and a different actual sector host', () => {
  const actual = capture('repeatedLoad', ['-Dnf.profiling.callback.enabled=false',
    '-Dnf.profiling.traceId=previous-trace']);
  assert.equal(actual.firstOwnedCount, 1);
  assert.equal(actual.sameSectorOwnedCount, 1);
  assert.equal(actual.firstClosedRemoved, true);
  assert.equal(actual.sameSectorReplacementDistinct, true);
  assert.equal(actual.oldSentinelAfterRepeat, true);
  assert.equal(actual.oldSectorOwnedCount, 0);
  assert.equal(actual.newSectorOwnedCount, 1);
  assert.equal(actual.secondClosedRemovedFromOldSector, true);
  assert.equal(actual.currentDistinct, true);
  assert.equal(actual.bothSentinelsIntact, true);
  assert.equal(actual.finalOldCount, 0);
  assert.equal(actual.finalNewCount, 0);
  assert.equal(actual.bothSentinelsFinal, true);
  assert.equal(actual.previousEnableUnset, false);
  assert.equal(actual.previousTraceUnset, false);
  assert.equal(actual.restoredSector, true);
  assert.equal(actual.restoredProperties, true);
});
test('after-save opt-in transitions keep zero or one current observer and stale closed callbacks emit nothing', () => {
  const actual = capture('afterSave', ['-Dnf.profiling.traceId=previous-trace']);
  assert.deepEqual(actual.ownedCounts, [0, 1, 1, 0]);
  assert.equal(actual.firstClosedRemoved, true);
  assert.equal(actual.replacementDistinct, true);
  assert.equal(actual.secondClosedRemoved, true);
  assert.equal(actual.sentinelIntactEveryCut, true);
  // One genuine current callback, despite three deliberate calls on subsequently closed stale observers.
  assert.equal(actual.actualCallbackEvents, 1);
  assert.equal(actual.callbackTraceMatches, true);
  assert.ok(Number.isSafeInteger(actual.recordingBytes) && actual.recordingBytes > 0
    && actual.recordingBytes <= 8 * 1024 * 1024);
  assert.equal(actual.finalOwnedCount, 0);
  assert.equal(actual.sentinelFinal, true);
  assert.equal(actual.previousEnableUnset, true);
  assert.equal(actual.previousTraceUnset, false);
  assert.equal(actual.restoredSector, true);
  assert.equal(actual.restoredProperties, true);
});
