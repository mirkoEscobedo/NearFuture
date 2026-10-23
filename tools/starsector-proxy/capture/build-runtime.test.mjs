// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { buildCaptureRuntime } from './build-runtime.mjs';
const sha = (/** @type {Buffer} */ b) => createHash('sha256').update(b).digest('hex');
test('pinned vendor runtime builds the qualified awaited-result hook without changing the installed server', () => {
 const installed = new URL('../../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/server.js', import.meta.url);
 const before = sha(readFileSync(installed));
 const result = buildCaptureRuntime();
 assert.equal(sha(readFileSync(resolve(result, 'dist/server.js'))), '70aee012d22d311656f55dbc69f1287def308e093b0563efa5d124e607aaa180');
 assert.equal(sha(readFileSync(resolve(result, 'dist/server.d.ts'))), 'c04337dbd9235da661c9126d041feec5d7b73ce78b084b7554d5cd8bf739b68c');
 assert.equal(sha(readFileSync(installed)), before);
 assert.equal(buildCaptureRuntime(), result, 'existing cache is verified, never overwritten');
});
