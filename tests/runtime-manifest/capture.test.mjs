import test from 'node:test';
import assert from 'node:assert/strict';
import { captureInstallation } from '../../tools/runtime-manifest/capture.mjs';
import { installation } from './support/installation.mjs';
test('same installation produces deterministic versioned hashes without private paths or secrets', async t => {
  const root = await installation(t);
  const options = { gameDir: root, gameVersion: '0.98a-RC8' };
  const first = await captureInstallation(options);
  assert.deepEqual(first, await captureInstallation(options));
  assert.equal(first.game.version, '0.98a-RC8');
  assert.equal(first.jvm.vendor, 'Example JVM');
  assert.equal(first.mods[0].version, '0.12.2c');
  assert.equal(first.artifacts.find(x => x.path === 'starsector-core/starfarer.api.jar')?.sha256,
    'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  assert.equal(first.compatibility.status, 'unknown');
  assert.equal(first.compatibility.allowedMode, 'legacy-read-only');
  assert.ok(first.jvm.flags.includes('-Xmx4g'));
  assert.ok(!JSON.stringify(first).includes(root));
  assert.ok(!JSON.stringify(first).includes('secret'));
  assert.ok(!JSON.stringify(first).includes('C:\\private'));
});
