// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {captureStopHistoryFixture} from './seed-capture-stop-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {value} from './seed-failed-history-test-support.mjs';

test('a closed unreadable capture after known Play carries74/72 into cap82 through both real validators',()=>{
 const f=captureStopHistoryFixture();
 try{
  const b=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(caught){error=caught;}
  const launch=f.admit();assert.equal(launch.status,0,launch.stderr);if(error)throw error;
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:44,uiAttempts:41,reservedActions:74,sentActions:72});
  assert.equal(admitted.binding.campaign.maxReservedActions,82);
  assert.equal(value(f.evidencePaths.plan).absoluteMaxReservedActions,81);
  assert.deepEqual(value(f.evidencePaths.ledger).history,{overallAttempts:43,uiAttempts:40,reservedActions:74,sentActions:72});
  assert.equal(value(f.evidencePaths.result).failureCategory,'CAPTURE_UNCERTAIN');
 }finally{f.close();}
});
