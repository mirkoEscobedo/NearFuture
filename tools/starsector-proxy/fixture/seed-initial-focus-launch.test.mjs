// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('explicit control5/auth7 admits nine requests with eight clicks and one initial focus allowance through both launch gates',()=>{
    const f=initialFocusFixture();
    try{
        let limit;try{limit=seedCreationActionLimit(f.plan,f.launchLease,f.scenario);}catch(error){limit=error;}
        const b=f.launcher();
    /** @type {any} */let admitted;try{admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){admitted=error;}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({limit,psStatus:ps.status,psError:ps.status===0?'':ps.stderr},{limit:100,psStatus:0,psError:''});
        assert.ok(admitted&&'binding'in admitted,admitted?.message);assert.equal(admitted.binding.schemaVersion,7);
        assert.equal(admitted.binding.campaign.maxReservedActions,100);
        assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:53,uiAttempts:50,reservedActions:91,sentActions:89});
    }finally{f.close();}
});
