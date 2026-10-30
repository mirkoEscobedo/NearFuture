// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher} from './campaign-input-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('a staged modern seed reaches both existing immutable seeded-entry launch validators',async()=>{
    const f=await modernSeedFixture();
    try {
        const stage=f.target.stage();assert.equal(stage.status,0,stage.stderr);
        const launched=campaignLaunchFixture({preparedSeed:f.target,staged:true,
            historyBaseline:{overallAttempts:41,uiAttempts:38,reservedActions:80,sentActions:78}});
        const admission=launched.admit();assert.equal(admission.status,0,admission.stderr);
        const b1=claimSeededLauncher(launched,'modern-seed-launch',false);
        const result=readBatchAuthorization(b1.source,{root:launched.root,runID:b1.runID});
        assert.equal(result.binding.schemaVersion,3);assert.ok('campaign' in result.binding);
        assert.equal(result.binding.campaign.maxReservedActions,88);
        assert.equal(result.binding.campaign.historyBaseline.reservedActions,80);
    } finally {f.close();}
});
