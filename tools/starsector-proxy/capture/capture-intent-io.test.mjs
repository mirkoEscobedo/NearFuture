// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync,writeFileSync,existsSync,unlinkSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { custodyFixture } from './capture-custody-fixture.mjs';
import { createCaptureIntentIO } from './capture-intent-io.mjs';
const sha=(/** @type {string|Buffer} */ v)=>createHash('sha256').update(v).digest('hex');
function fixture() {
    const f=custodyFixture('intent-publication-'), intentText=readFileSync(f.intentPath,'utf8');
    unlinkSync(f.intentPath);
    const pointerPath=resolve(f.output,'current-capture.private.json');
    writeFileSync(pointerPath,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_POINTER_V1',status:'UNSET',profileId:f.admitted.profileId})+'\n',{flag:'wx'});
    return {f,intentText,pointerPath,initialSHA:sha(readFileSync(pointerPath)),
        io:createCaptureIntentIO({admitted:f.admitted,pointerPath})};
}
test('one immutable exact intent and atomic current pointer select the admitted capture',()=>{
    const {f,io,intentText,initialSHA}=fixture();
    const proof=io.publish({intentText,expectedPointerSHA256:initialSHA});
    assert.equal(proof.kind,'NATIVE99_CAPTURE_INTENT_PUBLICATION_V1');
    assert.equal(proof.intentSHA256,sha(intentText));
    assert.equal(proof.intentPath,f.slot.intentPath);
    assert.deepEqual(io.readIntent(),{path:f.slot.intentPath,bytes:Buffer.from(intentText)});
    assert.throws(()=>io.publish({intentText,expectedPointerSHA256:proof.pointerSHA256}));
    assert.equal(readFileSync(f.intentPath,'utf8'),intentText);
});
test('missing/changed pointer and unknown intent refuse without creating intent',()=>{
    for(const mode of ['missing','changed','unknown']) {
        const {f,io,intentText,pointerPath,initialSHA}=fixture();
        if(mode==='missing') unlinkSync(pointerPath);
        if(mode==='changed') writeFileSync(pointerPath,'{}');
        const text=mode==='unknown'?JSON.stringify({...f.intent,frameId:'UNKNOWN'}):intentText;
        assert.throws(()=>io.publish({intentText:text,expectedPointerSHA256:initialSHA}));
        assert.equal(existsSync(f.intentPath),false);
    }
});
test('reader rejects pointer mismatch or changed immutable bytes',()=>{
    const {io,intentText,initialSHA,f,pointerPath}=fixture();
    io.publish({intentText,expectedPointerSHA256:initialSHA});
    writeFileSync(f.intentPath,intentText+' ');
    assert.throws(()=>io.readIntent());
    const pointer=JSON.parse(readFileSync(pointerPath,'utf8')); pointer.intentPath=resolve(f.output,'OTHER.json');
    writeFileSync(pointerPath,JSON.stringify(pointer));
    assert.throws(()=>io.readIntent());
});
