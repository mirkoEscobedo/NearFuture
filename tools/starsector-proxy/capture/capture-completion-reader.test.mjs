// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, writeFileSync, copyFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { custodyFixture, expectedImage, screenshotArgs } from './capture-custody-fixture.mjs';
import { openCaptureClient } from './capture-custody-test-client.mjs';
import { readCaptureCompletion } from './capture-completion-reader.mjs';
const digest = (/** @type {string} */ text) => createHash('sha256').update(text).digest('hex');
async function completed() {
    const f = custodyFixture('completion-consumer-');
    const result = { content: [...expectedImage.content, { type:'text', text:'480x220' }] };
    const c = await openCaptureClient(f,{dispatch:async()=>result});
    try { assert.deepEqual(await c.client.callTool({name:'screenshot',arguments:screenshotArgs}),result); }
    finally { await c.close(); }
    const expected = { intent: f.intent, intentText:readFileSync(f.intentPath,'utf8'), slot:f.slot,
        inlineImageSHA256:digest(expectedImage.content[0].data), inlineImageCharacters:expectedImage.content[0].data.length,
        nonImageContent:[{type:'text',text:'480x220'}], captureOrdinal:1, captureStartMs:1000000, captureEndMs:1000000 };
    return {f,expected};
}
test('completion consumer binds current intent and actual inline image to original artifacts without returning base64', async()=>{
    const {f,expected}=await completed(), proof=readCaptureCompletion(expected);
    assert.equal(proof.kind,'NATIVE99_CAPTURE_COMPLETION_READBACK_V1');
    assert.equal(proof.png,f.slot.png); assert.equal(proof.width,480); assert.equal(proof.height,220);
    assert.equal(proof.inlineImageDigestMatched,true); assert.equal(proof.freshness,'UNKNOWN');
    assert.equal(JSON.stringify(proof).includes(expectedImage.content[0].data),false);
});
for (const part of /** @type {const} */ (['png','payload','callbackReceipt'])) test('refuse changed '+part,async()=>{
    const {f,expected}=await completed();
    writeFileSync(f.slot[part],readFileSync(f.slot[part]).subarray(1));
    assert.throws(()=>readCaptureCompletion(expected));
});
test('refuse another caller image, text, frozen intent, or capture clock',async()=>{
    const {expected}=await completed();
    for(const change of [
        {inlineImageSHA256:'0'.repeat(64)}, {nonImageContent:[]},
        {intentText:expected.intentText+' '}, {captureStartMs:1000001},
        {intent:{...expected.intent, frameId:'OTHER'}}]) {
        assert.throws(()=>readCaptureCompletion({...expected,...change}));
    }
});
test('refuse a different raw ordinal even with the same original bytes and recomputed completion paths',async()=>{
    const {f,expected}=await completed(),proof=JSON.parse(readFileSync(f.slot.callbackReceipt,'utf8'));
    const otherRaw=proof.rawPath.replace('capture-001','capture-002');
    const otherMetadata=proof.metadataPath.replace('capture-001','capture-002');
    copyFileSync(proof.rawPath,otherRaw); copyFileSync(proof.metadataPath,otherMetadata);
    writeFileSync(f.slot.callbackReceipt,JSON.stringify({...proof,rawPath:otherRaw,metadataPath:otherMetadata})+'\n');
    assert.throws(()=>readCaptureCompletion(expected),/EXACT_CALLBACK_ARTIFACT_PATHS_REQUIRED/);
});
