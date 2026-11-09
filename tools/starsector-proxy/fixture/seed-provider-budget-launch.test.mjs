// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {providerBudgetFixture} from './seed-provider-budget-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('paired eight-minute provider selection admits the unchanged nine-request root through both real launch readers',()=>{
    const f=providerBudgetFixture();
    try{
        /** @type {any} */let limit;
        /** @type {any} */let admitted;
        try{limit=seedCreationActionLimit(f.plan,f.launchLease,f.scenario);}catch(error){limit=error;}
        const b=f.launcher();
        try{admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){admitted=error;}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({limit,psStatus:ps.status,psError:ps.status===0?'':ps.stderr},
            {limit:100,psStatus:0,psError:''});
        assert.ok(admitted&&'binding'in admitted,admitted?.message);
        assert.equal(admitted.binding.schemaVersion,7);
        assert.equal(admitted.binding.providerTimeoutMs,480000);
        assert.equal(admitted.binding.campaign.maxReservedActions,100);
        assert.equal(admitted.binding.campaign.rootClock.sessionWatchdogMs,1800000);
    }finally{f.close();}
});
