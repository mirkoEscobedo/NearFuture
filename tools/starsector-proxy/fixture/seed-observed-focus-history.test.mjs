// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {observedFocusHistoryFixture} from './seed-observed-focus-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';

test('the immutable capture-first no-input focus stop carries truthful counters through both real gates',()=>{
 const f=observedFocusHistoryFixture();
 try{
  const first=f.launcher();let admitted,nodeError;
  try{admitted=readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch(error){nodeError=error;}
  const launch=f.admit();
  assert.deepEqual([nodeError===undefined,launch.status],[true,0],String(nodeError)+'\n'+launch.stderr);
  assert.ok(admitted&&'campaign' in admitted.binding);
  assert.deepEqual(admitted.binding.campaign.historyBaseline,{overallAttempts:41,uiAttempts:38,reservedActions:72,sentActions:70});
  assert.equal(admitted.binding.campaign.maxReservedActions,80);
 }finally{f.close();}
});

/** @param {{refresh:()=>void,root:string,launcher:()=>{source:string,runID:string},admit:()=>{status:number|null,stderr:string}}} f */
function refuse(f){
 f.refresh();const first=f.launcher();let nodeRefused=false;
 try{readBatchAuthorization(first.source,{root:f.root,runID:first.runID});}catch{nodeRefused=true;}
 const launch=f.admit();assert.deepEqual([nodeRefused,launch.status],[true,1],launch.stderr);
}

/** @param {{evidencePaths:{result:string}}} f */
function refreshCallPins(f){
 const r=value(f.evidencePaths.result);r.calls=r.calls.map((/** @type {any} */ row)=>Object.fromEntries(
  Object.entries(row).map(([key,p])=>[key,ref(/** @type {{path:string}} */(p).path)])));
 json(f.evidencePaths.result,r);
}

for(const order of [
 ['doctor','policy_status','get_window','get_frontmost_app','screenshot'],
 ['doctor','get_window','policy_status','screenshot','get_frontmost_app'],
])test('the observed representation refuses another call order: '+order.join('/'),()=>{
 const f=observedFocusHistoryFixture({order});try{refuse(f);}finally{f.close();}
});

const mutations={
 'an unknown raw observation field':(/** @type {any} */ o)=>{o.unknown=true;},
 'an image-count array':(/** @type {any} */ o)=>{o.originalImages=[1];},
 'a changed observation kind':(/** @type {any} */ o)=>{o.kind='ACTUAL_DRIVER_NO_INPUT_FOCUS_STOP';},
 'a changed caller session':(/** @type {any} */ o)=>{o.originalCallerSession++;},
 'a rewritten root-protection snapshot':(/** @type {any} */ o)=>{o.rootProtection='PASS';},
 'a different raw foreground':(/** @type {any} */ o)=>{o.foreground.raw.structuredContent.app.pid++;},
 'an altered vendor argument':(/** @type {any} */ o)=>{o.actualVendorCalls[3].args.width++;},
 'a changed doctor caller option':(/** @type {any} */ o)=>{o.actualVendorCalls[0].args={include_remediation:false};},
 'a return timestamp before completion':(/** @type {any} */ o)=>{o.actualVendorCalls[0].atMs=0;},
 'a duplicate terminal text field':(/** @type {any} */ o)=>{
  o.providerFinish.raw.content[0].text=o.providerFinish.raw.content[0].text.replace('"verdict":"PASS"','"verdict":"PASS","verdict":"PASS"');
 },
};
for(const [name,mutate]of Object.entries(mutations))test('observed focus history refuses '+name+' in both gates',()=>{
 const f=observedFocusHistoryFixture();try{const o=value(f.observedPath);mutate(o);json(f.observedPath,o);refuse(f);}finally{f.close();}
});

for(const [name,mutate]of Object.entries({
 'a seed claim':(/** @type {any} */ r)=>{r.claims.seed=true;},
 'an ordinary-exit claim':(/** @type {any} */ r)=>{r.closure.ordinaryExit=true;},
 'an unknown provider verifier':(/** @type {any} */ r)=>{r.closure.provider.verifierPid++;},
 'a clock renewal claim':(/** @type {any} */ r)=>{r.timing.clockRenewal=true;},
 'a missing owned PID':(/** @type {any} */ r)=>{r.closure.knownPidsAbsent.pop();},
 'an extra result field':(/** @type {any} */ r)=>{r.extra=true;},
}))test('observed focus history refuses '+name+' in both gates',()=>{
 const f=observedFocusHistoryFixture();try{const r=value(f.evidencePaths.result);mutate(r);json(f.evidencePaths.result,r);refuse(f);}finally{f.close();}
});

test('the observed focus record refuses an arbitrary evidence path',()=>{
 const f=observedFocusHistoryFixture();try{
  const path=resolve(f.priorScenario,'other-stop.private.json');writeFileSync(path,readFileSync(f.observedPath));
  f.evidencePaths.observation=path;refuse(f);
 }finally{f.close();}
});

test('the new case refuses duplicate raw keys after refreshing the outer hashes',()=>{
 const f=observedFocusHistoryFixture();try{
  writeFileSync(f.observedPath,readFileSync(f.observedPath,'utf8').replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'));refuse(f);
 }finally{f.close();}
});

test('the new case refuses duplicate pinned call keys after refreshing their hashes',()=>{
 const f=observedFocusHistoryFixture();try{
  const call=value(f.evidencePaths.result).calls[3].completed.path;
  writeFileSync(call,readFileSync(call,'utf8').replace('"tool":"screenshot"','"tool":"screenshot","tool":"screenshot"'));
  refreshCallPins(f);refuse(f);
 }finally{f.close();}
});

test('observed focus history refuses zero native width even when all copies agree',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath),r=value(f.evidencePaths.result);o.window.bounds.width=0;r.visibleAssertion.originalNativeDimensions.width=0;
  const rawPath=r.calls[2].raw.path,raw=value(rawPath);raw.structuredContent.bounds.width=0;
  for(const slot of ['completed','attempt']){const path=r.calls[3][slot].path,post=value(path);post.args.width=0;json(path,post);}
  o.actualVendorCalls[3].args.width=0;
  raw.content[0].text=JSON.stringify(raw.structuredContent);json(rawPath,raw);json(f.observedPath,o);json(f.evidencePaths.result,r);
  refreshCallPins(f);refuse(f);
 }finally{f.close();}
});

