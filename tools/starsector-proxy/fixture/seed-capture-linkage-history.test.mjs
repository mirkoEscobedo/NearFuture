// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {captureLinkageHistoryFixture} from './seed-capture-linkage-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('a focused four-call pre-input capture linkage failure carries context through both real public validators',()=>{
 const f=captureLinkageHistoryFixture();
 try{
  const first=f.launcher();let admitted,error;
  // Always exercise PS as well, even when JS still rejects the new historical kind.
  try{admitted=readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch(cause){error=cause;}
  const launch=f.admit();assert.equal(launch.error,undefined);assert.equal(launch.status,0,launch.stderr);
  if(error)throw error;
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,
   {overallAttempts:46,uiAttempts:43,reservedActions:76,sentActions:74});
  assert.equal(admitted.binding.campaign.maxReservedActions,84);
  assert.equal(value(f.evidencePaths.plan).absoluteMaxReservedActions,84);
  assert.deepEqual(value(f.evidencePaths.ledger).history,
   {overallAttempts:45,uiAttempts:42,reservedActions:76,sentActions:74});
  assert.equal(value(f.evidencePaths.ledger).pending,null);
  assert.equal(value(f.evidencePaths.result).counts.vendorCalls,4);
  assert.equal(value(f.evidencePaths.result).counts.reservedThisRun,0);
  assert.equal(value(f.evidencePaths.result).failureCategory,'PRIVATE_CAPTURE_LINKAGE_REFUSAL_BEFORE_INPUT');
  assert.notEqual(f.original.raw.path,f.calls[3].raw.path);
  assert.equal(f.original.raw.sha256,f.calls[3].raw.sha256);
  assert.equal(value(f.original.metadata.path).rawPath,f.original.raw.path);
  assert.equal(value(f.evidencePaths.observation).helperInvocation.timingFailure,false);
  assert.equal(value(f.evidencePaths.result).seed.created,false);
 }finally{f.close();}
});
