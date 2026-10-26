// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused,editCaptureEvidence as editHistory} from './seed-capture-stop-test-support.mjs';

/** @type {[string,string,(v:any)=>void][]} */
const cases=[
 ['relabelled capture failure','result',v=>{v.failureCategory='CAPTURE_UNCERTAIN';}],
 ['unacknowledged New Game','result',v=>{v.actions[1].acknowledged=false;}],
 ['different clicked point','result',v=>{v.actions[1].args.coordinate[0]++;}],
 ['unknown E1 closure','result',v=>{v.providerStatus.E1='UNKNOWN';}],
 ['failed history posing as seed','result',v=>{v.seed.created=true;}],
 ['missing AFTER original','observation',v=>{v.providers[1].originals.pop();}],
 ['extra provider call','observation',v=>{v.totalCalls++;}],
 ['uncertain launch input','ledger',v=>{v.uncertain=true;}],
 ['unresolved B1 handoff','ledger',v=>{delete v.resolvedB1;}],
 ['unknown final click','ledger',v=>{v.pending.status='UNKNOWN';}],
 ['missing E1 verifier absence','pidAbsence',v=>{v.queriedExactRecordedPids=v.queriedExactRecordedPids.filter((/** @type {number} */ n)=>n!==204);}],
 ['known provider missing child linkage','e1ProviderTerminal',v=>{v.terminalBoundToChild=false;}],
];
for(const [name,key,change]of cases)test('both readers reject '+name,()=>{
 const f=wrongSceneHistoryFixture();
 try{editHistory(f,key,change);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});
