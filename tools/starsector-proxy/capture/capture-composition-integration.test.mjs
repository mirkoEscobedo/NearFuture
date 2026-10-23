// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { readFileSync,writeFileSync,unlinkSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { custodyFixture,expectedImage } from './capture-custody-fixture.mjs';
import { createCaptureIntentIO } from './capture-intent-io.mjs';
import { createCaptureCustody } from './capture-custody.mjs';
import { Client,InMemoryTransport } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { createComputerUseServer } from './vendor-runtime.mjs';
const sha=(/** @type {Buffer} */ b)=>createHash('sha256').update(b).digest('hex');
const shell=process.env.NF_CAPTURE_TEST_POWERSHELL;
if(!shell)throw Error('EXPLICIT_CAPTURE_TEST_POWERSHELL_REQUIRED');
const base=new URL('./',import.meta.url);
function setup() {
    const f=custodyFixture('composition-adapter-'),delta=Date.now()-1000000,intent=f.intent;
    for(const name of /** @type {const} */ (['startMs','deadlineMs','callerAdmissionDeadlineMs','originalCallerAdmissionDeadlineMs','effectiveDeadlineMs'])) intent.rootClock[name]+=delta;
    for(const name of /** @type {const} */ (['startMs','lastMs','deadlineMs'])) intent.batchClock[name]+=delta;
    intent.createdMs+=delta;intent.rootCallRecord.value.startMs+=delta;intent.ownedWitness.atMs+=delta;
    intent.ownedWitness.source.startMs+=delta;intent.ownedWitness.source.endMs+=delta;
    intent.producerWitness.effectiveDeadlineMs+=delta;
    for(const name of /** @type {const} */ (['conservativeOuterDeadlineUtc','controllerDeadlineUtc'])) intent.producerWitness[name]=new Date(Date.parse(intent.producerWitness[name])+delta).toISOString().replace(/(\.\d{3})Z$/,'$10000Z');
    unlinkSync(f.intentPath);
    const pointerPath=resolve(f.output,'current.private.json');
    writeFileSync(pointerPath,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_POINTER_V1',status:'UNSET',profileId:f.admitted.profileId})+'\n');
    const configuration={schemaVersion:1,kind:'NATIVE99_CAPTURE_ADAPTER_CONFIGURATION_V1',admitted:f.admitted,pointerPath};
    const configPath=resolve(f.output,'configuration.private.json');writeFileSync(configPath,JSON.stringify(configuration)+'\n');
    const path=(/** @type {string} */ name)=>new URL(name,base).pathname.replace(/^\/([A-Z]:)/,'$1');
    const cli=path('capture-file-cli.mjs');
    const adapter={kind:'NATIVE99_CAPTURE_ADAPTER_CALLER_BINDING_V1',profileId:f.admitted.profileId,sourcePins:f.admitted.sourcePins,
        node:{path:process.execPath,sha256:sha(readFileSync(process.execPath))},cli:{path:cli,sha256:sha(readFileSync(cli))},
        configuration:{path:configPath,sha256:sha(readFileSync(configPath))},pointerSHA256:sha(readFileSync(pointerPath))};
    const state={bindingId:f.admitted.bindings[0].id,initializedBeforeClock:true,pending:[],errors:[],captureAttempts:[],transfers:/** @type {{mode:'publish'|'read'}[]} */ ([]),imageReferences:/** @type {Record<string,{originalPNG:{sha256:string}}>} */ ({}),committedCalls:0,committedQueries:0,frames:[],rawDurabilityGap:false};
    /** @type {Map<string,any>} */
    const values=new Map(Object.entries({native99EvidenceCompositionState:state,feedbackDir:f.output,feedbackWindow:intent.window,
        native99CaptureAdapterBinding:adapter,native99CaptureAsciiHashSource:readFileSync(new URL('capture-ascii-sha256.mjs',base),'utf8').replace('export function','function').replace(/^([\s\S]*?)(function sha256Ascii[\s\S]*)$/,'($2)'),
        native99UiProfileData:{id:f.admitted.profileId},native99UiBatchData:{id:'B1'},native99OwnedRootClock:intent.rootClock,
        native99UiProducerClockWitness:intent.producerWitness,native99UiLastOwnedWitness:intent.ownedWitness,
        feedbackClock:intent.batchClock,feedbackCalls:[],feedbackQueries:[],feedbackFrames:[],feedbackStates:[],feedbackInputs:[],feedbackFileTerminals:[],
        [intent.rootClock.recordKey]:intent.rootCallRecord.value}));
    const binding={...f.admitted.bindings[0],admitted:true,fixtureSourceAdmitted:true,expectedExeDataAdmitted:true,rootPreflightFreshOrdinaryPathsAdmitted:true};
    return {f,configuration,values,state,binding};
}
test('actual composition publishes and reads original through real CLI and MCP, stopping at unsupplied metadata boundary', {timeout:20000},async()=>{
    const {f,configuration,values,state,binding}=setup();
    let native=0;
    const io=createCaptureIntentIO(configuration),custody=createCaptureCustody({admitted:f.admitted,readIntent:io.readIntent});
    const server=createComputerUseServer({...custody,profile:'core',activeProfile:'core',session:/** @type {any} */({dispatch:async()=>{native++;return expectedImage;}})});
    const client=new Client({name:'composition-integration',version:'1'},{capabilities:{},supportedProtocolVersions:['2025-11-25']});
    const [remote,local]=InMemoryTransport.createLinkedPair();
    await server.connect(local);await client.connect(remote);
    const h={now:()=>Date.now(),requireTime:(gate=0)=>assert.ok(values.get('feedbackClock').deadlineMs-Date.now()>=gate),
        call:async(/** @type {string} */ id,/** @type {string} */ tool,/** @type {any} */ args)=>{
            assert.equal(tool,'mcp__starsector_direct__screenshot');
            const startMs=Date.now(),raw=await client.callTool({name:'screenshot',arguments:args}),endMs=Date.now();
            const record={id,args,raw,startMs,endMs};values.set('feedback_'+id,record);values.get('feedbackCalls').push(record);
        },
        shell:async(/** @type {string} */ command)=>{
            const startMs=Date.now(),r=spawnSync(shell,['-NoLogo','-NoProfile','-NonInteractive','-Command',command],{encoding:'utf8',timeout:10000,windowsHide:true});
            assert.equal(r.error,undefined);return {startMs,endMs:Date.now(),terminal:{exit_code:r.status,output:r.stdout},stderr:r.stderr};
        }};
    const context=vm.createContext({load:(/** @type {string} */ key)=>values.get(key),store:(/** @type {string} */ key,/** @type {any} */ v)=>values.set(key,v),
        tools:{apply_patch:()=>{throw Error('GIANT_PATCH_FORBIDDEN');}}});
    const factory=vm.runInContext(readFileSync(new URL('composition-callback.js.txt',base),'utf8'),context);
    const composition=factory({h,write:async()=>{throw Error('UNSUPPLIED_METADATA_BOUNDARY');},verifyMetadataFiles:()=>{}},binding);
    try {
        await assert.rejects(()=>composition.capture('F1'),/UNSUPPLIED_METADATA_BOUNDARY/);
        assert.equal(native,1);
        assert.equal(state.transfers.length,2);
        assert.equal(state.imageReferences['F1-capture'].originalPNG.sha256,sha(readFileSync(f.slot.png)));
        assert.equal(state.transfers[0].mode,'publish');assert.equal(state.transfers[1].mode,'read');
        assert.deepEqual(state.pending,['metadata:'+f.slot.receiptRelative]);
        assert.equal(state.frames.length,0,'metadata not supplied; no completed frame or seal awarded');
    } finally {await client.close();await server.close();}
});
test('invalid publication proof stays pending and stops before native dispatch',async()=>{
    const {values,state,binding}=setup();let captures=0;
    const context=vm.createContext({load:(/** @type {string} */ k)=>values.get(k),store:(/** @type {string} */ k,/** @type {any} */ v)=>values.set(k,v)});
    const factory=vm.runInContext(readFileSync(new URL('composition-callback.js.txt',base),'utf8'),context);
    const composition=factory({h:{now:()=>Date.now(),requireTime:()=>{},call:()=>{captures++;},shell:async()=>({terminal:{exit_code:0,output:'{}'}})},
        write:()=>{},verifyMetadataFiles:()=>{}},binding);
    await assert.rejects(()=>composition.capture('F1'),/EXACT_INTENT_PUBLICATION_PROOF/);
    assert.equal(captures,0);assert.deepEqual(state.pending,['capture-file:publish:F1']);
    await assert.rejects(()=>composition.capture('F1'),/CLOSED_FRESH_FRAME_SLOT/);
});
