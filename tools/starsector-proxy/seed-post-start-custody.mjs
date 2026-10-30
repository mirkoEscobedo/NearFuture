// @ts-check
// A closed historical root/phase graph. Historical grants are pinned, never re-admitted.
import {resolve,basename,dirname} from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {inspectSeedCreationLauncher} from './campaign-authority.mjs';
import {seedWizardInput,seedWizardHandoff} from './seed-wizard-proof.mjs';
import {postStartNeed as need,postStartObject as object,postStartInteger as integer,postStartTime as time,
    postStartPinned,postStartPreference,postStartStages} from './seed-post-start-records.mjs';

export const postStartRootNames=['plan.json','native-lane-lease.private.json','controller-start.private.json','root-call.actual.private.json',
    'campaign-postlaunch.private.json','root-postlaunch-mechanical-delegation.private.json','guard-launch-history.private.json',
    'B1-authorization.private.json','E1-authorization.private.json','E2-authorization.private.json','guard-B1-consumed.private.json',
    'guard-E1-consumed.private.json','guard-E2-consumed.private.json','B1-menu-inspected.private.json','E1-occupation-inspected.private.json',
    'launch-caller-terminal.actual.private.json','launch-supervisor.private.json','after-smoke-protection.private.json',
    'owned-terminal-full-pid-absence.actual.private.json','driver-metadata-calls.actual.private.json','driver-inspections.actual.private.json',
    'driver-provider-calls.actual.private.json','driver-chronology.actual.private.json','driver-call-original-index.actual.private.json',
    'protected-fixture-receipt.private.json','fixture-binding.private.json','frozen-support-pins.private.json','source-pins.private.json',
    'root-history-baseline.admitted.private.json','seed-creation-preference.private.json'];

