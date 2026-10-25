// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {knownPlayHistoryFixture} from './seed-known-play-history-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @param {any} f @param {string} key @param {(v:any)=>void} mutate */
function edit(f,key,mutate){const path=f.evidencePaths[key],v=value(path);mutate(v);json(path,v);}
/** @param {any} f @param {number} index @param {'post'|'raw'|'attempt'} key @param {(v:any)=>void} mutate */
function call(f,index,key,mutate){
 const o=value(f.evidencePaths.observation),path=o.calls[index][key].path,v=value(path);mutate(v);json(path,v);
 o.calls[index][key]=ref(path);json(f.evidencePaths.observation,o);
}
/** @type {[string,(f:any)=>void][]} */
const cases=[
 ['PID reuse with changed birth',f=>edit(f,'owned',v=>{v.processes[0].creationTime='2020-01-01T00:00:00Z';})],
 ['changed kernel identity',f=>edit(f,'owned',v=>{v.processes[0].kernelCreationFileTime='999';})],
 ['changed Windows session',f=>edit(f,'owned',v=>{v.processes[0].sessionId=2;})],
 ['changed parent',f=>edit(f,'owned',v=>{v.processes[0].parentPid=999;})],
 ['changed executable',f=>edit(f,'owned',v=>{v.processes[0].executable=resolve(f.priorScenario,'other.exe');})],
 ['missing owned Job member',f=>edit(f,'owned',v=>{v.processes[0].supervisorJobPids=v.processes[0].supervisorJobPids.filter((/** @type {number} */ n)=>n!==100);})],
 ['foreign current PID',f=>edit(f,'owned',v=>{v.processes[0].pid=999;})],
 ['uncertain input',f=>edit(f,'ledger',v=>{v.uncertain=true;})],
 ['missing pending Play',f=>edit(f,'ledger',v=>{v.pending=null;})],
 ['unacknowledged Play',f=>call(f,6,'raw',v=>{v.content[0].text='Input pending';})],
 ['extra input tool',f=>{call(f,9,'post',v=>{v.tool='left_click';});call(f,9,'attempt',v=>{v.tool='left_click';});}],
 ['wrong Play ordinal',f=>edit(f,'ledger',v=>{v.pending.wizardStep.ordinal=2;})],
 ['refunded spent action',f=>{f.closeout.actions.actualGuard.reservedActions=72;edit(f,'ledger',v=>{v.history.reservedActions=72;});edit(f,'result',v=>{v.actualGuardHistory.reservedActions=72;});}],
 ['erased unsent reservations',f=>edit(f,'result',v=>{v.historicalUnsentReservations=0;})],
 ['skipped actual context',f=>{f.closeout.actions.currentRunContext.overallAttempts=42;edit(f,'result',v=>{v.actualRunContext.overallAttempts=42;});}],
 ['false integer action count',f=>edit(f,'result',v=>{v.actions.activations=false;})],
 ['unknown provider closure',f=>edit(f,'providerTerminal',v=>{v.cleanupUnknown=true;})],
 ['unreaped caller',f=>edit(f,'caller',v=>{v.terminalReaped=false;})],
 ['missing PID absence',f=>edit(f,'pidAbsence',v=>{v.queriedExactRecordedPids=v.queriedExactRecordedPids.filter((/** @type {number} */ n)=>n!==101);})],
 ['changed original save protection',f=>edit(f,'protection',v=>{v.saveCensusUnchanged=false;})],
 ['no-input kind masquerade',f=>{f.closeout.kind='ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY';}],
 ['successful seed kind masquerade',f=>{f.closeout.kind='ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT';}],
 ['corrupt PNG despite coherent refreshed hashes',f=>{
  const o=value(f.evidencePaths.observation),frame=o.originals[2],bytes=readFileSync(frame.png.path);bytes[bytes.length-1]^=1;
  writeFileSync(frame.png.path,bytes);const raw=value(frame.raw.path);raw.content[0].data=bytes.toString('base64');
  json(frame.raw.path,raw);json(o.calls[13].raw.path,raw);
  const metadata=value(frame.metadata.path);metadata.pngSha256=ref(frame.png.path).sha256;metadata.rawSha256=ref(frame.raw.path).sha256;json(frame.metadata.path,metadata);
  frame.png=ref(frame.png.path);frame.raw=ref(frame.raw.path);frame.metadata=ref(frame.metadata.path);o.calls[13].raw=ref(o.calls[13].raw.path);
  json(f.evidencePaths.observation,o);
 }]
];
for(const [name,change] of cases)test('known Play history refuses '+name+' in both real validators',()=>{
 const f=knownPlayHistoryFixture();
 try{
  change(f);f.refresh();const b=f.launcher();
  assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
  assert.notEqual(f.admit().status,0);
  assert.equal(existsSync(resolve(f.scenario,'guard-B1-consumed.private.json')),false);
 }finally{f.close();}
});

test('known Play refuses an explicit target app other than its owned Java process',()=>{
 const f=knownPlayHistoryFixture({targetApp:'other.exe'});
 try{
  const b=f.launcher();assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
  assert.notEqual(f.admit().status,0);
 }finally{f.close();}
});
