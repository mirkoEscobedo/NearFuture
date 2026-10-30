// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {captureStopHistoryFixture} from './seed-capture-stop-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

for(const direction of ['before terminal','after protection'])test('capture-stop observation refuses '+direction+' through both readers',()=>{
 const f=captureStopHistoryFixture();
 try{
  const o=value(f.evidencePaths.observation);
  o.atUtc=direction==='before terminal'?'2000-01-01T00:00:00Z':new Date(Date.parse(value(f.evidencePaths.protection).at)+1000).toISOString();
  json(f.evidencePaths.observation,o);f.refresh();
  const b=f.launcher();let refused=false;
  try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch{refused=true;}
  const launch=f.admit();
  assert.deepEqual({jsRefused:refused,psRefused:launch.status!==0},{jsRefused:true,psRefused:true});
 }finally{f.close();}
});
