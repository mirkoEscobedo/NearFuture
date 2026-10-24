// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,unlinkSync} from 'node:fs';
import {inspectGenerate,withGenerationEntry} from './generation-input-test-support.mjs';
import {json} from './seed-test-support.mjs';

/** @type {Array<[string,(proof:any)=>void]>} */
const cases=[
    ['missing inspection',proof=>unlinkSync(proof.path)],
    ['changed inspected scene',proof=>{proof.receipt.inspection.state='CAMPAIGN';json(proof.path,proof.receipt);}],
    ['wrong control label',proof=>{proof.receipt.inspection.generateLabel='CONTINUE';json(proof.path,proof.receipt);}],
    ['nonblank name',proof=>{proof.receipt.inspection.nameBlank=false;json(proof.path,proof.receipt);}],
    ['changed reservation',proof=>{proof.receipt.reservationID+='changed';json(proof.path,proof.receipt);}],
    ['changed original PNG',proof=>writeFileSync(proof.receipt.frame.pngPath,Buffer.concat([
        readFileSync(proof.receipt.frame.pngPath),Buffer.from('changed')]))],
];
for(const [label,change] of cases)test('Generate refuses '+label+' before native dispatch',async()=>{
    await withGenerationEntry('generation-refusal',async(f,h,effects)=>{
        const proof=await inspectGenerate(f,h);change(proof);
        await assert.rejects(()=>h.call('left_click',{target_window_id:21,target_app:'java.exe',
            focus_strategy:'strict',coordinate:[70,90]}));
        assert.deepEqual(effects,['left_click']);assert.equal(f.ledger().history.sentActions,2);
        assert.equal(f.ledger().uncertain,true);
    });
});

test('Generate refuses a different in-window coordinate despite a valid scene proof',async()=>{
    await withGenerationEntry('generation-coordinate-refusal',async(f,h,effects)=>{
        await inspectGenerate(f,h);
        await assert.rejects(()=>h.call('left_click',{target_window_id:21,target_app:'java.exe',
            focus_strategy:'strict',coordinate:[71,90]}),/inspected Generate coordinate/);
        assert.deepEqual(effects,['left_click']);
    });
});
