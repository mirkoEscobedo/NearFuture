// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {unlinkSync} from 'node:fs';
import {zeroProviderHistoryFixture} from './seed-zero-provider-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @param {any} f @param {boolean} [refresh] */
function refuse(f,refresh=true){
 if(refresh)f.refresh();const first=f.launcher();let rejected=false;
 try{readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch{rejected=true;}
 const ps=f.admit();assert.equal(ps.error,undefined);assert.ok(rejected,'JS must refuse');assert.notEqual(ps.status,0,'PS must refuse');
}
test('the fixed preclock evidence descriptor requires a scalar path',()=>{
 const f=zeroProviderHistoryFixture();try{
  const r=value(f.closeoutPath);r.evidence.preclockIndex.path=[r.evidence.preclockIndex.path];json(f.closeoutPath,r);
  const grant=value(f.grantPath);grant.previous.result=ref(f.closeoutPath);json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);f.refreshLaunch();refuse(f,false);
 }finally{f.close();}
});
for(const [name,key,field] of [['caller kind','caller','kind'],['campaign flow','phasePlan','campaign']])test('closed zero-provider '+name+' cannot be an array',()=>{
 const f=zeroProviderHistoryFixture();try{
  const e=f.evidencePaths,v=value(e[key]);if(field==='kind'){v.kind=[v.kind];json(e[key],v);}
  else{v.campaign.flow=[v.campaign.flow];json(e.phasePlan,v);const l=value(e.phaseLease);l.campaign=v.campaign;l.planSha256=ref(e.phasePlan).sha256;json(e.phaseLease,l);const s=value(e.source);s.planSha256=ref(e.phasePlan).sha256;s.leaseSha256=ref(e.phaseLease).sha256;json(e.source,s);}
  refuse(f);
 }finally{f.close();}
});
test('an array-shaped object cannot impersonate the initial process array',()=>{
 const f=zeroProviderHistoryFixture();try{
  const e=f.evidencePaths,v=value(e.initialOwned);v.processes={'0':v.processes[0],length:1};json(e.initialOwned,v);
  const s=value(e.source);s.ownedReceiptSha256=ref(e.initialOwned).sha256;json(e.source,s);refuse(f);
 }finally{f.close();}
});
for(const which of ['history','delegation'])for(const operation of ['changed','deleted'])test('direct original '+which+' bytes remain bound when '+operation,()=>{
 const f=zeroProviderHistoryFixture();try{
  const path=which==='history'?value(f.evidencePaths.plan).historyAdmission.path:resolve(f.priorScenario,'root-postlaunch-mechanical-delegation.private.json');
  if(operation==='deleted')unlinkSync(path);else json(path,{changed:true});refuse(f);
 }finally{f.close();}
});
test('the original lease cannot diverge from its immediate history descriptor',()=>{
 const f=zeroProviderHistoryFixture({divergentHistory:true});try{refuse(f);}finally{f.close();}
});
test('a JSON array cannot impersonate an evidence object through pipeline unwrapping',()=>{
 const f=zeroProviderHistoryFixture();try{const p=f.evidencePaths.ledger;json(p,[value(p)]);refuse(f);}finally{f.close();}
});
test('a JSON array cannot impersonate the closed failed-history receipt',()=>{
 const f=zeroProviderHistoryFixture();try{
  json(f.closeoutPath,[value(f.closeoutPath)]);const grant=value(f.grantPath);grant.previous.result=ref(f.closeoutPath);json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);f.refreshLaunch();refuse(f,false);
 }finally{f.close();}
});
for(const field of ['path','sha256'])test('the direct result descriptor requires a scalar '+field,()=>{
 const f=zeroProviderHistoryFixture();try{
  const grant=value(f.grantPath);grant.previous.result[field]=[grant.previous.result[field]];json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);f.refreshLaunch();refuse(f,false);
 }finally{f.close();}
});
test('the current history grant kind remains a scalar',()=>{
 const f=zeroProviderHistoryFixture();try{
  const grant=value(f.grantPath);grant.kind=[grant.kind];json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);f.refreshLaunch();refuse(f,false);
 }finally{f.close();}
});
