// @ts-check
// Campaign reached and saved; automated verification incomplete. Never successful seed provenance.
import {resolve,dirname,basename,isAbsolute} from 'node:path';
import {existsSync,readdirSync,lstatSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {inspectModernSeedPins} from './seed-creation-custody.mjs';
import {readPostStartAuthority,assertPostStartWizardControls} from './seed-post-start-custody.mjs';
import {assertPostStartGeometry} from './seed-post-start-geometry.mjs';
import {postStartReader,postStartPinned,readPostStartProviders,postStartNeed as need,postStartObject as object,
    postStartInteger as integer,postStartTime as time,postStartStages,postStartPreference} from './seed-post-start-records.mjs';

export const postStartVerificationIncompleteKind='ROOT_CLOSED_POST_START_VERIFICATION_INCOMPLETE_HISTORY';
const category='PRE_CLEANUP_SAVE_STABILITY_MARGIN_REFUSAL_AND_PROVIDER_CLOSE_UNKNOWN';
const resultFields=['schemaVersion','kind','scenarioId','driver','status','state','failureCategory','sourceHead','atUtc','actions','calls','generatedName',
    'steps','visibleEndState','finalOriginal','stability','providerClosure','cleanup','protection','createdReceiptPresent','stableSaveReceiptPresent',
    'modernSeedCloseoutPresent','seedQualified','xmlSemanticsAccepted','ordinaryExitQualified','native99OrAlphaAcceptance','currentClockRenewed',
    'retryPerformed','authorityChanged','historicalFailuresAndSpentAllowancesPreserved','evidence','reportCallerFailure'];

/** @param {any} v @param {any} ledger @param {any} ledgerPin */
export function readPostStartVerificationIncomplete(v,ledger,ledgerPin) {
    need(closed(v,['schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence'])&&
        v.schemaVersion===1&&v.kind===postStartVerificationIncompleteKind&&v.issuer==='/root'&&v.driver==='/root/recovery_game_driver'&&
        v.allOwnedHandlesTerminalReaped===true&&typeof v.scenario==='string'&&isAbsolute(v.scenario)&&resolve(v.scenario)===v.scenario&&
        typeof v.sourceWorkspace==='string'&&isAbsolute(v.sourceWorkspace)&&resolve(v.sourceWorkspace)===v.sourceWorkspace&&
        dirname(v.scenario)===resolve(v.sourceWorkspace,'.tmp/starsector-proxy/scenarios')&&typeof v.sourceHead==='string'&&/^[a-f0-9]{40}$/.test(v.sourceHead)&&
        closed(v.actions,['actualGuard','currentRunContext','absoluteMaxReservedActions'])&&
        closed(v.evidence,['result','files','providers','geometry','unqualifiedOutput'])&&closed(v.evidence.providers,['B1','E1','E2']));
    const reader=postStartReader(),s=v.scenario;
    postStartPinned(reader,v.evidence.result,resolve(s,'driver-flow-result.private.json'));
    const r=reader.json(resolve(s,'driver-flow-result.private.json'));
    need(closed(r,resultFields)&&r.schemaVersion===1&&r.kind==='STARSECTOR_DRIVER_FLOW_RESULT'&&r.scenarioId===basename(s)&&r.driver===v.driver&&
        r.status==='CLOSED_FAIL'&&r.state==='FINAL_FAILED_CLEANED'&&r.failureCategory===category&&r.sourceHead===v.sourceHead&&time(r.atUtc)&&
        typeof r.generatedName==='string'&&r.generatedName.length>0&&r.generatedName.length<=128&&!/[\u0000-\u001f\u007f]/u.test(r.generatedName)&&
        same(r.calls,{total:61,B1:11,E1:22,E2:28,originalPngs:11,providersStarted:3,providersPassed:2,providersUnknown:1})&&
        Array.isArray(r.steps)&&r.steps.length===8&&same(r.steps.map((/** @type {any} */ x)=>x.id),postStartStages)&&
        r.visibleEndState==='COMPLETE_CAMPAIGN_VISIBLE_WITH_GAME_SAVED_ON_ORIGINAL_E2_CAPTURE005'&&r.historicalFailuresAndSpentAllowancesPreserved===true);
    for(const name of ['createdReceiptPresent','stableSaveReceiptPresent','modernSeedCloseoutPresent','seedQualified','xmlSemanticsAccepted',
        'ordinaryExitQualified','native99OrAlphaAcceptance','currentClockRenewed','retryPerformed','authorityChanged'])need(r[name]===false);
    need(object(r.evidence));
    for(const [name,pin] of Object.entries(/** @type {Record<string,any>} */(r.evidence)))postStartPinned(reader,pin,resolve(s,name));
    const root=readPostStartAuthority(v,reader,ledger,ledgerPin),{a,plan,controller,controllerPin,clock,actions,phases}=root;
    need(same(r.actions,{inheritedBaseline:plan.historyBaseline,actualGuard:ledger.history,currentRunContext:v.actions.currentRunContext,
        absoluteMaxReservedActions:v.actions.absoluteMaxReservedActions,historicalReservedUnsent:2,refunds:0,acknowledgedClicks:8,activations:0,
        pending:ledger.pending,uncertain:false}));
    for(const [i,row] of r.steps.entries())need(object(row)&&row.id===actions[i].wizardStep.stage&&row.batch===actions[i].batch&&
        row.state==='INPUT_ACKNOWLEDGED_VISIBLE_TRANSITION_VERIFIED'&&same(row.action,actions[i].args)&&
        row.inputCall==='call-'+String([7,7,13,19,7,13,19,25][i]).padStart(3,'0'));
    const index=a['driver-call-original-index.actual.private.json'];
    need(object(index)&&index.schemaVersion===1&&index.scenario===s&&Array.isArray(index.calls)&&index.calls.length===61&&
        Array.isArray(index.originals)&&index.originals.length===11&&index.vendorCalls===61&&index.originalPngs===11);
    const ctx={v,r,reader,...root,index},found=readPostStartProviders(ctx),providers=found.providers,originals=found.originals;
    assertPostStartWizardControls({...ctx,providers});
    need(same(r.finalOriginal,index.originals[10]));
    const calls=Object.fromEntries(['B1','E1','E2'].map(batch=>[batch,index.calls.filter((/** @type {any} */ c)=>c.batch===batch)
        .map((/** @type {any} */ c)=>({post:reader.ref(c.post.path),attempt:reader.ref(c.attempt.path),raw:reader.ref(c.raw.path)}))]));
    assertPostStartGeometry({...ctx,providers,v:{...v,evidence:{...v.evidence,calls}},
        r:{...r,successfulGeometry:v.evidence.geometry,evidence:{...r.evidence,'B1/owned-provider-terminal.private.json':v.evidence.providers.B1['owned-provider-terminal.private.json']}}});
    const chronology=a['driver-chronology.actual.private.json'],final=providers.E2.calls[27],capturedAt=Date.parse(final.post.callCompleted);
    const inspected=chronology.finalInspectionCompletedAtMs,outer=Date.parse(providers.E2.terminal.deadline),providerClock=ledger.providerClock;
    need(chronology.schemaVersion===1&&chronology.kind==='ACTUAL_DRIVER_CALLER_CHRONOLOGY_NOT_CAUSE_DIAGNOSIS'&&
        same(chronology.sourceFinalCall,{callStarted:final.post.callStarted,callCompleted:final.post.callCompleted,
            outerProtocolID:final.post.outerProtocolID,innerProtocolID:final.post.innerProtocolID})&&
        chronology.startGameAckAtMs===actions[7].knownResultAtMs&&integer(inspected,capturedAt)&&inspected-capturedAt===55775&&
        chronology.postCaptureToInspectionMs===55775&&chronology.finalInspectionUtc===new Date(inspected).toISOString()&&
        same(chronology.ledgerProviderClock,providerClock)&&chronology.outerProviderDeadline===providers.E2.terminal.deadline&&
        chronology.ledgerStabilityCutoffMs===providerClock.deadlineMs-30000&&chronology.earlierOuterObservationClosureCutoffMs===outer-30000&&
        outer<=providerClock.deadlineMs&&inspected>providerClock.deadlineMs&&chronology.rootEffectiveDeadlineMs===clock.effectiveDeadlineMs&&
        chronology.rootDeadlineRenewed===false&&capturedAt<=Math.min(outer,providerClock.deadlineMs)-30000);
    const metadata=a['driver-metadata-calls.actual.private.json'];
    need(metadata.schemaVersion===1&&metadata.kind==='ACTUAL_DRIVER_METADATA_CALLER_RECORDS'&&Array.isArray(metadata.calls));
    const failures=metadata.calls.filter((/** @type {any} */ x)=>x?.stage==='E2_PRE_CLEANUP_STABILITY');need(failures.length===1);
    const failure=failures[0],raw=failure.raw;
    const canonicalNode=providers.B1.ready.node.executable;
    const stabilityCommand="$ErrorActionPreference='Stop'\n$env:PATH='"+dirname(canonicalNode)+';'+dirname(root.powershell)+
        ";'+$env:PATH\n& '"+canonicalNode.replaceAll('\\','/')+"' '"+resolve(s,'seed-save-closeout.template.private.mjs').replaceAll('\\','/')+
        "' stability\nif($LASTEXITCODE-ne0){throw 'Actual precleanup save stability failed; preserve refusal'}";
    need(same(chronology.stabilityInvocation,failure)&&object(failure.args)&&typeof failure.args.workdir==='string'&&resolve(failure.args.workdir)===v.sourceWorkspace&&
        typeof failure.args.shell==='string'&&basename(failure.args.shell).toLowerCase()==='pwsh.exe'&&
        root.support.pins.some((/** @type {any} */ p)=>resolve(p.path)===resolve(failure.args.shell))&&
        failure.args.cmd===stabilityCommand&&failure.args.sandbox_permissions==='require_escalated'&&failure.args.yield_time_ms===10000&&failure.args.max_output_tokens===2500&&
        closed(raw,['chunk_id','wall_time_seconds','exit_code','original_token_count','output'])&&raw.exit_code===1&&
        typeof raw.chunk_id==='string'&&raw.chunk_id.length>0&&Number.isFinite(raw.wall_time_seconds)&&raw.wall_time_seconds>=0&&
        integer(raw.original_token_count)&&typeof raw.output==='string'&&Buffer.byteLength(raw.output)<=262144&&
        raw.output.includes('retain20s observation admission plus10s closure and original30s cleanup reserve')&&
        (!Object.hasOwn(failure,'polls')||(Array.isArray(failure.polls)&&failure.polls.length===0)));
    need(object(r.stability)&&r.stability.status==='REFUSED_ORIGINAL_PROVIDER_OBSERVATION_CLOSURE_MARGIN'&&r.stability.exitCode===1&&
        r.stability.actualToolChunk===raw.chunk_id&&r.stability.receiptCreated===false&&r.stability.modeRetried===false&&
        same(r.stability.exactInvocationAndRaw,reader.ref(resolve(s,'driver-metadata-calls.actual.private.json')))&&
        same(r.stability.chronology,reader.ref(resolve(s,'driver-chronology.actual.private.json')))&&r.stability.finalCaptureCompleted===final.post.callCompleted&&
        r.stability.inspectionCompletedAtMs===inspected&&r.stability.postCaptureToInspectionMs===55775&&
        Date.parse(r.stability.cutoffLedgerUtc)===providerClock.deadlineMs-30000&&Date.parse(r.stability.cutoffEarlierOuterUtc)===outer-30000&&
        same(chronology.finishE2?.actualRaw,r.providerClosure.E2)&&chronology.finishE2.callStartUtc===null&&
        r.providerClosure.allProvidersPassed===false&&r.providerClosure.E2FinishStartWallUtc===null&&
        r.providerClosure.E2Assessment==='UNKNOWN_SIGTERM_STATUS_NULL_CLOSE_FAILED_TERMINAL_NOT_BOUND_TO_CHILD');
    const inspections=a['driver-inspections.actual.private.json'];need(Array.isArray(inspections.inspections));
    const campaign=inspections.inspections.filter((/** @type {any} */ x)=>x?.stage==='CAMPAIGN');
    need(campaign.length===1&&campaign[0].batch==='E2'&&campaign[0].call==='call-028'&&campaign[0].capture==='capture-005'&&
        campaign[0].inspectedAtMs===inspected&&campaign[0].generatedName===r.generatedName&&
        campaign[0].assertion==='PASS_COMPLETE_CAMPAIGN_VISIBLE_GAME_SAVED');
    const caller=a['launch-caller-terminal.actual.private.json'],cleanup=r.cleanup;
    need(closed(caller,['chunk_id','wall_time_seconds','exit_code','original_token_count','output'])&&caller.exit_code===1&&
        typeof caller.chunk_id==='string'&&caller.chunk_id.length>0&&Number.isFinite(caller.wall_time_seconds)&&caller.wall_time_seconds>=0&&
        integer(caller.original_token_count)&&typeof caller.output==='string'&&Buffer.byteLength(caller.output)<=131072);
    const summary=zeroProviderObject(caller.output,need);
    const supervisor=a['launch-supervisor.private.json'];
    need(closed(supervisor,['schemaVersion','command','ok','result'])&&supervisor.schemaVersion===1&&supervisor.command==='verify'&&supervisor.ok===true&&
        closed(supervisor.result,['blocked','graphFingerprint','scope','selection','steps','verdict'])&&supervisor.result.verdict==='FAIL'&&
        same(supervisor.result.blocked,[])&&typeof supervisor.result.graphFingerprint==='string'&&/^[a-f0-9]{64}$/.test(supervisor.result.graphFingerprint)&&
        supervisor.result.scope==='module'&&same(supervisor.result.selection,{changedPaths:[],modules:['node:.']})&&
        Array.isArray(supervisor.result.steps)&&supervisor.result.steps.length===1);
    const launch=supervisor.result.steps[0];
    need(closed(launch,['args','command','cwd','module','processOwnership','status','stderr','stdout','timedOut'])&&
        launch.command===root.powershell&&launch.cwd==='.'&&launch.module==='node:.'&&launch.stdout===''&&
        same(summary.steps,{status:launch.status,timedOut:launch.timedOut,processOwnership:launch.processOwnership,stdout:launch.stdout,stderr:launch.stderr})&&
        same(launch.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),
            '-EvidenceRoot',s,'-BindingPath',plan.fixtureBinding.path,'-BindingSha256',plan.fixtureBinding.sha256,
            '-OuterDeadlineUtc',controller.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','1800000'])&&
        root.support.pins.some((/** @type {any} */ p)=>p.path===launch.args[3]));
    need(summary.verdict==='FAIL'&&Array.isArray(summary.blocked)&&summary.blocked.length===0&&object(summary.steps)&&
        summary.steps.status===1&&summary.steps.timedOut===false&&summary.steps.processOwnership==='windows-job-object'&&
        typeof summary.steps.stderr==='string'&&summary.steps.stderr.includes('Driver requested owned cleanup; native success not established')&&
        object(cleanup)&&cleanup.rootOriginalSession===root.root.value.raw.session_id&&same(cleanup.rootActualTerminal,caller)&&
        cleanup.rootLaunchVerdict==='FAIL'&&cleanup.rootTimedOut===false&&cleanup.processOwnership==='windows-job-object'&&
        cleanup.maintainedRequestedCleanup===true&&cleanup.ordinaryExit===false&&cleanup.all14RecordedPidsAbsent===true&&
        cleanup.providerE2PassOrChildLinkageProved===false);
    const absence=a['owned-terminal-full-pid-absence.actual.private.json'],protection=a['after-smoke-protection.private.json'];
    const ids=[...new Set([...controller.supervisorJobPids,...Object.values(phases).flatMap((/** @type {any} */ phase)=>
        phase.owned.processes.flatMap((/** @type {any} */ p)=>[p.pid,...p.supervisorJobPids])),
        ...Object.values(providers).flatMap((/** @type {any} */ p)=>[p.terminal.verifierPid,...p.ready.supervisorJobPids])])].sort((x,y)=>x-y);
    need(ids.length===14&&absence.kind==='ACTUAL_COMPLETE_ROOT_PROVIDER_CHILD_PID_ABSENCE_AFTER_MAINTAINED_CLEANUP'&&time(absence.atUtc)&&
        same(absence.pids,ids)&&same(cleanup.exactRecordedPids,ids)&&same(absence.liveRecordedPids,[])&&absence.allAbsent===true&&absence.ordinaryExit===false&&
        absence.providerOutcomePreserved==='B1 PASS;E1 PASS;E2UNKNOWN'&&Date.parse(absence.atUtc)>=Date.parse(providers.E2.terminal.at)&&
        same(r.protection,protection)&&time(protection.at)&&protection.phase==='after-smoke'&&protection.protectedOriginalFiles===182&&
        protection.protectedSaveFiles===178&&protection.allOriginalAndBackupHashesMatch===true&&protection.saveCensusUnchanged===true&&
        protection.liveGameProcesses===0&&protection.originalWrites===false&&protection.privateAutosaveEnabled===false&&protection.privateSaveEntries===2&&
        same(protection.originalAutosave,{enabled:true,maximumSlots:3})&&same(protection.sharedPreference,postStartPreference)&&
        Date.parse(protection.at)>=Date.parse(providers.E2.terminal.at));
    need(Array.isArray(v.evidence.unqualifiedOutput)&&v.evidence.unqualifiedOutput.length===2);
    let total=0;const directories=new Set();
    for(const [i,pin] of v.evidence.unqualifiedOutput.entries()) {
        need(typeof pin.path==='string'&&basename(pin.path)===(i===0?'campaign.xml':'descriptor.xml')&&
            dirname(dirname(pin.path))===resolve(s,'installation/saves')&&/^save_[A-Za-z0-9_]{1,128}$/.test(basename(dirname(pin.path)))&&lstatSync(pin.path).nlink===1);
        directories.add(dirname(pin.path));total+=postStartPinned(reader,pin,pin.path,16777216).bytes.length;
    }
    need(directories.size===1&&total>0&&total<=16777216&&same(readdirSync([...directories][0]).sort(),['campaign.xml','descriptor.xml']));
    for(const name of ['seed-save-stability.actual.private.json','E2-created-seed-inspected.private.json','seed-creation-closeout.private.json'])need(!existsSync(resolve(s,name)));
    return v.actions;
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
    try {
        need(process.argv.length===11&&process.argv[2]==='inspect');
        const [receiptPath,receiptSha,ledgerPath,ledgerSha,pinsPath,pinsSha,powershell,supervisor]=process.argv.slice(3);
        inspectModernSeedPins(pinsPath,pinsSha,powershell,supervisor);
        const reader=postStartReader(),receipt=reader.pinJson({path:receiptPath,sha256:receiptSha},receiptPath),ledger=reader.pinJson({path:ledgerPath,sha256:ledgerSha},ledgerPath);
        const actions=readPostStartVerificationIncomplete(receipt,ledger,{path:ledgerPath,sha256:ledgerSha});
        process.stdout.write(JSON.stringify({schemaVersion:1,kind:'POST_START_VERIFICATION_INCOMPLETE_HISTORY_CUSTODY',
            receipt:{path:receiptPath,sha256:receiptSha},ledger:{path:ledgerPath,sha256:ledgerSha},actions,scope:'HISTORICAL_CUSTODY_ONLY'})+'\n');
    }catch{process.stderr.write('Post-Start verification-incomplete history refused\n');process.exitCode=1;}
}
