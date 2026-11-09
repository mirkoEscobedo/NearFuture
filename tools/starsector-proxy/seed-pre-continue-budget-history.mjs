// @ts-check
// A failed automation prefix is historical custody only; Generate never becomes a qualified seed.
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {preStartReader} from './seed-pre-start-margin-records.mjs';
import {inspectModernSeedPins} from './seed-creation-custody.mjs';
import {assertEarlyWindowCaller,earlyWindowPreference} from './seed-early-window-custody.mjs';
import {readPreContinueBudgetAuthority} from './seed-pre-continue-budget-authority.mjs';
import {assertPreContinueBudgetRecords} from './seed-pre-continue-budget-records.mjs';
import {assertPreContinueBudgetGeometry} from './seed-pre-continue-budget-geometry.mjs';
import {budgetHistoryNeed as need,budgetHistoryInteger as integer,budgetHistoryObject as object,budgetHistoryTime as time} from './seed-pre-continue-budget-index.mjs';

/** @param {any} v @param {any} ledger @param {any} ledgerPin */
export function readPreContinueBudgetHistory(v,ledger,ledgerPin){
    const ctx=assertPreContinueBudgetRecords(readPreContinueBudgetAuthority(v,ledger,ledgerPin));
    const geometries=assertPreContinueBudgetGeometry(ctx),{reader,g,p,c,root,clock,providers,actions,custody,phases}=ctx,{json,ref}=reader,s=v.scenario;
    const r=json(resolve(s,'driver-flow-result.private.json')),stop=json(resolve(s,'driver-stop-observation.actual.private.json'));
    need(r.schemaVersion===1&&r.kind==='ACTUAL_FINITE_DRIVER_FLOW_RESULT'&&r.scenario===s&&r.sourceHead===v.sourceHead&&typeof r.sourceTree==='string'&&/^[a-f0-9]{40}$/.test(r.sourceTree)&&
        r.verdict==='FAIL'&&r.state==='FINAL_FAILED_CLEANED'&&r.reason==='E1_CONTINUE_GEOMETRY_ORIGINAL_PROVIDER_BUDGET_REFUSAL'&&
        same(r.historyBaseline,p.historyBaseline)&&closed(r.currentRunContext,['overallAttempts','uiAttempts','scope'])&&
        r.currentRunContext.overallAttempts===v.actions.currentRunContext.overallAttempts&&r.currentRunContext.uiAttempts===v.actions.currentRunContext.uiAttempts&&
        typeof r.currentRunContext.scope==='string'&&r.currentRunContext.scope.length>0&&r.currentRunContext.scope.length<=4096&&same(r.finalGuard,g.history)&&r.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&
        r.unsentReservationsCarried===2&&r.refunds===0&&same(r.actualDeltas,{reservedActions:3,sentActions:3,wizardClicksAcknowledged:3,initialActivationsAcknowledged:0})&&
        same(r.actionSummary,actions.map((/** @type {any} */ a)=>({stage:a.wizardStep.stage,batch:a.batch,...a.args,acknowledged:true})))&&same(r.pendingKnownAction,g.pending)&&
        r.continueSent===false&&r.E2Started===false&&same(r.vendorCalls,{B1:12,E1:17,E2:0,total:29})&&r.originalPNGs?.count===5&&
        same(r.originalPNGs.references,[...providers.B1.calls,...providers.E1.calls].filter((/** @type {any} */ a)=>a.post.tool==='screenshot').map((/** @type {any} */ a)=>{
            const provider=a.post.batch==='B1'?providers.B1:providers.E1,number=provider.calls.filter((/** @type {any} */ x)=>x.post.tool==='screenshot'&&Date.parse(x.post.callCompleted)<=Date.parse(a.post.callCompleted)).length;
            return ref(resolve(provider.output,'capture-'+String(number).padStart(3,'0')+'.png'));}))&&
        same(r.seed,{created:false,stabilityReceiptCreated:false,createdReceiptCreated:false,modernCloseoutCreated:false,qualified:false}));
    need(stop.schemaVersion===1&&stop.kind==='ACTUAL_S15_DRIVER_STOP_OBSERVATION'&&stop.scenario===s&&integer(stop.atMs,custody.metadataCommands[4].completedAtMs)&&stop.status==='FAILED'&&
        stop.reason===r.reason&&same(stop.actionsAcknowledged,['PLAY','NEW_GAME','GENERATE'])&&stop.pendingKnownAction==='GENERATE'&&stop.continueInputDispatched===false&&
        stop.providerB1==='KNOWN_PASS'&&stop.providerE1==='UNKNOWN_TERMINAL_SIGTERM_STATUS_NULL_STEPS_EMPTY_NOT_BOUND_TO_CHILD'&&stop.E2==='NOT_STARTED'&&
        stop.vendorCalls===29&&stop.originalPNGs===5&&stop.activations===0&&stop.rootCallerOwner==='/root'&&stop.noRetry===true);
    need(same(r.authority,{plan:ref(resolve(s,'plan.json')),lease:ref(resolve(s,'native-lane-lease.private.json')),B1:phases.B1.source,E1:phases.E1.source,
        rootDeadlineMs:clock.effectiveDeadlineMs,cleanupByMs:clock.effectiveDeadlineMs-30000,mutated:false})&&
        same(r.evidence,{actualCallsAndMetadata:ref(resolve(s,'driver-call-metadata-custody.actual.private.json')),originalStopObservation:ref(resolve(s,'driver-stop-observation.actual.private.json')),
            finalLedger:ledgerPin,originalLaunchCaller:ref(resolve(s,'launch-caller-terminal.actual.private.json')),afterSmokeProtection:ref(resolve(s,'after-smoke-protection.private.json')),
            exactOwnedPidAbsence:ref(resolve(s,'owned-terminal-pid-absence.actual.private.json'))}));
    const f=r.failure,last=custody.metadataCommands[4];
    need(object(f)&&f.stage==='E1-CONTINUE geometry'&&f.commandRecordIndex===4&&f.actualExecChunk===last.raw.chunk_id&&f.actualExitCode===1&&
        f.message==='No original budget for geometry plus bounded cleanup'&&f.childGeometryStarted===false&&f.providerOriginalOuterDeadline===providers.E1.terminal.deadline&&
        f.metadataStartedAtMs===last.startedAtMs&&f.finishStartedAtMs===providers.E1.finish.startedAtMs&&f.finishCompletedAtMs===providers.E1.finish.completedAtMs&&
        f.noClockExtension===true&&f.noRetry===true&&f.individualNativeTimeoutClaim===false);
    for(const batch of ['B1','E1']){
        const row=r.batches?.find((/** @type {any} */ x)=>x.batch===batch),t=providers[batch].terminal;
        need(object(row)&&row.verdict===t.verdict&&same(row.providerTerminal,ref(t.terminalReceipt))&&row.terminalObserved===true&&row.terminalBoundToChild===t.terminalBoundToChild&&
            row.cleanupUnknown===false&&row.verifierStatus===t.verifierStatus&&(batch==='B1'||row.signal==='SIGTERM'&&same(row.steps,[])&&row.continueInputDispatched===false));
    }
    need(Array.isArray(r.batches)&&r.batches.length===3&&same(r.batches[2],{batch:'E2',verdict:'NOT_STARTED',providerStarted:false}));
    const names=['launch-caller-terminal.actual.private.json','launch-supervisor.private.json'];
    const a=Object.fromEntries(names.map(name=>[name,json(resolve(s,name))]));
    const caller=assertEarlyWindowCaller({...ctx,a});
    const owned=json(resolve(s,'owned-processes-current.private.json')),absence=json(resolve(s,'owned-terminal-pid-absence.actual.private.json')),protection=json(resolve(s,'after-smoke-protection.private.json'));
    need(object(owned)&&owned.controllerPid===c.controllerPid&&owned.controllerCreationTime===c.controllerCreationTime&&owned.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&
        time(owned.at)&&Array.isArray(owned.processes)&&owned.processes.length===2);
    const snapshots=[phases.B1.owned,phases.E1.owned,owned];
    for(const snap of snapshots){
        need(time(snap.at)&&Array.isArray(snap.processes)&&new Set(snap.processes.map((/** @type {any} */ x)=>x.pid)).size===snap.processes.length);
        for(const row of snap.processes)need(object(row)&&integer(row.pid,1)&&integer(row.parentPid,1)&&row.pid!==row.parentPid&&row.sessionId===c.sessionId&&typeof row.executable==='string'&&
            [resolve(s,'installation/starsector.exe'),resolve(s,'installation/jre/bin/java.exe')].includes(row.executable)&&row.inSupervisorJob===true&&time(row.creationTime)&&
            Date.parse(row.creationTime)>=Date.parse(c.controllerCreationTime)&&Date.parse(row.creationTime)>=Date.parse(c.at)&&Date.parse(row.creationTime)<=Date.parse(snap.at)&&typeof row.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(row.kernelCreationFileTime)&&
            Array.isArray(row.supervisorJobPids)&&new Set(row.supervisorJobPids).size===row.supervisorJobPids.length&&row.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1))&&
            [row.pid,row.parentPid,c.controllerPid,...c.supervisorJobPids].every(id=>row.supervisorJobPids.includes(id)));
        const games=snap.processes.filter((/** @type {any} */ row)=>row.executable===resolve(s,'installation/jre/bin/java.exe'));
        need(games.length===1&&['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(games[0][key],phases.B1.owned.processes[0][key])));
    }
    const ids=[...new Set([c.controllerPid,...c.supervisorJobPids,...snapshots.flatMap(x=>x.processes.flatMap((/** @type {any} */ row)=>[row.pid,...row.supervisorJobPids])),
        ...geometries.flatMap(x=>[x.pid,...x.supervisorJobPids]),...['B1','E1'].flatMap(batch=>[providers[batch].terminal.verifierPid,...providers[batch].ready.supervisorJobPids])])].sort((x,y)=>x-y);
    need(ids.length===11&&absence.schemaVersion===1&&absence.kind==='ACTUAL_OWNED_TERMINAL_PID_ABSENCE'&&same(absence.queriedExactRecordedPids,ids)&&same(absence.liveRows,[])&&
        absence.allAbsent===true&&absence.ordinaryExitClaim===false&&absence.providerE1Outcome==='UNKNOWN_RETAINED'&&absence.originalCallerKnownTerminal===true&&time(absence.observedAtUtc)&&
        time(protection.at)&&Date.parse(owned.at)<=Date.parse(protection.at)&&Date.parse(owned.at)<=Date.parse(absence.observedAtUtc)&&
        ['B1','E1'].every(batch=>Date.parse(providers[batch].terminal.at)<=Date.parse(protection.at)&&Date.parse(providers[batch].terminal.at)<=Date.parse(absence.observedAtUtc))&&
        protection.phase==='after-smoke'&&protection.protectedOriginalFiles===182&&protection.protectedSaveFiles===178&&protection.allOriginalAndBackupHashesMatch===true&&protection.saveCensusUnchanged===true&&
        same(protection.originalAutosave,{maximumSlots:3,enabled:true})&&protection.privateAutosaveEnabled===false&&protection.privateSaveEntries===0&&protection.liveGameProcesses===0&&protection.originalWrites===false&&
        same(protection.sharedPreference,earlyWindowPreference));
    need(same(r.cleanup,{method:'MAINTAINED_REQUESTED_OWNED_STOP',originalCallerSession:caller.session,actualOriginalTerminalChunk:caller.actual.chunk_id,originalCallerKnownTerminal:true,
        originalCallerReaped:true,originalCallerExitCode:1,nestedWindowsJobTimedOut:false,rawLaunchVerdict:'FAIL',ordinaryExit:false,allRecordedOwnedPidsAbsent:true,recordedOwnedPidCount:11,providerUnknownOutcomePreserved:true}));
    return v.actions;
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
    try{
        need(process.argv.length===11&&process.argv[2]==='inspect');
        const [path,sha,ledgerPath,ledgerSha,pinsPath,pinsSha,powershell,supervisor]=process.argv.slice(3);
        inspectModernSeedPins(pinsPath,pinsSha,powershell,supervisor);
        const reader=preStartReader(),receipt=reader.pinJson({path,sha256:sha},path),ledger=reader.pinJson({path:ledgerPath,sha256:ledgerSha},ledgerPath);
        const actions=readPreContinueBudgetHistory(receipt,ledger,{path:ledgerPath,sha256:ledgerSha});
        process.stdout.write(JSON.stringify({schemaVersion:1,kind:'PRE_CONTINUE_BUDGET_HISTORY_CUSTODY',receipt:{path,sha256:sha},ledger:{path:ledgerPath,sha256:ledgerSha},actions,scope:'HISTORICAL_CUSTODY_ONLY'})+'\n');
    }catch{process.stderr.write('Closed pre-Continue provider budget history refused\n');process.exitCode=1;}
}
