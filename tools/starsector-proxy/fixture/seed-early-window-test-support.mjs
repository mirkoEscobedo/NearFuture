// @ts-check
// Generated three-call OS/terminal data. No image, game, native call or usable save.
import {resolve,basename,sep} from 'node:path';
import {mkdirSync,writeFileSync,rmSync,realpathSync,existsSync,unlinkSync} from 'node:fs';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {value,ref} from './seed-failed-history-test-support.mjs';

export const earlyWindowKind='ROOT_CLOSED_EARLY_WINDOW_FOCUS_STOP_HISTORY';
export const earlyWindowFiles=['plan.json','native-lane-lease.private.json','controller-start.private.json','root-call.actual.private.json',
    'campaign-postlaunch.private.json','root-postlaunch-mechanical-delegation.private.json','guard-launch-history.private.json',
    'B1-authorization.private.json','guard-B1-consumed.private.json','owned-processes-current.private.json','launch-caller-terminal.actual.private.json',
    'launch-supervisor.private.json','after-smoke-protection.private.json','owned-terminal-pid-absence.actual.private.json',
    'seed-creation-preference.private.json','frozen-support-pins.private.json','source-pins.private.json','fixture-binding.private.json',
    'protected-fixture-receipt.private.json','root-history-baseline.admitted.private.json','driver-calls.actual.private.json','driver-stop-observation.actual.private.json'];
