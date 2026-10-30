// @ts-check
// Generated OS/provider/visual boundary records. No game, save or live authority.
import {resolve,basename,sep} from 'node:path';
import {readFileSync,writeFileSync,mkdirSync,rmSync,realpathSync} from 'node:fs';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {generatedPreStartAction} from './seed-pre-start-record-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

export const postStartKind='ROOT_CLOSED_POST_START_VERIFICATION_INCOMPLETE_HISTORY';
const category='PRE_CLEANUP_SAVE_STABILITY_MARGIN_REFUSAL_AND_PROVIDER_CLOSE_UNKNOWN';
const margin='retain20s observation admission plus10s closure and original30s cleanup reserve';

/** @param {any} [options] */
export function postStartHistoryFixture(options={}) {
    return preStartHistoryFixture({...options,watchdogMs:1800000,e2ActionCount:4,lastClickDelayMs:110915,supervisorJobPids:[100,105,106,107],canonicalRootEnvironment:true,
        historicalBaseline:{overallAttempts:51,uiAttempts:48,reservedActions:83,sentActions:81},
        prepareHistoricalCase:prepare,finishHistoricalCase:finish});
}
/** @param {any} c */
function prepare(c) {
    c.postStartGeometryReview=c.options.geometryReview;
    if(c.options.movedWindow){
        const {batch,call}=c.options.movedWindow,row=c.phases[batch].calls[call-1],packet=JSON.parse(row.raw.content[0].text);
        packet.bounds.x++;packet.bounds.y++;row.raw.content[0].text=JSON.stringify(packet);row.raw.structuredContent=packet;
        const path=row.prefix+'.raw.private.json';json(path,row.raw);row.refs.raw=ref(path);
    }
    const e2=c.phases.E2,start=generatedPreStartAction(c,e2,25,'START_GAME',4,84);
    if(c.options.startSourceSha!==undefined)start.sourceSha256=c.options.startSourceSha;
    c.control(e2,4,22,c.actions[6],start,'NEW_GAME_STATS','Start game','Seed Captain');
    c.actions.push(start);c.guard.reservedActions=91;c.guard.sentActions=89;
    for(const [name,next] of /** @type {[string,number][]} */ ([['E1-wizard-2',2],['E1-wizard-3',3],['E2-wizard-1',4],
        ['E2-wizard-2',5],['E2-wizard-3',6],['E2-wizard-4',7]])){
        const path=resolve(c.s,name+'-inspected.private.json'),proof=value(path);
        if(name==='E2-wizard-4'&&c.options.startEnabled!==undefined)proof.inspection.enabled=c.options.startEnabled;
        proof.inspection.inspectedAtMs=c.actions[next].sentAtMs-200;json(path,proof);
        c.actions[next].wizardStep.inspectionSha256=ref(path).sha256;
        c.actions[next-1].seedWizardInspectionSha256=ref(path).sha256;
        c.actions[next-1].after.observedAtMs=proof.inspection.inspectedAtMs;
    }
    const ledger=value(c.ledgerPath);ledger.history=c.guard;ledger.resolvedB1=c.actions[0];ledger.resolvedE1=c.actions[3];
    ledger.resolvedActions=[c.actions[1],c.actions[2],c.actions[4],c.actions[5],c.actions[6]];ledger.pending=start;
    json(c.ledgerPath,ledger);c.completedTerminalInput=true;
    c.additionalGeometryStages=[['E2-START_GAME','E2',22,7]];
}
/** @param {any} c */
function finish(c) {
    const {s,root,iso,anchor,old,future,phases,actions,guard,baseline,ledgerPath,powershell,generated}=c;
    const node=realpathSync.native(process.execPath);
    const e2=phases.E2,final=e2.calls[27],capturedAtMs=Date.parse(final.post.callCompleted),inspectionMs=capturedAtMs+55775;
    const outerDeadline=e2.providerClock.deadlineMs-1092,terminalAt=outerDeadline+25070;
    const e2Terminal={at:new Date(terminalAt).toISOString(),verifierPid:402,verifierStatus:null,signal:'SIGTERM',
        deadline:new Date(outerDeadline).toISOString(),closeFailed:true,overflow:false,spawnFailed:false,terminalObserved:true,
        cleanupUnknown:false,steps:[],terminalBoundToChild:false,verdict:'UNKNOWN',terminalReceipt:resolve(e2.output,'owned-provider-terminal.private.json')};
    json(e2Terminal.terminalReceipt,e2Terminal);writeFileSync(resolve(e2.output,'owned-provider.stdout.private.txt'),'');
    const refusal={chunk_id:'synthetic-stability-refusal',wall_time_seconds:0.3,exit_code:1,original_token_count:10,
        output:'Private seed serializer refused: '+margin+'\nActual precleanup save stability failed; preserve refusal\n'};
    const finishRaw={content:[{type:'text',text:JSON.stringify(e2Terminal)}],isError:true};
    const rootCall=value(resolve(s,'root-call.actual.private.json'));
    const step={status:1,timedOut:false,processOwnership:'windows-job-object',stdout:'',cwd:'.',module:'node:.',command:powershell,
        args:['-NoProfile','-NonInteractive','-File',resolve(root,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',s,
            '-BindingPath',old.plan.fixtureBinding.path,'-BindingSha256',old.plan.fixtureBinding.sha256,'-OuterDeadlineUtc',old.controller.conservativeOuterDeadlineUtc,
            '-SessionWatchdogMs','1800000'],stderr:'Driver requested owned cleanup; native success not established'};
    const nested={verdict:'FAIL',blocked:[],steps:{status:step.status,timedOut:step.timedOut,processOwnership:step.processOwnership,stdout:step.stdout,stderr:step.stderr}};
    const caller={chunk_id:'synthetic-original-terminal',wall_time_seconds:0.4,exit_code:1,original_token_count:10,output:JSON.stringify(nested)};
    json(resolve(s,'launch-caller-terminal.actual.private.json'),caller);
    json(resolve(s,'launch-supervisor.private.json'),{schemaVersion:1,command:'verify',ok:true,result:{verdict:'FAIL',blocked:[],steps:[step],
        graphFingerprint:'0'.repeat(64),scope:'module',selection:{changedPaths:[],modules:['node:.']}}});
    const providerPins=Object.fromEntries(Object.values(/** @type {Record<string,any>} */(phases)).map(p=>[p.batch,
        Object.fromEntries(['guarded-http-ready.private.json','owned-provider-terminal.private.json','owned-provider.stdout.private.txt',
            'owned-provider.stderr.private.txt','batch-dispatch.private.jsonl'].map(n=>[n,ref(resolve(p.output,n))]))]));
    const ids=[100,101,105,106,107,200,201,202,300,301,302,400,401,402];
    json(resolve(s,'owned-terminal-full-pid-absence.actual.private.json'),{kind:'ACTUAL_COMPLETE_ROOT_PROVIDER_CHILD_PID_ABSENCE_AFTER_MAINTAINED_CLEANUP',
        atUtc:iso(500000),pids:ids,liveRecordedPids:[],allAbsent:true,scope:'Complete generated original-root/provider identity union',
        providerOutcomePreserved:'B1 PASS;E1 PASS;E2UNKNOWN',ordinaryExit:false});
    json(resolve(s,'after-smoke-protection.private.json'),{at:iso(499000),protectedOriginalFiles:182,allOriginalAndBackupHashesMatch:true,
        protectedSaveFiles:178,saveCensusUnchanged:true,originalAutosave:{enabled:true,maximumSlots:3},privateAutosaveEnabled:false,
        privateSaveEntries:2,liveGameProcesses:0,originalWrites:false,restoration:'UNNECESSARY_ORIGINALS_UNCHANGED',
        sharedPreference:value(old.preferencePath),phase:'after-smoke'});
    mkdirSync(resolve(s,'installation/saves/common'));writeFileSync(resolve(s,'installation/saves/common/core_shared_settings.json.data'),'{}');
    const directory=resolve(s,'installation/saves/save_generated_unqualified');mkdirSync(directory);
    writeFileSync(resolve(directory,'campaign.xml'),'<synthetic-unqualified-campaign/>');
    writeFileSync(resolve(directory,'descriptor.xml'),'<synthetic-unqualified-descriptor/>');
    const calls=Object.values(/** @type {Record<string,any>} */(phases)).flatMap(p=>p.calls.map((/** @type {any} */ row,/** @type {number} */ i)=>({
        batch:p.batch,runID:p.runID,label:'call-'+String(i+1).padStart(3,'0'),tool:row.post.tool,args:row.post.args,isError:false,
        callStarted:row.post.callStarted,callCompleted:row.post.callCompleted,outerProtocolID:row.post.outerProtocolID,
        innerProtocolID:row.post.innerProtocolID,...row.refs})));
    const originals=Object.values(/** @type {Record<string,any>} */(phases)).flatMap(p=>p.originals.map((/** @type {any} */ row,/** @type {number} */ i)=>{
        const metadata=value(row.metadata.path);return{batch:p.batch,runID:p.runID,label:'capture-'+String(i+1).padStart(3,'0'),
            png:row.original,automaticMetadata:row.metadata,raw:row.raw,args:metadata.args,window:metadata.window,dimensions:metadata.dimensions,
            callStarted:metadata.callStarted,callCompleted:metadata.callCompleted,outerProtocolID:metadata.outerProtocolID,
            innerProtocolID:metadata.innerProtocolID,captureReceipt:true,freshnessClaim:'Generated visible boundary data only'};}));
    json(resolve(s,'driver-call-original-index.actual.private.json'),{schemaVersion:1,kind:'ACTUAL_DRIVER_CALL_AND_ORIGINAL_INDEX',scenario:s,
        calls,originals,vendorCalls:61,originalPngs:11,scope:'Generated original-call boundary data'});
    const metadata={schemaVersion:1,kind:'ACTUAL_DRIVER_METADATA_CALLER_RECORDS',
        calls:[{stage:'E2_PRE_CLEANUP_STABILITY',args:{cmd:"$ErrorActionPreference='Stop'\n$env:PATH='"+resolve(node,'..')+';'+
            resolve(powershell,'..')+";'+$env:PATH\n& '"+node.replaceAll('\\','/')+"' '"+resolve(s,'seed-save-closeout.template.private.mjs').replaceAll('\\','/')+
            "' stability\nif($LASTEXITCODE-ne0){throw 'Actual precleanup save stability failed; preserve refusal'}",shell:powershell,
            workdir:root,sandbox_permissions:'require_escalated',yield_time_ms:10000,max_output_tokens:2500},raw:refusal,polls:[]}],
        scope:'Generated immutable caller boundary data'};
    json(resolve(s,'driver-metadata-calls.actual.private.json'),metadata);
    json(resolve(s,'driver-inspections.actual.private.json'),{schemaVersion:1,kind:'ACTUAL_DRIVER_INSPECTIONS',
        inspections:[{batch:'E2',stage:'CAMPAIGN',call:'call-028',capture:'capture-005',inspectedAtMs:inspectionMs,
            generatedName:'Seed Captain',assertion:'PASS_COMPLETE_CAMPAIGN_VISIBLE_GAME_SAVED',freshness:'VISIBLE_CAMPAIGN_AFTER_KNOWN_START'}]});
    json(resolve(s,'driver-provider-calls.actual.private.json'),{schemaVersion:1,kind:'ACTUAL_DRIVER_PROVIDER_LIFECYCLE_CALLS',
        ...Object.fromEntries(Object.values(/** @type {Record<string,any>} */(phases)).map(p=>{
            const terminal=value(resolve(p.output,'owned-provider-terminal.private.json'));
            const raw={content:[{type:'text',text:JSON.stringify(terminal)}],...(p.batch==='E2'?{isError:true}:{})};
            return[p.batch,{begin:{args:{consentMode:'finite-explicit-consent',runID:p.runID,source:p.source.path.replaceAll('\\','/')},
                raw:{content:[{type:'text',text:JSON.stringify({runID:p.runID,output:p.output,deadline:terminal.deadline})}]}},finish:raw}];})),
        sourceReadyTerminalPins:Object.values(/** @type {Record<string,any>} */(phases)).map(p=>({runID:p.runID,
            ready:providerPins[p.batch]['guarded-http-ready.private.json'],terminal:providerPins[p.batch]['owned-provider-terminal.private.json'],
            terminalValue:value(resolve(p.output,'owned-provider-terminal.private.json'))})),allProvidersPass:false});
    const chronology={schemaVersion:1,kind:'ACTUAL_DRIVER_CALLER_CHRONOLOGY_NOT_CAUSE_DIAGNOSIS',
        sourceFinalCall:{callStarted:final.post.callStarted,callCompleted:final.post.callCompleted,
            outerProtocolID:final.post.outerProtocolID,innerProtocolID:final.post.innerProtocolID},startGameAckAtMs:actions[7].knownResultAtMs,
        finalInspectionCompletedAtMs:inspectionMs,finalInspectionUtc:new Date(inspectionMs).toISOString(),postCaptureToInspectionMs:55775,
        ledgerProviderClock:e2.providerClock,outerProviderDeadline:e2Terminal.deadline,ledgerStabilityCutoffMs:e2.providerClock.deadlineMs-30000,
        earlierOuterObservationClosureCutoffMs:outerDeadline-30000,stabilityInvocation:metadata.calls[0],
        finishE2:{actualRaw:finishRaw,callStartUtc:null,callStartReason:'Start not independently captured',invocationOrder:'After stability refusal once'},
        rootEffectiveDeadlineMs:c.clock.effectiveDeadlineMs,rootDeadlineRenewed:false,scope:'Chronology only; subcomponent cause unknown'};
    json(resolve(s,'driver-chronology.actual.private.json'),chronology);
    const steps=actions.map((/** @type {any} */ action,/** @type {number} */ i)=>({id:action.wizardStep.stage,batch:action.batch,
        state:'INPUT_ACKNOWLEDGED_VISIBLE_TRANSITION_VERIFIED',action:action.args,inputCall:'call-'+String([7,7,13,19,7,13,19,25][i]).padStart(3,'0')}));
    const actualContext={overallAttempts:52,uiAttempts:49},files=['plan.json','native-lane-lease.private.json','controller-start.private.json',
        'root-call.actual.private.json','campaign-postlaunch.private.json','root-postlaunch-mechanical-delegation.private.json',
        'guard-launch-history.private.json','B1-authorization.private.json','E1-authorization.private.json','E2-authorization.private.json',
        'guard-B1-consumed.private.json','guard-E1-consumed.private.json','guard-E2-consumed.private.json','B1-menu-inspected.private.json',
        'E1-occupation-inspected.private.json','launch-caller-terminal.actual.private.json','launch-supervisor.private.json',
        'after-smoke-protection.private.json','owned-terminal-full-pid-absence.actual.private.json','driver-metadata-calls.actual.private.json',
        'driver-inspections.actual.private.json','driver-provider-calls.actual.private.json','driver-chronology.actual.private.json',
        'driver-call-original-index.actual.private.json','protected-fixture-receipt.private.json','fixture-binding.private.json',
        'frozen-support-pins.private.json','source-pins.private.json','root-history-baseline.admitted.private.json','seed-creation-preference.private.json'];
    const resultPath=resolve(s,'driver-flow-result.private.json'),evidence=Object.fromEntries(files.map(n=>[n,ref(resolve(s,n))]));
    const result={schemaVersion:1,kind:'STARSECTOR_DRIVER_FLOW_RESULT',scenarioId:basename(s),driver:'/root/recovery_game_driver',
        status:'CLOSED_FAIL',state:'FINAL_FAILED_CLEANED',failureCategory:category,sourceHead:'1'.repeat(40),
        atUtc:iso(501000),actions:{inheritedBaseline:baseline,actualGuard:guard,currentRunContext:actualContext,absoluteMaxReservedActions:91,
            historicalReservedUnsent:2,refunds:0,acknowledgedClicks:8,activations:0,pending:actions[7],uncertain:false},
        calls:{total:61,B1:11,E1:22,E2:28,originalPngs:11,providersStarted:3,providersPassed:2,providersUnknown:1},generatedName:'Seed Captain',steps,
        visibleEndState:'COMPLETE_CAMPAIGN_VISIBLE_WITH_GAME_SAVED_ON_ORIGINAL_E2_CAPTURE005',finalOriginal:originals.at(-1),
        stability:{status:'REFUSED_ORIGINAL_PROVIDER_OBSERVATION_CLOSURE_MARGIN',actualToolChunk:refusal.chunk_id,exitCode:1,receiptCreated:false,
            modeRetried:false,exactInvocationAndRaw:ref(resolve(s,'driver-metadata-calls.actual.private.json')),chronology:ref(resolve(s,'driver-chronology.actual.private.json')),
            finalCaptureCompleted:final.post.callCompleted,inspectionCompletedAtMs:inspectionMs,postCaptureToInspectionMs:55775,
            cutoffLedgerUtc:new Date(e2.providerClock.deadlineMs-30000).toISOString(),cutoffEarlierOuterUtc:new Date(outerDeadline-30000).toISOString(),scope:'Cause split unknown'},
        providerClosure:{...Object.fromEntries(Object.values(/** @type {Record<string,any>} */(phases)).map(p=>[p.batch,
            {content:[{type:'text',text:JSON.stringify(value(resolve(p.output,'owned-provider-terminal.private.json')))}],...(p.batch==='E2'?{isError:true}:{})}])),
            allProvidersPassed:false,E2Assessment:'UNKNOWN_SIGTERM_STATUS_NULL_CLOSE_FAILED_TERMINAL_NOT_BOUND_TO_CHILD',E2FinishStartWallUtc:null,reason:'Start not recorded'},
        cleanup:{rootOriginalSession:777,rootActualTerminal:caller,rootTerminalReceiptFormat:'Actual raw tool result',rootLaunchVerdict:'FAIL',rootTimedOut:false,
            processOwnership:'windows-job-object',maintainedRequestedCleanup:true,ordinaryExit:false,all14RecordedPidsAbsent:true,exactRecordedPids:ids,
            providerE2PassOrChildLinkageProved:false},protection:value(resolve(s,'after-smoke-protection.private.json')),
        createdReceiptPresent:false,stableSaveReceiptPresent:false,modernSeedCloseoutPresent:false,seedQualified:false,xmlSemanticsAccepted:false,
        ordinaryExitQualified:false,native99OrAlphaAcceptance:false,currentClockRenewed:false,retryPerformed:false,authorityChanged:false,
        historicalFailuresAndSpentAllowancesPreserved:true,evidence,reportCallerFailure:{preserved:true}};
    json(resultPath,result);
    const path=resolve(s,'root-closed-post-start-verification-incomplete.private.json');
    const closeout={schemaVersion:1,kind:postStartKind,issuer:'/root',driver:'/root/recovery_game_driver',scenario:s,sourceWorkspace:root,
        sourceHead:'1'.repeat(40),allOwnedHandlesTerminalReaped:true,actions:{actualGuard:guard,currentRunContext:actualContext,absoluteMaxReservedActions:91},
        evidence:{result:ref(resultPath),files:evidence,providers:providerPins,geometry:generated.successfulGeometry,
            unqualifiedOutput:['campaign.xml','descriptor.xml'].map(n=>ref(resolve(directory,n)))}};
    function refresh(){
        json(path,closeout);const next={overallAttempts:52,uiAttempts:49,reservedActions:91,sentActions:89};
        json(future.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',
            historyBaseline:next,maxReservedActions:99,previous:{result:ref(path),ledger:ref(ledgerPath)}});
        json(future.launchLease.supportPinsPath,c.pins());
        for(const v of [future.plan,future.launchLease,future.phasePlan,future.phaseLease])Object.assign(v,{historyBaseline:next,absoluteMaxReservedActions:99,historyAdmission:ref(future.grantPath)});
        future.launchLease.supportPinsSha256=future.phaseLease.supportPinsSha256=ref(future.launchLease.supportPinsPath).sha256;
        future.campaign.historyBaseline=future.predecessor.historyBaseline=next;future.campaign.maxReservedActions=future.predecessor.maxReservedActions=99;
        future.refreshLaunch();
    }
    function rebindLateRecords(){
        for(const name of files)evidence[name]=ref(resolve(s,name));
        for(const key of /** @type {const} */ (['exactInvocationAndRaw','chronology'])){
            const pin=result.stability[key];result.stability[key]=ref(pin.path);
        }
        json(resultPath,result);closeout.evidence.result=ref(resultPath);refresh();
    }
    /** Reclose every terminal copy; changing UNKNOWN must fail on meaning, not on a stale hash. @param {(value:any)=>void} change */
    function mutateE2Terminal(change){
        const terminalPath=resolve(e2.output,'owned-provider-terminal.private.json'),terminal=value(terminalPath);change(terminal);json(terminalPath,terminal);
        providerPins.E2['owned-provider-terminal.private.json']=ref(terminalPath);
        const raw={content:[{type:'text',text:JSON.stringify(terminal)}],isError:true};result.providerClosure.E2=raw;
        const lifecyclePath=resolve(s,'driver-provider-calls.actual.private.json'),lifecycle=value(lifecyclePath);
        lifecycle.E2.finish=raw;lifecycle.sourceReadyTerminalPins[2].terminal=ref(terminalPath);lifecycle.sourceReadyTerminalPins[2].terminalValue=terminal;json(lifecyclePath,lifecycle);
        const chronologyPath=resolve(s,'driver-chronology.actual.private.json'),chronology=value(chronologyPath);chronology.finishE2.actualRaw=raw;json(chronologyPath,chronology);
        rebindLateRecords();
    }
    refresh();
    return{...future,closeout,closeoutPath:path,result,resultPath,priorScenario:s,ledgerPath,ctx:c,refresh,rebindLateRecords,mutateE2Terminal,
        close(){for(const output of c.outputs){if(!resolve(output).startsWith(resolve(root,'.tmp')+sep))throw Error('Fixture cleanup escaped workspace');rmSync(output,{recursive:true});}old.close();future.close();}};
}
