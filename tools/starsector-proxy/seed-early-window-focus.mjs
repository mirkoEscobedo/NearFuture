// @ts-check
// Exactly the closed doctor/policy/unfocused-owned-window prefix. No screenshot or foreground observation is invented.
import {resolve,dirname,isAbsolute} from 'node:path';
import {readdirSync,existsSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {preStartPacket,preStartReader} from './seed-pre-start-margin-records.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {inspectModernSeedPins} from './seed-creation-custody.mjs';
import {readEarlyWindowAuthority,assertEarlyWindowCaller,earlyWindowProviderFiles,earlyWindowPreference,
    earlyWindowNeed as need,earlyWindowObject as object,earlyWindowInteger as integer,earlyWindowTime as time} from './seed-early-window-custody.mjs';

export const earlyWindowFocusHistoryKind='ROOT_CLOSED_EARLY_WINDOW_FOCUS_STOP_HISTORY';
const resultFields=['schemaVersion','kind','scenario','sourceHead','driver','verdict','status','failure','failureAtMs','historyBaseline','currentRunContext',
    'absoluteMaxReservedActions','counts','callOrder','focus','beforeImage','B1','E1','E2','ordinaryExit','forcedOwnedCleanup','rawLaunchVerdict','seedCreated',
    'seedAccepted','stabilityObserved','finalCallerInvoked','noCurrentClockRenewal','originalRootDeadlineMs','rootPostSmokeWrapperFailure','metadataPreservationFailure',
    'typedHistoryAdmissionCreated','allHistoricalFailuresAndSpentBudgetsRetained','next','evidence','cleanup'];
/** @param {any} raw */
function terminalPacket(raw){
    need(closed(raw,['content'])&&Array.isArray(raw.content)&&raw.content.length===1&&closed(raw.content[0],['type','text'])&&raw.content[0].type==='text'&&
        typeof raw.content[0].text==='string'&&Buffer.byteLength(raw.content[0].text)<=262144);return zeroProviderObject(raw.content[0].text,need);
}
/** @param {any} ids */
function pids(ids){return Array.isArray(ids)&&ids.length>0&&ids.every((/** @type {any} */ id)=>integer(id,1))&&new Set(ids).size===ids.length;}
/** @param {any} v @param {any} ledger @param {any} ledgerPin */
export function readEarlyWindowFocusHistory(v,ledger,ledgerPin){
    const ctx=readEarlyWindowAuthority(v,ledger,ledgerPin),{a,r,p,c,root,g,clock,node,powershell,reader,initial,binding,output}=ctx,s=v.scenario;
    const {json,pinJson,pinned,ref}=reader,stop=a['driver-stop-observation.actual.private.json'],record=a['driver-calls.actual.private.json'];
    need(closed(r,resultFields)&&r.schemaVersion===1&&r.kind==='GENERIC_ACTUAL_DRIVER_FLOW_RESULT'&&r.scenario===s&&r.sourceHead===v.sourceHead&&r.driver===v.driver&&
        r.verdict==='FAIL'&&r.status==='FINAL_FAILED_CLEANED'&&r.failure==='FOCUS_BLOCKED'&&same(r.historyBaseline,g.history)&&same(r.currentRunContext,v.actions.currentRunContext)&&
        r.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&same(r.counts,{providers:1,vendorCalls:3,screenshots:0,originalPNGs:0,geometryModes:0,operationalHelperModes:0,inputs:0,newReservations:0,sentActions:0})&&
        same(r.callOrder,['doctor','policy_status','get_window'])&&same(r.B1,{status:'FOCUS_BLOCKED_BEFORE_INPUT',providerClosure:'PASS'})&&same(r.E1,{status:'NOT_STARTED'})&&same(r.E2,{status:'NOT_STARTED'})&&
        r.forcedOwnedCleanup===true&&r.rawLaunchVerdict==='FAIL'&&r.noCurrentClockRenewal===true&&r.originalRootDeadlineMs===clock.effectiveDeadlineMs&&
        r.allHistoricalFailuresAndSpentBudgetsRetained===true&&r.next==='HOLD; no retry, input, helper mode or authority from this result');
    for(const key of ['ordinaryExit','seedCreated','seedAccepted','stabilityObserved','finalCallerInvoked','typedHistoryAdmissionCreated'])need(r[key]===false);
    need(closed(r.beforeImage,['path','sha256','available','reason'])&&r.beforeImage.path===null&&r.beforeImage.sha256===null&&r.beforeImage.available===false&&
        r.beforeImage.reason==='Stopped at actual owned get_window isFocused=false before screenshot dispatch'&&
        closed(r.rootPostSmokeWrapperFailure,['rawCall','scope'])&&typeof r.rootPostSmokeWrapperFailure.rawCall==='string'&&r.rootPostSmokeWrapperFailure.rawCall.length>0&&r.rootPostSmokeWrapperFailure.rawCall.length<=128&&
        typeof r.rootPostSmokeWrapperFailure.scope==='string'&&r.rootPostSmokeWrapperFailure.scope.length<=4096&&r.rootPostSmokeWrapperFailure.scope.includes('LASTEXITCODE')&&
        closed(r.metadataPreservationFailure,['kind','reason','gameOrProviderEffects'])&&r.metadataPreservationFailure.kind==='ACTUAL_METADATA_WRITE_PRECREATION_FAILURE'&&
        typeof r.metadataPreservationFailure.reason==='string'&&r.metadataPreservationFailure.reason.length<=4096&&r.metadataPreservationFailure.reason.includes('206')&&r.metadataPreservationFailure.gameOrProviderEffects===false);
    need(closed(stop,['schemaVersion','kind','scenario','driver','status','observedAtMs','historyBaseline','currentRunContext','absoluteMaxReservedActions','vendorCalls','inputs','reservations',
        'originalPNGs','batches','failure','rootCleanupRequested','rootCleanupKnown','seedAccepted','ordinaryExit'])&&stop.schemaVersion===1&&stop.kind==='GENERIC_ACTUAL_DRIVER_STOP_OBSERVATION'&&
        stop.scenario===s&&stop.driver===v.driver&&stop.status==='FOCUS_BLOCKED'&&integer(stop.observedAtMs,clock.startMs)&&stop.observedAtMs===r.failureAtMs&&
        same(stop.historyBaseline,g.history)&&same(stop.currentRunContext,v.actions.currentRunContext)&&stop.absoluteMaxReservedActions===p.absoluteMaxReservedActions&&
        stop.vendorCalls===3&&stop.inputs===0&&stop.reservations===0&&stop.originalPNGs===0&&stop.rootCleanupRequested===true&&stop.rootCleanupKnown===false&&stop.seedAccepted===false&&stop.ordinaryExit===false&&
        closed(stop.batches,['B1'])&&closed(stop.batches.B1,['begin','calls','finish']));
    need(closed(record,['schemaVersion','kind','scenario','driver','batches','noOperationalHelperModes','metadataWritePrecreationFailure'])&&record.schemaVersion===1&&
        record.kind==='GENERIC_ACTUAL_FINITE_DRIVER_CALL_RECORDS'&&record.scenario===s&&record.driver===v.driver&&same(record.batches,stop.batches)&&record.noOperationalHelperModes===true&&
        same(record.metadataWritePrecreationFailure,r.metadataPreservationFailure));
    const batch=stop.batches.B1;
    need(closed(v.evidence.provider,earlyWindowProviderFiles));for(const name of earlyWindowProviderFiles)pinned(v.evidence.provider[name],resolve(output,name));
    const ready=json(resolve(output,'guarded-http-ready.private.json')),terminal=json(resolve(output,'owned-provider-terminal.private.json')),spool=json(resolve(output,'owned-provider.stdout.private.txt'));
    need(reader.bytes(resolve(output,'owned-provider.stderr.private.txt')).bytes.length===0);
    const providerClock=g.providerClock;
    need(closed(providerClock,['startMs','deadlineMs'])&&integer(providerClock.startMs,clock.startMs)&&
        providerClock.deadlineMs===Math.min(providerClock.startMs+180000,clock.effectiveDeadlineMs-30000)&&
        ready.schemaVersion===1&&ready.runID===binding.runID&&ready.batch==='B1'&&ready.sourceSha256===ref(resolve(s,'B1-authorization.private.json')).sha256&&
        ready.startupTimeoutMs===20000&&ready.consentMode==='finite-explicit-consent'&&time(ready.at)&&Date.parse(ready.at)>=providerClock.startMs&&Date.parse(ready.at)<=providerClock.startMs+20000&&
        pids(ready.supervisorJobPids)&&ready.supervisorJobPids.length===2&&ready.query==='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)');
    for(const [row,executable] of [[ready.node,node],[ready.helper,powershell]])need(object(row)&&integer(row.pid,1)&&integer(row.parentPid,1)&&row.sessionId===c.sessionId&&row.executable===executable&&
        time(row.creationTime)&&Date.parse(row.creationTime)>=providerClock.startMs&&Date.parse(row.creationTime)<=Date.parse(ready.at)&&ready.supervisorJobPids.includes(row.pid));
    need(closed(terminal,['at','verifierPid','verifierStatus','signal','deadline','closeFailed','overflow','spawnFailed','terminalObserved','cleanupUnknown','steps','terminalBoundToChild','verdict','terminalReceipt'])&&
        integer(terminal.verifierPid,1)&&ready.node.parentPid===terminal.verifierPid&&ready.helper.parentPid===ready.node.pid&&ready.node.pid!==ready.helper.pid&&
        terminal.verifierStatus===0&&terminal.signal===null&&terminal.closeFailed===false&&terminal.overflow===false&&terminal.spawnFailed===false&&terminal.terminalObserved===true&&
        terminal.cleanupUnknown===false&&terminal.terminalBoundToChild===true&&terminal.verdict==='PASS'&&terminal.terminalReceipt===resolve(output,'owned-provider-terminal.private.json')&&
        time(terminal.at)&&time(terminal.deadline)&&Date.parse(terminal.at)>=Date.parse(ready.at)&&Date.parse(terminal.at)<=Date.parse(terminal.deadline)&&
        Date.parse(terminal.deadline)<=providerClock.deadlineMs&&Array.isArray(terminal.steps)&&terminal.steps.length===1&&
        spool.schemaVersion===1&&spool.command==='verify'&&spool.ok===true&&spool.result?.verdict==='PASS'&&Array.isArray(spool.result.steps)&&spool.result.steps.length===1);
    for(const step of [terminal.steps[0],spool.result.steps[0]])need(object(step)&&step.command===node&&same(step.args,[resolve(v.sourceWorkspace,'tools/starsector-proxy/guarded-http-child.mjs')])&&
        step.cwd==='.'&&step.status===0&&step.timedOut===false&&step.processOwnership==='windows-job-object');
    const begin=batch.begin,finish=batch.finish;
    need(closed(begin,['tool','args','startedAtMs','raw','completedAtMs'])&&begin.tool==='begin_batch'&&closed(begin.args,['consentMode','runID','source'])&&
        begin.args.consentMode==='finite-explicit-consent'&&begin.args.runID===binding.runID&&typeof begin.args.source==='string'&&isAbsolute(begin.args.source)&&
        resolve(begin.args.source)===ref(resolve(s,'B1-authorization.private.json')).path&&same(terminalPacket(begin.raw),{runID:binding.runID,output,deadline:terminal.deadline})&&
        integer(begin.startedAtMs,clock.startMs)&&integer(begin.completedAtMs,begin.startedAtMs)&&begin.startedAtMs<=providerClock.startMs&&
        begin.startedAtMs<=Date.parse(ready.at)&&Date.parse(ready.at)<=begin.completedAtMs&&
        closed(finish,['args','startedAtMs','raw','completedAtMs'])&&same(finish.args,{})&&same(terminalPacket(finish.raw),terminal)&&
        finish.startedAtMs===stop.observedAtMs&&integer(finish.completedAtMs,finish.startedAtMs)&&finish.startedAtMs<=Date.parse(terminal.at)&&Date.parse(terminal.at)<=finish.completedAtMs);
    const expectedNames=['doctor','policy_status','get_window'];
    need(Array.isArray(v.evidence.calls)&&v.evidence.calls.length===3&&Array.isArray(batch.calls)&&batch.calls.length===3);
    for(const suffix of ['.private.json','.attempt.private.json','.raw.private.json'])need(same(readdirSync(output).filter(n=>new RegExp('^call-[0-9]{3}'+suffix.replaceAll('.','\\.')+'$').test(n)).sort(),
        [1,2,3].map(i=>'call-'+String(i).padStart(3,'0')+suffix)));
    need(!readdirSync(output).some(n=>/\.png$/i.test(n)||/^capture-|^geometry|^client-point/.test(n)));
    let previous=begin.completedAtMs,protocol=0;const packets=[];
    for(const [i,name] of expectedNames.entries()){
        const pins=v.evidence.calls[i],prefix=resolve(output,'call-'+String(i+1).padStart(3,'0')),call=batch.calls[i];need(closed(pins,['post','attempt','raw']));
        const post=pinJson(pins.post,prefix+'.private.json'),attempt=pinJson(pins.attempt,prefix+'.attempt.private.json'),raw=pinJson(pins.raw,prefix+'.raw.private.json');
        need(closed(call,['name','args','startedAtMs','raw','completedAtMs'])&&call.name===name&&same(call.raw,raw)&&same(call.args,post.args)&&
            integer(call.startedAtMs,previous)&&integer(call.completedAtMs,call.startedAtMs)&&call.completedAtMs<=Date.parse(terminal.at)&&
            post.tool===name&&attempt.tool===name&&same(post.args,attempt.args)&&post.runID===binding.runID&&attempt.runID===binding.runID&&post.batch==='B1'&&attempt.batch==='B1'&&
            attempt.sourceSha256===ref(resolve(s,'B1-authorization.private.json')).sha256&&post.freshness==='UNKNOWN'&&post.isError===false&&same(post.precedingActions,[])&&
            integer(post.outerProtocolID,protocol+1)&&attempt.outerProtocolID===post.outerProtocolID&&post.innerProtocolID===i+1&&attempt.innerProtocolID==='UNKNOWN'&&
            post.callStarted===attempt.callStarted&&time(post.callStarted)&&time(post.callCompleted)&&call.startedAtMs<=Date.parse(post.callStarted)&&
            Date.parse(post.callStarted)<=Date.parse(post.callCompleted)&&Date.parse(post.callCompleted)<=call.completedAtMs&&call.completedAtMs<=Math.min(providerClock.deadlineMs,Date.parse(terminal.deadline)));
        previous=call.completedAtMs;protocol=post.outerProtocolID;packets.push(preStartPacket(raw));
    }
    const doctor=packets[0],policy=packets[1],window=packets[2];
    need(object(doctor)&&doctor.ok===true&&Array.isArray(doctor.checks)&&object(policy)&&policy.approval_required_for_all===true&&policy.approval_token_configured===false&&
        same(batch.calls[0].args,{include_remediation:false})&&same(batch.calls[1].args,{})&&object(window)&&integer(window.pid,1)&&integer(window.windowId,1)&&
        window.pid===binding.ownedPids[0]&&window.windowId===binding.campaign.windowId&&window.bundleId==='java.exe'&&window.isFocused===false&&window.isOnScreen===true&&
        closed(window.bounds,['x','y','width','height'])&&Object.values(window.bounds).every(Number.isSafeInteger)&&window.bounds.width>0&&window.bounds.height>0&&same(batch.calls[2].args,{window_id:window.windowId})&&
        stop.observedAtMs>=batch.calls[2].completedAtMs&&stop.observedAtMs<=Date.parse(terminal.at)&&closed(stop.failure,['kind','atMs','actualWindow','reason'])&&
        stop.failure.kind==='FOCUS_BLOCKED'&&stop.failure.atMs===stop.observedAtMs&&same(stop.failure.actualWindow,batch.calls[2].raw)&&
        typeof stop.failure.reason==='string'&&stop.failure.reason.length>0&&stop.failure.reason.length<=4096);
    const diagnostics=doctor.checks.filter((/** @type {any} */ row)=>row?.id==='ui_automation');need(diagnostics.length===1);
    const diagnostic=diagnostics[0].details?.frontmost;
    need(object(diagnostic)&&integer(diagnostic.pid,1)&&typeof diagnostic.bundleId==='string'&&typeof diagnostic.displayName==='string'&&
        same(r.focus,{ownedPid:window.pid,ownedHwnd:window.windowId,ownedWindowFocused:false,doctorFrontmost:diagnostic,
            cause:'No cause inferred beyond actual observations',activationAllowed:false}));
    const caller=assertEarlyWindowCaller(ctx),owned=a['owned-processes-current.private.json'],protection=a['after-smoke-protection.private.json'];
    need(object(owned)&&owned.controllerPid===c.controllerPid&&owned.controllerCreationTime===c.controllerCreationTime&&owned.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&
        time(owned.at)&&Array.isArray(owned.processes)&&owned.processes.length>=1&&owned.processes.length<=2&&
        closed(initial,['at','sourceSnapshot','processes'])&&time(initial.at)&&Array.isArray(initial.processes)&&initial.processes.length===1&&
        closed(initial.sourceSnapshot,['path','sha256'])&&initial.sourceSnapshot.path===resolve(s,'owned-processes-current.private.json')&&
        typeof initial.sourceSnapshot.sha256==='string'&&/^[a-f0-9]{64}$/.test(initial.sourceSnapshot.sha256)&&Date.parse(initial.at)<=Date.parse(json(v.evidence.calls[0].post.path).callStarted));
    const java=resolve(s,'installation/jre/bin/java.exe'),launcher=resolve(s,'installation/starsector.exe');
    for(const snapshot of [initial,owned]){
        need(new Set(snapshot.processes.map((/** @type {any} */ row)=>row.pid)).size===snapshot.processes.length);
        for(const row of snapshot.processes)need(object(row)&&integer(row.pid,1)&&integer(row.parentPid,1)&&row.parentPid!==row.pid&&[java,launcher].includes(row.executable)&&
            row.sessionId===c.sessionId&&row.inSupervisorJob===true&&time(row.creationTime)&&Date.parse(row.creationTime)>=Date.parse(c.controllerCreationTime)&&Date.parse(row.creationTime)<=Date.parse(snapshot.at)&&
            typeof row.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(row.kernelCreationFileTime)&&pids(row.supervisorJobPids)&&
            [row.pid,row.parentPid,c.controllerPid,...c.supervisorJobPids].every(id=>row.supervisorJobPids.includes(id)));
    }
    const games=owned.processes.filter((/** @type {any} */ row)=>row.executable===java),first=initial.processes[0];
    need(games.length===1&&first.executable===java&&same(binding.ownedPids,[window.pid]));const game=games[0];
    for(const row of [first,game])need(row.pid===window.pid&&((typeof row.mainWindowHandle==='number'&&integer(row.mainWindowHandle,1))||
        (typeof row.mainWindowHandle==='string'&&/^[1-9][0-9]*$/.test(row.mainWindowHandle)&&integer(Number(row.mainWindowHandle),1)))&&Number(row.mainWindowHandle)===window.windowId);
    need(['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(first[key],game[key])));
    const ids=[...new Set([c.controllerPid,...c.supervisorJobPids,...owned.processes.flatMap((/** @type {any} */ row)=>[row.pid,...row.supervisorJobPids]),
        ...initial.processes.flatMap((/** @type {any} */ row)=>[row.pid,...row.supervisorJobPids]),terminal.verifierPid,...ready.supervisorJobPids])].sort((x,y)=>x-y);
    const absence=a['owned-terminal-pid-absence.actual.private.json'];
    need(ids.length===8&&closed(absence,['schemaVersion','kind','observedAtUtc','queriedExactRecordedPids','liveRows','allAbsent','scope'])&&
        absence.schemaVersion===1&&absence.kind==='ACTUAL_EXACT_OWNED_PID_ABSENCE'&&time(absence.observedAtUtc)&&typeof absence.scope==='string'&&absence.scope.length<=4096&&
        same(absence.queriedExactRecordedPids,ids)&&same(absence.liveRows,[])&&absence.allAbsent===true&&Date.parse(absence.observedAtUtc)>=Date.parse(terminal.at)&&
        time(protection.at)&&Date.parse(protection.at)>=Date.parse(terminal.at)&&Date.parse(owned.at)<=Date.parse(protection.at)&&Date.parse(owned.at)<=Date.parse(absence.observedAtUtc)&&
        protection.phase==='after-smoke'&&protection.protectedOriginalFiles===182&&protection.protectedSaveFiles===178&&protection.allOriginalAndBackupHashesMatch===true&&
        protection.saveCensusUnchanged===true&&same(protection.originalAutosave,{enabled:true,maximumSlots:3})&&protection.privateAutosaveEnabled===false&&protection.privateSaveEntries===0&&
        protection.liveGameProcesses===0&&protection.originalWrites===false&&same(protection.sharedPreference,earlyWindowPreference));
    need(same(r.cleanup,{rootOriginalSession:caller.session,actualOuterExit:1,actualNestedStatus:1,timedOut:false,processOwnership:'windows-job-object',
        reason:'Driver requested owned cleanup; native success not established',providerKnownTerminalReaped:true,rootCallerKnownTerminalReaped:true,
        allExactRecordedPidsAbsent:true,allExactRecordedPids:ids,original182AndBackupHashesUnchanged:true,originalSaveCensus178Unchanged:true,originalAutosaveSlots:3,
        privateAutosaveOff:true,privateSaveEntries:0,liveGames:0,sharedPreferenceUnchanged:true,originalWrites:false,scope:r.cleanup?.scope})&&
        typeof r.cleanup.scope==='string'&&r.cleanup.scope.length>0&&r.cleanup.scope.length<=4096);
    const aliases={calls:ref(resolve(s,'driver-calls.actual.private.json')),stop:ref(resolve(s,'driver-stop-observation.actual.private.json')),plan:ref(resolve(s,'plan.json')),
        lease:ref(resolve(s,'native-lane-lease.private.json')),delegation:ref(resolve(s,'root-postlaunch-mechanical-delegation.private.json')),ledger:ref(resolve(s,'guard-launch-history.private.json')),
        B1source:ref(resolve(s,'B1-authorization.private.json')),B1ready:ref(resolve(output,'guarded-http-ready.private.json')),B1terminal:ref(resolve(output,'owned-provider-terminal.private.json')),
        launchCaller:ref(resolve(s,'launch-caller-terminal.actual.private.json')),afterSmoke:ref(resolve(s,'after-smoke-protection.private.json')),pidAbsence:ref(resolve(s,'owned-terminal-pid-absence.actual.private.json'))};
    need(same(r.evidence,aliases));
    for(const name of ['guard-E1-consumed.private.json','guard-E2-consumed.private.json','E1-authorization.private.json','E2-authorization.private.json',
        'seed-save-stability.actual.private.json','E2-created-seed-inspected.private.json','seed-creation-closeout.private.json'])need(!existsSync(resolve(s,name)));
    return v.actions;
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
    try{
        need(process.argv.length===11&&process.argv[2]==='inspect');
        const [path,sha,ledgerPath,ledgerSha,pinsPath,pinsSha,powershell,supervisor]=process.argv.slice(3);
        inspectModernSeedPins(pinsPath,pinsSha,powershell,supervisor);
        const reader=preStartReader();
        const receipt=reader.pinJson({path,sha256:sha},path),ledger=reader.pinJson({path:ledgerPath,sha256:ledgerSha},ledgerPath);
        const actions=readEarlyWindowFocusHistory(receipt,ledger,{path:ledgerPath,sha256:ledgerSha});
        process.stdout.write(JSON.stringify({schemaVersion:1,kind:'EARLY_WINDOW_FOCUS_HISTORY_CUSTODY',receipt:{path,sha256:sha},ledger:{path:ledgerPath,sha256:ledgerSha},actions,scope:'HISTORICAL_CUSTODY_ONLY'})+'\n');
    }catch{process.stderr.write('Closed early window focus history refused\n');process.exitCode=1;}
}