for(const [key,changed]of Object.entries({target_window_id:999,width:685,quality:80}))
test('observed focus history refuses an altered screenshot request '+key+' even when every copy agrees',()=>{
 const f=observedFocusHistoryFixture();try{
  const r=value(f.evidencePaths.result),o=value(f.observedPath);
  for(const slot of ['completed','attempt']){const path=r.calls[3][slot].path,record=value(path);record.args[key]=changed;json(path,record);}
  o.actualVendorCalls[3].args[key]=changed;json(f.observedPath,o);refreshCallPins(f);refuse(f);
 }finally{f.close();}
});

for(const key of ['kind','driver','failureCategory','rootProtection'])
test('new-case scalar label '+key+' refuses a one-element array in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath);o[key]=[o[key]];json(f.observedPath,o);refuse(f);
 }finally{f.close();}
});

test('new-case scalar packet type refuses a one-element array in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath);o.providerFinish.raw.content[0].type=['text'];json(f.observedPath,o);refuse(f);
 }finally{f.close();}
});

test('new-case scalar audit name refuses a one-element array in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath);o.actualVendorCalls[0].name=['doctor'];json(f.observedPath,o);refuse(f);
 }finally{f.close();}
});

test('new-case scalar result label refuses a one-element array in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const r=value(f.evidencePaths.result);r.status=[r.status];json(f.evidencePaths.result,r);refuse(f);
 }finally{f.close();}
});

for(const place of ['observation','result','completed-call'])
test('new-case scalar date in '+place+' refuses a one-element array in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  if(place==='completed-call'){
   const path=value(f.evidencePaths.result).calls[0].completed.path,post=value(path);post.callCompleted=[post.callCompleted];json(path,post);refreshCallPins(f);
  }else{const path=place==='observation'?f.observedPath:f.evidencePaths.result,v=value(path);v.atUtc=[v.atUtc];json(path,v);}
  refuse(f);
 }finally{f.close();}
});