/** @param {any} [options] */
export function earlyWindowFixture(options={}){
    return preStartHistoryFixture({...options,watchdogMs:1800000,canonicalRootEnvironment:true,
        originalCallerRaw:{chunk_id:'generated-early-live',wall_time_seconds:1,session_id:777,original_token_count:0,output:''},
        supervisorJobPids:[100,105,106,107],historicalBaseline:options.baseline??{overallAttempts:52,uiAttempts:49,reservedActions:91,sentActions:89},earlyWindowCase:build});
}
/** @param {any} c */
function build(c){
    const {s,root,anchor,iso,old,future,powershell,game}=c,b1=c.phase('B1',{}),run=b1.runID;
    // These are inert bootstrap prototypes from the common launch fixture, never claimed phases.
    for(const name of ['E1-authorization.private.json','E1-plan.private.json','E1-lease.private.json','driver-E1-owned.private.json','guard-E1-consumed.private.json']){
        const path=resolve(s,name);if(existsSync(path))unlinkSync(path);
    }
    const output=resolve(root,'.tmp/starsector-proxy',run);mkdirSync(output);c.outputs.push(output);
    const put=(/** @type {string} */ name,/** @type {any} */ v)=>{const path=resolve(s,name);json(path,v);return path;};
    const firstOwned={at:iso(600),controllerPid:100,controllerCreationTime:old.controller.controllerCreationTime,
        controllerKernelCreationFileTime:old.controller.controllerKernelCreationFileTime,processes:[{...game,mainWindowHandle:c.options.handle??'19'}]};
    put('owned-processes-current.private.json',firstOwned);
    const auth=value(b1.source.path);
    json(auth.ownedReceiptPath,{at:iso(600),sourceSnapshot:ref(resolve(s,'owned-processes-current.private.json')),processes:firstOwned.processes});
    auth.ownedReceiptSha256=ref(auth.ownedReceiptPath).sha256;json(b1.source.path,auth);b1.source=ref(b1.source.path);
    b1.binding.sourceSha256=b1.source.sha256;json(b1.claim.path,{sourcePath:b1.source.path,sourceSha256:b1.source.sha256,binding:b1.binding});b1.claim=ref(b1.claim.path);
    const packet=(/** @type {any} */ data)=>({content:[{type:'text',text:JSON.stringify(data)}],structuredContent:data});
    const clock={startMs:anchor+1000,deadlineMs:anchor+181000};
    const ledgerPath=put('guard-launch-history.private.json',{schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',
        rootRecordKey:c.clock.recordKey,controllerSha256:c.controllerPin.sha256,history:old.plan.historyBaseline,pending:null,uncertain:false,
        provider:{batch:'B1',runID:run,sourceSha256:b1.source.sha256},b1ClaimSha256:b1.claim.sha256,providerClock:clock});
    const ready={schemaVersion:1,at:iso(2000),runID:run,batch:'B1',sourceSha256:b1.source.sha256,startupTimeoutMs:20000,
        consentMode:'finite-explicit-consent',node:{pid:200,parentPid:202,sessionId:1,creationTime:iso(1500),executable:realpathSync.native(process.execPath)},
        helper:{pid:201,parentPid:200,sessionId:1,creationTime:iso(1700),executable:powershell},supervisorJobPids:[200,201],
        query:'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'};
    json(resolve(output,'guarded-http-ready.private.json'),ready);
    const child={command:ready.node.executable,args:[resolve(root,'tools/starsector-proxy/guarded-http-child.mjs')],cwd:'.',
        processOwnership:'windows-job-object',status:0,timedOut:false};
    const terminal={at:iso(10000),verifierPid:202,verifierStatus:0,signal:null,deadline:iso(180000),closeFailed:false,overflow:false,spawnFailed:false,
        terminalObserved:true,cleanupUnknown:false,steps:[child],terminalBoundToChild:true,verdict:'PASS',terminalReceipt:resolve(output,'owned-provider-terminal.private.json')};
    json(terminal.terminalReceipt,terminal);json(resolve(output,'owned-provider.stdout.private.txt'),{schemaVersion:1,command:'verify',ok:true,
        result:{verdict:'PASS',steps:[child]}});writeFileSync(resolve(output,'owned-provider.stderr.private.txt'),'');
    writeFileSync(resolve(output,'batch-dispatch.private.jsonl'),'');
    const diagnostic=c.options.diagnosticOwnedJava?{bundleId:'java.exe',displayName:'Earlier Java diagnostic',pid:101}:
        {bundleId:'diagnostic.exe',displayName:'Earlier diagnostic',pid:300};
    const window={bounds:{height:1,width:1,x:10,y:20},bundleId:'java.exe',displayName:'java.exe',isFocused:false,isOnScreen:true,
        pid:101,title:'Generated launcher',windowId:19};
    const rows=[['doctor',{include_remediation:false},{ok:true,checks:[{id:'ui_automation',details:{frontmost:diagnostic}}]}],
        ['policy_status',{}, {approval_required_for_all:true,approval_token_configured:false}],['get_window',{window_id:19},window]];
    /** @type {any[]} */const calls=[];
    /** @type {any[]} */const pins=[];
    for(const [i,[name,args,data]] of rows.entries()){
        const post={tool:name,args,outerProtocolID:i+1,innerProtocolID:i+1,callStarted:iso(4000+i*1000),callCompleted:iso(4100+i*1000),
            precedingActions:[],runID:run,batch:'B1',freshness:'UNKNOWN',isError:false},raw=packet(data);
        const prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));json(prefix+'.private.json',post);
        json(prefix+'.attempt.private.json',{tool:name,args,outerProtocolID:i+1,innerProtocolID:'UNKNOWN',callStarted:post.callStarted,
            sourceSha256:b1.source.sha256,runID:run,batch:'B1'});json(prefix+'.raw.private.json',raw);
        pins.push({post:ref(prefix+'.private.json'),attempt:ref(prefix+'.attempt.private.json'),raw:ref(prefix+'.raw.private.json')});
        calls.push({name,args,startedAtMs:anchor+3900+i*1000,raw,completedAtMs:anchor+4200+i*1000});
    }
    const terminalPacket={content:[{type:'text',text:JSON.stringify(terminal)}]};
    const batches={B1:{begin:{tool:'begin_batch',args:{consentMode:'finite-explicit-consent',runID:run,source:b1.source.path},
        startedAtMs:anchor+900,raw:{content:[{type:'text',text:JSON.stringify({runID:run,output,deadline:terminal.deadline})}]},completedAtMs:anchor+2500},
        calls,finish:{args:{},startedAtMs:anchor+7000,raw:terminalPacket,completedAtMs:anchor+10100}}};
    const metadataFailure={kind:'ACTUAL_METADATA_WRITE_PRECREATION_FAILURE',reason:'Windows CreateProcess error206 filename/extension too long; unified exec process not created',gameOrProviderEffects:false};
    put('driver-calls.actual.private.json',{schemaVersion:1,kind:'GENERIC_ACTUAL_FINITE_DRIVER_CALL_RECORDS',scenario:s,driver:'/root/recovery_game_driver',
        batches,noOperationalHelperModes:true,metadataWritePrecreationFailure:metadataFailure});
    const baseline=old.plan.historyBaseline,context={overallAttempts:53,uiAttempts:50},max=99;
    put('driver-stop-observation.actual.private.json',{schemaVersion:1,kind:'GENERIC_ACTUAL_DRIVER_STOP_OBSERVATION',scenario:s,driver:'/root/recovery_game_driver',
        status:'FOCUS_BLOCKED',observedAtMs:anchor+7000,historyBaseline:baseline,currentRunContext:context,absoluteMaxReservedActions:max,
        vendorCalls:3,inputs:0,reservations:0,originalPNGs:0,batches,failure:{kind:'FOCUS_BLOCKED',atMs:anchor+7000,actualWindow:calls[2].raw,
            reason:'Actual owned launcher get_window isFocused false; no activation allowed; zero inputs'},rootCleanupRequested:true,rootCleanupKnown:false,
        seedAccepted:false,ordinaryExit:false});
    const step={args:['-NoProfile','-NonInteractive','-File',resolve(root,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',s,
        '-BindingPath',old.plan.fixtureBinding.path,'-BindingSha256',old.plan.fixtureBinding.sha256,'-OuterDeadlineUtc',old.controller.conservativeOuterDeadlineUtc,
        '-SessionWatchdogMs','1800000'],command:powershell,cwd:'.',module:'node:.',processOwnership:'windows-job-object',status:1,
        stderr:'Driver requested owned cleanup; native success not established',stdout:'',timedOut:false};
    put('launch-supervisor.private.json',{schemaVersion:1,command:'verify',ok:true,result:{blocked:[],graphFingerprint:'0'.repeat(64),scope:'module',
        selection:{changedPaths:[],modules:['node:.']},steps:[step],verdict:'FAIL'}});
    const rootCall=value(resolve(s,'root-call.actual.private.json'));
    const actual={chunk_id:'generated-early-terminal',wall_time_seconds:0.1,exit_code:1,original_token_count:1,
        output:JSON.stringify({verdict:'FAIL',blocked:[],steps:{status:1,timedOut:false,processOwnership:'windows-job-object',stdout:'',stderr:step.stderr}})};
    put('launch-caller-terminal.actual.private.json',{schemaVersion:1,kind:'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL',session:777,
        launchRaw:rootCall.value.raw,launchPolls:[actual],actual,terminalReaped:true});
    put('owned-processes-current.private.json',{at:iso(11000),controllerPid:100,controllerCreationTime:old.controller.controllerCreationTime,
        controllerKernelCreationFileTime:old.controller.controllerKernelCreationFileTime,processes:[{...game,mainWindowHandle:c.options.handle??'19'}]});
    const ids=[100,101,105,106,107,200,201,202];
    put('owned-terminal-pid-absence.actual.private.json',{schemaVersion:1,kind:'ACTUAL_EXACT_OWNED_PID_ABSENCE',observedAtUtc:iso(13000),
        queriedExactRecordedPids:ids,liveRows:[],allAbsent:true,scope:'Generated exact recorded union'});
    const protection={at:iso(12000),phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,
        saveCensusUnchanged:true,originalAutosave:{enabled:true,maximumSlots:3},privateAutosaveEnabled:false,privateSaveEntries:0,
        liveGameProcesses:0,originalWrites:false,sharedPreference:value(old.preferencePath),restoration:'UNNECESSARY_ORIGINALS_UNCHANGED'};
    put('after-smoke-protection.private.json',protection);
    const aliases={calls:'driver-calls.actual.private.json',stop:'driver-stop-observation.actual.private.json',plan:'plan.json',lease:'native-lane-lease.private.json',
        delegation:'root-postlaunch-mechanical-delegation.private.json',ledger:'guard-launch-history.private.json',B1source:'B1-authorization.private.json',
        B1ready:null,B1terminal:null,launchCaller:'launch-caller-terminal.actual.private.json',afterSmoke:'after-smoke-protection.private.json',pidAbsence:'owned-terminal-pid-absence.actual.private.json'};
    const result={schemaVersion:1,kind:'GENERIC_ACTUAL_DRIVER_FLOW_RESULT',scenario:s,sourceHead:'1'.repeat(40),driver:'/root/recovery_game_driver',
        verdict:'FAIL',status:'FINAL_FAILED_CLEANED',failure:'FOCUS_BLOCKED',failureAtMs:anchor+7000,historyBaseline:baseline,currentRunContext:context,
        absoluteMaxReservedActions:max,counts:{providers:1,vendorCalls:3,screenshots:0,originalPNGs:0,geometryModes:0,operationalHelperModes:0,inputs:0,newReservations:0,sentActions:0},
        callOrder:['doctor','policy_status','get_window'],focus:{ownedPid:101,ownedHwnd:19,ownedWindowFocused:false,doctorFrontmost:diagnostic,
            cause:'No cause inferred beyond actual observations',activationAllowed:false},beforeImage:{path:null,sha256:null,available:false,
            reason:'Stopped at actual owned get_window isFocused=false before screenshot dispatch'},B1:{status:'FOCUS_BLOCKED_BEFORE_INPUT',providerClosure:'PASS'},
        E1:{status:'NOT_STARTED'},E2:{status:'NOT_STARTED'},ordinaryExit:false,forcedOwnedCleanup:true,rawLaunchVerdict:'FAIL',seedCreated:false,seedAccepted:false,
        stabilityObserved:false,finalCallerInvoked:false,noCurrentClockRenewal:true,originalRootDeadlineMs:old.plan.callerAdmissionDeadlineMs,
        rootPostSmokeWrapperFailure:{rawCall:'generated-postwrapper',scope:'Stale/null LASTEXITCODE after maintained protection PASS; exact PID query separate; no rerun'},
        metadataPreservationFailure:metadataFailure,typedHistoryAdmissionCreated:false,allHistoricalFailuresAndSpentBudgetsRetained:true,
        next:'HOLD; no retry, input, helper mode or authority from this result',evidence:{},cleanup:{rootOriginalSession:777,actualOuterExit:1,actualNestedStatus:1,
            timedOut:false,processOwnership:'windows-job-object',reason:step.stderr,providerKnownTerminalReaped:true,rootCallerKnownTerminalReaped:true,
            allExactRecordedPidsAbsent:true,allExactRecordedPids:ids,original182AndBackupHashesUnchanged:true,originalSaveCensus178Unchanged:true,
            originalAutosaveSlots:3,privateAutosaveOff:true,privateSaveEntries:0,liveGames:0,sharedPreferenceUnchanged:true,originalWrites:false,
            scope:'Known recorded union only'}};
    const resultPath=resolve(s,'driver-flow-result.private.json'),closeoutPath=resolve(s,'early-window-history.private.json');
    const providerNames=['guarded-http-ready.private.json','owned-provider-terminal.private.json','owned-provider.stdout.private.txt','owned-provider.stderr.private.txt','batch-dispatch.private.jsonl'];
    const closeout={schemaVersion:1,kind:earlyWindowKind,issuer:'/root',driver:'/root/recovery_game_driver',scenario:s,sourceWorkspace:root,
        sourceHead:'1'.repeat(40),allOwnedHandlesTerminalReaped:true,actions:{actualGuard:baseline,currentRunContext:context,absoluteMaxReservedActions:max},
        evidence:{result:{},files:{},provider:{},calls:pins}};
    function refresh(){
        result.evidence=Object.fromEntries(Object.entries(aliases).map(([key,name])=>[key,ref(name?resolve(s,name):resolve(output,key==='B1ready'?'guarded-http-ready.private.json':'owned-provider-terminal.private.json'))]));
        json(resultPath,result);closeout.evidence.result=ref(resultPath);closeout.evidence.files=Object.fromEntries(earlyWindowFiles.map(name=>[name,ref(resolve(s,name))]));
        closeout.evidence.provider=Object.fromEntries(providerNames.map(name=>[name,ref(resolve(output,name))]));json(closeoutPath,closeout);
        const next={...baseline,...context};json(future.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',
            historyBaseline:next,maxReservedActions:max,previous:{result:ref(closeoutPath),ledger:ref(ledgerPath)}});
        json(future.launchLease.supportPinsPath,c.pins());
        for(const x of [future.plan,future.launchLease,future.phasePlan,future.phaseLease])Object.assign(x,{historyBaseline:next,absoluteMaxReservedActions:max,historyAdmission:ref(future.grantPath)});
        future.launchLease.supportPinsSha256=future.phaseLease.supportPinsSha256=ref(future.launchLease.supportPinsPath).sha256;
        future.campaign.historyBaseline=future.predecessor.historyBaseline=next;future.campaign.maxReservedActions=future.predecessor.maxReservedActions=max;future.refreshLaunch();
    }
    refresh();
    /** Reclose the raw API copy, caller trace, source post/attempt and every outer digest. @param {number} index @param {(row:any)=>void} change */
    function reviseCall(index,change){
        const oldPins=pins[index],post=value(oldPins.post.path),attempt=value(oldPins.attempt.path),raw=value(oldPins.raw.path),call=calls[index];
        change({post,attempt,raw,call});attempt.args=post.args;call.args=post.args;call.raw=raw;
        json(oldPins.post.path,post);json(oldPins.attempt.path,attempt);json(oldPins.raw.path,raw);
        pins[index]={post:ref(oldPins.post.path),attempt:ref(oldPins.attempt.path),raw:ref(oldPins.raw.path)};
        const stop=value(resolve(s,'driver-stop-observation.actual.private.json')),records=value(resolve(s,'driver-calls.actual.private.json'));
        stop.batches=batches;stop.failure.actualWindow=calls[2].raw;records.batches=batches;
        json(resolve(s,'driver-stop-observation.actual.private.json'),stop);json(resolve(s,'driver-calls.actual.private.json'),records);refresh();
    }
    return{...future,closeout,closeoutPath,result,resultPath,priorScenario:s,ledgerPath,output,ctx:c,refresh,reviseCall,
        close(){if(!output.startsWith(resolve(root,'.tmp')+sep))throw Error('Fixture escaped workspace');rmSync(output,{recursive:true});old.close();future.close();}};
}
