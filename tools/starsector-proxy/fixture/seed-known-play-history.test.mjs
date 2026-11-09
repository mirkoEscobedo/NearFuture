// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {knownPlayHistoryFixture} from './seed-known-play-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('a closed known Play failure carries its spent action into both validators and a fresh eight-click allowance',()=>{
 const f=knownPlayHistoryFixture();
 try{
  const first=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch(caught){error=caught;}
  const launch=f.admit();assert.equal(launch.status,0,launch.stderr);
  if(error)throw error;
  assert.ok(admitted);
  assert.ok('campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:41,uiAttempts:38,reservedActions:73,sentActions:71});
  assert.equal(admitted.binding.campaign.maxReservedActions,81);
 }finally{f.close();}
});

test('known Play with explicit Java targeting preserves the same spent action and bounded grant',()=>{
 const f=knownPlayHistoryFixture({targetApp:'java.exe'});
 try{
  const b=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(caught){error=caught;}
  const launch=f.admit();assert.equal(launch.status,0,launch.stderr);if(error)throw error;
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:41,uiAttempts:38,reservedActions:73,sentActions:71});
  assert.equal(admitted.binding.campaign.maxReservedActions,81);
 }finally{f.close();}
});
