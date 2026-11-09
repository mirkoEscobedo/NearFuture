// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationFixture,creationFlow} from './seed-creation-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {readFixtureBinding} from '../fixture-binding.mjs';

test('explicit seed creation extends the carried allowance through both real launch validators without resetting history',()=>{
    const f=seedCreationFixture();
    try {
        const launch=f.admit();assert.equal(launch.status,0,launch.stderr);
        const first=f.launcher(),admitted=readBatchAuthorization(first.source,{root:f.root,runID:first.runID});
        assert.equal(admitted.binding.schemaVersion,6);
        const binding=admitted.binding;assert.ok('campaign' in binding);
        assert.equal(binding.campaign.flow,creationFlow);
        assert.deepEqual(binding.campaign.historyBaseline,
            {overallAttempts:40,uiAttempts:37,reservedActions:72,sentActions:70});
        assert.equal(binding.campaign.maxReservedActions,80);
        assert.deepEqual(readFixtureBinding(admitted),{path:f.bindingPath,sha256:f.bindingPin.sha256.toLowerCase()});
    } finally {f.close();}
});
