// @ts-check
// Generated, closed two-provider history and a separate fresh admission fixture.
import {resolve,basename,relative,isAbsolute} from 'node:path';
import {writeFileSync,readFileSync,rmSync,realpathSync} from 'node:fs';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {generatedBudgetProvider,generatedBudgetAction} from './seed-pre-continue-record-test-support.mjs';
import {generatedBudgetGeometry} from './seed-pre-continue-geometry-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

export const generatedBudgetHistoryKind='ROOT_CLOSED_PRE_CONTINUE_PROVIDER_BUDGET_FAILED_SEED_ATTEMPT_HISTORY';
/** @param {any} [options] */
export function preContinueHistoryFixture(options={}){
    return preStartHistoryFixture({watchdogMs:1800000,historicalBaseline:{overallAttempts:53,uiAttempts:50,reservedActions:91,sentActions:89},
        canonicalRootEnvironment:true,supervisorJobPids:[100,106,107],originalCallerRaw:{chunk_id:'generated-live',wall_time_seconds:1,session_id:777,original_token_count:0,output:''},
        earlyWindowCase:(/** @type {any} */ c)=>build({...c,options})});
}
/** @param {any} c */
function build(c){
    const {s,root,old,future,anchor,iso,powershell,clock}=c,control={schemaVersion:5,profile:'DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'};
    c.indexed=new Set();c.protocol=0;c.hostCalls=[];c.begins=[];c.finishes=[];c.game.supervisorJobPids=[100,106,107,105,101];
    if(c.options.gameBirthOffsetMs!==undefined)c.game.creationTime=iso(c.options.gameBirthOffsetMs);
    const put=(/** @type {string} */ name,/** @type {any} */ data)=>{const path=resolve(s,name);json(path,data);c.indexed.add(path);return path;};c.put=put;
    for(const p of [old.plan,old.phasePlan]){p.campaignControl=control;Object.assign(p.bounds,{ownedGameHardTimeoutMs:1800000,maxScenarioActionsIncludingActivations:9,
        maxCallsPerBatch:32,plannedMutationsWithoutActivation:8,maxPlannedMutationsIncludingActivation:9,optionalActivations:1});}
    for(const l of [old.launchLease,old.phaseLease])Object.assign(l,{campaignControl:control,maxActions:9,absoluteMaxReservedActions:100,historyBaseline:old.plan.historyBaseline,directHostConsent:old.plan.directHostConsent});
    old.campaign.flow=control.profile;old.campaign.maxReservedActions=old.predecessor.maxReservedActions=100;old.plan.absoluteMaxReservedActions=100;
    json(old.grantPath,{schemaVersion:3,kind:'ROOT_ADMITTED_INITIAL_FOCUS_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',historyBaseline:old.plan.historyBaseline,
        maxReservedActions:100,previous:{result:ref(old.priorResultPath),ledger:ref(old.priorLedgerPath)}});
    for(const o of [old.plan,old.launchLease])o.historyAdmission=ref(old.grantPath);
    old.controller.sourcePinsSha256=ref(old.launchLease.supportPinsPath).sha256;
    old.refreshLaunch();c.controllerPin=ref(old.controllerPath);clock.producerRecordKey='controller-start:'+c.controllerPin.sha256;old.campaign.producerWitness.producerRecordKey=clock.producerRecordKey;
    Object.assign(old.predecessor,{plan:ref(old.planPath),lease:ref(old.leasePath),controller:c.controllerPin});
    const dpath=resolve(s,'root-postlaunch-mechanical-delegation.private.json'),d=value(dpath),helpers=old.plan.preclockHelpers;
    Object.assign(d,{planSha256:ref(old.planPath).sha256,leaseSha256:ref(old.leasePath).sha256,historyAdmissionSha256:ref(old.grantPath).sha256});json(dpath,d);
    const q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'",node=realpathSync.native(process.execPath);
    old.campaign.rootCallRecord.value.args={shell:powershell,workdir:root,sandbox_permissions:'require_escalated',yield_time_ms:1000,max_output_tokens:1500,
        cmd:"$env:PATH='"+resolve(node,'..').replaceAll('\\','/')+';'+resolve(powershell,'..').replaceAll('\\','/')+";'+$env:PATH;& "+q(powershell)+
            ' -NoProfile -NonInteractive -File '+q(resolve(s,'launch-owned-caller.private.ps1'))+' -PlanSha256 '+q(d.planSha256)+' -LeaseSha256 '+q(d.leaseSha256)+
            ' -DelegationSha256 '+q(ref(dpath).sha256)+' -HelperSha256 '+q(helpers['construct-actual-phase-inputs.private.mjs'].sha256)+' -HistoryAdmissionSha256 '+q(d.historyAdmissionSha256)};
    json(old.originalLaunchPath,old.predecessor);old.campaign.launchPredecessor=ref(old.originalLaunchPath);put('root-call.actual.private.json',old.campaign.rootCallRecord);
    /** @type {Record<string,any>} */const phases={};c.phases=phases;
    function phase(/** @type {string} */ batch,/** @type {any} */ extra){
        const campaign={...old.campaign,windowId:batch==='B1'?19:20},planPath=put(batch+'-plan.private.json',{...old.plan,campaign,batches:old.plan.batches.filter((/** @type {any} */ row)=>row.id===batch),...extra});
        const leasePath=put(batch+'-lease.private.json',{...old.launchLease,schemaVersion:7,phase:control.profile,campaign,planSha256:ref(planPath).sha256,...extra});
        const observed={at:iso(batch==='B1'?600:19000),sourceSnapshot:{path:resolve(s,'owned-processes-current.private.json'),sha256:'a'.repeat(64)},processes:[{...c.game,mainWindowHandle:batch==='B1'?'19':'20'}]};
        c.options.phaseOwned?.(observed,batch);
        const ownedReceiptPath=put('driver-'+batch+'-owned.private.json',observed);
        const source=ref(put(batch+'-authorization.private.json',{schemaVersion:7,scenario:s,batch,planPath,planSha256:ref(planPath).sha256,leasePath,leaseSha256:ref(leasePath).sha256,ownedReceiptPath,ownedReceiptSha256:ref(ownedReceiptPath).sha256}));
        const binding={schemaVersion:7,batch,runID:basename(s)+'-'+batch,scenario:s,workspaceRoot:root,planSha256:ref(planPath).sha256,ownedPids:[101],campaign,
            sourcePath:source.path,sourceSha256:source.sha256,controllerSha256:c.controllerPin.sha256,originalPlanSha256:ref(old.planPath).sha256,
            ...(batch==='E1'?{b1MenuReceipt:{...extra.b1MenuReceipt,newGameCoordinate:[10,20]}}:{})};
        const claim=ref(put('guard-'+batch+'-consumed.private.json',{sourcePath:source.path,sourceSha256:source.sha256,binding}));
        return{batch,runID:binding.runID,binding,source,claim,claimPin:claim,auth:value(source.path)};
    }
    const b1=phases.B1=generatedBudgetProvider(c,phase('B1',{}),1000,[3000,3100,3200,3300,3400,4000,4100,4200,15000,15100,15200,15300]);
    const play=generatedBudgetAction(c,b1,8,'PLAY',1,92);
    const menuFrame=copy(c,b1,12,2,'B1-after','B1-frame-custody.private.json','B1-after-source-capture.private.json',play);
    const menu=put('B1-menu-inspected.private.json',{schemaVersion:1,kind:'STARSECTOR_B1_MAIN_MENU_INSPECTION',scenario:s,flow:control.profile,rootRecordKey:clock.recordKey,controllerSha256:c.controllerPin.sha256,
        b1:{sourceSha256:b1.source.sha256,claimSha256:b1.claim.sha256,runID:b1.runID,reservationID:play.reservationID},frame:menuFrame,
        inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:anchor+18500+(c.options.menuInspectionOffsetMs??0),state:'STARSECTOR_MAIN_MENU',newGameCoordinate:[10,20]},
        provider:{readySha256:ref(resolve(b1.output,'guarded-http-ready.private.json')).sha256,terminalSha256:ref(b1.terminal.terminalReceipt).sha256,observedClosedAtMs:anchor+18650,providerReaped:true}});
    Object.assign(play,{providerClock:b1.providerClock,menuReceiptSha256:ref(menu).sha256,providerClosedAtMs:anchor+18000,resolvedAtMs:anchor+20000});
    const e1=phases.E1=generatedBudgetProvider(c,phase('E1',{b1MenuReceipt:ref(menu)}),20000,[22000,22100,22200,22300,22400,23000,23100,23200,24000,24100,24200,25000,25100,25200,196000,196100,196200]);
    const entry=generatedBudgetAction(c,e1,8,'NEW_GAME',1,93),generate=generatedBudgetAction(c,e1,14,'GENERATE',2,94);c.actions=[play,entry,generate];
    const frame2=copy(c,e1,11,2,'E1-wizard-2','E1-wizard-2-frame-custody.private.json','E1-wizard-2-source-capture.private.json',entry);
    const inspection=put('E1-wizard-2-inspected.private.json',{schemaVersion:1,kind:'STARSECTOR_SEED_WIZARD_CONTROL',scenario:s,rootRecordKey:clock.recordKey,controllerSha256:c.controllerPin.sha256,
        sourceSha256:e1.source.sha256,claimSha256:e1.claim.sha256,runID:e1.runID,reservationID:entry.reservationID,frame:frame2,
        inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:anchor+24300+(c.options.generateInspectionOffsetMs??0),state:'IDENTITY_EMPTY',controlLabel:'GENERATE',enabled:true,coordinate:[10,20],generatedName:''}});
    generate.wizardStep.inspectionSha256=ref(inspection).sha256;
    Object.assign(entry,{seedWizardInspectionSha256:ref(inspection).sha256,after:{captureReceipt:true,freshness:'UNKNOWN',pngSha256:frame2.pngSha256,window:e1.window,capturedAtMs:frame2.capturedAtMs,observedAtMs:anchor+24450}});
    copy(c,e1,17,3,'E1-wizard-3','E1-wizard-3-frame-custody.private.json','E1-wizard-3-source-capture.private.json',generate);
    const ledger={schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',rootRecordKey:clock.recordKey,controllerSha256:c.controllerPin.sha256,history:{...old.plan.historyBaseline,reservedActions:94,sentActions:92},
        pending:generate,uncertain:false,provider:{batch:'E1',runID:e1.runID,sourceSha256:e1.source.sha256},b1ClaimSha256:b1.claim.sha256,providerClock:e1.providerClock,resolvedB1:play,initialClaimSha256:e1.claim.sha256,resolvedActions:[entry]};
    c.options.ledger?.(ledger);const ledgerPath=put('guard-launch-history.private.json',ledger);
    const generated=generatedBudgetGeometry(c);
    const custody={schemaVersion:1,kind:'ACTUAL_S15_DRIVER_CALL_AND_METADATA_CUSTODY',scope:'Generated historical boundaries only',scenario:s,root,rootGo:true,rootDeadline:clock.effectiveDeadlineMs,cleanupBy:clock.effectiveDeadlineMs-30000,
        begins:c.begins,finishes:c.finishes,calls:c.hostCalls,metadataCommands:generated.commands,inspections:generated.inspections,observedGeneratedName:'Seed Captain',acknowledgedActions:3,initialActivations:0,continueSent:false,e2Started:false,seedCreated:false,ordinaryExit:false};
    put('driver-call-metadata-custody.actual.private.json',custody);
    put('driver-stop-observation.actual.private.json',{schemaVersion:1,kind:'ACTUAL_S15_DRIVER_STOP_OBSERVATION',scenario:s,atMs:anchor+205000,status:'FAILED',reason:'E1_CONTINUE_GEOMETRY_ORIGINAL_PROVIDER_BUDGET_REFUSAL',
        actionsAcknowledged:['PLAY','NEW_GAME','GENERATE'],pendingKnownAction:'GENERATE',continueInputDispatched:false,providerB1:'KNOWN_PASS',providerE1:'UNKNOWN_TERMINAL_SIGTERM_STATUS_NULL_STEPS_EMPTY_NOT_BOUND_TO_CHILD',E2:'NOT_STARTED',vendorCalls:29,originalPNGs:5,activations:0,rootCallerOwner:'/root',noRetry:true});
    const caller=closedRoot(c);
    const protection={at:iso(210000),phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,originalAutosave:{maximumSlots:3,enabled:true},
        privateAutosaveEnabled:false,privateSaveEntries:0,liveGameProcesses:0,originalWrites:false,sharedPreference:value(old.preferencePath)};c.options.protection?.(protection);put('after-smoke-protection.private.json',protection);
    const finalOwned={at:iso(206000),controllerPid:100,controllerCreationTime:old.controller.controllerCreationTime,controllerKernelCreationFileTime:old.controller.controllerKernelCreationFileTime,
        processes:[{...c.game,mainWindowHandle:'20'},{...c.game,pid:105,parentPid:100,executable:resolve(s,'installation/starsector.exe'),mainWindowHandle:'0'}]};put('owned-processes-current.private.json',finalOwned);
    const ids=[100,101,105,106,107,200,201,202,300,301,302];
    put('owned-terminal-pid-absence.actual.private.json',{schemaVersion:1,kind:'ACTUAL_OWNED_TERMINAL_PID_ABSENCE',observedAtUtc:iso(211000),queriedExactRecordedPids:ids,allAbsent:true,liveRows:[],ordinaryExitClaim:false,providerE1Outcome:'UNKNOWN_RETAINED',originalCallerKnownTerminal:true});
    const result={schemaVersion:1,kind:'ACTUAL_FINITE_DRIVER_FLOW_RESULT',scenario:s,sourceHead:'1'.repeat(40),sourceTree:'2'.repeat(40),verdict:'FAIL',state:'FINAL_FAILED_CLEANED',reason:'E1_CONTINUE_GEOMETRY_ORIGINAL_PROVIDER_BUDGET_REFUSAL',
        historyBaseline:old.plan.historyBaseline,currentRunContext:{overallAttempts:54,uiAttempts:51,scope:'Separate spent dispatch context'},finalGuard:ledger.history,absoluteMaxReservedActions:100,unsentReservationsCarried:2,refunds:0,
        actualDeltas:{reservedActions:3,sentActions:3,wizardClicksAcknowledged:3,initialActivationsAcknowledged:0},actionSummary:c.actions.map((/** @type {any} */ a)=>({stage:a.wizardStep.stage,batch:a.batch,...a.args,acknowledged:true})),pendingKnownAction:generate,
        continueSent:false,E2Started:false,vendorCalls:{B1:12,E1:17,E2:0,total:29},originalPNGs:{count:5,references:[...b1.originals,...e1.originals].map(a=>a.png)},
        seed:{created:false,stabilityReceiptCreated:false,createdReceiptCreated:false,modernCloseoutCreated:false,qualified:false},authority:{plan:ref(old.planPath),lease:ref(old.leasePath),B1:b1.source,E1:e1.source,rootDeadlineMs:clock.effectiveDeadlineMs,cleanupByMs:clock.effectiveDeadlineMs-30000,mutated:false},
        evidence:{actualCallsAndMetadata:ref(resolve(s,'driver-call-metadata-custody.actual.private.json')),originalStopObservation:ref(resolve(s,'driver-stop-observation.actual.private.json')),finalLedger:ref(ledgerPath),originalLaunchCaller:ref(resolve(s,'launch-caller-terminal.actual.private.json')),
            afterSmokeProtection:ref(resolve(s,'after-smoke-protection.private.json')),exactOwnedPidAbsence:ref(resolve(s,'owned-terminal-pid-absence.actual.private.json'))},
        failure:{stage:'E1-CONTINUE geometry',commandRecordIndex:4,actualExecChunk:'generated-budget-stop',actualExitCode:1,message:'No original budget for geometry plus bounded cleanup',childGeometryStarted:false,providerOriginalOuterDeadline:e1.terminal.deadline,
            metadataStartedAtMs:anchor+197000,finishStartedAtMs:e1.finish.startedAtMs,finishCompletedAtMs:e1.finish.completedAtMs,noClockExtension:true,noRetry:true,individualNativeTimeoutClaim:false},
        batches:[...['B1','E1'].map(batch=>{const a=c.phases[batch].terminal;return{batch,verdict:a.verdict,providerTerminal:ref(a.terminalReceipt),terminalObserved:true,terminalBoundToChild:a.terminalBoundToChild,cleanupUnknown:false,verifierStatus:a.verifierStatus,
            ...(batch==='E1'?{signal:a.signal,steps:a.steps,continueInputDispatched:false}:{})};}),{batch:'E2',verdict:'NOT_STARTED',providerStarted:false}],
        cleanup:{method:'MAINTAINED_REQUESTED_OWNED_STOP',originalCallerSession:777,actualOriginalTerminalChunk:caller.actual.chunk_id,originalCallerKnownTerminal:true,originalCallerReaped:true,originalCallerExitCode:1,nestedWindowsJobTimedOut:false,rawLaunchVerdict:'FAIL',ordinaryExit:false,allRecordedOwnedPidsAbsent:true,recordedOwnedPidCount:11,providerUnknownOutcomePreserved:true}};
    c.options.result?.(result);put('driver-flow-result.private.json',result);
    for(const name of ['plan.json','native-lane-lease.private.json','controller-start.private.json','campaign-postlaunch.private.json','root-postlaunch-mechanical-delegation.private.json','frozen-support-pins.private.json','source-pins.private.json','fixture-binding.private.json','protected-fixture-receipt.private.json','seed-creation-preference.private.json','root-history-baseline.admitted.private.json'])c.indexed.add(resolve(s,name));
    const indexPath=resolve(s,'budget-history-index.private.json'),closeoutPath=resolve(s,'budget-history.private.json');
    const closeout={schemaVersion:1,kind:generatedBudgetHistoryKind,issuer:'/root',driver:'/root/recovery_game_driver',scenario:s,sourceWorkspace:root,sourceHead:'1'.repeat(40),allOwnedHandlesTerminalReaped:true,
        actions:{actualGuard:ledger.history,currentRunContext:{overallAttempts:54,uiAttempts:51},absoluteMaxReservedActions:100},evidence:{index:{}}};c.options.header?.(closeout);
    function refresh(){
        const files=[...c.indexed].map((/** @type {string} */ path)=>({...ref(path),bytes:readFileSync(path).length})),index={scenario:s,authorityIssuedByIndex:false,operationalPermission:false,newSourceQualifiedByIndex:false,originalFileNamesPreserved:true,
            fileCount:files.length,totalIndexedBytes:files.reduce((sum,row)=>sum+row.bytes,0),files,calls:[...b1.calls,...e1.calls].map(call=>({batch:call.post.batch,ordinal:call.post.innerProtocolID,tool:call.post.tool,args:call.host.args,
                callerTimes:{startedAtMs:call.host.startedAtMs,completedAtMs:call.host.completedAtMs},post:ref(call.prefix+'.private.json'),attempt:ref(call.prefix+'.attempt.private.json'),raw:ref(call.prefix+'.raw.private.json')})),originals:[...b1.originals,...e1.originals]};
        c.options.index?.(index);json(indexPath,index);closeout.evidence.index=ref(indexPath);json(closeoutPath,closeout);
        const baseline={overallAttempts:54,uiAttempts:51,reservedActions:94,sentActions:92};
        json(future.grantPath,{schemaVersion:3,kind:'ROOT_ADMITTED_INITIAL_FOCUS_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',historyBaseline:baseline,maxReservedActions:103,previous:{result:ref(closeoutPath),ledger:ref(ledgerPath)}});
        configureFuture(c,baseline);future.refreshLaunch();
    }
    refresh();
    return{...future,ctx:c,indexPath,closeoutPath,ledgerPath,closeout,result,refresh,priorScenario:s,providers:phases,
        close(){for(const path of c.outputs){const rel=relative(resolve(root,'.tmp/starsector-proxy'),path);if(!rel||rel.startsWith('..')||isAbsolute(rel))throw Error('Fixture output escaped workspace');rmSync(path,{recursive:true});}old.close();future.close();}};
}
/** @param {any} c @param {any} provider @param {number} ordinal @param {number} capture @param {string} label @param {string} frameName @param {string} sourceName @param {any} action */
function copy(c,provider,ordinal,capture,label,frameName,sourceName,action){
    const call=provider.calls[ordinal-1],base=resolve(provider.output,'capture-'+String(capture).padStart(3,'0')),folder=resolve(c.s,label),png=resolve(folder,'F1.original.png'),metadata=resolve(folder,'capture-001.metadata.private.json'),raw=resolve(folder,'capture-001.raw.private.json');
    json(metadata,{...call.post,sourceSha256:provider.source.sha256,reservationID:action.reservationID,capturedAtMs:Date.parse(call.post.callCompleted),window:call.window,dimensions:{width:call.window.bounds.width,height:call.window.bounds.height},rawPath:raw,pngPath:png,pngSha256:ref(base+'.png').sha256,rawSha256:ref(base+'.raw.private.json').sha256,captureReceipt:true});
    writeFileSync(png,readFileSync(base+'.png'));writeFileSync(raw,readFileSync(base+'.raw.private.json'));for(const path of [png,metadata,raw])c.indexed.add(path);
    const frame={pngPath:png,pngSha256:ref(png).sha256,metadataPath:metadata,metadataSha256:ref(metadata).sha256,capturedAtMs:Date.parse(call.post.callCompleted)};c.put(frameName,frame);
    c.put(sourceName,{schemaVersion:1,source:provider.source,claim:provider.claim,sourceCall:ref(call.prefix+'.private.json'),attempt:ref(call.prefix+'.attempt.private.json'),raw:ref(call.prefix+'.raw.private.json'),automaticMetadata:ref(base+'.metadata.private.json'),reservationID:action.reservationID,capturedAtMs:frame.capturedAtMs,frame:ref(resolve(c.s,frameName))});return frame;
}
/** @param {any} c */
function closedRoot(c){
    const {old,s,powershell}=c,step={args:['-NoProfile','-NonInteractive','-File',resolve(c.root,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',s,'-BindingPath',old.plan.fixtureBinding.path,'-BindingSha256',old.plan.fixtureBinding.sha256,'-OuterDeadlineUtc',old.controller.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','1800000'],
        command:powershell,cwd:'.',module:'node:.',processOwnership:'windows-job-object',status:1,stderr:'Driver requested owned cleanup; native success not established',stdout:'',timedOut:false};
    c.put('launch-supervisor.private.json',{schemaVersion:1,command:'verify',ok:true,result:{blocked:[],graphFingerprint:'0'.repeat(64),scope:'module',selection:{changedPaths:[],modules:['node:.']},steps:[step],verdict:'FAIL'}});
    const actual={chunk_id:'generated-root-closed',wall_time_seconds:0.1,exit_code:1,original_token_count:1,output:JSON.stringify({verdict:'FAIL',blocked:[],steps:{status:1,timedOut:false,processOwnership:'windows-job-object',stdout:'',stderr:step.stderr}})},
        caller={schemaVersion:1,kind:'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL',session:777,launchRaw:old.campaign.rootCallRecord.value.raw,launchPolls:[actual],actual,terminalReaped:true};c.put('launch-caller-terminal.actual.private.json',caller);return caller;
}
/** @param {any} c @param {any} baseline */
function configureFuture(c,baseline){
    const f=c.future,control={schemaVersion:5,profile:'DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'},start=f.campaign.rootClock.startMs,cap=start+1740000,outer=new Date(start+1800000).toISOString(),timing={schemaVersion:1,profile:'INITIAL_FOCUS_PROVIDER_480000',providerTimeoutMs:480000};
    json(f.launchLease.supportPinsPath,c.pins());
    for(const o of [f.plan,f.phasePlan,f.launchLease,f.phaseLease])Object.assign(o,{campaignControl:control,historyBaseline:baseline,historyAdmission:ref(f.grantPath),absoluteMaxReservedActions:103,sessionWatchdogMs:1800000,callerAdmissionDeadlineMs:cap,providerTiming:timing});
    for(const p of [f.plan,f.phasePlan])Object.assign(p.bounds,{maxScenarioActionsIncludingActivations:9,maxCallsPerBatch:32,plannedMutationsWithoutActivation:8,maxPlannedMutationsIncludingActivation:9,optionalActivations:1,ownedGameHardTimeoutMs:1800000,proxyTimeoutMs:480000});
    for(const l of [f.launchLease,f.phaseLease])Object.assign(l,{maxActions:9,ownedGameHardTimeoutMs:1800000,supportPinsSha256:ref(f.launchLease.supportPinsPath).sha256});
    Object.assign(f.controller,{sessionWatchdogMs:1800000,hardTimeoutMs:1800000,conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:outer});
    Object.assign(f.campaign.rootClock,{sessionWatchdogMs:1800000,deadlineMs:start+1800000,callerAdmissionDeadlineMs:cap,originalCallerAdmissionDeadlineMs:cap,effectiveDeadlineMs:cap});
    Object.assign(f.campaign.producerWitness,{sessionWatchdogMs:1800000,conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:outer,effectiveDeadlineMs:cap});
    f.campaign.flow=control.profile;f.campaign.historyBaseline=f.predecessor.historyBaseline=baseline;f.campaign.maxReservedActions=f.predecessor.maxReservedActions=103;f.phaseLease.schemaVersion=7;f.phaseLease.phase=control.profile;
}
