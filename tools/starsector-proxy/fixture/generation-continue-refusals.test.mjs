// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,unlinkSync,readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {withGenerationEntry,completeGeneratedNameEntry} from './generation-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';

/** @type {Array<[string,(f:any,next:any)=>void]>} */
const cases=[
    ['blank generated name',(_,n)=>{n.receipt.inspection.generatedName=' ';}],
    ['a later Continue label',(_,n)=>{n.receipt.inspection.continueLabel='2. Continue';}],
    ['wrong scene',(_,n)=>{n.receipt.inspection.state='CAMPAIGN';}],
    ['ordinary Exit coordinate field',(_,n)=>{n.receipt.inspection.exitCoordinate=[100,120];delete n.receipt.inspection.continueCoordinate;}],
    ['unknown provider closure',(_,n)=>{n.receipt.provider.providerReaped=false;}],
    ['wrong Generate reservation',(_,n)=>{n.receipt.e1.generateReservationID+='changed';}],
    ['missing generated-name original',(_,n)=>{unlinkSync(n.receipt.frame.pngPath);}],
    ['changed original New Game source',(f)=>{writeFileSync(f.source,readFileSync(f.source,'utf8')+' ');}],
    ['changed original B1 claim',(f)=>{const p=resolve(f.scenario,'guard-B1-consumed.private.json');writeFileSync(p,readFileSync(p,'utf8')+' ');}],
];
for(const [label,change] of cases)test('first Continue refuses '+label+' before E2 claim',async()=>{
    await withGenerationEntry('generation-e2-refusal',async(f,h,effects)=>{
        const next=await completeGeneratedNameEntry(f,h);change(f,next);
        json(next.receiptPath,next.receipt);
        for(const path of [next.planPath,next.leasePath]){
            const value=JSON.parse(readFileSync(path,'utf8'));
            value.e1GeneratedNameReceipt.sha256=filePin(next.receiptPath).sha256.toLowerCase();
            if(path===next.leasePath)value.planSha256=filePin(next.planPath).sha256;
            json(path,value);
        }
        const auth=JSON.parse(readFileSync(next.source,'utf8'));
        auth.planSha256=filePin(next.planPath).sha256;auth.leaseSha256=filePin(next.leasePath).sha256;json(next.source,auth);
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
        assert.equal(existsSync(resolve(f.scenario,'guard-E2-consumed.private.json')),false);
        assert.deepEqual(effects,['left_click','left_click']);assert.equal(f.ledger().history.sentActions,3);
    });
});

test('first Continue refuses an uninspected coordinate without dispatch or a second provider claim',async()=>{
    await withGenerationEntry('generation-e2-coordinate',async(f,h,effects)=>{
        const next=await completeGeneratedNameEntry(f,h);
        const admitted=readBatchAuthorization(next.source,{root:f.root,runID:next.runID});
        claimBatchAuthorization(admitted);
        assert.throws(()=>claimBatchAuthorization(admitted));
        const e2=seedInputSession(next,tool=>effects.push(tool),{window:()=>({...seedTarget,windowId:21})});
        try {
            await e2.before();
            await assert.rejects(()=>e2.call('left_click',{target_window_id:21,target_app:'java.exe',
                focus_strategy:'strict',coordinate:[101,120]}),/inspected first Continue/);
            assert.deepEqual(effects,['left_click','left_click']);assert.equal(f.ledger().history.sentActions,3);
        } finally {await e2.embedded.server.close();}
    });
});
