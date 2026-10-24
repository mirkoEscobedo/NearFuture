// @ts-check
// Generated OS observations and scene declarations only; no native/seed qualification.
import {mkdirSync,readFileSync,writeFileSync,realpathSync,rmSync} from 'node:fs';
import {resolve,relative,isAbsolute} from 'node:path';
import assert from 'node:assert/strict';
import {persistDirectFrame} from '../direct-frames.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {completedSeedLaunch} from './campaign-seed-handoff-test-support.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';

/** Starts the real finite guard with generated external observations, then New Game.
 * @param {string} runID @param {(f:any,h:ReturnType<typeof seedInputSession>,effects:string[])=>Promise<void>} operation */
export async function withGenerationEntry(runID,operation) {
    const names=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const prior=names.map(name=>process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>>|undefined} */let f;
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];
    /** @type {string[]} */const privateTestOutputs=[];
    try {
        f=await completedSeedLaunch({generation:true});
        const authority={...f,runID,privateTestOutputs};
        claimBatchAuthorization(readBatchAuthorization(authority.source,{root:f.root,runID}));
        h=seedInputSession(authority,tool=>effects.push(tool),{window:()=>({...seedTarget,windowId:21})});
        await h.before();await h.call('left_click',{target_window_id:21,target_app:'java.exe',
            focus_strategy:'strict',coordinate:[40,80]});
        await operation(authority,h,effects);
    } finally {
        await h?.embedded.server.close();
        if(f)for(const output of privateTestOutputs){
            const suffix=relative(resolve(f.root,'.tmp/starsector-proxy'),output);
            assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));rmSync(output,{recursive:true});
        }
        await f?.close();
        names.forEach((name,index)=>{if(prior[index]===undefined)delete process.env[name];else process.env[name]=prior[index];});
    }
}

/** Generated provider terminal/scene evidence only, never a real OS cleanup claim.
 * @param {any} f @param {ReturnType<typeof seedInputSession>} h */
export async function completeGeneratedNameEntry(f,h) {
    await inspectGenerate(f,h);
    await h.call('left_click',{target_window_id:21,target_app:'java.exe',focus_strategy:'strict',coordinate:[70,90]});
    const window={...seedTarget,windowId:21};
    await h.call('get_window',{window_id:21});
    const args={target_window_id:21,quality:0,width:480};
    const result=await h.call('screenshot',args),capturedAtMs=Date.now();
    const state=f.ledger(),action=state.pending;
    const output=resolve(f.scenario,'E1-generated-name');mkdirSync(output);
    const frame=persistDirectFrame({output,label:'capture-001',pngName:'F1.original.png',result,args,window,ownedPids:[101],
        provenance:{runID:f.runID,batch:'E1',sourceSha256:filePin(f.source).sha256.toLowerCase(),
            reservationID:action.reservationID,capturedAtMs}});
    assert.ok(frame.pngPath);
    await h.embedded.server.close();
    const providerOutput=resolve(f.root,'.tmp/starsector-proxy',f.runID);mkdirSync(providerOutput);
    f.privateTestOutputs.push(providerOutput);
    const creationTime=new Date(state.providerClock.startMs).toISOString(),nodeExecutable=realpathSync.native(process.execPath);
    const helper=f.ready.helper.executable;
    const child={command:nodeExecutable,args:[resolve(f.root,'tools/starsector-proxy/guarded-http-child.mjs')],
        cwd:'.',processOwnership:'windows-job-object',status:0,timedOut:false};
    const readyPath=resolve(providerOutput,'guarded-http-ready.private.json');
    json(readyPath,{schemaVersion:1,at:creationTime,runID:f.runID,batch:'E1',sourceSha256:filePin(f.source).sha256.toLowerCase(),
        startupTimeoutMs:20000,consentMode:'finite-explicit-consent',
        node:{pid:39012,parentPid:39011,creationTime,executable:nodeExecutable,sessionId:1},
        helper:{pid:39013,parentPid:39012,creationTime,executable:helper,sessionId:1},
        supervisorJobPids:[39012,39013],query:'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'});
    const terminalPath=resolve(providerOutput,'owned-provider-terminal.private.json');
    json(terminalPath,{at:new Date().toISOString(),verifierPid:39011,verifierStatus:0,signal:null,
        deadline:new Date(state.providerClock.deadlineMs).toISOString(),closeFailed:false,overflow:false,spawnFailed:false,
        terminalObserved:true,cleanupUnknown:false,terminalBoundToChild:true,verdict:'PASS',steps:[child],terminalReceipt:terminalPath});
    json(resolve(providerOutput,'owned-provider.stdout.private.txt'),{ok:true,command:'verify',result:{verdict:'PASS',steps:[child]}});
    writeFileSync(resolve(providerOutput,'owned-provider.stderr.private.txt'),'');
    const sha=(/** @type {string} */ path)=>filePin(path).sha256.toLowerCase();
    const receipt={schemaVersion:1,kind:'STARSECTOR_E1_GENERATED_NAME_INSPECTION',scenario:f.scenario,
        flow:f.plan.campaign.flow,rootRecordKey:f.campaign.rootClock.recordKey,controllerSha256:sha(f.controllerPath),
        e1:{sourceSha256:sha(f.source),claimSha256:sha(resolve(f.scenario,'guard-E1-consumed.private.json')),
            runID:f.runID,newGameReservationID:state.resolvedActions[0].reservationID,generateReservationID:action.reservationID},
        frame:{pngPath:frame.pngPath,pngSha256:sha(frame.pngPath),metadataPath:frame.metadataPath,
            metadataSha256:sha(frame.metadataPath),capturedAtMs},
        inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:Date.now(),
            state:'NEW_GAME_IDENT_CHIP_GENERATED_NAME',generatedName:'Generated Captain',continueLabel:'1. Continue',continueCoordinate:[100,120]},
        provider:{readySha256:sha(readyPath),terminalSha256:sha(terminalPath),observedClosedAtMs:Date.now(),providerReaped:true}};
    const receiptPath=resolve(f.scenario,'E1-generated-name-inspected.private.json');json(receiptPath,receipt);
    const e1GeneratedNameReceipt={path:receiptPath,sha256:sha(receiptPath)};
    const planPath=resolve(f.scenario,'E2-plan.private.json'),leasePath=resolve(f.scenario,'E2-lease.private.json');
    json(planPath,{...f.plan,batches:[{id:'E2'}],e1GeneratedNameReceipt});
    json(leasePath,{...f.lease,planSha256:sha(planPath),e1GeneratedNameReceipt});
    const ownedReceiptPath=resolve(f.scenario,'driver-E2-owned.private.json');
    json(ownedReceiptPath,JSON.parse(readFileSync(resolve(f.scenario,'driver-E1-owned.private.json'),'utf8')));
    const source=resolve(f.scenario,'E2-authorization.private.json');
    json(source,{schemaVersion:5,scenario:f.scenario,batch:'E2',planPath,planSha256:sha(planPath),leasePath,leaseSha256:sha(leasePath),
        ownedReceiptPath,ownedReceiptSha256:sha(ownedReceiptPath)});
    return {...f,source,runID:f.runID+'-E2',receiptPath,receipt,planPath,leasePath};
}

