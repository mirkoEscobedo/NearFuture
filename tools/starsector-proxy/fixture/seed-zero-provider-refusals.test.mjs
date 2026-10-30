// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {zeroProviderHistoryFixture} from './seed-zero-provider-test-support.mjs';
import {json,write} from './seed-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @type {[string,string,(v:any)=>void][]} */
const changes=[
 ['deadline no longer inverted','controller',v=>{v.controllerDeadlineUtc=v.conservativeOuterDeadlineUtc;}],
 ['changed outer deadline','controller',v=>{v.conservativeOuterDeadlineUtc='2026-10-27T08:58:48Z';}],
 ['controller birth','controller',v=>{v.controllerCreationTime='2026-10-27T09:00:00Z';}],
 ['kernel identity','controller',v=>{v.controllerKernelCreationFileTime='0';}],
 ['wrong source','plan',v=>{v.source.qualifiedHead='2'.repeat(40);}],
 ['refunded ledger','ledger',v=>{v.history.reservedActions--;}],
 ['uncertain ledger','ledger',v=>{v.uncertain=true;}],
 ['provider in ledger','ledger',v=>{v.provider={batch:'B1'};}],
 ['input reservation','ledger',v=>{v.pending={action:'click'};}],
 ['unreaped caller','caller',v=>{v.terminalReaped=false;}],
 ['reused caller handle','caller',v=>{v.session++;}],
 ['terminal unknown','caller',v=>{v.actual.session_id=v.session;}],
 ['timed out owned tree','launch',v=>{v.result.steps[0].timedOut=true;}],
 ['fabricated launch PASS','launch',v=>{v.result.verdict='PASS';v.result.steps[0].status=0;}],
 ['wrong admission error','combined',v=>{v.stagePolls[0].output='different error';}],
 ['successful stage','combined',v=>{v.stagePolls[0].exit_code=0;}],
 ['missing stage terminal','combined',v=>{v.stagePolls=[];}],
 ['stage caller swap','combined',v=>{v.stage.args.cmd+=' extra';}],
 ['fabricated B1 admission','combined',v=>{v.b1Admit={readOnlyActualJsAdmission:'PASS'};}],
 ['remaining stage handle','combined',v=>{v.remainingStageSession=778;}],
 ['failed freeze admission','frozen',v=>{v.actualPsAdmissionExit=1;}],
 ['failed freeze supervisor','freezeSupervisor',v=>{v.raw.exit_code=1;}],
 ['changed private preference','preference',v=>{v.value='false';}],
 ['missing protection','protection',v=>{delete v.allOriginalAndBackupHashesMatch;}],
 ['unexpected save','protection',v=>{v.privateSaveEntries=1;}],
 ['missing owned PID','pidAbsence',v=>{v.expectedPids.pop();v.absentPids.pop();}],
 ['provider was started','pidAbsence',v=>{v.providerStarted=true;}],
 ['ordinary-exit masquerade','pidAbsence',v=>{v.ordinaryExitAccepted=true;}],
 ['changed Java identity','initialOwned',v=>{v.processes[0].kernelCreationFileTime='7';}],
 ['unclaimed source changed','source',v=>{v.batch='E1';}],
];
for(const [name,key,change] of changes)test('both real gates refuse zero-provider history: '+name,()=>{
 const f=zeroProviderHistoryFixture();
 try{
  const path=f.evidencePaths[key],v=value(path);change(v);json(path,v);f.refresh();
  const first=f.launcher();assert.throws(()=>readBatchAuthorization(first.source,{root:f.root,runID:first.runID}),/Campaign authority refused/);
  const launch=f.admit();assert.equal(launch.error,undefined);assert.notEqual(launch.status,0,name);
 }finally{f.close();}
});
for(const name of ['guard-B1-consumed.private.json','guard-E1-consumed.private.json','owned-provider-terminal.private.json','call-001.private.json','capture-001.png','geometry-PLAY.private.json'])
 test('both real gates refuse unexpected activity artifact '+name,()=>{
  const f=zeroProviderHistoryFixture();
  try{write(resolve(f.priorScenario,name),'unexpected');f.refresh();const first=f.launcher();
   assert.throws(()=>readBatchAuthorization(first.source,{root:f.root,runID:first.runID}),/Campaign authority refused/);
   assert.notEqual(f.admit().status,0);
  }finally{f.close();}
 });
