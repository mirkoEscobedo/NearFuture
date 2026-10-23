import { makeOutput, testOutputRoot } from './test-output.mjs';
// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, readFileSync, existsSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { persistDirectFrame } from '../direct-frames.mjs';
const fixture = readFileSync(new URL('./fixtures/readiness.synthetic.png', import.meta.url));
const result = { content: [{ type: 'image', mimeType: 'image/png', data: fixture.toString('base64') }] };
const args = { target_window_id: 19, width: 480, quality: 0 };
const window = { pid: 101, windowId: 19, bounds: { width: 480, height: 220 } };
test('an explicitly declared PNG basename retains the exact original at the existing scenario path', () => {
    const output = makeOutput('declared-path-');
    const receipt = persistDirectFrame({ output, label: 'capture-001', pngName: 'F1.original.png', result, args,
        window, ownedPids: [101], provenance: { native: false, kind: 'SYNTHETIC_ONLY' } });
    assert.equal(receipt.pngPath, resolve(output, 'F1.original.png'));
    assert.deepEqual(readFileSync(resolve(output, 'F1.original.png')), fixture);
    assert.equal(existsSync(resolve(output, 'capture-001.png')), false, 'one original PNG writer, no hidden rename or duplicate');
    assert.deepEqual(JSON.parse(readFileSync(receipt.rawPath, 'utf8')), result);
    const metadata = JSON.parse(readFileSync(receipt.metadataPath, 'utf8'));
    assert.equal(metadata.pngPath, resolve(output, 'F1.original.png'));
    assert.equal(metadata.captureReceipt, true);
    assert.equal(metadata.freshness, 'UNKNOWN');
});

test('declared basenames refuse paths and unknown names before creating any artifact', () => {
    for (const pngName of ['', '../escape.png', 'F1.original.png/other', 'C:\\temp\\escape.png',
        'CON.png', 'F0.original.png', 'F1000.original.png', 'F1.original.png.']) {
        const output = makeOutput('rejected-name-');
        assert.throws(() => persistDirectFrame({ output, label: 'capture-001', pngName, result, args,
            window, ownedPids: [101], provenance: { native: false } }), /Declared original frame basename required/);
        assert.deepEqual(readdirSync(output), []);
    }
});
test('a repeated declared PNG refuses replacement and retains the second raw result', () => {
    const output = makeOutput('duplicate-name-');
    const options = { output, pngName: 'F1.original.png', result, args, window, ownedPids: [101], provenance: { native: false } };
    persistDirectFrame({ ...options, label: 'capture-001' });
    assert.throws(() => persistDirectFrame({ ...options, label: 'capture-002' }), /EEXIST/);
    assert.deepEqual(readFileSync(resolve(output, 'F1.original.png')), fixture);
    assert.deepEqual(JSON.parse(readFileSync(resolve(output, 'capture-002.raw.private.json'), 'utf8')), result);
    assert.equal(existsSync(resolve(output, 'capture-002.metadata.private.json')), false);
});

test('untrusted declared names reject trailing newlines and nonstrings before any artifact', () => {
    for (const pngName of ['F1.original.png\n', 'F1.original.png\r', 'F1.original.png\r\n',
        ['F1.original.png'], 19, null, true, {}]) {
        const output = makeOutput('untrusted-name-');
        assert.throws(() => persistDirectFrame({ output, label: 'capture-001', pngName: /** @type {any} */ (pngName),
            result, args, window, ownedPids: [101], provenance: { native: false } }), /Declared original frame basename required/);
        assert.deepEqual(readdirSync(output), []);
    }
});
