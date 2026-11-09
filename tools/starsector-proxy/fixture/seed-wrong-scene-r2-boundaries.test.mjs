// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {editCaptureEvidence as editHistory} from './seed-capture-stop-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

test('both real readers refuse a string E1 start with a coherently extended deadline',()=>{
 const f=wrongSceneHistoryFixture();try{
  const ledger=value(f.evidencePaths.ledger);
  ledger.providerClock.startMs=String(ledger.providerClock.startMs);
  ledger.providerClock.deadlineMs=f.wrongScene.binding.campaign.rootClock.effectiveDeadlineMs-30000;
  json(f.evidencePaths.ledger,ledger);f.refresh();assertCaptureRefused(f);
 }finally{f.close();}
});

/** Both real entry points run even when the first one refuses. @param {any} f */
function assertCaptureRefused(f){
 const b=f.launcher();let error;
 try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(found){error=found;}
 const actual=f.admit();assert.equal(actual.error,undefined);assert.notEqual(actual.status,null);
 assert.ok(error instanceof Error,'real JS reader must refuse');
 assert.match(error.message,/authority|seed|history|custody|capture|previous/i);
 assert.notEqual(actual.status,0,'real PS reader must refuse');
 assert.match(actual.stderr,/seed|history|custody|capture|previous/i);
}

/** @type {[string,Parameters<typeof wrongSceneHistoryFixture>[0]][]} */
const coherent=[
 ['B1 clock starts after actual ready and Play',{b1ProviderStartOffsetMs:14000}],
 ['phase name array',{changeE1Lease:v=>{v.phase=['DISPOSABLE_SEED_CREATION'];}}],
 ['phase issuer array',{changeE1Lease:v=>{v.issuer=['/root'];}}],
 ['phase scenario array',{changeE1Plan:v=>{v.scenarioId=[v.scenarioId];}}],
 ['producer PID boolean',{changeCampaign:v=>{v.producerWitness.controllerPid=true;}}],
 ['original caller start string',{changeRootCall:v=>{v.value.startMs=String(v.value.startMs);}}],
 ['owned session string',{changeOwner:v=>{v.sessionId=String(v.sessionId);}}],
 ['owned executable array',{changeOwner:v=>{v.executable=[v.executable];}}],
 ['copied frame timestamp string',{changeCopiedFrame:v=>{v.capturedAtMs=String(v.capturedAtMs);}}],
 ['observed PID disconnected from owner and claims',{observedPid:102}],
];
for(const [name,options]of coherent)test('both real readers refuse coherent '+name,()=>{
 const f=wrongSceneHistoryFixture(options);try{assertCaptureRefused(f);}finally{f.close();}
});
/** @type {[string,string,(v:any)=>void][]} */
const direct=[
 ['spool schema string','e1ProviderSpool',v=>{v.schemaVersion='1';}],
 ['spool command array','e1ProviderSpool',v=>{v.command=['verify'];}],
 ['spool verdict array','e1ProviderSpool',v=>{v.result.verdict=['PASS'];}],
 ['pending result status array','result',v=>{v.actions[1].ledgerStatus=['AWAITING_AFTER'];}],
 ['observed expected assertion array','observation',v=>{v.actions[1].expectedAssertion=['FAIL'];}],
];
for(const [name,key,change]of direct)test('both real readers refuse '+name,()=>{
 const f=wrongSceneHistoryFixture();try{editHistory(f,key,change);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});
test('both real readers refuse coherent image MIME array with exact raw/metadata pins',()=>{
 const f=wrongSceneHistoryFixture();try{
  const row=value(f.evidencePaths.observation).providers[1],original=row.originals[0];
  for(const p of [row.calls[3].raw.path,original.raw.path]){
   const raw=value(p);raw.content.find((/** @type {any} */ item)=>item.type==='image').mimeType=['image/png'];json(p,raw);
  }
  const meta=value(original.metadata.path);meta.rawSha256=ref(original.raw.path).sha256;json(original.metadata.path,meta);
  f.refresh();assertCaptureRefused(f);
 }finally{f.close();}
});
