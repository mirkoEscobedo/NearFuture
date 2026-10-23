// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {writeFileSync,readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {makeOutput} from './test-output.mjs';
import {executeSingleCall} from './single-call.mjs';
test('one SDK call retains an opaque raw text result and actual child cleanup without a JSON-text assumption', {timeout:15000}, async()=>{
 const output=makeOutput('single-call-opaque-');
 const entrypoint=resolve(output,'fake-server.mjs');
 const runtime=new URL('./vendor-runtime.mjs',import.meta.url).href;
 writeFileSync(entrypoint,"import {serveComputerUseStdio} from "+JSON.stringify(runtime)+";const h=serveComputerUseStdio({profile:'core',activeProfile:'core',session:{dispatch:async()=>({content:[{type:'text',text:'OPAQUE_TEXT_$()_NOT_JSON'}]})}});process.once('SIGTERM',()=>{void h.close().finally(()=>process.exit(0));});");
 const request={schemaVersion:1,kind:'NATIVE99_SINGLE_SDK_CALL_V1',workspaceRoot:resolve(new URL('../../../',import.meta.url).pathname.replace(/^\/([A-Z]:)/,'$1')),
  node:process.execPath,entrypoint,configuration:{path:'UNUSED_SYNTHETIC',sha256:'0'.repeat(64)},environment:{},
  call:{name:'get_display_size',args:{}},actualPath:resolve(output,'call.actual.private.json'),rawPath:resolve(output,'call.raw.private.json'),
  stopPath:resolve(output,'driver-stop.private.json'),deadlineMs:Date.now()+10000};
 const actual=await executeSingleCall(request);
 assert.equal(actual.status,'SDK_CALL_AND_CLEANUP_COMPLETED');
 assert.ok(actual.serverPid!==null&&actual.serverPid>0);assert.equal(actual.serverPidAbsent,true);
 assert.equal(actual.closeReturned,true);
 assert.equal(actual.dispatchAttempted,true);
 assert.deepEqual(JSON.parse(readFileSync(request.rawPath,'utf8')),{content:[{type:'text',text:'OPAQUE_TEXT_$()_NOT_JSON'}]});
 assert.equal(JSON.parse(readFileSync(request.actualPath,'utf8')).resultScope,'SDK_TOOL_RESULT_NOT_JSONRPC_ENVELOPE');
});

// The only fake is the native desktop result; configuration, MCP, publication and files are real.
test('one SDK screenshot publishes exact caller intent and returns paths only after original persistence and readback', {timeout:15000}, async()=>{
 const {custodyFixture,png,screenshotArgs}=await import('./capture-custody-fixture.mjs');
 const {unlinkSync}=await import('node:fs');const {createHash}=await import('node:crypto');
 const f=custodyFixture('single-call-capture-'),delta=Date.now()-1000000,intent=f.intent;
 for(const key of /** @type {const} */ (['startMs','deadlineMs','callerAdmissionDeadlineMs','originalCallerAdmissionDeadlineMs','effectiveDeadlineMs']))intent.rootClock[key]+=delta;
 for(const key of /** @type {const} */ (['startMs','lastMs','deadlineMs']))intent.batchClock[key]+=delta;
 intent.createdMs+=delta;intent.rootCallRecord.value.startMs+=delta;intent.ownedWitness.atMs+=delta;
 intent.ownedWitness.source.startMs+=delta;intent.ownedWitness.source.endMs+=delta;intent.producerWitness.effectiveDeadlineMs+=delta;
 for(const key of /** @type {const} */ (['conservativeOuterDeadlineUtc','controllerDeadlineUtc']))intent.producerWitness[key]=new Date(Date.parse(intent.producerWitness[key])+delta).toISOString().replace(/(\.\d{3})Z$/,'$10000Z');
 unlinkSync(f.intentPath);
 const pointerPath=resolve(f.output,'pointer.private.json'),configPath=resolve(f.output,'config.private.json');
 writeFileSync(pointerPath,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_POINTER_V1',status:'UNSET',profileId:f.admitted.profileId})+'\n');
 writeFileSync(configPath,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_ADAPTER_CONFIGURATION_V1',admitted:f.admitted,pointerPath})+'\n');
 const sha=(/** @type {Buffer|string} */ b)=>createHash('sha256').update(b).digest('hex');
 const entrypoint=resolve(f.output,'fake-capture-server.mjs'),adapter=new URL('./capture-adapter.mjs',import.meta.url).href;
 const imagePath=new URL('./fixtures/readiness.synthetic.png',import.meta.url).href;
 writeFileSync(entrypoint,"import{readFileSync}from'node:fs';import{serveCaptureStdio,loadCaptureConfiguration}from "+JSON.stringify(adapter)+";const image=readFileSync(new URL("+JSON.stringify(imagePath)+"));const h=serveCaptureStdio({configuration:loadCaptureConfiguration(process.argv[2],process.argv[3]),session:{dispatch:async()=>({content:[{type:'image',mimeType:'image/png',data:image.toString('base64')}]})}});process.once('SIGTERM',()=>{void h.close().finally(()=>process.exit(0));});");
 const request={schemaVersion:1,kind:'NATIVE99_SINGLE_SDK_CALL_V1',workspaceRoot:f.admitted.workspaceRoot,node:process.execPath,entrypoint,
  configuration:{path:configPath,sha256:sha(readFileSync(configPath))},environment:{},call:{name:'screenshot',args:screenshotArgs},
  actualPath:resolve(f.output,'call.actual.private.json'),rawPath:resolve(f.output,'sdk-result.raw.private.json'),
  stopPath:resolve(f.output,'driver-stop.private.json'),deadlineMs:Date.now()+10000,
  capture:{intentText:JSON.stringify(intent)+'\n',expectedPointerSHA256:sha(readFileSync(pointerPath))}};
 const actual=await executeSingleCall(request);
 assert.equal(actual.status,'SDK_CALL_AND_CLEANUP_COMPLETED');assert.equal(actual.serverPidAbsent,true);
 assert.deepEqual(readFileSync(f.slot.png),png);
 assert.equal(actual.captureProof?.inlineImageDigestMatched,true);
 assert.equal(actual.captureProof?.png,f.slot.png);assert.equal(actual.captureProof?.freshness,'UNKNOWN');
 assert.equal(actual.captureProof?.wholeNativeAcceptance,false);
 assert.equal(JSON.stringify(actual).includes(png.toString('base64')),false,'return compact custody, never another large image payload');
});
