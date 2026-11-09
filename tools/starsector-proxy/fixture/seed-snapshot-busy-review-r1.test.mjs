// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {snapshotBusyHistoryFixture} from './seed-snapshot-busy-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {value,ref} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

/** @param {any} f @param {boolean} accepted */
function check(f,accepted){
 const next=f.launcher();let ok=true,error;
 try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){ok=false;error=cause;}
 const ps=f.admit();assert.deepEqual([ok,ps.status],[accepted,accepted?0:1],String(error)+'\n'+ps.stderr);
}

test('the busy history refuses a coherently repinned future process snapshot',()=>{
 const f=snapshotBusyHistoryFixture({change:(name,v)=>{
  if(name==='B1-PLAY-owned-snapshot.private.json')v.at='2050-01-01T00:00:00.000Z';
 }});
 try{check(f,false);}finally{f.close();}
});

/** Move only historical geometry and its later closeout observations; preserve all hash links.
 * @param {number} start @param {number} complete */
function movedGeometry(start,complete){
 let anchor=0;
 const f=snapshotBusyHistoryFixture({change:(name,v)=>{
  if(name==='B1-PLAY-client-geometry.actual.private.json'){
   anchor=Date.parse(v.observedAtUtc)-2000;v.observedAtUtc=new Date(anchor+start+100).toISOString();
  }
  if(name==='B1-PLAY-client-geometry-terminal.actual.private.json'){
   const anchor=v.startedAtMs-1900;v.startedAtMs=anchor+start;v.completedAtMs=anchor+complete;
  }
  if(name==='B1-PLAY-client-point.private.json')v.computedAtMs=v.computedAtMs-2300+complete+100;
  if(name==='owned-provider-terminal.private.json')v.at=new Date(anchor+170000).toISOString();
 }});
 const e=f.evidencePaths,o=value(e.observation),p=value(e.protection),absence=value(e.pidAbsence),r=value(e.result);
 o.providerFinish.at=f.anchor+170001;o.atUtc=new Date(f.anchor+170100).toISOString();
 p.at=new Date(f.anchor+175000).toISOString();absence.atUtc=new Date(f.anchor+175001).toISOString();r.atUtc=new Date(f.anchor+176000).toISOString();
 json(e.observation,o);json(e.protection,p);json(e.pidAbsence,absence);json(e.result,r);f.repin();return f;
}

test('busy geometry accepts the full unclipped budget before the strict provider cutoff',()=>{
 const f=movedGeometry(131000,131300);try{check(f,true);}finally{f.close();}
});

test('busy geometry refuses completion exactly at the producer provider cutoff',()=>{
 const f=movedGeometry(150700,151000);try{check(f,false);}finally{f.close();}
});

test('busy geometry refuses completion beyond the producer provider cutoff',()=>{
 const f=movedGeometry(165700,166000);try{check(f,false);}finally{f.close();}
});

test('busy geometry refuses an unclipped cleanup claim when the start budget clips it by one millisecond',()=>{
 const f=movedGeometry(131001,131301);try{check(f,false);}finally{f.close();}
});

test('busy history refuses a coherently shortened coupled root and provider budget',()=>{
 const f=snapshotBusyHistoryFixture();try{
  const e=f.evidencePaths,deadline=value(e.geometryTerminal).completedAtMs+45000;
  for(const path of [e.plan,e.lease]){const v=value(path);v.callerAdmissionDeadlineMs=deadline;json(path,v);}
  const claim=value(e.claim),campaign=claim.binding.campaign;
  Object.assign(campaign.rootClock,{effectiveDeadlineMs:deadline,callerAdmissionDeadlineMs:deadline,originalCallerAdmissionDeadlineMs:deadline});
  campaign.producerWitness.effectiveDeadlineMs=deadline;json(e.claim,claim);
  const ledger=value(e.ledger);ledger.providerClock.deadlineMs=deadline-30000;json(e.ledger,ledger);
  const terminal=value(e.providerTerminal);terminal.deadlineMs=ledger.providerClock.deadlineMs;json(e.providerTerminal,terminal);
  f.refresh();
  const o=value(e.observation),r=value(e.result);o.originalCallerDeadlineMs=deadline;r.timing.originalCallerDeadlineMs=deadline;
  json(e.observation,o);json(e.result,r);f.repin();
  assert.equal(value(e.geometryTerminal).rootClock.effectiveDeadlineMs-value(e.geometryTerminal).completedAtMs,45000);
  assert.equal(value(e.geometryTerminal).providerClock.deadlineMs-value(e.geometryTerminal).completedAtMs,15000);
  assert.deepEqual(value(e.sourceFrame).ownedSnapshot,ref(e.frameOwned));check(f,false);
 }finally{f.close();}
});
