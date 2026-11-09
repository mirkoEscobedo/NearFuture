// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {zeroProviderHistoryFixture} from './seed-zero-provider-test-support.mjs';
import {powershell,quote} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('a zero-provider controller-clock refusal preserves its consumed context through both real public validators',()=>{
 const f=zeroProviderHistoryFixture();
 try{
  const first=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch(cause){error=cause;}
  const launch=powershell(['-Command',"& "+quote(resolve(f.root,'tools/starsector-proxy/fixture/protection-check.ps1'))+' -BindingPath '+quote(f.bindingPath)+' -BindingSha256 '+quote(f.bindingPin.sha256)+' -Phase admit-launch -EvidenceRoot '+quote(f.scenario)]);
  assert.equal(launch.status,0,launch.stderr);if(error)throw error;
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:47,uiAttempts:44,reservedActions:76,sentActions:74});
  assert.equal(admitted.binding.campaign.maxReservedActions,84);
 }finally{f.close();}
});
