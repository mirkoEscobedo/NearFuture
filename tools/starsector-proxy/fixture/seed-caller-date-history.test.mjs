// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {callerDateHistoryFixture} from './seed-caller-date-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {value,ref} from './seed-failed-history-test-support.mjs';

test('a four-call caller date failure preserves zero input and spent context through both real history gates',()=>{
 const f=callerDateHistoryFixture();
 try{
  const next=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
  const ps=f.admit();assert.equal(ps.status,0,ps.stderr);if(error)throw error;
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:46,uiAttempts:43,reservedActions:76,sentActions:74});
  assert.equal(admitted.binding.campaign.maxReservedActions,84);
  const plan=value(f.evidencePaths.plan),initial=value(f.evidencePaths.initialOwned);
  assert.notEqual(plan.source.supportPins.path,f.evidencePaths.supportPins);
  assert.equal(plan.source.supportPins.sha256,ref(f.evidencePaths.supportPins).sha256);
  assert.deepEqual(Object.keys(initial).sort(),['at','processes','sourceSnapshot']);
  assert.notEqual(initial.sourceSnapshot.sha256,ref(f.evidencePaths.owned).sha256);
 }finally{f.close();}
});