/** @param {any} f @param {any} h */
export async function inspectGenerate(f,h) {
    const window={pid:101,windowId:21,title:'Starsector',bundleId:'java.exe',isFocused:true,isOnScreen:true,
        bounds:{x:10,y:10,width:480,height:220}};
    await h.call('get_window',{window_id:21});
    const args={target_window_id:21,quality:0,width:480};
    const result=await h.call('screenshot',args),capturedAtMs=Date.now();
    const action=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8')).pending;
    const output=resolve(f.scenario,'E1-generate');mkdirSync(output);
    const frame=persistDirectFrame({output,label:'capture-001',pngName:'F1.original.png',result,args,window,
        ownedPids:[101],provenance:{runID:f.runID,batch:'E1',sourceSha256:filePin(f.source).sha256.toLowerCase(),
            reservationID:action.reservationID,capturedAtMs}});
    const path=resolve(f.scenario,'E1-generate-inspected.private.json');
    const receipt={schemaVersion:1,kind:'STARSECTOR_NEW_GAME_GENERATE_INSPECTION',scenario:f.scenario,
        rootRecordKey:f.campaign.rootClock.recordKey,controllerSha256:filePin(f.controllerPath).sha256.toLowerCase(),
        sourceSha256:filePin(f.source).sha256.toLowerCase(),
        claimSha256:filePin(resolve(f.scenario,'guard-E1-consumed.private.json')).sha256.toLowerCase(),
        runID:f.runID,reservationID:action.reservationID,
        frame:{pngPath:frame.pngPath,pngSha256:filePin(/** @type {string} */ (frame.pngPath)).sha256.toLowerCase(),metadataPath:frame.metadataPath,
            metadataSha256:filePin(frame.metadataPath).sha256.toLowerCase(),capturedAtMs},
        inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:Date.now(),
            state:'NEW_GAME_IDENT_CHIP_BLANK_NAME',nameBlank:true,generateLabel:'GENERATE',generateCoordinate:[70,90]}};
    json(path,receipt);
    await h.call('get_frontmost_app',{});await h.call('get_window',{window_id:21});
    return {path,receipt};
}
