import test from 'node:test';
import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { captureInstallation } from '../../tools/runtime-manifest/capture.mjs';
import { installation } from './support/installation.mjs';

test('unverified JVM and undeclared game build have actionable diagnostics', async t => {
  const root = await installation(t, { 'jre/release': 'IMPLEMENTOR="Unknown"\nJAVA_VERSION="99"' });
  const manifest = await captureInstallation({ gameDir: root });
  assert.equal(manifest.compatibility.allowedMode, 'legacy-read-only');
  assert.ok(manifest.diagnostics.some(x => x.code === 'UNVERIFIED_JVM' && x.action));
  assert.ok(manifest.diagnostics.some(x => x.code === 'UNKNOWN_GAME_BUILD' && x.action));
});
test('missing API files give an actionable relative diagnostic', async t => {
  const root = await installation(t);
  const { rm } = await import('node:fs/promises');
  await rm(join(root, 'starsector-core/starfarer.api.jar'));
  await assert.rejects(captureInstallation({ gameDir: root }), /MISSING_INPUT: starsector-core\/starfarer.api.jar; restore/);
});
test('changed configuration changes its content identity', async t => {
  const root = await installation(t);
  const before = await captureInstallation({ gameDir: root });
  await writeFile(join(root, 'mods/Nex/exerelin_config.json'), '{"enableStrategicAI":false}');
  const after = await captureInstallation({ gameDir: root });
  /** @param {Awaited<ReturnType<typeof captureInstallation>>} manifest */
  const config = manifest => manifest.artifacts.find(x => x.path.endsWith('/exerelin_config.json'))?.sha256;
  assert.notEqual(config(before), config(after));
});
test('active JAR traversal paths are rejected before reading external files', async t => {
  const root = await installation(t, { 'mods/Nex/mod_info.json': '{"id":"nexerelin","version":"1","jars":["../../../private.jar"]}' });
  await assert.rejects(captureInstallation({ gameDir: root }), /UNSAFE_MOD_JAR: nexerelin/);
});
test('missing enabled mod stays unknown with recovery guidance', async t => {
  const root = await installation(t, { 'mods/enabled_mods.json': '{"enabledMods":["missing"]}' });
  const manifest = await captureInstallation({ gameDir: root });
  assert.deepEqual(manifest.diagnostics.find(x => x.code === 'MISSING_ENABLED_MOD'), {
    code: 'MISSING_ENABLED_MOD', modId: 'missing', action: 'Restore the enabled mod or disable its ID, then recapture.',
  });
});

test('malformed enabled-mod document has a domain recovery diagnostic', async t => {
  const root = await installation(t, { 'mods/enabled_mods.json': 'null' });
  await assert.rejects(captureInstallation({ gameDir: root }), /INVALID_ENABLED_MODS: enabledMods must be an array/);
});
