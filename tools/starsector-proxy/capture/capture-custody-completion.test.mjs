// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { custodyFixture, expectedImage, screenshotArgs } from './capture-custody-fixture.mjs';
import { openCaptureClient } from './capture-custody-test-client.mjs';
test('completion preserves the exact frozen intent bytes even when its source changes during native dispatch', { timeout: 10000 }, async () => {
    const f = custodyFixture(), original = readFileSync(f.intentPath);
    writeFileSync(resolve(f.output, 'original-intent-before-mutation.private.json'), original, { flag: 'wx' });
    const h = await openCaptureClient(f, { dispatch: async () => {
        const changed = JSON.parse(original.toString('utf8'));
        changed.window.pid = 999;
        changed.window.bounds.width = 1;
        writeFileSync(f.intentPath, JSON.stringify(changed) + '\n');
        return expectedImage;
    } });
    try {
        assert.deepEqual(await h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), expectedImage);
        const proof = JSON.parse(readFileSync(f.slot.callbackReceipt, 'utf8'));
        assert.equal(typeof proof.frozenIntentUtf8Base64, 'string', 'the completion must retain the exact original data handoff');
        assert.deepEqual(Buffer.from(proof.frozenIntentUtf8Base64, 'base64'), original);
        assert.equal(proof.intentSha256, createHash('sha256').update(original).digest('hex'));
        assert.equal(proof.width, 480);
        assert.equal(f.reads(), 1);
    } finally { await h.close(); }
});
