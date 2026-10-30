// @ts-check
// Synthetic old/future authority only. Never native evidence or a usable save.
import {resolve,basename} from 'node:path';
import {readFileSync,writeFileSync,mkdirSync,rmSync,realpathSync} from 'node:fs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {json,powershell as runPowerShell} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {requiredOwnedHttpFiles} from '../owned-http-batch.mjs';
import {generatedPreStartProvider,generatedPreStartAction,generatedPreStartCopy,preStartPreference} from './seed-pre-start-record-test-support.mjs';
import {generatedPreStartGeometry} from './seed-pre-start-geometry-test-support.mjs';

/** @param {{copiedFrameTimeOffsetMs?:number,e2Helper?:string,e2Query?:string,e2DeadlineOffsetMs?:number,
 * omitOwnedGame?:boolean,lastClickDelayMs?:number,rootCommand?:string,finalSnapshotExtraPid?:number,watchdogMs?:number,e2ActionCount?:number,
 * historicalBaseline?:{overallAttempts:number,uiAttempts:number,reservedActions:number,sentActions:number},supervisorJobPids?:number[],canonicalRootEnvironment?:boolean,phasePlanExtra?:Record<string,any>,
 * prepareHistoricalCase?:(context:any)=>void,finishHistoricalCase?:(context:any)=>any,earlyWindowCase?:(context:any)=>any,originalCallerRaw?:any}} [options] */
