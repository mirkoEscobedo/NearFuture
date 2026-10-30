// @ts-check
// Final consolidated review vectors, generated before the acyclic evidence is pinned.
import test from 'node:test';
import assert from 'node:assert/strict';
import {callerDateHistoryFixture} from './seed-caller-date-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
/** @param {string} file @param {(v:any)=>void} mutate */
const change=(file,mutate)=>({change:(/** @type {string} */ name,/** @type {any} */ v)=>{if(name===file)mutate(v);}});
/** @type {[string,Parameters<typeof callerDateHistoryFixture>[0]][]} */
const cases=[
 ['duplicate observed owned PID',change('guard-B1-consumed.private.json',v=>v.binding.ownedPids=[101,101])],
 ['extra owned PID',change('guard-B1-consumed.private.json',v=>v.binding.ownedPids=[101,100])],
 ['plan and lease array campaign profile',{change:(name,v)=>{if(name==='plan.json'||name==='native-lane-lease.private.json')v.campaignControl.profile=['DISPOSABLE_SEED_CREATION'];}}],
 ['array ledger provider batch',change('guard-launch-history.private.json',v=>v.provider.batch=['B1'])],
 ['array ledger provider digest',change('guard-launch-history.private.json',v=>v.provider.sourceSha256=[v.provider.sourceSha256])],
 ['array root call path',change('plan.json',v=>v.rootCallContract.path=[v.rootCallContract.path].flat())],
 ['array root call key',change('plan.json',v=>v.rootCallContract.key=[v.rootCallContract.key].flat())],
 ['array caller kind',change('launch-caller-terminal.actual.private.json',v=>v.kind=[v.kind])],
 ['boolean original autosave slots',change('after-smoke-protection.private.json',v=>v.originalAutosave.maximumSlots=true)],
 ['array original autosave slots',change('after-smoke-protection.private.json',v=>v.originalAutosave.maximumSlots=[3])],
 ['object initial processes',change('driver-B1-owned.private.json',v=>v.processes=v.processes[0])],
 ['object frame processes',change('B1-PLAY-owned-snapshot.private.json',v=>v.processes=v.processes[0])],
 ['array Java executable',change('owned-processes-current.private.json',v=>v.processes[0].executable=[v.processes[0].executable])],
 ['array Java session',change('owned-processes-current.private.json',v=>v.processes[0].sessionId=[1])],
 ['string Java session',change('owned-processes-current.private.json',v=>v.processes[0].sessionId='1')],
 ['scalar controller Job PIDs',{controller:v=>v.supervisorJobPids=100}],
 ['duplicate controller Job PIDs',{controller:v=>v.supervisorJobPids=[100,105,100]}],
 ['string provider Job PIDs',change('guarded-http-ready.private.json',v=>v.supervisorJobPids=[v.node.pid,v.helper.pid].join(','))],
 ['duplicate provider Job PIDs',change('guarded-http-ready.private.json',v=>v.supervisorJobPids=[v.node.pid,v.helper.pid,v.node.pid])],
 ['numeric controller kernel birth',{controller:v=>v.controllerKernelCreationFileTime=0}],
 ['array doctor structured content',change('call-001.raw.private.json',v=>v.structuredContent=[v.structuredContent])],
 ['array policy structured content',change('call-002.raw.private.json',v=>v.structuredContent=[v.structuredContent])],
 ['array window structured content',{window:v=>[v]}],
 ['wrapped image block candidate',{capture:v=>v.content=[v.content]}],
];
for(const [name,options]of cases)test('R2 both public readers refuse '+name,()=>{
 const f=callerDateHistoryFixture(options);
 try{
  const next=f.launcher();let error;try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
  const ps=f.admit();assert.ok(error,'JS must refuse coherent '+name);assert.equal(ps.status,1,'PS must refuse coherent '+name+'\n'+ps.stderr);
 }finally{f.close();}
});
