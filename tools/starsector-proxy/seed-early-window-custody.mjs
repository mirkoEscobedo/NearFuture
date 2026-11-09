// @ts-check
// Closed historical authority and recorded terminals. Never executes their helpers or re-admits an old grant.
import {resolve,dirname,basename,isAbsolute} from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,requireThat} from './campaign-proof.mjs';
import {preStartReader} from './seed-pre-start-margin-records.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';

export const earlyWindowNeed=(/** @type {any} */ v)=>requireThat(v,'closed early window focus history custody required');
export const earlyWindowObject=(/** @type {any} */ v)=>v!==null&&typeof v==='object'&&!Array.isArray(v);
export const earlyWindowInteger=(/** @type {any} */ v,min=0)=>Number.isSafeInteger(v)&&v>=min;
export const earlyWindowTime=(/** @type {any} */ v)=>typeof v==='string'&&Number.isSafeInteger(Date.parse(v));
const need=earlyWindowNeed,object=earlyWindowObject,integer=earlyWindowInteger,time=earlyWindowTime;
export const earlyWindowFiles=['plan.json','native-lane-lease.private.json','controller-start.private.json','root-call.actual.private.json',
    'campaign-postlaunch.private.json','root-postlaunch-mechanical-delegation.private.json','guard-launch-history.private.json',
    'B1-authorization.private.json','guard-B1-consumed.private.json','owned-processes-current.private.json','launch-caller-terminal.actual.private.json',
    'launch-supervisor.private.json','after-smoke-protection.private.json','owned-terminal-pid-absence.actual.private.json',
    'seed-creation-preference.private.json','frozen-support-pins.private.json','source-pins.private.json','fixture-binding.private.json',
    'protected-fixture-receipt.private.json','root-history-baseline.admitted.private.json','driver-calls.actual.private.json','driver-stop-observation.actual.private.json'];
export const earlyWindowProviderFiles=['guarded-http-ready.private.json','owned-provider-terminal.private.json',
    'owned-provider.stdout.private.txt','owned-provider.stderr.private.txt','batch-dispatch.private.jsonl'];
export const earlyWindowPreference={schemaVersion:1,kind:'NATIVE_SEED_SHARED_PREFERENCE_V1',view:'Registry64',node:'/com/fs/starfarer',
    valueName:'canSkipTutorial',registryValueName:'can/Skip/Tutorial',valueKind:'String',value:'true'};