/** @param {any} v @param {any} reader @param {any} ledger @param {any} ledgerPin */
export function readPostStartAuthority(v,reader,ledger,ledgerPin) {
    const {json,ref}=reader,s=v.scenario;
    need(closed(v.evidence.files,postStartRootNames));
    /** @type {Record<string,any>} */const a={};
    for(const name of postStartRootNames){postStartPinned(reader,v.evidence.files[name],resolve(s,name));a[name]=json(resolve(s,name));}
    const plan=a['plan.json'],lease=a['native-lane-lease.private.json'],controller=a['controller-start.private.json'],root=a['root-call.actual.private.json'];
    const g=a['guard-launch-history.private.json'],pre=a['campaign-postlaunch.private.json'],clock=pre.rootClock,b=plan.historyBaseline;
    need(same(g,ledger)&&same(v.evidence.files['guard-launch-history.private.json'],ledgerPin)&&
        g.schemaVersion===2&&g.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&g.uncertain===false&&
        closed(b,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(b).every((/** @type {any} */ n)=>integer(n))&&
        b.reservedActions-b.sentActions===2&&same(g.history,{...b,reservedActions:b.reservedActions+8,sentActions:b.sentActions+8})&&
        same(v.actions.actualGuard,g.history)&&same(v.actions.currentRunContext,{overallAttempts:b.overallAttempts+1,uiAttempts:b.uiAttempts+1})&&
        v.actions.absoluteMaxReservedActions===b.reservedActions+8);
    need(plan.scenarioId===basename(s)&&plan.paths?.scenario===s&&plan.source?.qualifiedHead===v.sourceHead&&
        same(plan.campaignControl,{schemaVersion:4,profile:'DISPOSABLE_SEED_CREATION'})&&same(lease.campaignControl,plan.campaignControl)&&
        plan.ownership?.driver===v.driver&&plan.ownership.onlyGameInputDriver===true&&plan.sessionWatchdogMs===1800000&&
        lease.sessionWatchdogMs===1800000&&lease.ownedGameHardTimeoutMs===1800000&&plan.bounds?.ownedGameHardTimeoutMs===1800000&&
        plan.bounds.maxScenarioActionsIncludingActivations===8&&plan.bounds.maxProxyInvocations===3&&plan.bounds.proxyTimeoutMs===180000&&
        lease.schemaVersion===1&&lease.scenarioId===basename(s)&&lease.authorized===true&&lease.issuer==='/root'&&lease.driver===v.driver&&
        lease.phase==='CAMPAIGN_BINDING_ONLY'&&lease.maxActions===8&&lease.maxProxyInvocations===3&&lease.campaignLoadSaveAllowed===false&&
        plan.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&lease.absoluteMaxReservedActions===plan.absoluteMaxReservedActions&&
        typeof lease.planSha256==='string'&&lease.planSha256.toLowerCase()===ref(resolve(s,'plan.json')).sha256&&
        typeof controller.rootLeaseSha256==='string'&&controller.rootLeaseSha256.toLowerCase()===ref(resolve(s,'native-lane-lease.private.json')).sha256&&
        g.controllerSha256===ref(resolve(s,'controller-start.private.json')).sha256&&root.key===g.rootRecordKey);
    const cp=ref(resolve(s,'controller-start.private.json'));
    need(object(root.value)&&object(root.value.args)&&typeof root.key==='string'&&root.key.length>0&&integer(root.value.startMs,1)&&
        object(root.value.raw)&&integer(root.value.raw.session_id,1)&&typeof root.value.args.cmd==='string'&&root.value.args.cmd.length>0&&
        root.value.args.workdir===v.sourceWorkspace&&typeof root.value.args.shell==='string'&&
        closed(clock,['recordKey','startMs','deadlineMs','sessionWatchdogMs','cleanupReserveMs','callerAdmissionDeadlineMs',
            'originalCallerAdmissionDeadlineMs','producerRecordKey','effectiveDeadlineMs'])&&
        clock.recordKey===root.key&&clock.startMs===root.value.startMs&&clock.sessionWatchdogMs===1800000&&
        clock.deadlineMs===clock.startMs+1800000&&clock.cleanupReserveMs===30000&&integer(clock.callerAdmissionDeadlineMs,clock.startMs+1)&&
        clock.callerAdmissionDeadlineMs<=clock.deadlineMs&&clock.callerAdmissionDeadlineMs===plan.callerAdmissionDeadlineMs&&
        clock.originalCallerAdmissionDeadlineMs===clock.callerAdmissionDeadlineMs&&lease.callerAdmissionDeadlineMs===clock.callerAdmissionDeadlineMs&&
        time(controller.at)&&time(controller.controllerCreationTime)&&Date.parse(controller.controllerCreationTime)>=clock.startMs&&
        Date.parse(controller.controllerCreationTime)<=Date.parse(controller.at)&&integer(controller.controllerPid,1)&&integer(controller.sessionId)&&
        typeof controller.controllerKernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(controller.controllerKernelCreationFileTime)&&
        time(controller.conservativeOuterDeadlineUtc)&&time(controller.controllerDeadlineUtc)&&
        clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,clock.callerAdmissionDeadlineMs,Date.parse(controller.conservativeOuterDeadlineUtc),Date.parse(controller.controllerDeadlineUtc))&&
        Array.isArray(controller.supervisorJobPids)&&controller.supervisorJobPids.every((/** @type {any} */ n)=>integer(n,1))&&
        new Set(controller.supervisorJobPids).size===controller.supervisorJobPids.length&&controller.supervisorJobPids.includes(controller.controllerPid)&&
        controller.hardTimeoutMs===1800000&&controller.sessionWatchdogMs===1800000);
    need(pre.schemaVersion===1&&pre.kind==='FINITE_CAMPAIGN_LAUNCH_PREDECESSOR'&&same(pre.plan,ref(resolve(s,'plan.json')))&&
        same(pre.lease,ref(resolve(s,'native-lane-lease.private.json')))&&same(pre.controller,cp)&&same(pre.rootCallRecord,root)&&
        same(pre.historyBaseline,b)&&pre.maxReservedActions===v.actions.absoluteMaxReservedActions&&
        clock.producerRecordKey==='controller-start:'+cp.sha256&&object(pre.producerWitness)&&
        pre.producerWitness.controllerPid===controller.controllerPid&&pre.producerWitness.controllerCreationTime===controller.controllerCreationTime&&
        pre.producerWitness.controllerKernelCreationFileTime===controller.controllerKernelCreationFileTime&&
        pre.producerWitness.conservativeOuterDeadlineUtc===controller.conservativeOuterDeadlineUtc&&
        pre.producerWitness.controllerDeadlineUtc===controller.controllerDeadlineUtc&&pre.producerWitness.effectiveDeadlineMs===clock.effectiveDeadlineMs);
    const preference=a['seed-creation-preference.private.json'],historicalGrant=a['root-history-baseline.admitted.private.json'];
    need(same(preference,postStartPreference)&&same(plan.seedCreation,lease.seedCreation)&&
        plan.seedCreation?.mode==='CREATE_ONE_PRIVATE_DISPOSABLE_SEED'&&same(plan.seedCreation.preference,ref(resolve(s,'seed-creation-preference.private.json')))&&
        same(plan.historyAdmission,lease.historyAdmission)&&same(plan.historyAdmission,ref(resolve(s,'root-history-baseline.admitted.private.json')))&&
        historicalGrant.schemaVersion===2&&historicalGrant.kind==='ROOT_ADMITTED_SEED_CREATION_HISTORY'&&historicalGrant.issuer==='/root'&&
        historicalGrant.driver===v.driver&&same(historicalGrant.historyBaseline,b)&&historicalGrant.maxReservedActions===plan.absoluteMaxReservedActions);
    for(const pin of Object.values(/** @type {Record<string,any>} */(historicalGrant.previous)))postStartPinned(reader,pin,pin.path);
    const support=a['frozen-support-pins.private.json'];
    need(closed(support,['pins'])&&Array.isArray(support.pins)&&support.pins.length>0&&support.pins.length<=1024&&
        reader.bytes(resolve(s,'frozen-support-pins.private.json')).bytes.equals(reader.bytes(resolve(s,'source-pins.private.json')).bytes)&&
        lease.supportPinsPath===resolve(s,'frozen-support-pins.private.json')&&lease.supportPinsSha256.toLowerCase()===ref(lease.supportPinsPath).sha256&&
        same(plan.source.supportPins,ref(lease.supportPinsPath)));
    const names=new Set();for(const pin of support.pins){need(object(pin)&&typeof pin.path==='string'&&!names.has(pin.path));names.add(pin.path);postStartPinned(reader,pin,pin.path,128*1024*1024);}
    const fixture=a['protected-fixture-receipt.private.json'],binding=a['fixture-binding.private.json'];
    need(binding.schemaVersion===3&&binding.fixtureProfile==='VANILLA_UNCOMPRESSED_GENERATION'&&binding.fixtureRoot===s&&
        same(plan.fixtureBinding,ref(resolve(s,'fixture-binding.private.json')))&&fixture.phase==='FROZEN_PREPARATION_ONLY'&&
        fixture.originalFilesVerified===182&&fixture.originalSaveFiles===178&&fixture.backupVerified===true&&fixture.originalWrites===false&&
        fixture.gameLaunched===false&&fixture.nativeLaunched===false&&fixture.privateAutosaveEnabled===false&&fixture.privateSaveRootEmpty===true&&
        same(fixture.originalAutosave,{enabledByDefault:true,maximumSlots:3,featureEnabled:true})&&typeof lease.fixtureReceiptSha256==='string'&&
        typeof controller.fixtureReceiptSha256==='string'&&controller.fixtureReceiptSha256.toLowerCase()===ref(resolve(s,'protected-fixture-receipt.private.json')).sha256&&
        lease.fixtureReceiptSha256.toLowerCase()===ref(resolve(s,'protected-fixture-receipt.private.json')).sha256);
    const sources=[g.resolvedB1,...g.resolvedActions,g.resolvedE1,g.pending];
    const actions=postStartStages.map(stage=>{const found=sources.filter(x=>x?.wizardStep?.stage===stage);need(found.length===1);return found[0];});
    need(new Set(actions.map(a=>a.reservationID)).size===8&&actions.every((a,i)=>object(a)&&a.status==='AWAITING_AFTER'&&a.tool==='left_click'&&
        a.batch===(i===0?'B1':i<4?'E1':'E2')&&a.reservationID===cp.sha256+':'+(b.reservedActions+i+1)&&
        integer(a.sentAtMs,clock.startMs)&&integer(a.knownResultAtMs,a.sentAtMs)&&object(a.args)));
    /** @type {Record<string,any>} */const phases={};
    for(const batch of ['B1','E1','E2']) {
        const auth=a[batch+'-authorization.private.json'],source=ref(resolve(s,batch+'-authorization.private.json'));
        need(closed(auth,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256'])&&
            auth.schemaVersion===6&&auth.scenario===s&&auth.batch===batch);
        for(const [pathKey,hashKey,leaf] of [['planPath','planSha256',batch+'-plan.private.json'],['leasePath','leaseSha256',batch+'-lease.private.json'],
            ['ownedReceiptPath','ownedReceiptSha256','driver-'+batch+'-owned.private.json']])postStartPinned(reader,{path:auth[pathKey],sha256:auth[hashKey]},resolve(s,leaf));
        const p=json(auth.planPath),l=json(auth.leasePath),claim=a['guard-'+batch+'-consumed.private.json'],owned=json(auth.ownedReceiptPath);
        need(closed(claim,['sourcePath','sourceSha256','binding'])&&claim.sourcePath===source.path&&claim.sourceSha256===source.sha256&&object(claim.binding)&&claim.binding.schemaVersion===6&&
            claim.binding.batch===batch&&claim.binding.scenario===s&&claim.binding.sourceSha256===source.sha256&&
            claim.binding.controllerSha256===cp.sha256&&same(p.campaign,l.campaign)&&same(p.campaign.rootClock,clock)&&
            same(p.campaign.rootCallRecord,root)&&l.schemaVersion===6&&l.phase==='DISPOSABLE_SEED_CREATION'&&
            l.planSha256===auth.planSha256&&same(p.historyAdmission,plan.historyAdmission)&&same(p.seedCreation,plan.seedCreation)&&
            Array.isArray(owned.processes)&&owned.processes.length===1&&object(owned.processes[0]));
        const campaign=claim.binding.campaign,extra=batch==='B1'?{}:batch==='E1'?{b1MenuReceipt:ref(resolve(s,'B1-menu-inspected.private.json'))}:
            {e1WizardReceipt:ref(resolve(s,'E1-occupation-inspected.private.json'))};
        need(closed(campaign,['flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor'])&&
            campaign.flow==='DISPOSABLE_SEED_CREATION'&&integer(campaign.windowId,1)&&same(campaign.launchPredecessor,ref(resolve(s,'campaign-postlaunch.private.json')))&&
            ['rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions'].every(key=>same(campaign[key],pre[key]))&&
            claim.binding.workspaceRoot===v.sourceWorkspace&&claim.binding.originalPlanSha256===ref(resolve(s,'plan.json')).sha256&&
            claim.binding.sourcePath===source.path&&claim.binding.planSha256===auth.planSha256&&
            same(p,{...plan,campaign,batches:plan.batches.filter((/** @type {any} */ row)=>row.id===batch),...extra})&&
            same(l,{...lease,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:auth.planSha256,...extra}));
        const game=owned.processes[0];need(integer(game.pid,1)&&integer(game.parentPid,1)&&game.parentPid!==game.pid&&
            time(game.creationTime)&&Date.parse(game.creationTime)>=Date.parse(controller.controllerCreationTime)&&
            typeof game.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(game.kernelCreationFileTime)&&
            game.executable===resolve(s,'installation/jre/bin/java.exe')&&game.sessionId===controller.sessionId&&game.inSupervisorJob===true&&
            time(owned.at)&&Date.parse(game.creationTime)<=Date.parse(owned.at)&&
            Array.isArray(game.supervisorJobPids)&&game.supervisorJobPids.every((/** @type {any} */ x)=>integer(x,1))&&
            new Set(game.supervisorJobPids).size===game.supervisorJobPids.length&&
            [controller.controllerPid,game.pid,game.parentPid,...controller.supervisorJobPids].every(id=>game.supervisorJobPids.includes(id))&&
            same(claim.binding.ownedPids,[game.pid]));
        phases[batch]={auth,source,claim,claimPin:ref(resolve(s,'guard-'+batch+'-consumed.private.json')),owned,p,l};
    }
    need(Array.isArray(g.resolvedActions)&&g.resolvedActions.length===5&&g.resolvedB1===actions[0]&&g.resolvedE1===actions[3]&&g.pending===actions[7]&&
        g.b1ClaimSha256===phases.B1.claimPin.sha256&&same(g.provider,{batch:'E2',runID:phases.E2.claim.binding.runID,sourceSha256:phases.E2.source.sha256}));
    for(const [i,action] of actions.entries())need(action.sourceSha256===phases[action.batch].source.sha256&&
        action.runID===phases[action.batch].claim.binding.runID&&action.wizardStep.ordinal===(i===0?1:i<4?i:i-3));
    const delegation=a['root-postlaunch-mechanical-delegation.private.json'];
    need(delegation.schemaVersion===1&&delegation.kind==='ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'&&delegation.issuer==='/root'&&
        delegation.driver===v.driver&&same(delegation.allowedModes,['init','phase B1'])&&delegation.planSha256===ref(resolve(s,'plan.json')).sha256&&
        delegation.leaseSha256===ref(resolve(s,'native-lane-lease.private.json')).sha256&&delegation.historyAdmissionSha256===plan.historyAdmission.sha256&&
        object(plan.preclockHelpers));
    for(const [key,name] of [['helperSha256','construct-actual-phase-inputs.private.mjs'],['callerSha256','launch-owned-caller.private.ps1'],
        ['orchestrationSha256','launch-and-stage.orchestration.proposed.private.js'],['stagerSha256','record-root-call-and-stage-B1.private.ps1']]) {
        postStartPinned(reader,plan.preclockHelpers[name],resolve(s,name));need(delegation[key]===plan.preclockHelpers[name].sha256);
    }
    for(const pin of Object.values(/** @type {Record<string,any>} */(plan.preclockHelpers)))postStartPinned(reader,pin,pin.path);
    const powershell=resolve(root.value.args.shell);
    need(basename(powershell).toLowerCase()==='pwsh.exe'&&support.pins.some((/** @type {any} */ p)=>p.path===powershell));
    const node=support.pins.filter((/** @type {any} */ p)=>basename(p.path).toLowerCase()==='node.exe');need(node.length===1);
    const q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    const command="$env:PATH='"+dirname(node[0].path).replaceAll('\\','/')+';'+dirname(powershell).replaceAll('\\','/')+
        ";'+$env:PATH;& "+q(powershell)+' -NoProfile -NonInteractive -File '+q(resolve(s,'launch-owned-caller.private.ps1'))+
        ' -PlanSha256 '+q(ref(resolve(s,'plan.json')).sha256)+' -LeaseSha256 '+q(ref(resolve(s,'native-lane-lease.private.json')).sha256)+
        ' -DelegationSha256 '+q(ref(resolve(s,'root-postlaunch-mechanical-delegation.private.json')).sha256)+
        ' -HelperSha256 '+q(plan.preclockHelpers['construct-actual-phase-inputs.private.mjs'].sha256)+
        ' -HistoryAdmissionSha256 '+q(plan.historyAdmission.sha256);
    need(root.value.args.cmd===command&&root.value.args.sandbox_permissions==='require_escalated'&&root.value.args.yield_time_ms===1000&&
        root.value.args.max_output_tokens===1500);
    return{a,plan,lease,controller,controllerPin:cp,clock,ledger:g,actions,phases,support,root,powershell};
}

/** Existing wizard decisions consume the original recorded time, not a renewed live clock. @param {any} ctx */
export function assertPostStartWizardControls(ctx) {
    const {reader,phases,ledger:g,actions,providers}=ctx,s=ctx.v.scenario;
    const now=actions[7].knownResultAtMs;
    inspectSeedCreationLauncher(phases.E1.claim.binding);
    seedWizardHandoff({binding:phases.E2.claim.binding,reference:phases.E2.p.e1WizardReceipt,now,
        initial:phases.E1.claim.binding,initialSource:phases.E1.source,
        claim:{value:phases.E1.claim,sha256:phases.E1.claimPin.sha256},state:g,action:g.resolvedE1,
        providerClock:g.resolvedE1.providerClock,alreadyTransferred:true});
    for(const batch of ['E1','E2']) {
        const entries=actions.filter((/** @type {any} */ a)=>a.batch===batch),binding=phases[batch].claim.binding;
        for(const [i,action] of entries.entries()) {
            const previous=i===0?g.resolvedE1:entries[i-1],prefix=batch+'-wizard-'+(i+1);
            if(batch==='E1'&&i===0) {
                const proof=seedWizardInput(binding,{resolvedActions:[],pending:null},action.args,undefined,action.sentAtMs);
                need(proof.stage===action.wizardStep.stage);continue;
            }
            const receipt=reader.json(resolve(s,prefix+'-inspected.private.json'));
            const png=reader.bytes(receipt.frame.pngPath,32*1024*1024),meta=reader.json(receipt.frame.metadataPath);
            reader.json(resolve(s,prefix,'capture-001.raw.private.json'),32*1024*1024);
            const after=i===0?{pngBytes:png.bytes,window:meta.window,capturedAtMs:receipt.frame.capturedAtMs,
                observedAtMs:receipt.inspection.inspectedAtMs}:{...previous.after,pngBytes:png.bytes};
            const proof=seedWizardInput(binding,{resolvedActions:entries.slice(0,Math.max(0,i-1)),resolvedE1:g.resolvedE1,
                pending:i===0?null:previous},action.args,after,action.sentAtMs);
            need(action.wizardStep.inspectionSha256===proof.inspectionSha256&&action.wizardStep.stage===proof.stage&&
                (i===0||previous.seedWizardInspectionSha256===proof.inspectionSha256));
        }
    }
}
