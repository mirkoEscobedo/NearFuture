// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {ref} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

test('known New Game wrong-scene failure carries both actions into cap84 through both real validators',()=>{
 const f=wrongSceneHistoryFixture();
 try{
  const b=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(caught){error=caught;}
  const launch=f.admit();assert.equal(launch.status,0,launch.stderr);if(error)throw error;
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:45,uiAttempts:42,reservedActions:76,sentActions:74});
  assert.equal(admitted.binding.campaign.maxReservedActions,84);
  assert.equal(value(f.evidencePaths.plan).absoluteMaxReservedActions,82);
  assert.equal(value(f.evidencePaths.result).failureCategory,'POSTCONDITION_FAILED');
  assert.equal(value(f.evidencePaths.ledger).pending.wizardStep.stage,'NEW_GAME');
 }finally{f.close();}
});

test('two acknowledged clicks cannot be refunded to one even with a coherent allowed future cap83',()=>{
 const f=wrongSceneHistoryFixture();
 try{
  Object.assign(f.wrongScene.guard,{reservedActions:75,sentActions:73});
  for(const [key,field]of [['ledger','history'],['result','actualGuardHistory'],['observation','actualGuardHistory']]){
   const p=f.evidencePaths[key],v=value(p);v[field]={...f.wrongScene.guard};json(p,v);
  }
  f.refresh();const grant=value(f.grantPath);grant.maxReservedActions=83;json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){v.absoluteMaxReservedActions=83;v.historyAdmission=ref(f.grantPath);}
  f.campaign.maxReservedActions=f.predecessor.maxReservedActions=83;f.refreshLaunch();
  assertCaptureRefused(f);
 }finally{f.close();}
});
