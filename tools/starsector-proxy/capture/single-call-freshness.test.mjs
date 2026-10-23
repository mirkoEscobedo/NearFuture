// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,unlinkSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,isAbsolute} from 'node:path';
import {executeSingleCall} from './single-call.mjs';
import {custodyFixture,png} from './capture-custody-fixture.mjs';
const sha=(/** @type {Buffer|string} */ bytes)=>createHash('sha256').update(bytes).digest('hex');
function fixture(/** @type {boolean} */ mismatch=false,/** @type {boolean} */ stale=false) {
 const f=custodyFixture('single-call-freshness-'),intent=f.intent,delta=Date.now()-1000000;
 for(const k of /** @type {const} */ (['startMs','deadlineMs','callerAdmissionDeadlineMs','originalCallerAdmissionDeadlineMs','effectiveDeadlineMs']))intent.rootClock[k]+=delta;
 for(const k of /** @type {const} */ (['startMs','lastMs','deadlineMs']))intent.batchClock[k]+=delta;
 intent.rootCallRecord.value.startMs+=delta;intent.producerWitness.effectiveDeadlineMs+=delta;
 for(const k of /** @type {const} */ (['conservativeOuterDeadlineUtc','controllerDeadlineUtc']))intent.producerWitness[k]=new Date(Date.parse(intent.producerWitness[k])+delta).toISOString().replace(/(\.\d{3})Z$/,'$10000Z');
 intent.createdMs=Date.now()-10000;intent.ownedWitness.atMs=Date.now()-11000;
 unlinkSync(f.intentPath);
 const sample={at:new Date(Date.now()-(stale?6000:0)).toISOString(),controllerPid:intent.producerWitness.controllerPid,controllerCreationTime:intent.producerWitness.controllerCreationTime,
 controllerKernelCreationFileTime:intent.producerWitness.controllerKernelCreationFileTime,conservativeOuterDeadlineUtc:intent.producerWitness.conservativeOuterDeadlineUtc,
 controllerDeadlineUtc:intent.producerWitness.controllerDeadlineUtc,processes:[{...intent.ownedWitness.row,mainWindowObservedAtUtc:new Date().toISOString()}]};
 if(mismatch)sample.processes[0].kernelCreationFileTime='999999999999999999';
 writeFileSync(resolve(f.output,'owned-processes-current.private.json'),JSON.stringify(sample)+'\n');
 const powershell=process.env.NF_TEST_POWERSHELL;
 if(!powershell||!isAbsolute(powershell))throw new Error('Explicit absolute NF_TEST_POWERSHELL required for Windows freshness checks');
 const pointer=resolve(f.output,'pointer.private.json'),configuration=resolve(f.output,'configuration.private.json');
 writeFileSync(pointer,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_POINTER_V1',status:'UNSET',profileId:f.admitted.profileId})+'\n');
 const admitted={...f.admitted,refreshOwnedWitnessAfterConnect:true};
 writeFileSync(configuration,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_ADAPTER_CONFIGURATION_V1',admitted,pointerPath:pointer,runtimePins:[{path:powershell,sha256:sha(readFileSync(powershell))}]})+'\n');
 const entrypoint=resolve(f.output,'fake-server.mjs'),adapter=new URL('./capture-adapter.mjs',import.meta.url).href;
 const imagePath=new URL('./fixtures/readiness.synthetic.png',import.meta.url).href,window={...intent.window,isOnScreen:true,isFocused:true};
 writeFileSync(entrypoint,"import{readFileSync}from'node:fs';import{serveCaptureStdio,loadCaptureConfiguration}from "+JSON.stringify(adapter)+";const image=readFileSync(new URL("+JSON.stringify(imagePath)+"));const h=serveCaptureStdio({configuration:loadCaptureConfiguration(process.argv[2],process.argv[3]),session:{dispatch:async(name)=>name==='get_window'?{content:[{type:'text',text:JSON.stringify("+JSON.stringify(window)+")}],structuredContent:"+JSON.stringify(window)+"}:{content:[{type:'image',mimeType:'image/png',data:image.toString('base64')}]}}});process.once('SIGTERM',()=>{void h.close().finally(()=>process.exit(0));});");
 const request={schemaVersion:1,kind:'NATIVE99_SINGLE_SDK_CALL_V1',workspaceRoot:f.admitted.workspaceRoot,node:process.execPath,entrypoint,
 configuration:{path:configuration,sha256:sha(readFileSync(configuration))},environment:{},call:{name:'screenshot',args:intent.args},
 actualPath:resolve(f.output,'call.actual.private.json'),rawPath:resolve(f.output,'sdk-result.private.json'),stopPath:resolve(f.output,'stop.private.json'),
 deadlineMs:Date.now()+12000,capture:{intentText:JSON.stringify(intent)+'\n',expectedPointerSHA256:sha(readFileSync(pointer)),refreshOwnedWitnessAfterConnect:true}};
 return {f,intent,sample,request};
}
test('after-connect refresh retains actual read provenance and the original grant without extending freshness', {timeout:15000},async()=>{
 const {f,intent,sample,request}=fixture(),original=request.capture.intentText;
 const actual=await executeSingleCall(request);
 assert.equal(actual.status,'SDK_CALL_AND_CLEANUP_COMPLETED');assert.equal(actual.serverPidAbsent,true);
 assert.deepEqual(readFileSync(f.slot.png),png);
 const published=JSON.parse(readFileSync(f.intentPath,'utf8'));
 assert.deepEqual(published.rootClock,intent.rootClock);assert.deepEqual(published.batchClock,intent.batchClock);
 assert.deepEqual(published.rootCallRecord,intent.rootCallRecord);
 assert.equal(published.ownedWitness.atMs,Date.parse(sample.at));
 assert.equal(published.ownedWitness.source.kind,'CHILD_PROCESS_TERMINAL');
 assert.equal(published.ownedWitness.source.terminal.exit_code,0);
 assert.deepEqual(JSON.parse(published.ownedWitness.source.terminal.output),sample);
 assert.equal(request.capture.intentText,original,'initial grant remains unchanged');
 assert.equal(actual.refreshProof?.nativeObservations,1);
 assert.ok(actual.refreshProof?.initialWitnessAgeMs>=10000);
 assert.equal(actual.captureProof?.inlineImageDigestMatched,true);
});
test('refresh refuses a different process identity before any screenshot and retains the closed reason', {timeout:15000},async()=>{
 const {f,request}=fixture(true);
 await assert.rejects(executeSingleCall(request),/SDK_SINGLE_CALL_HOLD/);
 const actual=JSON.parse(readFileSync(request.actualPath,'utf8'));
 assert.equal(actual.dispatchAttempted,false);assert.equal(actual.serverPidAbsent,true);
 assert.equal(actual.failure?.code,'CURRENT_CAPTURE_PRODUCER_IDENTITY_REQUIRED');
 assert.equal(actual.failure?.phase,'REFRESH');
 assert.equal(existsSync(f.slot.png),false);assert.equal(existsSync(request.stopPath),true);
});

test('an expired producer snapshot remains refused after connection, with no screenshot dispatch',{timeout:15000},async()=>{
 const {f,request}=fixture(false,true);
 await assert.rejects(executeSingleCall(request),/SDK_SINGLE_CALL_HOLD/);
 const actual=JSON.parse(readFileSync(request.actualPath,'utf8'));
 assert.equal(actual.dispatchAttempted,false);assert.equal(actual.serverPidAbsent,true);
 assert.equal(actual.failure?.code,'EXACT_FRESH_CALLER_CAPTURE_REQUIRED');
 assert.ok(actual.witnessAgeMs>5000);assert.equal(existsSync(f.slot.png),false);
});
