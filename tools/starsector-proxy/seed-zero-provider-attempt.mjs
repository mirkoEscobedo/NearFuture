// @ts-check
// A failed unclaimed B1 is historical custody only, never successful seed provenance.
import {resolve,dirname,basename,isAbsolute} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {file,closed,requireThat} from './campaign-proof.mjs';
import {assertZeroProviderTerminal,zeroProviderObject} from './seed-zero-provider-terminal.mjs';

export const zeroProviderFailedSeedAttemptKind='ROOT_CLOSED_ZERO_PROVIDER_CONTROLLER_CLOCK_FAILED_SEED_ATTEMPT_HISTORY';
const names={plan:'plan.json',lease:'native-lane-lease.private.json',controller:'controller-start.private.json',owned:'owned-processes-current.private.json',
 rootCall:'root-call.actual.private.json',caller:'launch-caller-terminal.actual.private.json',launch:'launch-supervisor.private.json',
 protection:'after-smoke-protection.private.json',pidAbsence:'poststop-pid-absence.actual.private.json',preference:'seed-creation-preference.private.json',
 supportPins:'frozen-support-pins.private.json',source:'B1-authorization.private.json',phasePlan:'B1-plan.private.json',phaseLease:'B1-lease.private.json',
 initialOwned:'driver-B1-owned.private.json',predecessor:'campaign-postlaunch.private.json',frozen:'root-final-prelaunch-actual.private.json',
 freezeSupervisor:'root-freeze-supervisor.actual.private.json',combined:'combined-launch-stage.actual.private.json',ledger:'guard-launch-history.private.json',preclockIndex:'seed-graph-preclock-status.proposed.private.json'};
