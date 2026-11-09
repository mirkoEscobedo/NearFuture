// @ts-check
import test from 'node:test';
import {resolve} from 'node:path';
import {captureLinkageHistoryFixture} from './seed-capture-linkage-test-support.mjs';
import {value,ref} from './seed-failed-history-test-support.mjs';
import {json,write} from './seed-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

for(const [label,key,change]of /** @type {[string,string,(v:any)=>void][]} */([
 ['extra reserved input','result',v=>{v.counts.reservedThisRun=1;}],
 ['fifth call summary','result',v=>{v.counts.vendorCalls=5;}],
 ['relabeled focus failure','result',v=>{v.failureCategory='FOCUS_BLOCKED';}],
 ['string schema','observation',v=>{v.schemaVersion='1';}],
 ['helper launched','observation',v=>{v.helperInvocation.geometryChildLaunched=true;}],
 ['uncertain input','ledger',v=>{v.uncertain=true;}],
 ['noninteger provider start','ledger',v=>{v.providerClock.startMs=String(v.providerClock.startMs);}],
 ['extended provider interval','ledger',v=>{v.providerClock.deadlineMs+=1;}],
 ['unknown provider child','providerTerminal',v=>{v.terminalBoundToChild=false;}],
 ['string provider schema','providerReady',v=>{v.schemaVersion='1';}],
 ['string provider startup bound','providerReady',v=>{v.startupTimeoutMs='20000';}],
 ['string provider session','providerReady',v=>{v.node.sessionId='1';}],
 ['string original save count','protection',v=>{v.protectedOriginalFiles='182';}],
 ['missing owned PID absence','pidAbsence',v=>{v.queriedExactRecordedPids.pop();}],
 ['changed save protection','protection',v=>{v.saveCensusUnchanged=false;}],
 ['nonterminal helper command','metadataCaller',v=>{v[0].raw.session_id=999;}],
 ['different helper failure','metadataCaller',v=>{v[0].raw.output='Other failure';}],
 ['helper command without failfast','metadataCaller',v=>{v[0].args.cmd=v[0].args.cmd.replace("if($LASTEXITCODE-ne0){throw 'Geometry failed; no mapping/input'};",'');}],
 ['object instead of command array','metadataCaller',v=>{return v[0];}]
])){
 test('both public history readers refuse '+label,()=>{
  const f=captureLinkageHistoryFixture();
  try{const path=f.evidencePaths[key],v=value(path),replacement=change(v);json(path,replacement??v);f.refresh();assertCaptureRefused(f);}finally{f.close();}
 });
}
test('four-call history refuses a fifth persisted attempt',()=>{
 const f=captureLinkageHistoryFixture();try{write(resolve(f.calls[0].attempt.path,'../call-005.attempt.private.json'),'{}');assertCaptureRefused(f);}finally{f.close();}
});
test('capture raw cannot be repinned to the call alias even with identical bytes',()=>{
 const f=captureLinkageHistoryFixture();
 try{const o=value(f.evidencePaths.observation),path=o.B1.original.metadata.path,m=value(path);m.rawPath=f.calls[3].raw.path;json(path,m);o.B1.original.metadata=ref(path);json(f.evidencePaths.observation,o);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});
