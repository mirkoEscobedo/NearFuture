// @ts-check
import test from 'node:test';
import {resolve} from 'node:path';
import {captureStopHistoryFixture,editCaptureEvidence,assertCaptureRefused} from './seed-capture-stop-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

/** @type {[string,(f:any)=>void][]} */
const cases=[
 ['changed original birth',f=>editCaptureEvidence(f,'owned',v=>{v.processes[0].creationTime='2020-01-01T00:00:00Z';})],
 ['changed kernel identity',f=>editCaptureEvidence(f,'owned',v=>{v.processes[0].kernelCreationFileTime='999';})],
 ['foreign executable',f=>editCaptureEvidence(f,'owned',v=>{v.processes[0].executable=resolve(f.priorScenario,'foreign.exe');})],
 ['changed session',f=>editCaptureEvidence(f,'owned',v=>{v.processes[0].sessionId=2;})],
 ['lost Job membership',f=>editCaptureEvidence(f,'owned',v=>{v.processes[0].inSupervisorJob=false;})],
 ['uncertain input',f=>editCaptureEvidence(f,'ledger',v=>{v.uncertain=true;})],
 ['cleared pending Play',f=>editCaptureEvidence(f,'ledger',v=>{v.pending=null;})],
 ['refunded reservation',f=>editCaptureEvidence(f,'ledger',v=>{v.history.reservedActions=73;})],
 ['refunded sent action',f=>editCaptureEvidence(f,'ledger',v=>{v.history.sentActions=71;})],
 ['skipped attempt',f=>editCaptureEvidence(f,'result',v=>{v.actualRunContext.overallAttempts=45;})],
 ['false provider closure',f=>editCaptureEvidence(f,'providerTerminal',v=>{v.cleanupUnknown=true;})],
 ['live owned PID',f=>editCaptureEvidence(f,'pidAbsence',v=>{v.allAbsent=false;v.liveRows=[{ProcessId:101}];})],
 ['changed save protection',f=>editCaptureEvidence(f,'protection',v=>{v.allOriginalAndBackupHashesMatch=false;})],
 ['nonempty private saves',f=>editCaptureEvidence(f,'protection',v=>{v.privateSaveEntries=1;})],
 ['changed canonical click arguments',f=>editCaptureEvidence(f,'result',v=>{v.actions.args.target_app='foreign.exe';})],
 ['different observed arguments',f=>editCaptureEvidence(f,'observation',v=>{v.actions.args.coordinate=[0,0];})],
 ['unacknowledged native Play',f=>{
  const o=value(f.evidencePaths.observation),p=o.calls[6].raw.path,v=value(p);v.content[0].text='Input pending';json(p,v);
  o.calls[6].raw=ref(p);json(f.evidencePaths.observation,o);
 }],
 ['old helper kind masquerade',f=>{f.closeout.kind='ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY';}]
];
for(const [name,change]of cases)test('capture-stop custody refuses '+name+' in both real readers',()=>{
 const f=captureStopHistoryFixture();
 try{change(f);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});

for(const limit of [81,83])test('capture-stop rejects grant mismatch '+limit+' against the explicit cap82 plan',()=>{
 const f=captureStopHistoryFixture();
 try{
  const grant=value(f.grantPath);grant.maxReservedActions=limit;json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);
  f.refreshLaunch();assertCaptureRefused(f);
 }finally{f.close();}
});

test('capture-stop rejects a coherently rebound nine-action cap83',()=>{
 const f=captureStopHistoryFixture();
 try{
  const grant=value(f.grantPath);grant.maxReservedActions=83;json(f.grantPath,grant);
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){v.historyAdmission=ref(f.grantPath);v.absoluteMaxReservedActions=83;}
  f.campaign.maxReservedActions=83;f.predecessor.maxReservedActions=83;f.refreshLaunch();assertCaptureRefused(f);
 }finally{f.close();}
});