for(const pid of [-1,0])test('new-case positive foreground PID refuses coherent '+pid+' in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath),r=value(f.evidencePaths.result),path=r.calls[4].raw.path,raw=value(path);
  raw.structuredContent.app.pid=pid;raw.content[0].text=JSON.stringify(raw.structuredContent);
  o.foreground.raw=raw;r.focus.foregroundPid=pid;json(path,raw);json(f.observedPath,o);json(f.evidencePaths.result,r);refreshCallPins(f);refuse(f);
 }finally{f.close();}
});

test('new-case scalar foreground identifier refuses a coherent array in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath),r=value(f.evidencePaths.result),path=r.calls[4].raw.path,raw=value(path);
  raw.structuredContent.app.bundleId=['generated.exe'];raw.content[0].text=JSON.stringify(raw.structuredContent);
  o.foreground.raw=raw;r.focus.foregroundApp=['generated.exe'];json(path,raw);json(f.observedPath,o);json(f.evidencePaths.result,r);refreshCallPins(f);refuse(f);
 }finally{f.close();}
});

for(const windowId of [-1,0])test('new-case positive window ID refuses coherent '+windowId+' in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const o=value(f.observedPath),r=value(f.evidencePaths.result),path=r.calls[2].raw.path,raw=value(path),claim=value(f.evidencePaths.claim);
  raw.structuredContent.windowId=windowId;raw.content[0].text=JSON.stringify(raw.structuredContent);
  o.window=raw.structuredContent;r.focus.ownedHwnd=windowId;claim.binding.campaign.windowId=windowId;
  for(const i of [2,3]){const key=i===2?'window_id':'target_window_id';
   for(const slot of ['completed','attempt']){const postPath=r.calls[i][slot].path,post=value(postPath);post.args[key]=windowId;json(postPath,post);}
   o.actualVendorCalls[i].args[key]=windowId;
  }
  json(path,raw);json(f.observedPath,o);json(f.evidencePaths.result,r);json(f.evidencePaths.claim,claim);refreshCallPins(f);refuse(f);
 }finally{f.close();}
});

for(const place of ['observation','result'])test('new-case scalar date in '+place+' refuses an invalid string in both gates',()=>{
 const f=observedFocusHistoryFixture();try{
  const path=place==='observation'?f.observedPath:f.evidencePaths.result,v=value(path);v.atUtc='invalid-date';json(path,v);refuse(f);
 }finally{f.close();}
});

/** Repin generated historical authority in dependency order; never touch actual evidence.
 * @param {{evidencePaths:Record<string,string>,observedPath:string}} f
 * @param {'deadline'|'watchdog'|'start'} field @param {any} changed */
function alterHistoricalTiming(f,field,changed){
 const e=f.evidencePaths,p=value(e.plan),lease=value(e.lease),controller=value(e.controller),claim=value(e.claim),ledger=value(e.ledger),
  o=value(f.observedPath),r=value(e.result);
 if(field==='deadline'){p.callerAdmissionDeadlineMs=lease.callerAdmissionDeadlineMs=changed;o.originalCallerDeadlineMs=r.timing.originalCallerDeadlineMs=changed;}
 else if(field==='watchdog'){p.sessionWatchdogMs=lease.sessionWatchdogMs=changed;r.timing.rootWatchdogMs=changed;}
 else{const root=value(e.rootCall);root.value.startMs=changed;json(e.rootCall,root);}
 json(e.plan,p);lease.planSha256=ref(e.plan).sha256;json(e.lease,lease);controller.rootLeaseSha256=ref(e.lease).sha256;json(e.controller,controller);
 claim.binding.originalPlanSha256=ref(e.plan).sha256;claim.binding.controllerSha256=ref(e.controller).sha256;json(e.claim,claim);
 ledger.controllerSha256=ref(e.controller).sha256;ledger.b1ClaimSha256=ref(e.claim).sha256;json(e.ledger,ledger);
 json(f.observedPath,o);json(e.result,r);
}

for(const [field,changed]of /** @type {Array<['deadline'|'watchdog'|'start',any]>} */([
 ['deadline',0],['watchdog',0],['watchdog',[1800000]],['deadline',Number.MAX_SAFE_INTEGER],['start',0],
]))test('new-case bounded timing refuses coherent '+field+' '+JSON.stringify(changed)+' in both gates',()=>{
 const f=observedFocusHistoryFixture();try{alterHistoricalTiming(f,field,changed);refuse(f);}finally{f.close();}
});
