// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {zeroProviderHistoryFixture} from './seed-zero-provider-test-support.mjs';
import {json,write} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** Always exercise both real gates before asserting, including on RED. @param {any} f */
function refusedByBoth(f){
 f.refresh();const first=f.launcher();let rejected=false;
 try{readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch{rejected=true;}
 const ps=f.admit();assert.equal(ps.error,undefined);assert.ok(rejected,'JS must refuse');assert.notEqual(ps.status,0,'PS must refuse');
}
for(const name of ['guarded-http-ready.private.json','call-001.attempt.private.json','capture-001.png'])test('zero-provider history refuses activity in its actual B1 output: '+name,()=>{
 const f=zeroProviderHistoryFixture();try{write(resolve(f.priorOutput,name),'unexpected');refusedByBoth(f);}finally{f.close();}
});
test('zero-provider source batch must be a string in both real gates',()=>{
 const f=zeroProviderHistoryFixture();try{const path=f.evidencePaths.source,v=value(path);v.batch=['B1'];json(path,v);refusedByBoth(f);}finally{f.close();}
});
test('a coherently frozen caller cap remains a numeric scalar',()=>{
 const f=zeroProviderHistoryFixture();try{
  const e=f.evidencePaths,v=value(e.frozen);v.callerAdmissionDeadlineMs=String(v.callerAdmissionDeadlineMs);json(e.frozen,v);
  const z=value(e.freezeSupervisor),report=JSON.parse(z.raw.output);report.result.steps[0].stdout=JSON.stringify(v);z.raw.output=JSON.stringify(report);json(e.freezeSupervisor,z);
  refusedByBoth(f);
 }finally{f.close();}
});
test('a forged freeze command cannot borrow a matching PASS stdout',()=>{
 const f=zeroProviderHistoryFixture();try{
  const path=f.evidencePaths.freezeSupervisor,z=value(path),report=JSON.parse(z.raw.output);
  Object.assign(report.result.steps[0],{args:['-Command','Write-Output forged'],cwd:'foreign'});z.raw.output=JSON.stringify(report);json(path,z);refusedByBoth(f);
 }finally{f.close();}
});
test('embedded root-call JSON refuses a duplicate field even with equal values',()=>{
 const f=zeroProviderHistoryFixture();try{
  const path=f.evidencePaths.combined,x=value(path);x.stage.args.cmd=x.stage.args.cmd.replace('{"key":','{"key":"generated-zero-provider-root","key":');json(path,x);refusedByBoth(f);
 }finally{f.close();}
});
for(const [name,field,v] of [['future Java birth','creationTime','2026-10-27T08:52:39Z'],['foreign Java parent','parentPid',999]])
 test('coherently pinned snapshots refuse '+name,()=>{
  const f=zeroProviderHistoryFixture();try{
   const e=f.evidencePaths;for(const path of [e.owned,e.initialOwned]){const x=value(path);x.processes[0][field]=v;json(path,x);}
   const source=value(e.source);source.ownedReceiptSha256=ref(e.initialOwned).sha256;json(e.source,source);refusedByBoth(f);
  }finally{f.close();}
 });
