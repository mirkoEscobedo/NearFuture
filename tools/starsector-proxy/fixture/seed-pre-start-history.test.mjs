// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {readPreStartMarginFailedSeedAttempt} from '../seed-pre-start-margin-attempt.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

test('closed seven-click pre-Start failure carries spent actions and the fresh eight-click cap through both real gates',()=>{
    const f=preStartHistoryFixture();
    try {
        const actions=readPreStartMarginFailedSeedAttempt(f.closeout,value(f.ledgerPath),ref(f.ledgerPath));
        assert.deepEqual(actions.actualGuard,{overallAttempts:48,uiAttempts:45,reservedActions:83,sentActions:81});
        const b=f.launcher(),admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});
        const ps=f.admit();assert.equal(ps.status,0,ps.stderr);
        assert.ok('campaign' in admitted.binding);
        assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:49,uiAttempts:46,reservedActions:83,sentActions:81});
        assert.equal(admitted.binding.campaign.maxReservedActions,91);
        assert.equal(value(f.resultPath).counts.StartGameInputs,0);
        assert.equal(value(f.resultPath).verdict,'FAIL');
    }finally{f.close();}
});