export function preStartHistoryFixture(options={}) {
    const old=/** @type {any} */(seedCreationFixture()),future=/** @type {any} */(seedCreationFixture());
    const watchdog=options.watchdogMs??480000,callerBudget=watchdog-60000;
    const s=old.scenario,root=old.root,anchor=Date.now()-400000,iso=(/** @type {number} */ n)=>new Date(anchor+n).toISOString();
    const baseline=options.historicalBaseline??{overallAttempts:48,uiAttempts:45,reservedActions:76,sentActions:74},maximum=baseline.reservedActions+8;
    const powershell=realpathSync.native(process.env.NF_TEST_POWERSHELL??runPowerShell(['-Command','(Get-Process -Id $PID).Path']).stdout.trim());
    const supervisor=resolve(root,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe');
    const pins=()=>({pins:[...new Set([...requiredOwnedHttpFiles(root,powershell,supervisor),
        ...['prepare-fixture.ps1','run-native.ps1','read-owned-client-geometry.ps1'].map(n=>resolve(root,'tools/starsector-proxy/fixture',n))])].map(ref)});
    const priorGuard={...baseline,overallAttempts:baseline.overallAttempts-1,uiAttempts:baseline.uiAttempts-1};
    json(old.priorLedgerPath,{schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',history:priorGuard,uncertain:false});
    json(old.priorResultPath,{schemaVersion:1,kind:'ACTUAL_SCHEMA5_COMMON_PREFERENCE_PREFIX_DRIVER_CLOSEOUT',allOwnedHandlesTerminalReaped:true,
        actions:{actualGuard:priorGuard,currentRunContext:{overallAttempts:baseline.overallAttempts,uiAttempts:baseline.uiAttempts},absoluteMaxReservedActions:baseline.reservedActions}});
    json(old.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',
        historyBaseline:baseline,maxReservedActions:maximum,previous:{result:ref(old.priorResultPath),ledger:ref(old.priorLedgerPath)}});
    const pinsPath=resolve(s,'frozen-support-pins.private.json');json(pinsPath,pins());
    writeFileSync(resolve(s,'source-pins.private.json'),readFileSync(pinsPath));
    for(const v of [old.plan,old.launchLease,old.phasePlan,old.phaseLease])Object.assign(v,{historyBaseline:baseline,
        absoluteMaxReservedActions:maximum,historyAdmission:ref(old.grantPath),sessionWatchdogMs:watchdog,callerAdmissionDeadlineMs:anchor+callerBudget});
    for(const v of [old.plan,old.phasePlan])v.bounds.ownedGameHardTimeoutMs=watchdog;
    for(const v of [old.launchLease,old.phaseLease])v.ownedGameHardTimeoutMs=watchdog;
    old.plan.source={qualifiedHead:'1'.repeat(40),supportPins:ref(pinsPath)};old.plan.preclockHelpers={};
    old.plan.fixtureBinding=ref(old.bindingPath);
    for(const name of ['launch-owned-caller.private.ps1','record-root-call-and-stage-B1.private.ps1','launch-and-stage.orchestration.proposed.private.js',
        'construct-actual-phase-inputs.private.mjs','driver-timing-capture.unissued.private.json','seed-save-closeout.template.private.mjs',
        'root-freeze-and-preflight.seed.proposed.private.ps1','frame-menu-phase.failfast.proposed.private.ps1','read-client-geometry.owned.private.mjs']) {
        const path=resolve(s,name);writeFileSync(path,'Synthetic historical helper identity; never executed.');old.plan.preclockHelpers[name]=ref(path);
    }
    old.plan.directHostConsent={schemaVersion:1,mode:'finite-explicit-consent',maxCallsPerBatch:32};
    old.launchLease.supportPinsSha256=old.phaseLease.supportPinsSha256=ref(pinsPath).sha256;
    Object.assign(old.controller,{at:iso(200),controllerCreationTime:iso(100),conservativeOuterDeadlineUtc:iso(watchdog),
        controllerDeadlineUtc:iso(watchdog-10000),supervisorJobPids:options.supervisorJobPids??[100,105]});
    const clock=old.campaign.rootClock;
    Object.assign(clock,{startMs:anchor,deadlineMs:anchor+watchdog,callerAdmissionDeadlineMs:anchor+callerBudget,
        originalCallerAdmissionDeadlineMs:anchor+callerBudget,effectiveDeadlineMs:anchor+callerBudget});
    Object.assign(old.campaign.producerWitness,{controllerCreationTime:iso(100),conservativeOuterDeadlineUtc:iso(watchdog),
        controllerDeadlineUtc:iso(watchdog-10000),effectiveDeadlineMs:anchor+callerBudget});
    if(options.watchdogMs!==undefined){
        old.controller.sessionWatchdogMs=watchdog;old.controller.hardTimeoutMs=watchdog;
        clock.sessionWatchdogMs=watchdog;old.campaign.producerWitness.sessionWatchdogMs=watchdog;
    }
    Object.assign(old.campaign,{historyBaseline:baseline,maxReservedActions:maximum});
    old.campaign.rootCallRecord.value.startMs=anchor;
    if(options.originalCallerRaw)old.campaign.rootCallRecord.value.raw=options.originalCallerRaw;
    old.refreshLaunch();
    const cp=ref(old.controllerPath),producerRecordKey='controller-start:'+cp.sha256;
    clock.producerRecordKey=producerRecordKey;old.campaign.producerWitness.producerRecordKey=producerRecordKey;
    old.predecessor.historyBaseline=baseline;old.predecessor.maxReservedActions=maximum;
    old.refreshLaunch();
    Object.assign(old.predecessor,{plan:ref(old.planPath),lease:ref(old.leasePath),controller:ref(old.controllerPath)});
    const h=old.plan.preclockHelpers,delegationPath=resolve(s,'root-postlaunch-mechanical-delegation.private.json');
    json(delegationPath,{schemaVersion:1,kind:'ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION',issuer:'/root',driver:'/root/recovery_game_driver',
        planSha256:ref(old.planPath).sha256,leaseSha256:ref(old.leasePath).sha256,helperSha256:h['construct-actual-phase-inputs.private.mjs'].sha256,
        callerSha256:h['launch-owned-caller.private.ps1'].sha256,orchestrationSha256:h['launch-and-stage.orchestration.proposed.private.js'].sha256,
        stagerSha256:h['record-root-call-and-stage-B1.private.ps1'].sha256,historyAdmissionSha256:ref(old.grantPath).sha256,
        allowedModes:['init','phase B1'],allowed:'Synthetic historical init and B1 only',limits:'Original480000 and unchanged finite limits',
        forbidden:'No extra input or clock renewal',actualLaunch:'Synthetic historical boundary data',atUtc:iso(-1)});
    const q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    old.campaign.rootCallRecord.value.args={cmd:options.rootCommand??'& '+q(powershell)+' -NoProfile -NonInteractive -File '+q(resolve(s,'launch-owned-caller.private.ps1'))+
        ' -PlanSha256 '+q(ref(old.planPath).sha256)+' -LeaseSha256 '+q(ref(old.leasePath).sha256)+' -DelegationSha256 '+q(ref(delegationPath).sha256)+
        ' -HelperSha256 '+q(h['construct-actual-phase-inputs.private.mjs'].sha256)+' -HistoryAdmissionSha256 '+q(ref(old.grantPath).sha256),
        workdir:root,shell:powershell.replaceAll('\\','/'),sandbox_permissions:'require_escalated',yield_time_ms:1000,max_output_tokens:1500};
    if(options.canonicalRootEnvironment)old.campaign.rootCallRecord.value.args.cmd="$env:PATH='"+resolve(realpathSync.native(process.execPath),'..').replaceAll('\\','/')+';'+
        resolve(powershell,'..').replaceAll('\\','/')+";'+$env:PATH;"+old.campaign.rootCallRecord.value.args.cmd;
    json(old.originalLaunchPath,old.predecessor);old.campaign.launchPredecessor=ref(old.originalLaunchPath);
    json(resolve(s,'root-call.actual.private.json'),old.campaign.rootCallRecord);
    const game={pid:101,parentPid:105,creationTime:iso(500),kernelCreationFileTime:'134371266963297183',executable:resolve(s,'installation/jre/bin/java.exe'),
        sessionId:1,inSupervisorJob:true,supervisorJobPids:[...old.controller.supervisorJobPids,101],mainWindowHandle:20};
    const ctx={s,root,anchor,iso,powershell,supervisor,clock,old,options,controllerPin:ref(old.controllerPath),game,outer:1,
        reservationShift:baseline.reservedActions-76,outputs:/** @type {string[]} */([])};
    /** @type {Record<string,any>} */const phases={};
    /** @param {string} batch @param {any} extra */
    function phase(batch,extra) {
        const campaign={...old.campaign,windowId:batch==='B1'?19:20};
        const planPath=resolve(s,batch+'-plan.private.json'),leasePath=resolve(s,batch+'-lease.private.json');
        json(planPath,{...old.plan,campaign,batches:old.plan.batches.filter((/** @type {any} */ row)=>row.id===batch),...extra,
            ...(batch==='E2'?options.phasePlanExtra??{}:{})});
        json(leasePath,{...old.launchLease,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:ref(planPath).sha256,...extra});
        const ownedReceiptPath=resolve(s,'driver-'+batch+'-owned.private.json');json(ownedReceiptPath,{at:iso(600),processes:[game]});
        const sourcePath=resolve(s,batch+'-authorization.private.json');json(sourcePath,{schemaVersion:6,scenario:s,batch,planPath,
            planSha256:ref(planPath).sha256,leasePath,leaseSha256:ref(leasePath).sha256,ownedReceiptPath,ownedReceiptSha256:ref(ownedReceiptPath).sha256});
        const runID=basename(s)+'-'+batch;
        const binding=batch==='B1'?readBatchAuthorization(sourcePath,{root,runID,now:anchor+900}).binding:
            {...phases.B1.binding,batch,runID,campaign,sourcePath,sourceSha256:ref(sourcePath).sha256,planSha256:ref(planPath).sha256,
                ...(batch==='E1'?{b1MenuReceipt:{...extra.b1MenuReceipt,newGameCoordinate:[10,20]}}:
                    {e1WizardReceipt:{...extra.e1WizardReceipt,generatedName:'Seed Captain'}})};
        const claimPath=resolve(s,'guard-'+batch+'-consumed.private.json');json(claimPath,{sourcePath,sourceSha256:ref(sourcePath).sha256,binding});
        return {batch,runID,binding,source:ref(sourcePath),claim:ref(claimPath)};
    }
    if(options.earlyWindowCase)return options.earlyWindowCase({...ctx,future,pins,phase});
    const b1=phases.B1=generatedPreStartProvider(ctx,phase('B1',{}),1000,15000);
    const play=generatedPreStartAction(ctx,b1,7,'PLAY',1,77);
    const menuFrame=generatedPreStartCopy(ctx,b1,11,'B1-after',play),menuPath=resolve(s,'B1-menu-inspected.private.json');
    json(menuPath,{schemaVersion:1,kind:'STARSECTOR_B1_MAIN_MENU_INSPECTION',scenario:s,flow:'DISPOSABLE_SEED_CREATION',
        rootRecordKey:clock.recordKey,controllerSha256:ctx.controllerPin.sha256,b1:{sourceSha256:b1.source.sha256,claimSha256:b1.claim.sha256,
            runID:b1.runID,reservationID:play.reservationID},frame:menuFrame,
        inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:anchor+15100,state:'STARSECTOR_MAIN_MENU',newGameCoordinate:[10,20]},provider:b1.proof});
    Object.assign(play,{menuReceiptSha256:ref(menuPath).sha256,providerClock:b1.providerClock});
    const e1=phases.E1=generatedPreStartProvider(ctx,phase('E1',{b1MenuReceipt:ref(menuPath)}),20000,44000);
    const entry=generatedPreStartAction(ctx,e1,7,'NEW_GAME',1,78),generate=generatedPreStartAction(ctx,e1,13,'GENERATE',2,79),
        next=generatedPreStartAction(ctx,e1,19,'CONTINUE',3,80);
    /** @param {any} provider @param {number} ordinal @param {number} index @param {any} previous @param {any} nextAction @param {string} state @param {string} label @param {string} name */
    function control(provider,ordinal,index,previous,nextAction,state,label,name) {
        const prefix=provider.batch+'-wizard-'+ordinal,frame=generatedPreStartCopy(ctx,provider,index,prefix,previous);
        if(provider.batch==='E1'&&ordinal===2&&options.copiedFrameTimeOffsetMs) {
            const m=value(frame.metadataPath);m.capturedAtMs+=options.copiedFrameTimeOffsetMs;json(frame.metadataPath,m);
            frame.metadataSha256=ref(frame.metadataPath).sha256;frame.capturedAtMs+=options.copiedFrameTimeOffsetMs;
        }
        const path=resolve(s,prefix+'-inspected.private.json'),inspectionAt=frame.capturedAtMs+50;
        json(path,{schemaVersion:1,kind:'STARSECTOR_SEED_WIZARD_CONTROL',scenario:s,rootRecordKey:clock.recordKey,
            controllerSha256:ctx.controllerPin.sha256,sourceSha256:provider.source.sha256,claimSha256:provider.claim.sha256,runID:provider.runID,
            reservationID:previous.reservationID,frame,inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:inspectionAt,
                state,controlLabel:label,enabled:true,coordinate:[10,20],generatedName:name}});
        previous.after={pngSha256:frame.pngSha256,window:provider.window,capturedAtMs:frame.capturedAtMs,observedAtMs:inspectionAt};
        previous.seedWizardInspectionSha256=ref(path).sha256;nextAction.wizardStep.inspectionSha256=ref(path).sha256;
    }
    control(e1,2,10,entry,generate,'IDENTITY_EMPTY','GENERATE','');
    control(e1,3,16,generate,next,'IDENTITY_NAMED','1. Continue','Seed Captain');
    const occupationFrame=generatedPreStartCopy(ctx,e1,22,'E1-occupation',next),occupation=resolve(s,'E1-occupation-inspected.private.json');
    json(occupation,{schemaVersion:1,kind:'STARSECTOR_E1_OCCUPATION_INSPECTION',scenario:s,flow:'DISPOSABLE_SEED_CREATION',
        rootRecordKey:clock.recordKey,controllerSha256:ctx.controllerPin.sha256,e1:{sourceSha256:e1.source.sha256,claimSha256:e1.claim.sha256,
            runID:e1.runID,reservations:[entry,generate,next].map(a=>a.reservationID)},frame:occupationFrame,
        inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:anchor+43000,state:'OCCUPATION_SELECTION',generatedName:'Seed Captain'},provider:e1.proof});
    Object.assign(next,{menuReceiptSha256:ref(occupation).sha256,providerClock:e1.providerClock});
    const e2=phases.E2=generatedPreStartProvider(ctx,phase('E2',{e1WizardReceipt:ref(occupation)}),250000,275000);
    const explorer=generatedPreStartAction(ctx,e2,7,'EXPLORER',1,81),normal=generatedPreStartAction(ctx,e2,13,'NORMAL',2,82),
        skip=generatedPreStartAction(ctx,e2,19,'SKIP',3,83);
    control(e2,1,4,next,explorer,'OCCUPATION_SELECTION','An explorer, leading a salvage expedition from aboard an Apogee-class cruiser (faster start)','Seed Captain');
    control(e2,2,10,explorer,normal,'CAMPAIGN_DIFFICULTY','Normal','Seed Captain');
    control(e2,3,16,normal,skip,'TUTORIAL_CHOICE','Skip it','Seed Captain');
    const actions=[play,entry,generate,next,explorer,normal,skip],guard={...baseline,reservedActions:baseline.reservedActions+7,sentActions:baseline.sentActions+7};
    const ledgerPath=resolve(s,'guard-launch-history.private.json');
    json(ledgerPath,{schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',rootRecordKey:clock.recordKey,controllerSha256:ctx.controllerPin.sha256,
        history:guard,uncertain:false,b1ClaimSha256:b1.claim.sha256,resolvedB1:play,resolvedE1:next,resolvedActions:[entry,generate,explorer,normal],
        pending:skip,provider:{batch:'E2',runID:e2.runID,sourceSha256:e2.source.sha256},providerClock:e2.providerClock});
    const complete={...ctx,phases,actions,guard,baseline,ledgerPath,future,pins,control};
    options.prepareHistoricalCase?.(complete);
    const generated=generatedPreStartGeometry(complete);
    return options.finishHistoricalCase?options.finishHistoricalCase({...complete,generated}):finishPreStartFixture({...complete,generated});
}

