// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {snapshotBusyHistoryFixture} from './seed-snapshot-busy-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

/** @param {any} f */
function refuse(f){
 const next=f.launcher();let refused=false;
 try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch{refused=true;}
 const ps=f.admit();assert.deepEqual([refused,ps.status],[true,1],ps.stderr);
}

/** @type {Record<string,(g:any)=>void>} */
const geometryChanges={
 'a foreign geometry PID':g=>{g.pid=999;},
 'a geometry input grant':g=>{g.inputGranted=true;},
 'a moved client origin':g=>{g.clientOriginScreen.x=0;},
 'an array width':g=>{g.clientSize.width=[1];},
 'a future geometry timestamp':g=>{g.observedAtUtc=new Date(Date.parse(g.observedAtUtc)+10000).toISOString();},
 'a stale geometry timestamp':g=>{g.observedAtUtc=new Date(Date.parse(g.observedAtUtc)-10000).toISOString();},
};
for(const [name,mutate]of Object.entries(geometryChanges))test('snapshot-busy facts refuse '+name+' through both gates',()=>{
 const f=snapshotBusyHistoryFixture({change:(n,v)=>{if(n==='B1-PLAY-client-geometry.actual.private.json')mutate(v);}});
 try{refuse(f);}finally{f.close();}
});

/** @type {Record<string,(t:any)=>void>} */
const terminalChanges={
 'an extended geometry timeout':t=>{t.outerTimeoutMs=16000;},
 'an unreaped geometry command':t=>{t.terminalReaped=false;},
 'an array start time':t=>{t.startedAtMs=[t.startedAtMs];},
 'outer success with inner failure':t=>{t.supervisorJSON.result.steps[0].status=1;t.stdout=JSON.stringify(t.supervisorJSON);},
 'a child without Job ownership':t=>{t.supervisorJSON.result.steps[0].processOwnership='none';t.stdout=JSON.stringify(t.supervisorJSON);},
};
for(const [name,mutate]of Object.entries(terminalChanges))test('snapshot-busy facts refuse '+name+' through both gates',()=>{
 const f=snapshotBusyHistoryFixture({change:(n,v)=>{if(n==='B1-PLAY-client-geometry-terminal.actual.private.json')mutate(v);}});
 try{refuse(f);}finally{f.close();}
});

test('a coherently repinned outer-origin point cannot replace the measured client point',()=>{
 const f=snapshotBusyHistoryFixture({change:(n,v)=>{if(n==='B1-PLAY-client-point.private.json')v.coordinate=[0,0];}});
 try{const r=value(f.evidencePaths.result);r.visibleAssertion.mappedScreenPoint=[0,0];json(f.evidencePaths.result,r);f.repin();refuse(f);}finally{f.close();}
});

for(const change of ['missing','extra'])test('snapshot-busy facts require the exact PID union: '+change,()=>{
 const f=snapshotBusyHistoryFixture();try{
  const p=value(f.evidencePaths.pidAbsence),r=value(f.evidencePaths.result);
  if(change==='missing')p.queriedExactRecordedPids.pop();else p.queriedExactRecordedPids.push(999);
  r.closure.knownPidsAbsent=p.queriedExactRecordedPids;json(f.evidencePaths.pidAbsence,p);json(f.evidencePaths.result,r);f.repin();refuse(f);
 }finally{f.close();}
});

test('snapshot-busy facts retain full original PNG CRC refusal after outer repinning',()=>{
 const f=snapshotBusyHistoryFixture();try{
  const path=f.original.png.path,bytes=readFileSync(path);bytes[30]^=1;writeFileSync(path,bytes);
  const r=value(f.evidencePaths.result);r.evidence.originalPng=ref(path);json(f.evidencePaths.result,r);f.repin();refuse(f);
 }finally{f.close();}
});
