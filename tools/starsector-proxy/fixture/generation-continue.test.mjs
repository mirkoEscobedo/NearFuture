// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {withGenerationEntry,completeGeneratedNameEntry} from './generation-input-test-support.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';

test('known closed generated-name entry hands E2 exactly one first Continue and then observation only',async()=>{
    await withGenerationEntry('generation-first-continue',async(f,h,effects)=>{
        const next=await completeGeneratedNameEntry(f,h);
        const admitted=readBatchAuthorization(next.source,{root:f.root,runID:next.runID});
        assert.ok('e1GeneratedNameReceipt' in admitted.binding);
        assert.deepEqual(admitted.binding.e1GeneratedNameReceipt.continueCoordinate,[100,120]);
        claimBatchAuthorization(admitted);
        const e2=seedInputSession(next,tool=>effects.push(tool),{window:()=>({...seedTarget,windowId:21})});
        try {
            await e2.before();
            const args={target_window_id:21,target_app:'java.exe',focus_strategy:'strict',coordinate:[100,120]};
            assert.notEqual((await e2.call('left_click',args)).isError,true);
            await e2.before();
            assert.deepEqual(effects,['left_click','left_click','left_click']);
            assert.equal(f.ledger().history.reservedActions,4);assert.equal(f.ledger().history.sentActions,4);
            await assert.rejects(()=>e2.call('left_click',args),/remaining sequence/);
            assert.deepEqual(effects,['left_click','left_click','left_click']);
        } finally {await e2.embedded.server.close();}
    });
});
