// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {delimiter, join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

test('enabled observer reattaches after a failed save and preserves unrelated transient scripts', async t => {
  const game = process.env.STARSECTOR_HOME;
  assert.ok(game, 'licensed local STARSECTOR_HOME is required; setup failure is not behavioral RED');
  const directory = await mkdtemp(join(tmpdir(), 'nf-observer-save-failed-'));
  t.after(() => rm(directory, {recursive: true, force: true}));
  const libraries = ['starfarer.api.jar', 'xstream-1.4.10.jar', 'log4j-1.2.9.jar',
    'lwjgl.jar', 'lwjgl_util.jar', 'json.jar', 'fs.common_obf.jar', 'fs.sound_obf.jar'];
  const api = libraries.map(name => join(game, 'starsector-core', name)).join(delimiter);
  const options = {cwd: fileURLToPath(new URL('../../', import.meta.url)),
    encoding: /** @type {const} */ ('utf8'), timeout: 30000, maxBuffer: 100000};
  const compiled = spawnSync('javac', ['--release', '17', '-Xlint:all', '-Werror',
    '-cp', api, '-sourcepath', 'java/adapter/src/main/java', '-d', directory,
    'java/contract/src/main/java/nf/contract/BuildIdentity.java',
    'java/adapter/src/main/java/nf/adapter/NearFutureModPlugin.java',
    'tests/game/support/CampaignObserverSaveFailedCapture.java'], options);
  assert.equal(compiled.status, 0, compiled.stderr);
  const capture = spawnSync('java', ['-cp', [directory, api].join(delimiter),
    'CampaignObserverSaveFailedCapture'], options);
  assert.equal(capture.status, 0, capture.stderr);
  const identity = JSON.parse(capture.stdout);
  assert.equal(identity.restoredSector, true);
  assert.equal(identity.restoredProperties, true);
  assert.equal(identity.loadedCount, 1);
  assert.equal(identity.beforeSaveCount, 0);
  assert.equal(identity.oldDone, true);
  assert.equal(identity.sentinelBefore, true);
  // First behavioral RED: the real inherited onGameSaveFailed leaves this count at zero.
  assert.equal(identity.resumedCount, 1);
  assert.equal(identity.replacementDistinct, true);
  assert.equal(identity.sentinelAfter, true);
  assert.equal(identity.finalOwnedCount, 0);
  assert.equal(identity.sentinelFinal, true);
});
