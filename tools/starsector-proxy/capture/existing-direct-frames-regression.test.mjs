import { makeOutput, testOutputRoot } from './test-output.mjs';
// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, existsSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { persistDirectFrame } from '../direct-frames.mjs';
import { syntheticPng as png } from '../direct-test-fixture.mjs';
const root = testOutputRoot;
function fixture() { mkdirSync(root, { recursive: true }); const path = mkdtempSync(resolve(root, 'direct-frame-'));
    return { path, close: () => { const suffix = relative(root, path); assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix)); rmSync(path, { recursive: true }); } }; }
test('original result and PNG are saved without edits before returning capture custody', () => {
    const f = fixture();
    try {
        const result = { content: [{ type: 'image', mimeType: 'image/png', data: png }], _meta: { exact: true } };
        const record = persistDirectFrame({ output: f.path, label: 'capture-001', result,
            args: { target_window_id: 19, width: 1, quality: 0 }, window: { pid: 101, windowId: 19, bounds: { width: 1, height: 1 } },
            ownedPids: [101], provenance: { outerProtocolID: 4, innerProtocolID: 6, freshness: 'UNKNOWN' } });
        assert.ok(record.pngPath);
        assert.deepEqual(readFileSync(record.pngPath), Buffer.from(png, 'base64'));
        assert.deepEqual(JSON.parse(readFileSync(record.rawPath, 'utf8')), result);
        assert.equal(JSON.parse(readFileSync(record.metadataPath, 'utf8')).freshness, 'UNKNOWN');
        assert.throws(() => persistDirectFrame({ output: f.path, label: 'capture-001', result, args: {}, window: {}, ownedPids: [], provenance: {} }), /EEXIST/);
    } finally { f.close(); }
});
test('invalid capture retains full raw response and refuses PNG success', () => {
    const f = fixture();
    try {
        assert.throws(() => persistDirectFrame({ output: f.path, label: 'capture-002', result: { content: [{ type: 'image', mimeType: 'image/png', data: png }] },
            args: { target_window_id: 19, width: 2, quality: 0 }, window: { pid: 101, windowId: 19, bounds: { width: 2, height: 1 } }, ownedPids: [101], provenance: {} }), /dimensions/);
        assert.ok(existsSync(resolve(f.path, 'capture-002.raw.private.json')));
        assert.equal(existsSync(resolve(f.path, 'capture-002.png')), false);
    } finally { f.close(); }
});
test('a valid 24-byte PNG header prefix is not a complete PNG capture', () => {
    const f = fixture();
    try {
        const prefix = Buffer.from(png, 'base64').subarray(0, 24).toString('base64');
        assert.throws(() => persistDirectFrame({ output: f.path, label: 'capture-003', result: { content: [{ type: 'image', mimeType: 'image/png', data: prefix }] },
            args: { target_window_id: 19, width: 1, quality: 0 }, window: { pid: 101, windowId: 19, bounds: { width: 1, height: 1 } }, ownedPids: [101], provenance: {} }), /Complete original PNG/);
        assert.ok(existsSync(resolve(f.path, 'capture-003.raw.private.json')));
        assert.equal(existsSync(resolve(f.path, 'capture-003.png')), false);
    } finally { f.close(); }
});
test('a complete-looking PNG with an invalid chunk CRC is rejected with raw evidence retained', () => {
    const f = fixture();
    try {
        const invalid = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=';
        assert.throws(() => persistDirectFrame({ output: f.path, label: 'capture-004', result: { content: [{ type: 'image', mimeType: 'image/png', data: invalid }] },
            args: { target_window_id: 19, width: 1, quality: 0 }, window: { pid: 101, windowId: 19, bounds: { width: 1, height: 1 } }, ownedPids: [101], provenance: {} }), /CRC mismatch/);
        assert.ok(existsSync(resolve(f.path, 'capture-004.raw.private.json'))); assert.equal(existsSync(resolve(f.path, 'capture-004.png')), false);
    } finally { f.close(); }
});
