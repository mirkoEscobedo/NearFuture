// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {failedSeedHistoryFixture} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('a closed no-input focus failure carries actual attempt context through both real validators',()=>{
 const f=failedSeedHistoryFixture();
 try{
  const first=f.launcher(),admitted=readBatchAuthorization(first.source,{root:f.root,runID:first.runID});
  const launch=f.admit();assert.equal(launch.status,0,launch.stderr);
  assert.ok('campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:41,uiAttempts:38,reservedActions:72,sentActions:70});
  assert.equal(admitted.binding.campaign.maxReservedActions,80);
 }finally{f.close();}
});