/** @param {any} v @param {any} ledger @param {any} ledgerPin @param {(p:any,l:any,s:string)=>any} preference */
export function readZeroProviderFailedSeedAttempt(v,ledger,ledgerPin,preference){
 const need=(/** @type {any} */ ok)=>requireThat(ok,'closed zero-provider controller-clock failed seed custody required');
 const normal=(/** @type {any} */ p)=>typeof p==='string'&&isAbsolute(p)&&resolve(p)===p;
 need(closed(v,['schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence'])&&
  v.schemaVersion===1&&v.kind===zeroProviderFailedSeedAttemptKind&&v.issuer==='/root'&&v.driver==='/root/recovery_game_driver'&&
  v.allOwnedHandlesTerminalReaped===true&&normal(v.scenario)&&normal(v.sourceWorkspace)&&/^[a-f0-9]{40}$/.test(v.sourceHead)&&
  dirname(v.scenario)===resolve(v.sourceWorkspace,'.tmp/starsector-proxy/scenarios')&&
  closed(v.actions,['actualGuard','currentRunContext','absoluteMaxReservedActions'])&&closed(v.evidence,Object.keys(names)));
 /** @param {any} pin @param {string} expected */
 function pinned(pin,expected){need(closed(pin,['path','sha256'])&&pin.path===expected&&/^[a-f0-9]{64}$/.test(pin.sha256));
  const found=file(expected);need(found.sha256===pin.sha256);return zeroProviderObject(new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(found.bytes),need);}
 const e=v.evidence,s=v.scenario;
 /** @type {any} */
 const a={};
 for(const [k,name] of Object.entries(names))a[k]=pinned(e[k],resolve(s,name));
 const {plan:p,lease:l,controller:c,owned,rootCall,source,phasePlan:bp,phaseLease:bl,initialOwned:initial,predecessor:pre,protection:protect,pidAbsence:abs}=a;
 for(const [object,fields] of [[c,['at','controllerCreationTime','conservativeOuterDeadlineUtc','controllerDeadlineUtc']],
  [owned,['at','controllerCreationTime','controllerKernelCreationFileTime']],[initial,['at']],[protect,['at','phase']],[abs,['atUtc']]])
  for(const field of /** @type {string[]} */(fields))need(typeof object[field]==='string');
 const g=a.ledger,guard=g.history,ctx=v.actions.currentRunContext;
 need(closed(g,['schemaVersion','kind','rootRecordKey','controllerSha256','history','pending','uncertain'])&&g.schemaVersion===2&&
  g.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&g.pending===null&&g.uncertain===false&&same(g,ledger)&&same(e.ledger,ledgerPin)&&
  closed(guard,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(guard).every(n=>Number.isSafeInteger(n)&&n>=0)&&
  guard.sentActions<=guard.reservedActions&&same(guard,p.historyBaseline)&&same(guard,v.actions.actualGuard)&&
  closed(ctx,['overallAttempts','uiAttempts'])&&ctx.overallAttempts===guard.overallAttempts+1&&ctx.uiAttempts===guard.uiAttempts+1&&
  Number.isSafeInteger(v.actions.absoluteMaxReservedActions)&&v.actions.absoluteMaxReservedActions>guard.reservedActions&&
  v.actions.absoluteMaxReservedActions<=guard.reservedActions+8&&p.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&l.absoluteMaxReservedActions===p.absoluteMaxReservedActions);
 need(p.scenarioId===basename(s)&&p.paths?.scenario===s&&p.source?.qualifiedHead===v.sourceHead&&p.ownership?.driver===v.driver&&p.ownership.onlyGameInputDriver===true&&
  same(p.campaignControl,{schemaVersion:4,profile:'DISPOSABLE_SEED_CREATION'})&&same(l.campaignControl,p.campaignControl)&&l.schemaVersion===1&&
  l.scenarioId===p.scenarioId&&l.authorized===true&&l.issuer==='/root'&&l.driver===v.driver&&l.phase==='CAMPAIGN_BINDING_ONLY'&&
  l.planSha256.toLowerCase()===e.plan.sha256&&c.rootLeaseSha256.toLowerCase()===e.lease.sha256&&g.controllerSha256===e.controller.sha256&&
  rootCall.key===g.rootRecordKey&&rootCall.value.args.workdir===v.sourceWorkspace&&c.installation===resolve(s,'installation'));
 need(same(p.source.supportPins,e.supportPins)&&l.supportPinsPath===e.supportPins.path&&l.supportPinsSha256.toLowerCase()===e.supportPins.sha256&&
  closed(a.supportPins,['pins'])&&Array.isArray(a.supportPins.pins)&&a.supportPins.pins.length<=1024);
 for(const [name,pin] of Object.entries(/** @type {Record<string,any>} */(p.preclockHelpers))){need(pin.path===resolve(s,name)&&file(pin.path).sha256===pin.sha256);}
 need(same(p.historyAdmission,l.historyAdmission));pinned(p.historyAdmission,resolve(s,'root-history-baseline.admitted.private.json'));
 pinned({path:resolve(s,'root-postlaunch-mechanical-delegation.private.json'),sha256:a.frozen.delegationSha256},resolve(s,'root-postlaunch-mechanical-delegation.private.json'));
 need(normal(p.source.loadedFrontendWitness?.path));pinned(p.source.loadedFrontendWitness,p.source.loadedFrontendWitness.path);
 const shells=a.supportPins.pins.filter((/** @type {any} */ x)=>typeof x.path==='string'&&basename(x.path).toLowerCase()==='pwsh.exe');
 need(shells.length===1);const shell=shells[0];need(normal(shell.path)&&/^[a-fA-F0-9]{64}$/.test(shell.sha256)&&file(shell.path,128*1024*1024).sha256===shell.sha256.toLowerCase());
 const birth=Date.parse(c.controllerCreationTime),published=Date.parse(c.at),outer=Date.parse(c.conservativeOuterDeadlineUtc),inner=Date.parse(c.controllerDeadlineUtc),clock=pre.rootClock;
 need([birth,published,outer,inner,rootCall.value.startMs,p.callerAdmissionDeadlineMs].every(Number.isSafeInteger)&&
  rootCall.value.startMs>0&&p.callerAdmissionDeadlineMs>published&&rootCall.value.startMs<=birth&&birth<=published&&published<outer&&outer<inner&&outer<=birth+480000&&
  c.sessionWatchdogMs===480000&&c.hardTimeoutMs===480000&&Number.isSafeInteger(c.controllerPid)&&c.controllerPid>0&&
  Number.isSafeInteger(c.sessionId)&&c.sessionId>=0&&typeof c.controllerKernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(c.controllerKernelCreationFileTime)&&
  l.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&p.callerAdmissionDeadlineMs<=rootCall.value.startMs+480000&&
  pre.schemaVersion===1&&pre.kind==='FINITE_CAMPAIGN_LAUNCH_PREDECESSOR'&&same(pre.plan,e.plan)&&same(pre.lease,e.lease)&&same(pre.controller,e.controller)&&
  same(pre.rootCallRecord,rootCall)&&same(pre.historyBaseline,guard)&&pre.maxReservedActions===p.absoluteMaxReservedActions&&
  clock.recordKey===rootCall.key&&clock.startMs===rootCall.value.startMs&&clock.deadlineMs===clock.startMs+480000&&clock.sessionWatchdogMs===480000&&
  clock.cleanupReserveMs===30000&&clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.originalCallerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&
  clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,p.callerAdmissionDeadlineMs,outer,inner)&&clock.producerRecordKey==='controller-start:'+e.controller.sha256&&
  same(pre.producerWitness,{producerRecordKey:clock.producerRecordKey,controllerPid:c.controllerPid,controllerCreationTime:c.controllerCreationTime,
   controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,conservativeOuterDeadlineUtc:c.conservativeOuterDeadlineUtc,
   controllerDeadlineUtc:c.controllerDeadlineUtc,effectiveDeadlineMs:clock.effectiveDeadlineMs,sessionWatchdogMs:480000}));
 need(closed(source,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256'])&&
  source.schemaVersion===6&&source.scenario===s&&source.batch==='B1'&&source.planPath===e.phasePlan.path&&source.planSha256===e.phasePlan.sha256&&
  source.leasePath===e.phaseLease.path&&source.leaseSha256===e.phaseLease.sha256&&source.ownedReceiptPath===e.initialOwned.path&&source.ownedReceiptSha256===e.initialOwned.sha256);
 const campaign=bp.campaign;
 need(closed(campaign,['flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor'])&&
  campaign.flow==='DISPOSABLE_SEED_CREATION'&&same(campaign.launchPredecessor,e.predecessor)&&Number.isSafeInteger(campaign.windowId)&&campaign.windowId>0);
 for(const k of ['rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions'])need(same(campaign[k],pre[k]));
 need(same(bp,{...p,campaign,batches:p.batches.filter((/** @type {any} */ b)=>b.id==='B1')})&&same(bl,{...l,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:e.phasePlan.sha256}));
 need(bp.batches.length===1&&typeof bp.batches[0].runID==='string'&&/^[A-Za-z0-9_-]+$/.test(bp.batches[0].runID));
 need(!readdirSync(resolve(v.sourceWorkspace,'.tmp/starsector-proxy')).some(n=>n.toLowerCase()===bp.batches[0].runID.toLowerCase()));
 need(owned.controllerPid===c.controllerPid&&owned.controllerCreationTime===c.controllerCreationTime&&owned.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&
  Array.isArray(owned.processes)&&Array.isArray(initial.processes)&&initial.processes.length===1&&initial.sourceSnapshot?.path===e.owned.path&&
  typeof initial.sourceSnapshot.sha256==='string'&&/^[a-f0-9]{64}$/.test(initial.sourceSnapshot.sha256));
 const java=initial.processes[0],matches=owned.processes.filter((/** @type {any} */ r)=>r.pid===java.pid);
 for(const row of [java,...owned.processes])need(['creationTime','kernelCreationFileTime','executable'].every(k=>typeof row[k]==='string')&&
  ['pid','parentPid'].every(k=>Number.isSafeInteger(row[k])&&row[k]>0)&&Number.isSafeInteger(row.sessionId)&&row.sessionId>=0&&
  Array.isArray(row.supervisorJobPids)&&row.supervisorJobPids.every((/** @type {any} */ id)=>Number.isSafeInteger(id)&&id>0));
 need(matches.length===1);for(const k of ['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId','inSupervisorJob','supervisorJobPids','mainWindowHandle'])need(same(java[k],matches[0][k]));
 need(java.inSupervisorJob===true&&java.sessionId===c.sessionId&&java.executable===resolve(c.installation,'jre/bin/java.exe')&&
  String(java.mainWindowHandle)===String(campaign.windowId)&&Date.parse(java.creationTime)>=published&&Array.isArray(c.supervisorJobPids)&&
  c.supervisorJobPids.includes(c.controllerPid)&&c.supervisorJobPids.every((/** @type {any} */ id)=>java.supervisorJobPids.includes(id))&&java.supervisorJobPids.includes(java.pid));
 need(Number.isSafeInteger(java.pid)&&java.pid>0&&Number.isSafeInteger(java.parentPid)&&java.parentPid>0&&java.parentPid!==java.pid&&
  java.supervisorJobPids.includes(java.parentPid)&&typeof java.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(java.kernelCreationFileTime)&&
  typeof java.creationTime==='string'&&(typeof java.mainWindowHandle==='string'||Number.isSafeInteger(java.mainWindowHandle)));
 const chronology=[Date.parse(java.creationTime),Date.parse(initial.at),Date.parse(owned.at),Date.parse(protect.at),Date.parse(abs.atUtc)];
 need(chronology.every(Number.isSafeInteger)&&chronology[0]<=chronology[1]&&chronology[0]<=chronology[2]&&
  [chronology[1],chronology[2]].every(t=>t>=published&&t<=chronology[3]&&t<=chronology[4]));
 need(same(p.seedCreation,l.seedCreation)&&same(p.seedCreation.preference,e.preference)&&same(a.preference,preference(p,l,s))&&
  protect.phase==='after-smoke'&&protect.protectedOriginalFiles===182&&protect.protectedSaveFiles===178&&protect.allOriginalAndBackupHashesMatch===true&&
  protect.saveCensusUnchanged===true&&protect.liveGameProcesses===0&&protect.originalWrites===false&&protect.privateAutosaveEnabled===false&&protect.privateSaveEntries===0&&
  same(protect.originalAutosave,{enabled:true,maximumSlots:3})&&same(protect.sharedPreference,a.preference));
 const ids=[...new Set([c.controllerPid,...c.supervisorJobPids,...owned.processes.flatMap((/** @type {any} */ r)=>[r.pid,...r.supervisorJobPids])])].sort((a,b)=>a-b);
 need(closed(abs,['atUtc','expectedPids','absentPids','noLiveOwnedPids','providerStarted','nativeInputs','forcedOwnedCleanup','ordinaryExitAccepted'])&&
  Array.isArray(abs.expectedPids)&&abs.expectedPids.every((/** @type {any} */ id)=>Number.isSafeInteger(id)&&id>0)&&
  same([...abs.expectedPids].sort((a,b)=>a-b),ids)&&same(abs.absentPids,abs.expectedPids)&&abs.noLiveOwnedPids===true&&abs.providerStarted===false&&
  abs.nativeInputs===0&&abs.forcedOwnedCleanup===true&&abs.ordinaryExitAccepted===false&&Date.parse(abs.atUtc)>=published&&Date.parse(protect.at)>=published);
 need(!readdirSync(s).some(n=>/^(guard-(B1|E1|E2)-consumed|capture-|call-|geometry-|owned-provider|guarded-http|supervision-(B1|E1|E2))/.test(n)));
 assertZeroProviderTerminal({v,a,e,need,powershell:shell.path});return v.actions;
}
