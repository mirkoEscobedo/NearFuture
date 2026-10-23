// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { writeFileSync, readFileSync } from 'node:fs';
import { openCaptureClient } from './capture-custody-test-client.mjs';
import { custodyFixture, expectedImage, screenshotArgs } from './capture-custody-fixture.mjs';
test('full canonical frame identity remains separate from its declared short PNG basename', { timeout: 10000 }, async () => {
    const f = custodyFixture();
    const canonical = 'NATIVE99_SYNTHETIC_CASE_A_B1_F1';
    f.slot.id = canonical;
    f.intent.frameId = canonical;
    writeFileSync(f.intentPath, JSON.stringify(f.intent) + '\n');
    const h = await openCaptureClient(f, { dispatch: async () => expectedImage });
    try {
        assert.deepEqual(await h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), expectedImage);
        const proof = JSON.parse(readFileSync(f.slot.callbackReceipt, 'utf8'));
        assert.equal(proof.frameId, canonical);
        assert.equal(proof.png, f.slot.png);
        assert.match(proof.png, /F1\.original\.png$/);
    } finally { await h.close(); }
});
