// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { registeredFiles } from '../../tools/runtime-manifest/runtime-package-registry.mjs';
import { PUBLIC_FILES } from './support/bounded-runtime/fixture.mjs';
import { GOLDEN } from './support/bounded-runtime/literals.mjs';

// Read-only drift alarm. Expectations remain independently selected static literals.
test('selected public bytes, catalog, independent fixture and golden stay consistent', async () => {
  const checkout = new URL('../../', import.meta.url);
  const paths = ['package.json', 'tools/runtime-manifest/metadata.mjs',
    'config/runtime-support.json'];
  const golden = JSON.parse(GOLDEN);
  assert.deepEqual(PUBLIC_FILES.map(file => file.path), paths, 'independent selection');
  assert.equal(registeredFiles.length, 3, 'production three-artifact scope');
  assert.equal(golden.artifacts.length, 3, 'golden three-artifact scope');
  let total = 0;
  for (const [ordinal, path] of paths.entries()) {
    const actual = await readFile(new URL(path, checkout));
    const digest = createHash('sha256').update(actual).digest('hex');
    const fixture = PUBLIC_FILES[ordinal];
    const catalog = registeredFiles[ordinal];
    const artifact = golden.artifacts[ordinal];
    assert.equal(actual.length, fixture.bytes, path + ': independent fixture byte count drift');
    assert.equal(digest, fixture.sha256, path + ': independent fixture digest drift');
    assert.equal(catalog.ordinal, ordinal, path + ': catalog ordinal');
    assert.equal(new URL(catalog.relative,
      new URL('tools/runtime-manifest/observe-runtime-package.mjs', checkout)).href,
    new URL(path, checkout).href, path + ': catalog selection drift');
    assert.deepEqual(Buffer.from(catalog.body), actual, path + ': embedded catalog body drift');
    assert.equal(catalog.bytes, actual.length, path + ': catalog byte count drift');
    assert.equal(catalog.sha256, digest, path + ': catalog digest drift');
    assert.equal(artifact.alias, 'artifact' + ordinal, path + ': golden selection');
    assert.equal(artifact.byte_length, actual.length, path + ': golden byte count drift');
    assert.equal(artifact.sha256, digest, path + ': golden digest drift');
    total += actual.length;
  }
  const observer = await readFile(new URL(
    'tools/runtime-manifest/observe-runtime-package.mjs', checkout), 'utf8');
  const bound = observer.match(/if \(total !== (\d+) \|\| total > (\d+)\) fail\('Bounds'\)/);
  assert.ok(bound, 'observer keeps explicit exact total and ceiling');
  assert.equal(total, Number(bound[1]), 'observer exact combined catalog byte count drift');
  assert.equal(Number(bound[2]), 65536, 'observer ceiling stays fixed');
  const behavior = await readFile(new URL('bounded-runtime-package.test.mjs', import.meta.url), 'utf8');
  const pin = behavior.match(/sha256\(Buffer\.from\(GOLDEN\)\),\s*'([a-f0-9]{64})'/);
  assert.ok(pin, 'behavior test keeps independently pinned golden digest');
  assert.equal(Buffer.byteLength(GOLDEN), 1603, 'independent golden byte count');
  assert.equal(createHash('sha256').update(GOLDEN).digest('hex'), pin[1],
    'behavior test golden digest drift');
});
