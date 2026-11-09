// @ts-check
// Original authority is read by byte custody, never recursively re-admitted or refreshed.
import {resolve,basename,dirname} from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,file} from './campaign-proof.mjs';
import {seedCreationRequestAllowance} from './seed-creation-profile.mjs';
import {earlyWindowPreference} from './seed-early-window-custody.mjs';
import {readBudgetHistoryIndex,budgetHistoryNeed as need,budgetHistoryInteger as integer,budgetHistoryObject as object,budgetHistoryTime as time} from './seed-pre-continue-budget-index.mjs';

/** @param {any} v @param {any} ledger @param {any} ledgerPin */
export function readPreContinueBudgetAuthority(v,ledger,ledgerPin){
    const ctx=readBudgetHistoryIndex(v),{reader,index}=ctx,{json,ref,pinned}=reader,s=v.scenario;
    const p=json(resolve(s,'plan.json')),l=json(resolve(s,'native-lane-lease.private.json')),c=json(resolve(s,'controller-start.private.json')),
        root=json(resolve(s,'root-call.actual.private.json')),g=json(resolve(s,'guard-launch-history.private.json')),pre=json(resolve(s,'campaign-postlaunch.private.json')),
        b=p.historyBaseline,clock=pre.rootClock,controllerPin=ref(resolve(s,'controller-start.private.json'));
    need(closed(g,['schemaVersion','kind','rootRecordKey','controllerSha256','history','pending','uncertain','provider','b1ClaimSha256','providerClock','resolvedB1','initialClaimSha256','resolvedActions'])&&
        same(g,ledger)&&same(ref(resolve(s,'guard-launch-history.private.json')),ledgerPin)&&g.schemaVersion===2&&g.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&
        closed(b,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(b).every(x=>integer(x))&&b.reservedActions-b.sentActions===2&&
        closed(v.actions,['actualGuard','currentRunContext','absoluteMaxReservedActions'])&&same(v.actions.actualGuard,g.history)&&
        same(g.history,{...b,reservedActions:b.reservedActions+3,sentActions:b.sentActions+3})&&
        same(v.actions.currentRunContext,{overallAttempts:b.overallAttempts+1,uiAttempts:b.uiAttempts+1})&&v.actions.absoluteMaxReservedActions===b.reservedActions+9&&
        g.uncertain===false&&Array.isArray(g.resolvedActions)&&g.resolvedActions.length===1&&!Object.hasOwn(g,'resolvedInitialActivation'));
    const actions=[g.resolvedB1,g.resolvedActions[0],g.pending];
    for(const [i,a] of actions.entries())need(object(a)&&a.batch===(i===0?'B1':'E1')&&a.tool==='left_click'&&a.status==='AWAITING_AFTER'&&
        a.reservationID===controllerPin.sha256+':'+(b.reservedActions+i+1)&&a.wizardStep?.stage===['PLAY','NEW_GAME','GENERATE'][i]&&
        a.wizardStep.ordinal===(i===0?1:i)&&integer(a.sentAtMs,1)&&integer(a.knownResultAtMs,a.sentAtMs)&&closed(a.args,['coordinate','target_app','target_window_id','focus_strategy'])&&
        Array.isArray(a.args.coordinate)&&a.args.coordinate.length===2&&a.args.coordinate.every((/** @type {any} */ x)=>integer(x))&&
        a.args.target_app==='java.exe'&&integer(a.args.target_window_id,1)&&a.args.focus_strategy==='strict');
    const hash=(/** @type {any} */ value,/** @type {string} */ expected)=>typeof value==='string'&&/^[a-f0-9]{64}$/i.test(value)&&value.toLowerCase()===expected;
    need(p.scenarioId===basename(s)&&p.paths?.scenario===s&&p.source?.qualifiedHead===v.sourceHead&&p.ownership?.driver===v.driver&&p.ownership.onlyGameInputDriver===true&&
        same(p.campaignControl,{schemaVersion:5,profile:'DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'})&&same(l.campaignControl,p.campaignControl)&&
        !Object.hasOwn(p,'providerTiming')&&!Object.hasOwn(l,'providerTiming')&&seedCreationRequestAllowance(p,l)===9&&
        same(p.directHostConsent,{schemaVersion:1,mode:'finite-explicit-consent',maxCallsPerBatch:32})&&same(l.directHostConsent,p.directHostConsent)&&
        l.schemaVersion===1&&l.scenarioId===p.scenarioId&&l.issuer==='/root'&&l.driver===v.driver&&l.authorized===true&&l.phase==='CAMPAIGN_BINDING_ONLY'&&
        l.campaignLoadSaveAllowed===false&&p.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&l.absoluteMaxReservedActions===p.absoluteMaxReservedActions&&
        hash(l.planSha256,ref(resolve(s,'plan.json')).sha256)&&hash(c.rootLeaseSha256,ref(resolve(s,'native-lane-lease.private.json')).sha256)&&
        g.controllerSha256===controllerPin.sha256&&g.rootRecordKey===root.key&&same(l.historyBaseline,b));
    need(closed(root,['key','value'])&&typeof root.key==='string'&&root.key.length>0&&closed(root.value,['startMs','args','raw'])&&integer(root.value.startMs,1)&&
        object(root.value.args)&&root.value.args.workdir===v.sourceWorkspace&&typeof root.value.args.shell==='string'&&
        closed(clock,['recordKey','startMs','deadlineMs','sessionWatchdogMs','cleanupReserveMs','callerAdmissionDeadlineMs','originalCallerAdmissionDeadlineMs','producerRecordKey','effectiveDeadlineMs'])&&
        clock.recordKey===root.key&&clock.startMs===root.value.startMs&&clock.sessionWatchdogMs===1800000&&clock.deadlineMs===clock.startMs+1800000&&
        clock.cleanupReserveMs===30000&&integer(clock.callerAdmissionDeadlineMs,clock.startMs+1)&&clock.callerAdmissionDeadlineMs<=clock.deadlineMs&&
        clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.callerAdmissionDeadlineMs===l.callerAdmissionDeadlineMs&&
        clock.originalCallerAdmissionDeadlineMs===clock.callerAdmissionDeadlineMs&&c.installation===resolve(s,'installation')&&
        integer(c.controllerPid,1)&&integer(c.sessionId,1)&&typeof c.controllerKernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(c.controllerKernelCreationFileTime)&&
        [c.at,c.controllerCreationTime,c.conservativeOuterDeadlineUtc,c.controllerDeadlineUtc].every(time)&&
        clock.startMs<=Date.parse(c.controllerCreationTime)&&Date.parse(c.controllerCreationTime)<=Date.parse(c.at)&&Date.parse(c.at)<Date.parse(c.controllerDeadlineUtc)&&
        Date.parse(c.controllerDeadlineUtc)<=Date.parse(c.conservativeOuterDeadlineUtc)&&Date.parse(c.conservativeOuterDeadlineUtc)<=Date.parse(c.controllerCreationTime)+1800000&&
        c.sessionWatchdogMs===1800000&&c.hardTimeoutMs===1800000&&Array.isArray(c.supervisorJobPids)&&c.supervisorJobPids.length>0&&
        c.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1))&&new Set(c.supervisorJobPids).size===c.supervisorJobPids.length&&c.supervisorJobPids.includes(c.controllerPid)&&
        clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,clock.callerAdmissionDeadlineMs,Date.parse(c.conservativeOuterDeadlineUtc),Date.parse(c.controllerDeadlineUtc)));
    need(same(pre,{schemaVersion:1,kind:'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR',plan:ref(resolve(s,'plan.json')),lease:ref(resolve(s,'native-lane-lease.private.json')),
        controller:controllerPin,rootClock:clock,producerWitness:{producerRecordKey:'controller-start:'+controllerPin.sha256,controllerPid:c.controllerPid,
            controllerCreationTime:c.controllerCreationTime,controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,conservativeOuterDeadlineUtc:c.conservativeOuterDeadlineUtc,
            controllerDeadlineUtc:c.controllerDeadlineUtc,effectiveDeadlineMs:clock.effectiveDeadlineMs,sessionWatchdogMs:1800000},rootCallRecord:root,historyBaseline:b,maxReservedActions:p.absoluteMaxReservedActions})&&
        clock.producerRecordKey===pre.producerWitness.producerRecordKey);
    const support=json(resolve(s,'frozen-support-pins.private.json'));
    need(closed(support,['pins'])&&Array.isArray(support.pins)&&support.pins.length>0&&support.pins.length<=1024&&same(p.source.supportPins,ref(resolve(s,'frozen-support-pins.private.json')))&&
        l.supportPinsPath===resolve(s,'frozen-support-pins.private.json')&&hash(l.supportPinsSha256,ref(l.supportPinsPath).sha256)&&
        reader.bytes(resolve(s,'source-pins.private.json')).bytes.equals(reader.bytes(l.supportPinsPath).bytes));
    const seen=new Set();for(const pin of support.pins){need(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&resolve(pin.path)===pin.path&&typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/i.test(pin.sha256)&&!seen.has(pin.path));
        seen.add(pin.path);need(file(pin.path,128*1024*1024).sha256===pin.sha256.toLowerCase());}
    const executable=(/** @type {string} */ name)=>{const pins=support.pins.filter((/** @type {any} */ row)=>basename(row.path).toLowerCase()===name);need(pins.length===1);return pins[0].path;};
    const node=executable('node.exe'),powershell=executable('pwsh.exe');need(resolve(root.value.args.shell)===powershell);
    const grant=json(resolve(s,'root-history-baseline.admitted.private.json'));
    need(closed(grant,['schemaVersion','kind','issuer','driver','historyBaseline','maxReservedActions','previous'])&&grant.schemaVersion===3&&
        grant.kind==='ROOT_ADMITTED_INITIAL_FOCUS_SEED_CREATION_HISTORY'&&grant.issuer==='/root'&&grant.driver===v.driver&&same(grant.historyBaseline,b)&&
        grant.maxReservedActions===p.absoluteMaxReservedActions&&closed(grant.previous,['result','ledger'])&&same(p.historyAdmission,l.historyAdmission)&&
        same(p.historyAdmission,ref(resolve(s,'root-history-baseline.admitted.private.json'))));
    // Older history is retained by immutable bytes, not replayed as current authority.
    for(const pin of Object.values(/** @type {Record<string,any>} */(grant.previous)))need(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256)&&file(pin.path,262144).sha256===pin.sha256);
    const fixture=json(resolve(s,'protected-fixture-receipt.private.json')),binding=json(resolve(s,'fixture-binding.private.json'));
    need(same(p.fixtureBinding,ref(resolve(s,'fixture-binding.private.json')))&&binding.schemaVersion===3&&binding.fixtureProfile==='VANILLA_UNCOMPRESSED_GENERATION'&&binding.fixtureRoot===s&&
        fixture.phase==='FROZEN_PREPARATION_ONLY'&&fixture.originalFilesVerified===182&&fixture.originalSaveFiles===178&&fixture.backupVerified===true&&fixture.originalWrites===false&&
        fixture.nativeLaunched===false&&fixture.gameLaunched===false&&fixture.privateAutosaveEnabled===false&&fixture.privateSaveRootEmpty===true&&
        same(fixture.originalAutosave,{enabledByDefault:true,maximumSlots:3,featureEnabled:true})&&hash(l.fixtureReceiptSha256,ref(resolve(s,'protected-fixture-receipt.private.json')).sha256)&&
        hash(c.fixtureReceiptSha256,ref(resolve(s,'protected-fixture-receipt.private.json')).sha256)&&hash(c.sourcePinsSha256,ref(resolve(s,'frozen-support-pins.private.json')).sha256)&&
        same(p.seedCreation,l.seedCreation)&&same(p.seedCreation,{mode:'CREATE_ONE_PRIVATE_DISPOSABLE_SEED',preference:ref(resolve(s,'seed-creation-preference.private.json'))})&&
        same(json(resolve(s,'seed-creation-preference.private.json')),earlyWindowPreference));
    const helperNames=['launch-owned-caller.private.ps1','record-root-call-and-stage-B1.private.ps1','launch-and-stage.orchestration.proposed.private.js',
        'construct-actual-phase-inputs.private.mjs','driver-timing-capture.unissued.private.json','seed-save-closeout.template.private.mjs',
        'root-freeze-and-preflight.seed.proposed.private.ps1','frame-menu-phase.failfast.proposed.private.ps1','read-client-geometry.owned.private.mjs'];
    need(closed(p.preclockHelpers,helperNames));
    for(const [name,pin] of Object.entries(/** @type {Record<string,any>} */(p.preclockHelpers)))need(closed(pin,['path','sha256'])&&pin.path===resolve(s,name)&&
        typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256)&&file(pin.path,262144).sha256===pin.sha256);
    const d=json(resolve(s,'root-postlaunch-mechanical-delegation.private.json'));
    need(d.schemaVersion===1&&d.kind==='ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'&&d.issuer==='/root'&&d.driver===v.driver&&same(d.allowedModes,['init','phase B1'])&&
        d.planSha256===ref(resolve(s,'plan.json')).sha256&&d.leaseSha256===ref(resolve(s,'native-lane-lease.private.json')).sha256&&d.historyAdmissionSha256===p.historyAdmission.sha256&&
        d.helperSha256===p.preclockHelpers['construct-actual-phase-inputs.private.mjs'].sha256&&d.callerSha256===p.preclockHelpers['launch-owned-caller.private.ps1'].sha256&&
        d.orchestrationSha256===p.preclockHelpers['launch-and-stage.orchestration.proposed.private.js'].sha256&&d.stagerSha256===p.preclockHelpers['record-root-call-and-stage-B1.private.ps1'].sha256&&
        time(d.atUtc)&&Date.parse(d.atUtc)<=root.value.startMs);
    const q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    const command="$env:PATH='"+dirname(node).replaceAll('\\','/')+';'+dirname(powershell).replaceAll('\\','/')+";'+$env:PATH;& "+q(powershell)+
        ' -NoProfile -NonInteractive -File '+q(resolve(s,'launch-owned-caller.private.ps1'))+' -PlanSha256 '+q(ref(resolve(s,'plan.json')).sha256)+
        ' -LeaseSha256 '+q(ref(resolve(s,'native-lane-lease.private.json')).sha256)+' -DelegationSha256 '+q(ref(resolve(s,'root-postlaunch-mechanical-delegation.private.json')).sha256)+
        ' -HelperSha256 '+q(d.helperSha256)+' -HistoryAdmissionSha256 '+q(p.historyAdmission.sha256);
    need(root.value.args.cmd===command&&root.value.args.sandbox_permissions==='require_escalated'&&root.value.args.yield_time_ms===1000&&root.value.args.max_output_tokens===1500);
    /** @type {Record<string,any>} */const phases={};
    for(const batch of ['B1','E1']){
        const source=ref(resolve(s,batch+'-authorization.private.json')),auth=json(source.path),claimPin=ref(resolve(s,'guard-'+batch+'-consumed.private.json')),claim=json(claimPin.path);
        need(closed(auth,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256'])&&auth.schemaVersion===7&&auth.scenario===s&&auth.batch===batch);
        for(const [pathKey,hashKey,name] of [['planPath','planSha256',batch+'-plan.private.json'],['leasePath','leaseSha256',batch+'-lease.private.json'],['ownedReceiptPath','ownedReceiptSha256','driver-'+batch+'-owned.private.json']])pinned({path:auth[pathKey],sha256:auth[hashKey]},resolve(s,name));
        const bp=json(auth.planPath),bl=json(auth.leasePath),owned=json(auth.ownedReceiptPath),bv=claim.binding,cam=bv?.campaign;
        need(closed(claim,['sourcePath','sourceSha256','binding'])&&claim.sourcePath===source.path&&claim.sourceSha256===source.sha256&&bv.schemaVersion===7&&bv.batch===batch&&
            typeof bv.runID==='string'&&/^[A-Za-z0-9_-]+$/.test(bv.runID)&&bv.scenario===s&&bv.workspaceRoot===v.sourceWorkspace&&bv.planSha256===auth.planSha256&&
            Array.isArray(bv.ownedPids)&&bv.ownedPids.length===1&&integer(bv.ownedPids[0],1)&&bv.sourcePath===source.path&&bv.sourceSha256===source.sha256&&
            bv.controllerSha256===controllerPin.sha256&&bv.originalPlanSha256===ref(resolve(s,'plan.json')).sha256&&!Object.hasOwn(bv,'providerTimeoutMs')&&
            same(cam,{flow:'DISPOSABLE_SEED_CREATION_INITIAL_FOCUS',windowId:cam?.windowId,rootClock:clock,producerWitness:pre.producerWitness,rootCallRecord:root,
                historyBaseline:b,maxReservedActions:p.absoluteMaxReservedActions,launchPredecessor:ref(resolve(s,'campaign-postlaunch.private.json'))})&&integer(cam.windowId,1));
        const menu=batch==='E1'?{b1MenuReceipt:ref(resolve(s,'B1-menu-inspected.private.json'))}:{};
        if(batch==='E1')need(same(bv.b1MenuReceipt,{...menu.b1MenuReceipt,newGameCoordinate:json(resolve(s,'B1-menu-inspected.private.json')).inspection.newGameCoordinate}));
        need(same(bp,{...p,campaign:cam,batches:p.batches.filter((/** @type {any} */ row)=>row.id===batch),...menu})&&
            same(bl,{...l,schemaVersion:7,phase:'DISPOSABLE_SEED_CREATION_INITIAL_FOCUS',campaign:cam,planSha256:auth.planSha256,...menu})&&
            closed(owned,['at','sourceSnapshot','processes'])&&time(owned.at)&&Array.isArray(owned.processes)&&owned.processes.length===1&&
            closed(owned.sourceSnapshot,['path','sha256'])&&owned.sourceSnapshot.path===resolve(s,'owned-processes-current.private.json')&&
            typeof owned.sourceSnapshot.sha256==='string'&&/^[a-f0-9]{64}$/.test(owned.sourceSnapshot.sha256));
        phases[batch]={source,auth,claim,claimPin,owned,output:resolve(v.sourceWorkspace,'.tmp/starsector-proxy',bv.runID)};
    }
    need(g.b1ClaimSha256===phases.B1.claimPin.sha256&&g.initialClaimSha256===phases.E1.claimPin.sha256&&
        same(g.provider,{batch:'E1',runID:phases.E1.claim.binding.runID,sourceSha256:phases.E1.source.sha256})&&same(g.pending.sourceSha256,phases.E1.source.sha256));
    return{...ctx,p,l,c,controller:c,root,g,pre,clock,node,powershell,support,controllerPin,phases,actions};
}
