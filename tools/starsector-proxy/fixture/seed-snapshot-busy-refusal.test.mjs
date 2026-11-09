// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {snapshotBusyHistoryFixture} from './seed-snapshot-busy-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {readFileSync,writeFileSync} from 'node:fs';

/** @param {any} f */
function refuse(f){
 const next=f.launcher();let refused=false;
 try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch{refused=true;}
 const ps=f.admit();assert.deepEqual([refused,ps.status],[true,1],ps.stderr);
}
/** @param {string} path @param {(v:any)=>void} change */
function change(path,change){const v=value(path);change(v);json(path,v);}

test('the exact busy case refuses an array-valued geometry explanation in both gates',()=>{
 const f=snapshotBusyHistoryFixture({change:(name,v)=>{if(name==='B1-PLAY-client-geometry.actual.private.json')v.originMeaning=['Generated origin'];}});
 try{
  const next=f.launcher();let refused=false;
  try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch{refused=true;}
  const ps=f.admit();assert.deepEqual([refused,ps.status],[true,1],ps.stderr);
 }finally{f.close();}
});

/** @type {Record<string,(o:any)=>void>} */
const observationChanges={
 'another observation kind':o=>{o.kind='ACTUAL_S11_DRIVER_FOCUS_BLOCKED_STOP';},
 'a sixth-call claim':o=>{o.vendorCallCount=6;},
 'an input-count array':o=>{o.inputs=[0];},
 'a reservation':o=>{o.reservationsAdded=1;},
 'an activation':o=>{o.activations=1;},
 'a focus failure substitute':o=>{o.focusFailure=true;},
 'a deadline failure substitute':o=>{o.timingFailure=true;},
 'a caller-date substitute':o=>{o.dateFailure=true;},
 'another helper failure chunk':o=>{o.failure.chunk='unbound';},
 'a renewed caller deadline':o=>{o.originalCallerDeadlineMs+=1;},
 'an altered caller doctor option':o=>{o.actualVendorCalls[0].args={include_remediation:false};},
 'a reordered audit':o=>{[o.actualVendorCalls[1],o.actualVendorCalls[2]]=[o.actualVendorCalls[2],o.actualVendorCalls[1]];},
 'an extra observation field':o=>{o.extra=true;},
};
for(const [name,mutate]of Object.entries(observationChanges))test('snapshot-busy history refuses '+name+' in both gates',()=>{
 const f=snapshotBusyHistoryFixture();try{change(f.evidencePaths.observation,mutate);f.repin();refuse(f);}finally{f.close();}
});

/** @type {Record<string,(r:any)=>void>} */
const resultChanges={
 'a seed claim':r=>{r.claims.seed=true;},
 'an ordinary-exit claim':r=>{r.closure.ordinaryExit=true;},
 'an acknowledged click':r=>{r.actions.clicks=1;},
 'a missing geometry invocation':r=>{r.counts.geometryModes=0;},
 'unknown provider cleanup':r=>{r.closure.provider.terminalReaped=false;},
 'a caller success rewrite':r=>{r.closure.rootCaller.status=0;},
 'a missing PID closure':r=>{r.closure.knownPidsAbsent.pop();},
 'an array-valued disposition':r=>{r.readerDisposition=[r.readerDisposition];},
 'an extended watchdog':r=>{r.timing.rootWatchdogMs+=1;},
 'an extra result field':r=>{r.extra=true;},
};
for(const [name,mutate]of Object.entries(resultChanges))test('snapshot-busy history refuses '+name+' in both gates',()=>{
 const f=snapshotBusyHistoryFixture();try{change(f.evidencePaths.result,mutate);f.repin();refuse(f);}finally{f.close();}
});

/** @type {Record<string,(m:any)=>void>} */
const commandChanges={
 'unqualified caller shell':m=>{m.entries[0].args.shell='pwsh';},
 'removed date-string contract':m=>{m.entries[0].args.cmd=m.entries[0].args.cmd.replace('-DateKind String','');},
 'an unknown retained helper handle':m=>{m.entries[0].raw.session_id=123;},
 'an unbound helper error':m=>{m.entries[0].raw.output=m.entries[0].raw.output.replaceAll('EBUSY','EACCES');},
 'a different failed file':m=>{m.entries[0].raw.output=m.entries[0].raw.output.replaceAll('owned-processes-current.private.json','other.json');},
 'a helper success rewrite':m=>{m.entries[0].raw.exit_code=0;},
 'a string elapsed value':m=>{m.entries[0].raw.wall_time_seconds='2';},
 'an unrecorded helper poll':m=>{m.entries[0].polls=[{}];},
};
for(const [name,mutate]of Object.entries(commandChanges))test('snapshot-busy history refuses '+name+' in both gates',()=>{
 const f=snapshotBusyHistoryFixture();try{change(f.evidencePaths.metadataCaller,mutate);f.repin();refuse(f);}finally{f.close();}
});

for(const target of ['observation','metadataCaller','result'])test('snapshot-busy history refuses duplicate keys in '+target,()=>{
 const f=snapshotBusyHistoryFixture();try{
  f.repin();const path=f.evidencePaths[target];writeFileSync(path,readFileSync(path,'utf8').replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'));
  if(target==='result')f.repinOuter();else f.repin();refuse(f);
 }finally{f.close();}
});
test('the exact busy case refuses an array-valued geometry timestamp in both gates',()=>{
 const f=snapshotBusyHistoryFixture({change:(name,v)=>{if(name==='B1-PLAY-client-geometry.actual.private.json')v.observedAtUtc=[v.observedAtUtc];}});
 try{
  const next=f.launcher();let refused=false;
  try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch{refused=true;}
  const ps=f.admit();assert.deepEqual([refused,ps.status],[true,1],ps.stderr);
 }finally{f.close();}
});
