// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {snapshotBusyHistoryFixture} from './seed-snapshot-busy-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {value} from './seed-failed-history-test-support.mjs';

test('the exact four-call snapshot-busy stop carries zero-input history through both real gates',()=>{
 const f=snapshotBusyHistoryFixture();
 try{
  const next=f.launcher();let admitted,error;
  try{admitted=readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
  const ps=f.admit();assert.deepEqual([error===undefined,ps.status],[true,0],String(error)+'\n'+ps.stderr);
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:46,uiAttempts:43,reservedActions:76,sentActions:74});
  assert.equal(admitted.binding.campaign.maxReservedActions,84);
  assert.deepEqual(value(f.evidencePaths.point).localPoint,{x:0,y:0});
  assert.deepEqual(value(f.evidencePaths.point).coordinate,[1,1]);
 }finally{f.close();}
});
