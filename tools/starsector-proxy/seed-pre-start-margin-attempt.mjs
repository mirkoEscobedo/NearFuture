// @ts-check
// One closed seven-click margin stop. It is never successful seed provenance or live authority.
import {resolve,dirname,basename,isAbsolute} from 'node:path';
import {readdirSync,existsSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {inspectSeedCreationLauncher} from './campaign-authority.mjs';
import {seedWizardInput,seedWizardHandoff} from './seed-wizard-proof.mjs';
import {inspectModernSeedPins} from './seed-creation-custody.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {preStartReader,preStartNeed as need,preStartObject as object,preStartInteger as integer,
    preStartTime as time,readPreStartProviders} from './seed-pre-start-margin-records.mjs';
import {assertPreStartGeometry} from './seed-pre-start-margin-geometry.mjs';

export const preStartMarginFailedSeedAttemptKind='ROOT_CLOSED_PRE_START_INPUT_MARGIN_FAILED_SEED_ATTEMPT_HISTORY';
const flow='DISPOSABLE_SEED_CREATION',driver='/root/recovery_game_driver';
const preference={schemaVersion:1,kind:'NATIVE_SEED_SHARED_PREFERENCE_V1',view:'Registry64',node:'/com/fs/starfarer',
    valueName:'canSkipTutorial',registryValueName:'can/Skip/Tutorial',valueKind:'String',value:'true'};
const rootNames=['plan.json','native-lane-lease.private.json','controller-start.private.json','root-call.actual.private.json',
    'driver-metadata-calls.actual.private.json','driver-stop-observation.private.json','guard-launch-history.private.json',
    'B1-authorization.private.json','E1-authorization.private.json','E2-authorization.private.json',
    'guard-B1-consumed.private.json','guard-E1-consumed.private.json','guard-E2-consumed.private.json',
    'B1-menu-inspected.private.json','E1-occupation-inspected.private.json','launch-caller-terminal.actual.private.json',
    'launch-supervisor.private.json','after-smoke-protection.private.json','pid-absence.actual.private.json','progress.pre-native-preparation.original.private.json'];
const providerNames=['guarded-http-ready.private.json','owned-provider-terminal.private.json','owned-provider.stdout.private.txt',
    'owned-provider.stderr.private.txt','batch-dispatch.private.jsonl'];