/** @param {any} v @param {any} ledger @param {any} ledgerPin */
export function readEarlyWindowAuthority(v,ledger,ledgerPin){
    need(closed(v,['schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence'])&&
        v.schemaVersion===1&&v.kind==='ROOT_CLOSED_EARLY_WINDOW_FOCUS_STOP_HISTORY'&&v.issuer==='/root'&&v.driver==='/root/recovery_game_driver'&&
        v.allOwnedHandlesTerminalReaped===true&&typeof v.scenario==='string'&&isAbsolute(v.scenario)&&resolve(v.scenario)===v.scenario&&
        typeof v.sourceWorkspace==='string'&&isAbsolute(v.sourceWorkspace)&&resolve(v.sourceWorkspace)===v.sourceWorkspace&&
        dirname(v.scenario)===resolve(v.sourceWorkspace,'.tmp/starsector-proxy/scenarios')&&typeof v.sourceHead==='string'&&/^[a-f0-9]{40}$/.test(v.sourceHead)&&
        closed(v.actions,['actualGuard','currentRunContext','absoluteMaxReservedActions'])&&closed(v.evidence,['result','files','provider','calls'])&&
        closed(v.evidence.files,earlyWindowFiles));
    const reader=preStartReader(),s=v.scenario,{pinJson,json,ref}=reader;
    /** @type {Record<string,any>} */const a={};
    for(const name of earlyWindowFiles)a[name]=pinJson(v.evidence.files[name],resolve(s,name));
    const r=pinJson(v.evidence.result,resolve(s,'driver-flow-result.private.json')),p=a['plan.json'],l=a['native-lane-lease.private.json'],
        c=a['controller-start.private.json'],root=a['root-call.actual.private.json'],g=a['guard-launch-history.private.json'],
        pre=a['campaign-postlaunch.private.json'],b=p.historyBaseline,clock=pre.rootClock;
    need(same(g,ledger)&&same(v.evidence.files['guard-launch-history.private.json'],ledgerPin)&&g.schemaVersion===2&&g.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&
        g.pending===null&&g.uncertain===false&&closed(b,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(b).every(x=>integer(x))&&
        b.reservedActions-b.sentActions===2&&same(g.history,b)&&same(v.actions.actualGuard,b)&&
        same(v.actions.currentRunContext,{overallAttempts:b.overallAttempts+1,uiAttempts:b.uiAttempts+1})&&
        integer(v.actions.absoluteMaxReservedActions,b.reservedActions+1)&&v.actions.absoluteMaxReservedActions<=b.reservedActions+8);
    const hash=(/** @type {any} */ v,/** @type {string} */ h)=>typeof v==='string'&&/^[a-f0-9]{64}$/i.test(v)&&v.toLowerCase()===h;
    need(p.scenarioId===basename(s)&&p.paths?.scenario===s&&p.source?.qualifiedHead===v.sourceHead&&p.ownership?.driver===v.driver&&p.ownership.onlyGameInputDriver===true&&
        same(p.campaignControl,{schemaVersion:4,profile:'DISPOSABLE_SEED_CREATION'})&&same(l.campaignControl,p.campaignControl)&&
        p.sessionWatchdogMs===1800000&&l.sessionWatchdogMs===1800000&&l.ownedGameHardTimeoutMs===1800000&&p.bounds?.ownedGameHardTimeoutMs===1800000&&
        p.bounds.maxScenarioActionsIncludingActivations===8&&p.bounds.maxProxyInvocations===3&&p.bounds.proxyTimeoutMs===180000&&
        same(p.directHostConsent,{schemaVersion:1,mode:'finite-explicit-consent',maxCallsPerBatch:32})&&
        l.schemaVersion===1&&l.scenarioId===p.scenarioId&&l.issuer==='/root'&&l.driver===v.driver&&l.authorized===true&&
        l.phase==='CAMPAIGN_BINDING_ONLY'&&l.maxActions===8&&l.maxProxyInvocations===3&&l.campaignLoadSaveAllowed===false&&
        p.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&l.absoluteMaxReservedActions===p.absoluteMaxReservedActions&&
        hash(l.planSha256,ref(resolve(s,'plan.json')).sha256)&&hash(c.rootLeaseSha256,ref(resolve(s,'native-lane-lease.private.json')).sha256)&&
        g.controllerSha256===ref(resolve(s,'controller-start.private.json')).sha256&&g.rootRecordKey===root.key);
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
        c.sessionWatchdogMs===1800000&&c.hardTimeoutMs===1800000&&Array.isArray(c.supervisorJobPids)&&c.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1))&&
        new Set(c.supervisorJobPids).size===c.supervisorJobPids.length&&c.supervisorJobPids.includes(c.controllerPid)&&
        clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,clock.callerAdmissionDeadlineMs,Date.parse(c.conservativeOuterDeadlineUtc),Date.parse(c.controllerDeadlineUtc)));
    const cp=ref(resolve(s,'controller-start.private.json'));
    need(closed(pre,['schemaVersion','kind','plan','lease','controller','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions'])&&
        pre.schemaVersion===1&&pre.kind==='FINITE_CAMPAIGN_LAUNCH_PREDECESSOR'&&same(pre.plan,ref(resolve(s,'plan.json')))&&same(pre.lease,ref(resolve(s,'native-lane-lease.private.json')))&&
        same(pre.controller,cp)&&same(pre.rootCallRecord,root)&&same(pre.historyBaseline,b)&&pre.maxReservedActions===p.absoluteMaxReservedActions&&
        clock.producerRecordKey==='controller-start:'+cp.sha256&&same(pre.producerWitness,{producerRecordKey:clock.producerRecordKey,controllerPid:c.controllerPid,
            controllerCreationTime:c.controllerCreationTime,controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,conservativeOuterDeadlineUtc:c.conservativeOuterDeadlineUtc,
            controllerDeadlineUtc:c.controllerDeadlineUtc,effectiveDeadlineMs:clock.effectiveDeadlineMs,sessionWatchdogMs:1800000}));
    const support=a['frozen-support-pins.private.json'];
    need(closed(support,['pins'])&&Array.isArray(support.pins)&&support.pins.length>0&&support.pins.length<=1024&&same(p.source.supportPins,ref(resolve(s,'frozen-support-pins.private.json')))&&
        l.supportPinsPath===resolve(s,'frozen-support-pins.private.json')&&hash(l.supportPinsSha256,ref(l.supportPinsPath).sha256)&&
        reader.bytes(resolve(s,'source-pins.private.json')).bytes.equals(reader.bytes(l.supportPinsPath).bytes));
    const seen=new Set();for(const pin of support.pins){need(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&!seen.has(pin.path));seen.add(pin.path);reader.pinned(pin,pin.path,128*1024*1024);}
    const executable=(/** @type {string} */ name)=>{const found=support.pins.filter((/** @type {any} */ row)=>basename(row.path).toLowerCase()===name);need(found.length===1);return found[0].path;};
    const node=executable('node.exe'),powershell=executable('pwsh.exe');need(resolve(root.value.args.shell)===powershell);
    need(same(p.seedCreation,l.seedCreation)&&closed(p.seedCreation,['mode','preference'])&&p.seedCreation.mode==='CREATE_ONE_PRIVATE_DISPOSABLE_SEED'&&
        same(p.seedCreation.preference,ref(resolve(s,'seed-creation-preference.private.json')))&&same(a['seed-creation-preference.private.json'],earlyWindowPreference)&&
        same(p.historyAdmission,l.historyAdmission)&&same(p.historyAdmission,ref(resolve(s,'root-history-baseline.admitted.private.json'))));
    const grant=a['root-history-baseline.admitted.private.json'];
    need(closed(grant,['schemaVersion','kind','issuer','driver','historyBaseline','maxReservedActions','previous'])&&grant.schemaVersion===2&&
        grant.kind==='ROOT_ADMITTED_SEED_CREATION_HISTORY'&&grant.issuer==='/root'&&grant.driver===v.driver&&same(grant.historyBaseline,b)&&
        grant.maxReservedActions===p.absoluteMaxReservedActions&&closed(grant.previous,['result','ledger']));
    for(const pin of Object.values(/** @type {Record<string,any>} */(grant.previous)))reader.pinned(pin,pin.path);
    const fixture=a['protected-fixture-receipt.private.json'],binding=a['fixture-binding.private.json'];
    need(same(p.fixtureBinding,ref(resolve(s,'fixture-binding.private.json')))&&binding.schemaVersion===3&&binding.fixtureProfile==='VANILLA_UNCOMPRESSED_GENERATION'&&
        binding.fixtureRoot===s&&fixture.phase==='FROZEN_PREPARATION_ONLY'&&fixture.originalFilesVerified===182&&fixture.originalSaveFiles===178&&
        fixture.backupVerified===true&&fixture.originalWrites===false&&fixture.nativeLaunched===false&&fixture.gameLaunched===false&&fixture.privateAutosaveEnabled===false&&
        fixture.privateSaveRootEmpty===true&&same(fixture.originalAutosave,{enabledByDefault:true,maximumSlots:3,featureEnabled:true})&&
        hash(l.fixtureReceiptSha256,ref(resolve(s,'protected-fixture-receipt.private.json')).sha256)&&hash(c.fixtureReceiptSha256,ref(resolve(s,'protected-fixture-receipt.private.json')).sha256));
    const names=['launch-owned-caller.private.ps1','record-root-call-and-stage-B1.private.ps1','launch-and-stage.orchestration.proposed.private.js',
        'construct-actual-phase-inputs.private.mjs','driver-timing-capture.unissued.private.json','seed-save-closeout.template.private.mjs',
        'root-freeze-and-preflight.seed.proposed.private.ps1','frame-menu-phase.failfast.proposed.private.ps1','read-client-geometry.owned.private.mjs'];
    need(closed(p.preclockHelpers,names));for(const name of names)reader.pinned(p.preclockHelpers[name],resolve(s,name));
    const d=a['root-postlaunch-mechanical-delegation.private.json'];
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
    const source=a['B1-authorization.private.json'],claim=a['guard-B1-consumed.private.json'];
    need(closed(source,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256'])&&source.schemaVersion===6&&source.scenario===s&&source.batch==='B1');
    for(const [pathKey,hashKey,name] of [['planPath','planSha256','B1-plan.private.json'],['leasePath','leaseSha256','B1-lease.private.json'],['ownedReceiptPath','ownedReceiptSha256','driver-B1-owned.private.json']])
        reader.pinned({path:source[pathKey],sha256:source[hashKey]},resolve(s,name));
    const phasePlan=json(source.planPath),phaseLease=json(source.leasePath),initial=json(source.ownedReceiptPath),bindingValue=claim.binding,campaign=bindingValue?.campaign;
    need(closed(claim,['sourcePath','sourceSha256','binding'])&&claim.sourcePath===ref(resolve(s,'B1-authorization.private.json')).path&&claim.sourceSha256===ref(claim.sourcePath).sha256&&
        closed(bindingValue,['schemaVersion','batch','runID','scenario','workspaceRoot','planSha256','ownedPids','campaign','sourcePath','sourceSha256','controllerSha256','originalPlanSha256'])&&
        bindingValue.schemaVersion===6&&bindingValue.batch==='B1'&&bindingValue.scenario===s&&bindingValue.workspaceRoot===v.sourceWorkspace&&typeof bindingValue.runID==='string'&&/^[A-Za-z0-9_-]+$/.test(bindingValue.runID)&&
        bindingValue.sourcePath===claim.sourcePath&&bindingValue.sourceSha256===claim.sourceSha256&&bindingValue.planSha256===source.planSha256&&
        bindingValue.controllerSha256===cp.sha256&&bindingValue.originalPlanSha256===ref(resolve(s,'plan.json')).sha256&&
        closed(campaign,['flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor'])&&campaign.flow==='DISPOSABLE_SEED_CREATION'&&integer(campaign.windowId,1)&&
        same(campaign.launchPredecessor,ref(resolve(s,'campaign-postlaunch.private.json')))&&['rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions'].every(key=>same(campaign[key],pre[key]))&&
        same(phasePlan,{...p,campaign,batches:p.batches.filter((/** @type {any} */ row)=>row.id==='B1')})&&same(phaseLease,{...l,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:source.planSha256})&&
        same(g.provider,{batch:'B1',runID:bindingValue.runID,sourceSha256:claim.sourceSha256})&&g.b1ClaimSha256===ref(resolve(s,'guard-B1-consumed.private.json')).sha256);
    return{v,a,r,p,l,c,root,g,pre,clock,node,powershell,reader,support,initial,binding:bindingValue,output:resolve(v.sourceWorkspace,'.tmp/starsector-proxy',bindingValue.runID)};
}

/** Validate the original full supervisor against the original five-field return projection. @param {any} ctx */
export function assertEarlyWindowCaller(ctx){
    const {a,c,p,root,powershell,reader,support,v}=ctx,caller=a['launch-caller-terminal.actual.private.json'],launch=a['launch-supervisor.private.json'];
    need(closed(caller,['schemaVersion','kind','session','launchRaw','launchPolls','actual','terminalReaped'])&&caller.schemaVersion===1&&caller.kind==='ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL'&&
        integer(caller.session,1)&&caller.session===root.value.raw.session_id&&same(caller.launchRaw,root.value.raw)&&caller.terminalReaped===true&&Array.isArray(caller.launchPolls)&&caller.launchPolls.length>=1&&caller.launchPolls.length<=256);
    const chain=[caller.launchRaw,...caller.launchPolls];let bytes=0;
    for(const [i,row] of chain.entries()){
        const last=i===chain.length-1;need(closed(row,last?['chunk_id','wall_time_seconds','exit_code','original_token_count','output']:['chunk_id','wall_time_seconds','session_id','original_token_count','output'])&&
            typeof row.chunk_id==='string'&&row.chunk_id.length>0&&row.chunk_id.length<=128&&Number.isFinite(row.wall_time_seconds)&&row.wall_time_seconds>=0&&integer(row.original_token_count)&&
            typeof row.output==='string'&&Buffer.byteLength(row.output)<=65536&&(last?(row.exit_code===1&&same(row,caller.actual)):row.session_id===caller.session));
        bytes+=Buffer.byteLength(row.output);
    }
    need(bytes<=131072);const returned=zeroProviderObject(chain.map(x=>x.output).join(''),need);
    need(closed(launch,['schemaVersion','command','ok','result'])&&launch.schemaVersion===1&&launch.command==='verify'&&launch.ok===true&&
        closed(launch.result,['blocked','graphFingerprint','scope','selection','steps','verdict'])&&launch.result.verdict==='FAIL'&&same(launch.result.blocked,[])&&
        typeof launch.result.graphFingerprint==='string'&&/^[a-f0-9]{64}$/.test(launch.result.graphFingerprint)&&launch.result.scope==='module'&&
        same(launch.result.selection,{changedPaths:[],modules:['node:.']})&&Array.isArray(launch.result.steps)&&launch.result.steps.length===1);
    const step=launch.result.steps[0];need(closed(step,['args','command','cwd','module','processOwnership','status','stderr','stdout','timedOut'])&&step.command===powershell&&step.cwd==='.'&&step.module==='node:.'&&
        step.status===1&&step.timedOut===false&&step.processOwnership==='windows-job-object'&&step.stdout===''&&typeof step.stderr==='string'&&step.stderr.includes('Driver requested owned cleanup; native success not established')&&
        same(step.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',v.scenario,
            '-BindingPath',p.fixtureBinding.path,'-BindingSha256',p.fixtureBinding.sha256,'-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','1800000'])&&
        support.pins.some((/** @type {any} */ pin)=>pin.path===step.args[3])&&closed(returned,['verdict','blocked','steps'])&&returned.verdict==='FAIL'&&same(returned.blocked,[])&&
        same(returned.steps,{status:step.status,timedOut:step.timedOut,processOwnership:step.processOwnership,stdout:step.stdout,stderr:step.stderr}));
    return caller;
}
