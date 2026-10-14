// @ts-check
import assert from 'node:assert/strict';
import { readStable, sha256, gitBlob } from './io.mjs';

export const PUBLIC_FILES = [
  { path: 'package.json', bytes: 2639, sha256: '6abe1d97c5c58fadcb79435533022d3325cabb76c6fb56683c1bd49752315499' },
  { path: 'tools/runtime-manifest/metadata.mjs', bytes: 2309, sha256: '578cf38b6356116cdd2497fe7f29f98b09ac5284a2e5221df8e4b6b8c70719d2' },
  { path: 'config/runtime-support.json', bytes: 1615, sha256: '5a98cff8375686598578c4edc76e9412203b9fc27f04c6e37a0a697635333acf' },
];
export const PRODUCT_FILES = [
  'tools/runtime-manifest/observe-runtime-package.mjs',
  'tools/runtime-manifest/runtime-package-registry.mjs',
];
export const CANARIES = [
  ['config/user.private.json', '{"private":"FAKE_CONFIG_KEY_CANARY"}'],
  ['tools/runtime-manifest/not-selected.private.json', '{"path":"C:/FAKE_PRIVATE_PATH_CANARY"}'],
  ['mods/Fake/data/config/settings.json', '{"provider":"FAKE_PROVIDER_ID_CANARY","state":"FAKE_STATE_DIGEST_CANARY"}'],
  ['saves/fake-save.json', '{"chat":"FAKE_CHAT_SAVE_CANARY"}'],
  ['vault/fake-key.txt', 'FAKE_VAULT_SEED_CANARY'],
];
/** @param {import('../bounded-runtime-custody.mjs').RuntimeCase} evidence
 * @param {URL} checkout */
export async function installFixture(evidence, checkout) {
  /** @type {Array<{path:string,source:Awaited<ReturnType<typeof readStable>>}>} */
  const originals = [];
  for (const path of PRODUCT_FILES) {
    const source = await readStable(evidence.clock, new URL(path, checkout));
    originals.push({ path, source });
    await evidence.put('control/checkout/' + path, source.bytes);
    await evidence.put('package/' + path, source.bytes);
    const copied = await readStable(evidence.clock, evidence.path('package/' + path));
    assert.deepEqual(copied.bytes, source.bytes);
    assert.equal(sha256(copied.bytes), sha256(source.bytes));
    assert.equal(gitBlob(copied.bytes), gitBlob(source.bytes));
  }
  for (const expected of PUBLIC_FILES) {
    const source = await readStable(evidence.clock, new URL(expected.path, checkout));
    assert.equal(source.bytes.length, expected.bytes);
    assert.equal(sha256(source.bytes), expected.sha256);
    originals.push({ path: expected.path, source });
    await evidence.put('control/checkout/' + expected.path, source.bytes);
    await evidence.put('package/' + expected.path, source.bytes);
    const copied = await readStable(evidence.clock, evidence.path('package/' + expected.path));
    assert.deepEqual(copied.bytes, source.bytes);
  }
  for (const [path, value] of CANARIES)
    await evidence.put('package/' + path, Buffer.from(value));
  await verifyCheckout(evidence, checkout, originals);
  return originals;
}
/** @param {import('../bounded-runtime-custody.mjs').RuntimeCase} evidence
 * @param {URL} checkout
 * @param {Array<{path:string,source:Awaited<ReturnType<typeof readStable>>}>} originals */
export async function verifyCheckout(evidence, checkout, originals) {
  for (const original of originals) {
    const actual = await readStable(evidence.clock, new URL(original.path, checkout));
    assert.deepEqual(actual, original.source);
    const copied = await readStable(evidence.clock,
      evidence.path('package/' + original.path));
    assert.deepEqual(copied.bytes, original.source.bytes);
  }
}
