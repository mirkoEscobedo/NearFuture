// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {failedSeedHistoryFixture,ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @param {any} f @param {string} key @param {(v:any)=>void} change */
function edit(f,key,change){const p=f.evidencePaths[key],v=value(p);change(v);json(p,v);}
/** @type {[string,(f:any)=>void][]} */
const cases=[
 ['boolean provider status',f=>{for(const k of ['providerTerminal','providerSpool'])edit(f,k,v=>{(v.steps??v.result.steps)[0].status=false;});}],
 ['canonical provider command mismatch',f=>{for(const k of ['providerTerminal','providerSpool'])edit(f,k,v=>{(v.steps??v.result.steps)[0].command='other.exe';});}],
 ['canonical helper executable mismatch',f=>{edit(f,'providerReady',v=>{v.helper.executable='other.exe';});}],
 ['owned target omitted',f=>{edit(f,'owned',v=>{v.processes=[];});edit(f,'pidAbsence',v=>{v.queriedExactRecordedPids=v.queriedExactRecordedPids.filter((/** @type {number} */ pid)=>pid!==101);});}],
 ['changed historical preference',f=>{edit(f,'preference',v=>{v.value='false';});edit(f,'protection',v=>{v.sharedPreference.value='false';});}],
 ['skipped attempt context',f=>{f.closeout.actions.currentRunContext.overallAttempts=40;}],
 ['counter refund',f=>{edit(f,'ledger',v=>{v.history.reservedActions=71;});}],
 ['erased unsent reservations',f=>{edit(f,'result',v=>{v.historicalUnsentReservations=0;});}],
 ['wrong source head',f=>{f.closeout.sourceHead='2'.repeat(40);} ],
 ['wrong scenario',f=>{edit(f,'result',v=>{v.scenarioId='other';});}],
 ['wrong lease predecessor',f=>{edit(f,'lease',v=>{v.planSha256='0'.repeat(64);});}],
 ['wrong controller predecessor',f=>{edit(f,'controller',v=>{v.rootLeaseSha256='0'.repeat(64);});}],
 ['wrong provider claim',f=>{edit(f,'claim',v=>{v.binding.sourceSha256='0'.repeat(64);});}],
 ['pending action',f=>{edit(f,'ledger',v=>{v.pending={status:'SENT'};});}],
 ['uncertain action',f=>{edit(f,'ledger',v=>{v.uncertain=true;});}],
 ['recorded input',f=>{edit(f,'result',v=>{v.actions.sentThisRun=1;});}],
 ['unknown provider closure',f=>{edit(f,'providerTerminal',v=>{v.cleanupUnknown=true;});}],
 ['nonterminal caller',f=>{edit(f,'caller',v=>{v.terminalReaped=false;});}],
 ['changed original caller',f=>{edit(f,'caller',v=>{v.launchRaw.session_id=99;});}],
 ['ordinary-exit masquerade',f=>{edit(f,'caller',v=>{v.actual.exit_code=0;v.launchPolls[v.launchPolls.length-1].exit_code=0;});}],
 ['PID absence subset',f=>{edit(f,'pidAbsence',v=>{v.queriedExactRecordedPids.pop();});}],
 ['changed original protection',f=>{edit(f,'protection',v=>{v.allOriginalAndBackupHashesMatch=false;});}],
 ['nonempty private saves',f=>{edit(f,'protection',v=>{v.privateSaveEntries=1;});}],
 ['receipt extra field',f=>{f.closeout.grantMore=true;}],
 ['completed-seed kind masquerade',f=>{f.closeout.kind='ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT';}],
 ['input call despite zero summary',f=>{
  const o=value(f.evidencePaths.observation),row=o.calls[4];
  for(const key of ['completed','attempt']){const v=value(row[key].path);v.tool='left_click';json(row[key].path,v);row[key]=ref(row[key].path);}
  json(f.evidencePaths.observation,o);
 }]
];
for(const [name,change] of cases)test('closed failure history refuses '+name+' in both real validators',()=>{
 const f=failedSeedHistoryFixture();
 try{
  if(/canonical|owned target|changed historical/.test(name)){
   const b=f.launcher();assert.doesNotThrow(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
   assert.equal(f.admit().status,0,'review regression must start with admitted custody');
  }
  change(f);f.refresh();
  if(name==='boolean provider status')assert.notEqual(f.admit().status,0,'boolean false is not a terminal integer exit status');
  const b=f.launcher();assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
  assert.notEqual(f.admit().status,0);
  assert.equal(existsSync(resolve(f.scenario,'guard-B1-consumed.private.json')),false);
 }finally{f.close();}
});

test('duplicate failed receipt fields refuse even with a refreshed grant digest',()=>{
 const f=failedSeedHistoryFixture();
 try{
  const text=readFileSync(f.closeoutPath,'utf8');writeFileSync(f.closeoutPath,'{"issuer":"/root",'+text.slice(1));
  const grant=value(f.grantPath);grant.previous.result=ref(f.closeoutPath);json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);
  f.refreshLaunch();
  const b=f.launcher();assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}),/canonical|duplicate/i);
  assert.notEqual(f.admit().status,0);
 }finally{f.close();}
});