/** @param {any} v @param {any} ledger @param {any} ledgerPin */
export function readPreStartMarginFailedSeedAttempt(v,ledger,ledgerPin) {
    const normal=(/** @type {any} */ p)=>typeof p==='string'&&isAbsolute(p)&&resolve(p)===p;
    need(closed(v,['schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence'])&&
        v.schemaVersion===1&&v.kind===preStartMarginFailedSeedAttemptKind&&v.issuer==='/root'&&v.driver===driver&&
        v.allOwnedHandlesTerminalReaped===true&&normal(v.scenario)&&normal(v.sourceWorkspace)&&typeof v.sourceHead==='string'&&
        /^[a-f0-9]{40}$/.test(v.sourceHead)&&dirname(v.scenario)===resolve(v.sourceWorkspace,'.tmp/starsector-proxy/scenarios')&&
        closed(v.actions,['actualGuard','currentRunContext','absoluteMaxReservedActions'])&&
        closed(v.evidence,['result','calls','finalSourceCapture'])&&closed(v.evidence.calls,['B1','E1','E2']));
    const reader=preStartReader(),{json,pinJson,pinned,bytes,ref}=reader,s=v.scenario;
    const r=pinJson(v.evidence.result,resolve(s,'driver-flow-result.private.json'));
    need(closed(r,['schemaVersion','kind','scenarioId','driver','status','verdict','failureCategory','sourceHead','history','counts','steps',
        'generatedName','actions','visibleAssertion','failure','closure','protection','claims','evidence','originals','successfulGeometry','atUtc','absentFinalArtifacts'])&&
        r.schemaVersion===1&&r.kind==='ACTUAL_FINITE_DRIVER_FLOW_RESULT'&&r.scenarioId===basename(s)&&r.driver===driver&&
        r.status==='CLOSED_FAIL'&&r.verdict==='FAIL'&&r.failureCategory==='INSUFFICIENT_ORIGINAL_INPUT_MARGIN'&&r.sourceHead===v.sourceHead&&
        typeof r.generatedName==='string'&&r.generatedName.trim().length>0&&r.generatedName.length<=128&&!/[\u0000-\u001f\u007f]/u.test(r.generatedName)&&
        same(r.counts,{vendorCalls:55,originalPngs:10,providers:3,knownClicks:7,activations:0,StartGameInputs:0})&&
        same(r.claims,{campaign:false,seed:false,stability:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,closedHistoryKind:false})&&
        closed(r.evidence,[...rootNames,...['B1','E1','E2'].flatMap(batch=>providerNames.map(name=>batch+'/'+name))])&&
        Array.isArray(r.originals)&&r.originals.length===10);
    /** @type {Record<string,any>} */const a={};
    for(const name of rootNames)a[name]=pinJson(r.evidence[name],resolve(s,name));
    const p=a['plan.json'],l=a['native-lane-lease.private.json'],c=a['controller-start.private.json'],root=a['root-call.actual.private.json'];
    const g=a['guard-launch-history.private.json'],h=r.history,b=p.historyBaseline;
    need(closed(b,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(b).every((n)=>integer(n))&&b.reservedActions-b.sentActions===2&&
        same(g,ledger)&&same(r.evidence['guard-launch-history.private.json'],ledgerPin)&&g.schemaVersion===2&&g.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&
        g.uncertain===false&&same(h.guardBaseline,b)&&same(h.finalGuard,g.history)&&same(v.actions.actualGuard,g.history)&&
        closed(h,['guardBaseline','finalGuard','actualContext','absoluteMaxReservedActions','historicalUnsentReservations','addedReservations','addedSentActions',
            'refunds','pending','uncertain'])&&closed(h.finalGuard,Object.keys(b))&&Object.values(h.finalGuard).every(n=>integer(n))&&
        h.finalGuard.overallAttempts===b.overallAttempts&&h.finalGuard.uiAttempts===b.uiAttempts&&
        h.finalGuard.reservedActions===b.reservedActions+7&&h.finalGuard.sentActions===b.sentActions+7&&
        same(h.actualContext,{overallAttempts:b.overallAttempts+1,uiAttempts:b.uiAttempts+1})&&same(h.actualContext,v.actions.currentRunContext)&&
        h.absoluteMaxReservedActions===b.reservedActions+8&&v.actions.absoluteMaxReservedActions===h.absoluteMaxReservedActions&&
        h.historicalUnsentReservations===2&&h.addedReservations===7&&h.addedSentActions===7&&h.refunds===0&&
        h.pending==='KNOWN_SKIP_AWAITING_AFTER'&&h.uncertain===false);
    const hash=(/** @type {any} */ v,/** @type {string} */ expected)=>typeof v==='string'&&/^[a-f0-9]{64}$/i.test(v)&&v.toLowerCase()===expected;
    const pp=r.evidence['plan.json'],lp=r.evidence['native-lane-lease.private.json'],cp=r.evidence['controller-start.private.json'];
    need(p.scenarioId===basename(s)&&p.paths?.scenario===s&&p.source?.qualifiedHead===v.sourceHead&&p.ownership?.driver===driver&&
        p.ownership.onlyGameInputDriver===true&&same(p.campaignControl,{schemaVersion:4,profile:flow})&&same(l.campaignControl,p.campaignControl)&&
        p.sessionWatchdogMs===480000&&l.sessionWatchdogMs===480000&&p.bounds?.ownedGameHardTimeoutMs===480000&&
        p.bounds.maxScenarioActionsIncludingActivations===8&&p.bounds.maxProxyInvocations===3&&p.bounds.proxyTimeoutMs===180000&&
        l.schemaVersion===1&&l.scenarioId===basename(s)&&l.authorized===true&&l.issuer==='/root'&&l.driver===driver&&
        l.phase==='CAMPAIGN_BINDING_ONLY'&&l.ownedGameHardTimeoutMs===480000&&l.maxActions===8&&l.maxProxyInvocations===3&&
        l.campaignLoadSaveAllowed===false&&l.absoluteMaxReservedActions===h.absoluteMaxReservedActions&&p.absoluteMaxReservedActions===h.absoluteMaxReservedActions&&
        hash(l.planSha256,pp.sha256)&&hash(c.rootLeaseSha256,lp.sha256)&&g.controllerSha256===cp.sha256&&root.key===g.rootRecordKey&&
        integer(root.value?.raw?.session_id,1)&&integer(root.value.startMs,1)&&root.value.args?.workdir===v.sourceWorkspace);
    need(closed(p.fixtureBinding,['path','sha256'])&&p.fixtureBinding.path===resolve(s,'fixture-binding.private.json'));
    const fixture=pinJson(p.fixtureBinding,p.fixtureBinding.path);
    need(fixture.schemaVersion===3&&fixture.fixtureProfile==='VANILLA_UNCOMPRESSED_GENERATION'&&fixture.fixtureRoot===s&&fixture.driverRole===driver);
    const frozen=json(resolve(s,'protected-fixture-receipt.private.json'));
    need(hash(l.fixtureReceiptSha256,ref(resolve(s,'protected-fixture-receipt.private.json')).sha256)&&
        hash(c.fixtureReceiptSha256,ref(resolve(s,'protected-fixture-receipt.private.json')).sha256)&&frozen.originalFilesVerified===182&&
        frozen.originalSaveFiles===178&&frozen.originalWrites===false&&frozen.backupVerified===true&&frozen.privateAutosaveEnabled===false&&frozen.privateSaveRootEmpty===true&&
        same(frozen.originalAutosave,{enabledByDefault:true,maximumSlots:3,featureEnabled:true})&&frozen.nativeLaunched===false&&frozen.gameLaunched===false);
    need(same(p.historyAdmission,l.historyAdmission)&&p.historyAdmission.path===resolve(s,'root-history-baseline.admitted.private.json'));
    pinJson(p.historyAdmission,p.historyAdmission.path);
    need(same(p.seedCreation,l.seedCreation)&&closed(p.seedCreation,['mode','preference'])&&
        p.seedCreation.mode==='CREATE_ONE_PRIVATE_DISPOSABLE_SEED'&&same(pinJson(p.seedCreation.preference,resolve(s,'seed-creation-preference.private.json')),preference));
    need(l.supportPinsPath===resolve(s,'frozen-support-pins.private.json')&&
        hash(l.supportPinsSha256,ref(l.supportPinsPath).sha256)&&p.source.supportPins?.path===l.supportPinsPath&&
        p.source.supportPins.sha256===ref(l.supportPinsPath).sha256&&bytes(resolve(s,'source-pins.private.json')).bytes.equals(bytes(l.supportPinsPath).bytes));
    const support=pinJson(p.source.supportPins,p.source.supportPins.path);
    need(closed(support,['pins'])&&Array.isArray(support.pins)&&support.pins.length>0&&support.pins.length<=1024);
    const paths=new Set();for(const row of support.pins){need(!paths.has(row.path));paths.add(row.path);pinned(row,row.path,128*1024*1024);}
    for(const [name,pin] of Object.entries(/** @type {Record<string,any>} */(p.preclockHelpers)))pinned(pin,resolve(s,name));
    /** @type {Record<string,any>} */const phases={};
    const pre=json(resolve(s,'campaign-postlaunch.private.json')),clock=pre.rootClock,producer=pre.producerWitness;
    need(closed(pre,['schemaVersion','kind','plan','lease','controller','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions'])&&
        pre.schemaVersion===1&&pre.kind==='FINITE_CAMPAIGN_LAUNCH_PREDECESSOR'&&same(pre.plan,pp)&&same(pre.lease,lp)&&same(pre.controller,cp)&&
        same(pre.rootCallRecord,root)&&same(pre.historyBaseline,b)&&pre.maxReservedActions===h.absoluteMaxReservedActions&&
        closed(clock,['recordKey','startMs','deadlineMs','sessionWatchdogMs','cleanupReserveMs','callerAdmissionDeadlineMs',
            'originalCallerAdmissionDeadlineMs','producerRecordKey','effectiveDeadlineMs'])&&clock.recordKey===root.key&&clock.startMs===root.value.startMs&&
        clock.deadlineMs===clock.startMs+480000&&clock.sessionWatchdogMs===480000&&clock.cleanupReserveMs===30000&&
        integer(p.callerAdmissionDeadlineMs,1)&&p.callerAdmissionDeadlineMs===l.callerAdmissionDeadlineMs&&
        clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.originalCallerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&
        p.callerAdmissionDeadlineMs>clock.startMs&&p.callerAdmissionDeadlineMs<=clock.deadlineMs);
    need(integer(c.controllerPid,1)&&integer(c.sessionId,1)&&typeof c.controllerKernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(c.controllerKernelCreationFileTime)&&
        c.sessionWatchdogMs===480000&&c.hardTimeoutMs===480000&&c.installation===resolve(s,'installation')&&
        [c.controllerCreationTime,c.at,c.conservativeOuterDeadlineUtc,c.controllerDeadlineUtc].every(time)&&
        clock.startMs<=Date.parse(c.controllerCreationTime)&&Date.parse(c.controllerCreationTime)<=Date.parse(c.at)&&
        Date.parse(c.at)<Date.parse(c.controllerDeadlineUtc)&&Date.parse(c.controllerDeadlineUtc)<=Date.parse(c.conservativeOuterDeadlineUtc)&&
        Date.parse(c.conservativeOuterDeadlineUtc)<=Date.parse(c.controllerCreationTime)+480000&&
        Array.isArray(c.supervisorJobPids)&&c.supervisorJobPids.every((/** @type {any} */ n)=>integer(n,1))&&
        new Set(c.supervisorJobPids).size===c.supervisorJobPids.length&&c.supervisorJobPids.includes(c.controllerPid));
    need(clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,p.callerAdmissionDeadlineMs,Date.parse(c.conservativeOuterDeadlineUtc),Date.parse(c.controllerDeadlineUtc))&&
        clock.producerRecordKey==='controller-start:'+cp.sha256&&same(producer,{producerRecordKey:clock.producerRecordKey,
            controllerPid:c.controllerPid,controllerCreationTime:c.controllerCreationTime,controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,
            conservativeOuterDeadlineUtc:c.conservativeOuterDeadlineUtc,controllerDeadlineUtc:c.controllerDeadlineUtc,
            effectiveDeadlineMs:clock.effectiveDeadlineMs,sessionWatchdogMs:480000}));
    for(const batch of ['B1','E1','E2']) {
        const source=r.evidence[batch+'-authorization.private.json'],auth=a[batch+'-authorization.private.json'];
        const claimPin=r.evidence['guard-'+batch+'-consumed.private.json'],claim=a['guard-'+batch+'-consumed.private.json'],binding=claim.binding;
        need(closed(auth,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256'])&&
            auth.schemaVersion===6&&auth.scenario===s&&auth.batch===batch&&claim.sourcePath===source.path&&claim.sourceSha256===source.sha256&&
            binding?.schemaVersion===6&&binding.batch===batch&&binding.sourcePath===source.path&&binding.sourceSha256===source.sha256&&
            binding.scenario===s&&binding.workspaceRoot===v.sourceWorkspace&&binding.originalPlanSha256===pp.sha256&&binding.controllerSha256===cp.sha256&&
            Array.isArray(binding.ownedPids)&&binding.ownedPids.length===1&&integer(binding.ownedPids[0],1));
        const phasePlan=pinJson({path:auth.planPath,sha256:auth.planSha256},resolve(s,batch+'-plan.private.json'));
        const phaseLease=pinJson({path:auth.leasePath,sha256:auth.leaseSha256},resolve(s,batch+'-lease.private.json'));
        const owned=pinJson({path:auth.ownedReceiptPath,sha256:auth.ownedReceiptSha256},resolve(s,'driver-'+batch+'-owned.private.json'));
        const campaign=binding.campaign;
        need(binding.planSha256===auth.planSha256&&closed(campaign,['flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor'])&&
            campaign.flow===flow&&integer(campaign.windowId,1)&&same(campaign.launchPredecessor,ref(resolve(s,'campaign-postlaunch.private.json')))&&
            ['rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions'].every(key=>same(campaign[key],pre[key])));
        const extra=batch==='B1'?{}:batch==='E1'?{b1MenuReceipt:r.evidence['B1-menu-inspected.private.json']}:
            {e1WizardReceipt:r.evidence['E1-occupation-inspected.private.json']};
        need(same(phasePlan,{...p,campaign,batches:p.batches.filter((/** @type {any} */ row)=>row.id===batch),...extra})&&
            same(phaseLease,{...l,schemaVersion:6,phase:flow,campaign,planSha256:auth.planSha256,...extra})&&
            Array.isArray(owned.processes)&&owned.processes.filter((/** @type {any} */ row)=>row.pid===binding.ownedPids[0]).length===1);
        phases[batch]={source,auth,claim,claimPin,owned,plan:phasePlan,lease:phaseLease};
    }
    const e1=(g.resolvedActions??[]).filter((/** @type {any} */ row)=>row.batch==='E1'),e2=(g.resolvedActions??[]).filter((/** @type {any} */ row)=>row.batch==='E2');
    need(Array.isArray(g.resolvedActions)&&g.resolvedActions.length===4&&e1.length===2&&e2.length===2&&g.pending?.wizardStep?.stage==='SKIP'&&
        g.pending.wizardStep.ordinal===3&&g.pending.batch==='E2'&&g.pending.sourceSha256===phases.E2.source.sha256&&
        same(g.provider,{batch:'E2',runID:phases.E2.claim.binding.runID,sourceSha256:phases.E2.source.sha256}));
    const actions=[g.resolvedB1,...e1,g.resolvedE1,...e2,g.pending],stages=['PLAY','NEW_GAME','GENERATE','CONTINUE','EXPLORER','NORMAL','SKIP'];
    need(Array.isArray(r.actions)&&r.actions.length===7);
    for(const [i,action] of actions.entries()) {
        const batch=i===0?'B1':i<4?'E1':'E2',ordinal=i===0?1:i<4?i:i-3;
        need(object(action)&&action.status==='AWAITING_AFTER'&&action.tool==='left_click'&&action.batch===batch&&
            action.sourceSha256===phases[batch].source.sha256&&action.runID===phases[batch].claim.binding.runID&&
            action.reservationID===cp.sha256+':'+(b.reservedActions+i+1)&&integer(action.sentAtMs,1)&&integer(action.knownResultAtMs,action.sentAtMs)&&
            action.wizardStep?.stage===stages[i]&&action.wizardStep.ordinal===ordinal&&
            r.actions[i].stage===stages[i]&&r.actions[i].batch===batch&&same(r.actions[i].args,action.args));
    }
    const ctx={v,r,reader,phases,controller:c,controllerPin:cp,clock,ledger:g,actions,metadata:a['driver-metadata-calls.actual.private.json'],
        providers:/** @type {any} */({}),node:'',powershell:'',support};
    const records=readPreStartProviders(ctx);ctx.providers=records.providers;
    ctx.node=ctx.providers.B1.ready.node.executable;ctx.powershell=ctx.providers.B1.ready.helper.executable;
    for(const path of [ctx.node,ctx.powershell])need(support.pins.filter((/** @type {any} */ row)=>row.path===path).length===1);
    assertPreStartLaunch({...ctx,p,l,root,pp,lp});
    inspectSeedCreationLauncher(phases.E1.claim.binding);
    seedWizardHandoff({binding:phases.E2.claim.binding,reference:phases.E2.plan.e1WizardReceipt,now:Date.now(),
        initial:phases.E1.claim.binding,initialSource:phases.E1.source,claim:{value:phases.E1.claim,sha256:phases.E1.claimPin.sha256},
        state:g,action:g.resolvedE1,providerClock:g.resolvedE1.providerClock,alreadyTransferred:true});
    for(let i=0;i<3;i++) {
        const action=e2[i]??g.pending,previous=i===0?g.resolvedE1:e2[i-1],prefix='E2-wizard-'+(i+1);
        const receipt=json(resolve(s,prefix+'-inspected.private.json')),png=bytes(receipt.frame.pngPath,32*1024*1024),meta=json(receipt.frame.metadataPath);
        const after=i===0?{pngBytes:png.bytes,window:meta.window,capturedAtMs:receipt.frame.capturedAtMs,observedAtMs:receipt.inspection.inspectedAtMs}:
            {...previous.after,pngBytes:png.bytes};
        const proof=seedWizardInput(phases.E2.claim.binding,{resolvedActions:e2.slice(0,Math.max(0,i-1)),resolvedE1:g.resolvedE1,pending:i===0?null:previous},
            action.args,after,Date.now());
        need(action.wizardStep.inspectionSha256===proof.inspectionSha256&&action.wizardStep.stage===proof.stage&&
            (i===0||previous.seedWizardInspectionSha256===proof.inspectionSha256));
    }
    const geometryOwned=assertPreStartGeometry(ctx);
    assertPreStartClosure({...ctx,a,p,l,b,support,geometryOwned});
    return v.actions;
}

/** The original recorded caller must invoke the same pinned finite launch graph.
 * This reads historical files; it never executes their helpers. @param {any} ctx */
function assertPreStartLaunch({v,reader,p,l,root,pp,lp,powershell}) {
    const s=v.scenario,h=p.preclockHelpers,names=['launch-owned-caller.private.ps1','record-root-call-and-stage-B1.private.ps1',
        'launch-and-stage.orchestration.proposed.private.js','construct-actual-phase-inputs.private.mjs','driver-timing-capture.unissued.private.json',
        'seed-save-closeout.template.private.mjs','root-freeze-and-preflight.seed.proposed.private.ps1','frame-menu-phase.failfast.proposed.private.ps1',
        'read-client-geometry.owned.private.mjs'];
    need(closed(h,names));
    const dPath=resolve(s,'root-postlaunch-mechanical-delegation.private.json'),d=reader.json(dPath);
    need(closed(d,['schemaVersion','kind','issuer','driver','planSha256','leaseSha256','helperSha256','orchestrationSha256','callerSha256',
        'stagerSha256','historyAdmissionSha256','allowedModes','allowed','limits','forbidden','actualLaunch','atUtc'])&&
        d.schemaVersion===1&&d.kind==='ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'&&d.issuer==='/root'&&d.driver===v.driver&&
        d.planSha256===pp.sha256&&d.leaseSha256===lp.sha256&&d.helperSha256===h['construct-actual-phase-inputs.private.mjs'].sha256&&
        d.callerSha256===h['launch-owned-caller.private.ps1'].sha256&&d.orchestrationSha256===h['launch-and-stage.orchestration.proposed.private.js'].sha256&&
        d.stagerSha256===h['record-root-call-and-stage-B1.private.ps1'].sha256&&d.historyAdmissionSha256===p.historyAdmission.sha256&&
        same(p.historyAdmission,l.historyAdmission)&&same(d.allowedModes,['init','phase B1'])&&time(d.atUtc)&&Date.parse(d.atUtc)<=root.value.startMs);
    const q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    const command='& '+q(powershell)+' -NoProfile -NonInteractive -File '+q(resolve(s,'launch-owned-caller.private.ps1'))+
        ' -PlanSha256 '+q(pp.sha256)+' -LeaseSha256 '+q(lp.sha256)+' -DelegationSha256 '+q(reader.ref(dPath).sha256)+
        ' -HelperSha256 '+q(d.helperSha256)+' -HistoryAdmissionSha256 '+q(d.historyAdmissionSha256);
    need(root.value.args.cmd===command&&root.value.args.shell===powershell.replaceAll('\\','/')&&
        root.value.args.sandbox_permissions==='require_escalated'&&root.value.args.yield_time_ms===1000&&root.value.args.max_output_tokens===1500);
}

/** @param {any} ctx */
function assertPreStartClosure({v,r,reader,a,p,l,b,controller:c,controllerPin:cp,clock,phases,providers,actions,node,powershell,support,geometryOwned}) {
    const {json,pinJson,pinned,bytes,ref}=reader,s=v.scenario,close=r.closure;
    const caller=a['launch-caller-terminal.actual.private.json'],launch=a['launch-supervisor.private.json'],protect=a['after-smoke-protection.private.json'];
    const absent=a['pid-absence.actual.private.json'],root=a['root-call.actual.private.json'],stop=a['driver-stop-observation.private.json'];
    need(launch.schemaVersion===1&&launch.command==='verify'&&launch.ok===true&&launch.result?.verdict==='FAIL'&&
        Array.isArray(launch.result.steps)&&launch.result.steps.length===1);
    const step=launch.result.steps[0];
    need(step.status===1&&step.timedOut===false&&step.processOwnership==='windows-job-object'&&step.command===powershell&&step.cwd==='.'&&
        typeof step.stderr==='string'&&step.stderr.includes('Driver requested owned cleanup; native success not established')&&
        same(step.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),
            '-EvidenceRoot',s,'-BindingPath',p.fixtureBinding.path,'-BindingSha256',p.fixtureBinding.sha256,
            '-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000']));
    need(caller.schemaVersion===1&&caller.kind==='ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL_REAP_OBSERVATION'&&
        caller.sessionId===root.value.raw.session_id&&caller.terminal===true&&caller.reaped===true&&caller.exitCode===1&&caller.timedOut===false&&
        caller.actualNestedVerdict==='FAIL'&&caller.actualNestedChildStatus===1&&caller.processOwnership==='windows-job-object'&&
        caller.reason==='Driver requested owned cleanup; native success not established'&&caller.ordinaryExit===false&&
        same(close.rootCaller,{session:caller.sessionId,terminalToolChunk:caller.toolChunkId,exitCode:1,timedOut:false,reaped:true,verdict:'FAIL',
            kind:'ACTUAL_OBSERVED_TOOL_REAP_NOT_FABRICATED_CONTROLLER_TERMINAL'})&&close.allOwnedHandlesTerminalReaped===true&&
        close.ordinaryExit===false&&close.controllerTerminalExpectedAbsent===true);
    const request=caller.cleanupRequest;
    need(object(request)&&object(request.raw)&&request.raw.exit_code===0&&!Object.hasOwn(request.raw,'session_id')&&
        request.args?.shell===powershell.replaceAll('\\','/')&&typeof request.raw.output==='string');
    const stopped=zeroProviderObject(request.raw.output.trim(),need);
    need(closed(stopped,['atUtc','maintainedOwnedStopRequested','originalCallerSession','extraInput'])&&
        stopped.maintainedOwnedStopRequested===true&&stopped.originalCallerSession===caller.sessionId&&stopped.extraInput===false&&
        time(stopped.atUtc)&&stopped.atUtc.replace(/\+00:00$/u,'Z')===close.maintainedStopRequestedAtUtc&&
        Date.parse(stopped.atUtc)>=Date.parse(providers.E2.terminal.at)&&Date.parse(stopped.atUtc)<=clock.effectiveDeadlineMs-30000);
    need(protect.phase==='after-smoke'&&protect.protectedOriginalFiles===182&&protect.protectedSaveFiles===178&&
        protect.allOriginalAndBackupHashesMatch===true&&protect.saveCensusUnchanged===true&&protect.liveGameProcesses===0&&protect.originalWrites===false&&
        protect.privateAutosaveEnabled===false&&protect.privateSaveEntries===1&&same(protect.originalAutosave,{enabled:true,maximumSlots:3})&&
        same(protect.sharedPreference,preference)&&time(protect.at)&&Date.parse(protect.at)>=Date.parse(stopped.atUtc));
    const current=json(resolve(s,'owned-processes-current.private.json')).processes;
    need(Array.isArray(geometryOwned)&&geometryOwned.length===9);
    const proven=[...Object.values(/** @type {Record<string,any>} */(phases)).flatMap(phase=>phase.owned.processes),...geometryOwned];
    need(Array.isArray(current)&&current.length>0&&proven.length>=3);
    for(const row of proven)need(integer(row.pid,1)&&integer(row.parentPid,1)&&Array.isArray(row.supervisorJobPids)&&
        row.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1))&&row.supervisorJobPids.includes(row.pid)&&row.supervisorJobPids.includes(c.controllerPid)&&
        current.some((/** @type {any} */ actual)=>['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(actual[key],row[key]))));
    const ids=[...new Set([...c.supervisorJobPids,c.controllerPid,...proven.flatMap((/** @type {any} */ row)=>[row.pid,row.parentPid,...row.supervisorJobPids]),
        ...current.flatMap((/** @type {any} */ row)=>[row.pid,...row.supervisorJobPids]),...['B1','E1','E2'].map(batch=>providers[batch].terminal.verifierPid)])].sort((a,b)=>a-b);
    need(absent.schemaVersion===1&&absent.kind==='ACTUAL_S9_ALL_RECORDED_OWNED_PIDS_ABSENT'&&
        same(absent.pids,ids)&&absent.liveCount===0&&absent.allThreeProvidersTerminalPass===true&&absent.originalCallerSession===caller.sessionId&&
        absent.originalCallerReapedExitCode===1&&absent.originalCallerTerminalToolChunk===caller.toolChunkId&&absent.launchSupervisorVerdict==='FAIL'&&
        absent.launchSupervisorStatus===1&&absent.launchSupervisorTimedOut===false&&absent.launchSupervisorProcessOwnership==='windows-job-object'&&
        absent.forcedOwnedCleanup===true&&absent.ordinaryExit===false&&same(absent.preference,preference)&&absent.preferenceUnchanged===true&&absent.sharedWrites===false&&
        time(absent.atUtc)&&Date.parse(absent.atUtc)>=Date.parse(protect.at));
    const saves=resolve(s,'installation/saves'),common=resolve(saves,'common');
    need(same(readdirSync(saves),['common'])&&same(readdirSync(common),['core_shared_settings.json.data'])&&
        bytes(resolve(common,'core_shared_settings.json.data')).bytes.equals(Buffer.from('{"campaignHelpPopupsOptionChecked": true}')));
    need(same(r.protection,{originalAndBackupFiles:182,saveFiles:178,originalAndBackupHashesUnchanged:true,saveCensusUnchanged:true,
        privateAutosave:false,privateSaveEntries:1,privateOutput:r.protection.privateOutput,liveGameProcesses:0,originalWrites:false,
        restoration:'UNNECESSARY_ORIGINALS_UNCHANGED',typedSharedPreference:'Registry64/String/true unchanged'})&&
        typeof r.protection.privateOutput==='string'&&time(r.atUtc)&&Date.parse(r.atUtc)>=Date.parse(absent.atUtc));
    need(stop.schemaVersion===1&&stop.kind==='ACTUAL_S9_DRIVER_PRE_START_MARGIN_STOP'&&stop.driver===driver&&resolve(stop.scenario)===s&&
        stop.failureCategory===r.failureCategory&&stop.rootCallerSession===caller.sessionId&&stop.originalCallerDeadlineMs===clock.effectiveDeadlineMs&&
        stop.campaign===false&&stop.seed===false&&stop.stability===false&&stop.ordinaryExit===false&&stop.rootProtection==='PENDING'&&
        same(stop.counts,{calls:55,originalPngs:10,knownClicks:7,activations:0,providers:3}));
    const calls=Object.values(providers).flatMap((/** @type {any} */ provider)=>provider.calls);
    need(Array.isArray(stop.actualVendorCalls)&&stop.actualVendorCalls.length===55);
    for(const [i,row] of stop.actualVendorCalls.entries())need(row.ordinalOverall===i+1&&row.batch===calls[i].post.batch&&
        row.name===calls[i].post.tool&&same(row.args,row.name==='doctor'?{}:calls[i].post.args)&&
        integer(row.atMs,Date.parse(calls[i].post.callCompleted)));
    need(closed(r.visibleAssertion,['lastOriginal','state','character','localPoint','campaignVisible','freshness','noPauseInference'])&&
        r.visibleAssertion.lastOriginal==='E2/capture-004.png'&&r.visibleAssertion.state==='START_GAME_CONTROL_VISIBLE_ENABLED'&&
        r.visibleAssertion.character===r.generatedName&&Array.isArray(r.visibleAssertion.localPoint)&&r.visibleAssertion.localPoint.length===2&&
        r.visibleAssertion.localPoint.every(Number.isSafeInteger)&&r.visibleAssertion.campaignVisible===false&&
        r.visibleAssertion.freshness==='UNKNOWN'&&r.visibleAssertion.noPauseInference===true);
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
    try {
        need(process.argv.length===11&&process.argv[2]==='inspect');
        const [, , ,receiptPath,receiptSha,ledgerPath,ledgerSha,pinsPath,pinsSha,powershell,supervisor]=process.argv;
        const reader=preStartReader(),receipt={path:receiptPath,sha256:receiptSha},ledgerPin={path:ledgerPath,sha256:ledgerSha};
        const pins=reader.pinJson({path:pinsPath,sha256:pinsSha},pinsPath);
        need(Array.isArray(pins.pins)&&pins.pins.every((/** @type {any} */ row)=>closed(row,['path','sha256'])&&
            typeof row.path==='string'&&typeof row.sha256==='string'));
        inspectModernSeedPins(pinsPath,pinsSha,powershell,supervisor);
        const actions=readPreStartMarginFailedSeedAttempt(reader.pinJson(receipt,receiptPath),reader.pinJson(ledgerPin,ledgerPath),ledgerPin);
        process.stdout.write(JSON.stringify({schemaVersion:1,kind:'PRE_START_MARGIN_HISTORY_CUSTODY',receipt,ledger:ledgerPin,actions,scope:'HISTORICAL_CUSTODY_ONLY'})+'\n');
    } catch(error) {process.stderr.write('Pre-Start margin history custody refused: '+String(/** @type {Error} */(error)?.message??error)+'\n');process.exitCode=1;}
}
