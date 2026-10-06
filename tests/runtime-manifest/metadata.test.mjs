import test from 'node:test';
import assert from 'node:assert/strict';
import { parseGameJson } from '../../tools/runtime-manifest/metadata.mjs';
test('installed mod JSON accepts single quotes without changing hashes or comment-like string data', () => {
  assert.deepEqual(parseGameJson('{# heading\n"version":{"major":\'1\',"minor":\'5\',"patch":\'6\'}, "url":"https://host/#anchor",}', 'metadata'),
    { version: { major: '1', minor: '5', patch: '6' }, url: 'https://host/#anchor' });
});
import { safeJvmFlags } from '../../tools/runtime-manifest/metadata.mjs';
test('arbitrary JVM string options and agent secrets never enter public flags', () => {
  assert.deepEqual(safeJvmFlags('-Xmx4g -XX:PrivateKey=secret -XX:ShenandoahGCMode=iu -agentlib:jdwp=token -Dtoken=secret'),
    ['-Xmx4g', '-XX:ShenandoahGCMode=iu']);
});