/** @param {any} ctx */
function finishPreStartFixture(ctx) {
    const {s,root,iso,anchor,old,future,phases,actions,guard,baseline,ledgerPath,clock,powershell,generated}=ctx;
    const callerPath=resolve(s,'launch-caller-terminal.actual.private.json'),reason='Driver requested owned cleanup; native success not established';
    const stopped={atUtc:iso(300000),maintainedOwnedStopRequested:true,originalCallerSession:777,extraInput:false};
    json(callerPath,{schemaVersion:1,kind:'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL_REAP_OBSERVATION',sessionId:777,terminal:true,reaped:true,
        exitCode:1,timedOut:false,actualNestedVerdict:'FAIL',actualNestedChildStatus:1,processOwnership:'windows-job-object',reason,ordinaryExit:false,toolChunkId:'synthetic',
        cleanupRequest:{args:{shell:powershell.replaceAll('\\','/')},raw:{exit_code:0,output:JSON.stringify(stopped)}}});
    json(resolve(s,'launch-supervisor.private.json'),{schemaVersion:1,command:'verify',ok:true,result:{verdict:'FAIL',steps:[{status:1,timedOut:false,
        processOwnership:'windows-job-object',command:powershell,cwd:'.',stderr:reason,args:['-NoProfile','-NonInteractive','-File',
            resolve(root,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',s,'-BindingPath',old.plan.fixtureBinding.path,
            '-BindingSha256',old.plan.fixtureBinding.sha256,'-OuterDeadlineUtc',old.controller.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000']}]}});
    json(resolve(s,'after-smoke-protection.private.json'),{phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,
        saveCensusUnchanged:true,liveGameProcesses:0,originalWrites:false,privateAutosaveEnabled:false,privateSaveEntries:1,
        originalAutosave:{enabled:true,maximumSlots:3},sharedPreference:preStartPreference,at:iso(301000)});
    json(resolve(s,'owned-processes-current.private.json'),{processes:ctx.options.omitOwnedGame?[]:[ctx.game]});
    json(resolve(s,'pid-absence.actual.private.json'),{schemaVersion:1,kind:'ACTUAL_S9_ALL_RECORDED_OWNED_PIDS_ABSENT',pids:ctx.options.omitOwnedGame?[100,105,202,302,402]:[100,101,105,202,302,402],liveCount:0,
        allThreeProvidersTerminalPass:true,originalCallerSession:777,originalCallerReapedExitCode:1,originalCallerTerminalToolChunk:'synthetic',
        launchSupervisorVerdict:'FAIL',launchSupervisorStatus:1,launchSupervisorTimedOut:false,launchSupervisorProcessOwnership:'windows-job-object',
        forcedOwnedCleanup:true,ordinaryExit:false,preference:preStartPreference,preferenceUnchanged:true,sharedWrites:false,atUtc:iso(302000)});
    mkdirSync(resolve(s,'installation/saves/common'));writeFileSync(resolve(s,'installation/saves/common/core_shared_settings.json.data'),'{"campaignHelpPopupsOptionChecked": true}');
    const calls=Object.values(phases).flatMap((/** @type {any} */ p)=>p.calls);
    json(resolve(s,'driver-stop-observation.private.json'),{schemaVersion:1,kind:'ACTUAL_S9_DRIVER_PRE_START_MARGIN_STOP',driver:'/root/recovery_game_driver',scenario:s,
        failureCategory:'INSUFFICIENT_ORIGINAL_INPUT_MARGIN',rootCallerSession:777,originalCallerDeadlineMs:clock.effectiveDeadlineMs,
        campaign:false,seed:false,stability:false,ordinaryExit:false,rootProtection:'PENDING',counts:{calls:55,originalPngs:10,knownClicks:7,activations:0,providers:3},
        actualVendorCalls:calls.map((/** @type {any} */ c,/** @type {number} */ i)=>({ordinalOverall:i+1,batch:c.post.batch,name:c.post.tool,
            args:c.post.tool==='doctor'?{}:c.post.args,atMs:Date.parse(c.post.callCompleted)+1}))});
    json(resolve(s,'progress.pre-native-preparation.original.private.json'),{synthetic:true});
    const names=['plan.json','native-lane-lease.private.json','controller-start.private.json','root-call.actual.private.json','driver-metadata-calls.actual.private.json',
        'driver-stop-observation.private.json','guard-launch-history.private.json','B1-authorization.private.json','E1-authorization.private.json','E2-authorization.private.json',
        'guard-B1-consumed.private.json','guard-E1-consumed.private.json','guard-E2-consumed.private.json','B1-menu-inspected.private.json','E1-occupation-inspected.private.json',
        'launch-caller-terminal.actual.private.json','launch-supervisor.private.json','after-smoke-protection.private.json','pid-absence.actual.private.json',
        'progress.pre-native-preparation.original.private.json'];
    const evidence=Object.fromEntries(names.map(n=>[n,ref(resolve(s,n))]));
    for(const p of Object.values(/** @type {Record<string,any>} */(phases)))for(const name of ['guarded-http-ready.private.json','owned-provider-terminal.private.json',
        'owned-provider.stdout.private.txt','owned-provider.stderr.private.txt','batch-dispatch.private.jsonl'])evidence[p.batch+'/'+name]=ref(resolve(p.output,name));
    const resultPath=resolve(s,'driver-flow-result.private.json');
    const result={schemaVersion:1,kind:'ACTUAL_FINITE_DRIVER_FLOW_RESULT',scenarioId:basename(s),driver:'/root/recovery_game_driver',status:'CLOSED_FAIL',verdict:'FAIL',
        failureCategory:'INSUFFICIENT_ORIGINAL_INPUT_MARGIN',sourceHead:'1'.repeat(40),history:{guardBaseline:baseline,finalGuard:guard,actualContext:{overallAttempts:49,uiAttempts:46},
            absoluteMaxReservedActions:84,historicalUnsentReservations:2,addedReservations:7,addedSentActions:7,refunds:0,pending:'KNOWN_SKIP_AWAITING_AFTER',uncertain:false},
        counts:{vendorCalls:55,originalPngs:10,providers:3,knownClicks:7,activations:0,StartGameInputs:0},steps:[],generatedName:'Seed Captain',
        actions:actions.map((/** @type {any} */ a)=>({stage:a.wizardStep.stage,batch:a.batch,args:a.args})),
        visibleAssertion:{lastOriginal:'E2/capture-004.png',state:'START_GAME_CONTROL_VISIBLE_ENABLED',character:'Seed Captain',localPoint:[0,0],campaignVisible:false,freshness:'UNKNOWN',noPauseInference:true},
        failure:generated.failure,closure:{providers:Object.fromEntries(Object.values(phases).map((/** @type {any} */ p)=>[p.batch,p.closure])),
            rootCaller:{session:777,terminalToolChunk:'synthetic',exitCode:1,timedOut:false,reaped:true,verdict:'FAIL',kind:'ACTUAL_OBSERVED_TOOL_REAP_NOT_FABRICATED_CONTROLLER_TERMINAL'},
            maintainedStopRequestedAtUtc:stopped.atUtc,allOwnedHandlesTerminalReaped:true,ordinaryExit:false,controllerTerminalExpectedAbsent:true},
        protection:{originalAndBackupFiles:182,saveFiles:178,originalAndBackupHashesUnchanged:true,saveCensusUnchanged:true,privateAutosave:false,privateSaveEntries:1,
            privateOutput:'Only synthetic common preference',liveGameProcesses:0,originalWrites:false,restoration:'UNNECESSARY_ORIGINALS_UNCHANGED',typedSharedPreference:'Registry64/String/true unchanged'},
        claims:{campaign:false,seed:false,stability:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,closedHistoryKind:false},evidence,
        originals:Object.values(phases).flatMap((/** @type {any} */ p)=>p.originals),successfulGeometry:generated.successfulGeometry,atUtc:iso(303000),
        absentFinalArtifacts:{startGeometry:true,startGeometryTerminal:true,startControl:true,created:true,stability:true}};
    json(resultPath,result);
    const closeoutPath=resolve(s,'root-closed-pre-start-failure.private.json');
    const closeout={schemaVersion:1,kind:'ROOT_CLOSED_PRE_START_INPUT_MARGIN_FAILED_SEED_ATTEMPT_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',scenario:s,
        sourceWorkspace:root,sourceHead:'1'.repeat(40),allOwnedHandlesTerminalReaped:true,actions:{actualGuard:guard,currentRunContext:{overallAttempts:49,uiAttempts:46},absoluteMaxReservedActions:84},
        evidence:{result:ref(resultPath),calls:Object.fromEntries(Object.values(phases).map((/** @type {any} */ p)=>[p.batch,p.calls.map((/** @type {any} */ c)=>c.refs)])),finalSourceCapture:generated.finalSourceCapture}};
    function refresh() {
        json(closeoutPath,closeout);const next={overallAttempts:49,uiAttempts:46,reservedActions:83,sentActions:81};
        json(future.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',historyBaseline:next,
            maxReservedActions:91,previous:{result:ref(closeoutPath),ledger:ref(ledgerPath)}});
        json(future.launchLease.supportPinsPath,ctx.pins());
        for(const v of [future.plan,future.launchLease,future.phasePlan,future.phaseLease])Object.assign(v,{historyBaseline:next,absoluteMaxReservedActions:91,historyAdmission:ref(future.grantPath)});
        future.launchLease.supportPinsSha256=future.phaseLease.supportPinsSha256=ref(future.launchLease.supportPinsPath).sha256;
        future.campaign.historyBaseline=future.predecessor.historyBaseline=next;future.campaign.maxReservedActions=future.predecessor.maxReservedActions=91;future.refreshLaunch();
    }
    refresh();
    return {...future,priorScenario:s,closeout,closeoutPath,resultPath,ledgerPath,phases,refresh,
        close(){for(const output of ctx.outputs)rmSync(output,{recursive:true});old.close();future.close();}};
}
