// @ts-check
// Historical terminal custody only. This reader grants no input or seed provenance.
import {resolve,dirname,basename,isAbsolute} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {file,read,closed,requireThat} from './campaign-proof.mjs';
import {assertKnownPlayFailure} from './seed-known-play-attempt.mjs';
import {assertWrongSceneFailure} from './seed-wrong-scene-attempt.mjs';
import {assertCaptureLinkageFailure} from './seed-capture-linkage-attempt.mjs';
import {assertCallerDateFailure,assertCallerDateIdentityShapes,callerDateNames,callerDateProviderNames} from './seed-caller-date-attempt.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';

export const failedSeedAttemptKind='ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY';
export const knownPlayFailedSeedAttemptKind='ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY';
export const captureStopFailedSeedAttemptKind='ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY';
export const wrongSceneFailedSeedAttemptKind='ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY';
export const captureLinkageFailedSeedAttemptKind='ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY';
export const callerDateFailedSeedAttemptKind='ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY';
const driver='/root/recovery_game_driver';
const names={result:'driver-flow-result.private.json',ledger:'guard-launch-history.private.json',plan:'plan.json',
 lease:'native-lane-lease.private.json',controller:'controller-start.private.json',owned:'owned-processes-current.private.json',
 rootCall:'root-call.actual.private.json',caller:'launch-caller-terminal.actual.private.json',launch:'launch-supervisor.private.json',
 protection:'after-smoke-protection.private.json',pidAbsence:'owned-terminal-pid-absence.actual.private.json',
 preference:'seed-creation-preference.private.json',supportPins:'frozen-support-pins.private.json',source:'B1-authorization.private.json',claim:'guard-B1-consumed.private.json',
 observation:'driver-observation-stop.private.json'};
const providerNames={providerReady:'guarded-http-ready.private.json',providerTerminal:'owned-provider-terminal.private.json',
 providerSpool:'owned-provider.stdout.private.txt',providerStderr:'owned-provider.stderr.private.txt'};
const fail=(/** @type {any} */ ok)=>requireThat(ok,'closed no-input failed seed attempt custody required');
/** @param {any} pin @param {string} expected @param {number} [limit] */
function pinned(pin,expected,limit=262144){
 fail(closed(pin,['path','sha256'])&&pin.path===expected&&/^[a-f0-9]{64}$/.test(pin.sha256));
 const found=read(expected,limit);fail(found.sha256===pin.sha256);return found.value;
}
/** @param {any} step */
const critical=step=>({command:step?.command,args:step?.args,cwd:step?.cwd,processOwnership:step?.processOwnership,status:step?.status,timedOut:step?.timedOut});
/** The maintained runner returns these five fields, not its full verifier step. @param {any} step */
const returnedLaunchStep=step=>({status:step?.status,timedOut:step?.timedOut,processOwnership:step?.processOwnership,stdout:step?.stdout,stderr:step?.stderr});

/** Validate the distinct root failure receipt. Never accept it as a completed seed.
 * @param {any} receipt @param {any} previousLedger @param {any} ledgerPin
 * @param {(plan:any,lease:any,scenario:string)=>any} readPreference */
export function readFailedSeedAttempt(receipt,previousLedger,ledgerPin,readPreference){
 return readTerminalFailure(receipt,previousLedger,ledgerPin,readPreference,failedSeedAttemptKind);
}
/** @param {any} receipt @param {any} previousLedger @param {any} ledgerPin
 * @param {(plan:any,lease:any,scenario:string)=>any} readPreference */
export function readKnownPlayFailedSeedAttempt(receipt,previousLedger,ledgerPin,readPreference){
 return readTerminalFailure(receipt,previousLedger,ledgerPin,readPreference,knownPlayFailedSeedAttemptKind);
}
/** @param {any} receipt @param {any} previousLedger @param {any} ledgerPin
 * @param {(plan:any,lease:any,scenario:string)=>any} readPreference */
export function readCaptureStopFailedSeedAttempt(receipt,previousLedger,ledgerPin,readPreference){
 return readTerminalFailure(receipt,previousLedger,ledgerPin,readPreference,captureStopFailedSeedAttemptKind);
}
/** @param {any} receipt @param {any} ledger @param {any} pin @param {(p:any,l:any,s:string)=>any} preference */
export function readWrongSceneFailedSeedAttempt(receipt,ledger,pin,preference){
 return readTerminalFailure(receipt,ledger,pin,preference,wrongSceneFailedSeedAttemptKind);
}
/** @param {any} receipt @param {any} ledger @param {any} pin @param {(p:any,l:any,s:string)=>any} preference */
export function readCaptureLinkageFailedSeedAttempt(receipt,ledger,pin,preference){
 return readTerminalFailure(receipt,ledger,pin,preference,captureLinkageFailedSeedAttemptKind);
}
/** @param {any} receipt @param {any} ledger @param {any} pin @param {(p:any,l:any,s:string)=>any} preference */
export function readCallerDateFailedSeedAttempt(receipt,ledger,pin,preference){
 return readTerminalFailure(receipt,ledger,pin,preference,callerDateFailedSeedAttemptKind);
}
/** @param {any} receipt @param {any} previousLedger @param {any} ledgerPin
 * @param {(plan:any,lease:any,scenario:string)=>any} readPreference @param {string} kind */
function readTerminalFailure(receipt,previousLedger,ledgerPin,readPreference,kind){
 const v=receipt;
 const callerDate=kind===callerDateFailedSeedAttemptKind;
 /** New-case byte reader preserves scalar/array distinctions and rejects duplicate keys. */
 const pinRead=(/** @type {any} */ pin,/** @type {string} */ path,limit=262144)=>{
  if(!callerDate)return pinned(pin,path,limit);
  fail(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&pin.path===path&&typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256));
  const bytes=file(path,limit);fail(bytes.sha256===pin.sha256);
  return zeroProviderObject(new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes.bytes),fail);
 };
 const linkage=kind===captureLinkageFailedSeedAttemptKind,wrongScene=kind===wrongSceneFailedSeedAttemptKind,captureStop=kind===captureStopFailedSeedAttemptKind,
  known=kind===knownPlayFailedSeedAttemptKind||captureStop||wrongScene;
 const extra=callerDate?callerDateNames:wrongScene?{e1Source:'E1-authorization.private.json',e1Claim:'guard-E1-consumed.private.json',
  e1Plan:'E1-plan.private.json',e1Lease:'E1-lease.private.json',e1Owned:'driver-E1-owned.private.json',b1Menu:'B1-menu-inspected.private.json'}:{};
 const secondProvider=wrongScene?['e1ProviderReady','e1ProviderTerminal','e1ProviderSpool','e1ProviderStderr']:linkage?['helper','metadataCaller']:[];
 fail(closed(v,['schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence'])&&
  v.schemaVersion===1&&v.kind===kind&&v.issuer==='/root'&&v.driver===driver&&v.allOwnedHandlesTerminalReaped===true&&
  typeof v.scenario==='string'&&isAbsolute(v.scenario)&&resolve(v.scenario)===v.scenario&&typeof v.sourceWorkspace==='string'&&
  isAbsolute(v.sourceWorkspace)&&resolve(v.sourceWorkspace)===v.sourceWorkspace&&dirname(v.scenario)===resolve(v.sourceWorkspace,'.tmp/starsector-proxy/scenarios')&&
  /^[a-f0-9]{40}$/.test(v.sourceHead)&&closed(v.actions,['actualGuard','currentRunContext','absoluteMaxReservedActions'])&&
  closed(v.actions.currentRunContext,['overallAttempts','uiAttempts'])&&closed(v.evidence,[...Object.keys(names),...Object.keys(providerNames),...Object.keys(extra).filter(k=>!Object.hasOwn(names,k)),...secondProvider,...(callerDate?['helper','geometryRunner',...Object.keys(callerDateProviderNames)]:[])]));
 const e=v.evidence,scenario=v.scenario;
 /** @type {any} */const a={};
 for(const [key,name] of Object.entries({...names,...extra}))a[key]=pinRead(e[key],resolve(scenario,name));
 if(linkage)a.metadataCaller=pinned(e.metadataCaller,resolve(scenario,'driver-metadata-calls.actual.private.json'));
 const {result:r,ledger:l,plan:p,lease,controller:c,owned,rootCall,caller,launch,protection,preference,source,claim,observation:o}=a;
 fail(same(e.ledger,ledgerPin)&&same(l,previousLedger)&&l.schemaVersion===2&&l.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&
  (known||l.pending===null)&&l.uncertain===false&&same(l.history,v.actions.actualGuard)&&(known||same(l.history,p.historyBaseline))&&
  p.scenarioId===basename(scenario)&&p.paths?.scenario===scenario&&p.source?.qualifiedHead===v.sourceHead&&
  p.campaignControl?.profile==='DISPOSABLE_SEED_CREATION'&&p.campaignControl.schemaVersion===4&&
  p.ownership?.driver===driver&&p.ownership.onlyGameInputDriver===true&&lease.planSha256?.toLowerCase()===e.plan.sha256&&
  lease.scenarioId===p.scenarioId&&lease.driver===driver&&lease.authorized===true&&lease.issuer==='/root'&&
  lease.campaignControl?.profile===p.campaignControl.profile&&lease.campaignControl.schemaVersion===4&&
  c.rootLeaseSha256?.toLowerCase()===e.lease.sha256&&l.controllerSha256===e.controller.sha256&&
  rootCall.key===l.rootRecordKey&&rootCall.value?.args?.workdir===v.sourceWorkspace&&
  Number.isSafeInteger(rootCall.value.raw?.session_id)&&rootCall.value.raw.session_id>0&&
  (callerDate||(same(r.actualGuardHistory,l.history)&&same(r.inheritedGuardBaseline,known?p.historyBaseline:l.history)&&same(r.actualRunContext,v.actions.currentRunContext)))&&
  r.schemaVersion===1&&r.kind==='ACTUAL_FINITE_DRIVER_FLOW_RESULT'&&r.status==='CLOSED_FAIL'&&r.verdict==='FAIL'&&
  r.failureCategory===(callerDate?'CALLER_METADATA_SERIALIZATION_FAILED':linkage?'PRIVATE_CAPTURE_LINKAGE_REFUSAL_BEFORE_INPUT':wrongScene?'POSTCONDITION_FAILED':captureStop?'CAPTURE_UNCERTAIN':known?'PRIVATE_FRAME_CUSTODY_REFUSAL_AFTER_KNOWN_PLAY':'FOCUS_BLOCKED')&&r.scenarioId===p.scenarioId&&r.driver===driver&&r.sourceHead===v.sourceHead&&
  (callerDate||known||linkage||(same(r.actions,{reservedThisRun:0,sentThisRun:0,clicks:0,activations:0})&&
  same(r.providers,{started:1,B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED',calls:5})))&&
  (callerDate||(
  r.absoluteMaxReservedActions===p.absoluteMaxReservedActions&&r.absoluteMaxReservedActions===lease.absoluteMaxReservedActions&&
  r.absoluteMaxReservedActions===v.actions.absoluteMaxReservedActions&&Number.isSafeInteger(r.absoluteMaxReservedActions)&&
  r.absoluteMaxReservedActions>l.history.reservedActions&&r.absoluteMaxReservedActions<=l.history.reservedActions+8&&
  r.historicalUnsentReservations===l.history.reservedActions-l.history.sentActions)));
 if(!callerDate)for(const [key,alias] of Object.entries({observation:'observation',callerTerminal:'caller',launchSupervisor:'launch',protection:'protection',pidAbsence:'pidAbsence',ledger:'ledger',plan:'plan',lease:'lease'}))fail(same(r.evidence?.[key],e[alias]));
 const provider=wrongScene?{batch:'B1',runID:claim.binding?.runID,sourceSha256:claim.sourceSha256}:l.provider;
 fail(provider?.batch==='B1'&&/^[A-Za-z0-9_-]+$/.test(provider.runID)&&source.schemaVersion===6&&source.scenario===scenario&&source.batch==='B1'&&
  provider.sourceSha256===e.source.sha256&&l.b1ClaimSha256===e.claim.sha256&&claim.sourcePath===e.source.path&&
  claim.sourceSha256===e.source.sha256&&claim.binding?.schemaVersion===6&&claim.binding.batch==='B1'&&
  claim.binding.scenario===scenario&&claim.binding.workspaceRoot===v.sourceWorkspace&&claim.binding.runID===provider.runID&&
  claim.binding.sourceSha256===e.source.sha256&&claim.binding.originalPlanSha256===e.plan.sha256&&claim.binding.controllerSha256===e.controller.sha256);
 const output=resolve(v.sourceWorkspace,'.tmp/starsector-proxy',provider.runID);
 if(callerDate)for(const [key,name]of Object.entries(callerDateProviderNames))a[key]=pinRead(e[key],resolve(output,name),32*1024*1024);
 for(const [key,name] of Object.entries(providerNames)){
  if(key==='providerStderr'){fail(e[key].path===resolve(output,name)&&closed(e[key],['path','sha256']));const f=file(e[key].path);fail(f.sha256===e[key].sha256&&f.bytes.length===0);}
  else a[key]=pinRead(e[key],resolve(output,name));
 }
 const ready=a.providerReady,t=a.providerTerminal,spool=a.providerSpool;
 if(callerDate)assertCallerDateIdentityShapes(c,ready,fail);
 const step=t.steps?.[0];
 const supportSame=callerDate?
  same(pinRead(p.source?.supportPins,resolve(scenario,'source-pins.private.json')),a.supportPins)&&
  file(p.source.supportPins.path).bytes.equals(file(e.supportPins.path).bytes):same(p.source.supportPins,e.supportPins);
 fail(closed(a.supportPins,['pins'])&&Array.isArray(a.supportPins.pins)&&a.supportPins.pins.length<=1024&&
  supportSame&&lease.supportPinsPath===e.supportPins.path&&lease.supportPinsSha256?.toLowerCase()===e.supportPins.sha256);
 /** Historical binaries are selected from the original authority, not a new reader's runtime. @param {string} name */
 function executable(name){
  const pins=a.supportPins.pins.filter((/** @type {any} */ pin)=>typeof pin.path==='string'&&basename(pin.path).toLowerCase()===name);
  fail(pins.length===1);const pin=pins[0];
  fail(closed(pin,['path','sha256'])&&isAbsolute(pin.path)&&resolve(pin.path)===pin.path&&typeof pin.sha256==='string'&&/^[a-fA-F0-9]{64}$/.test(pin.sha256)&&
   file(pin.path,128*1024*1024).sha256===pin.sha256.toLowerCase());return pin.path;
 }
 const node=executable('node.exe'),powershell=executable('pwsh.exe');
 fail(t.terminalObserved===true&&t.cleanupUnknown===false&&t.terminalBoundToChild===true&&t.verifierStatus===0&&t.signal===null&&
  t.closeFailed===false&&t.overflow===false&&t.spawnFailed===false&&t.verdict==='PASS'&&t.terminalReceipt===e.providerTerminal.path&&
  t.steps?.length===1&&step.command===node&&step.status===0&&step.timedOut===false&&step.processOwnership==='windows-job-object'&&
  same(step.args,[resolve(v.sourceWorkspace,'tools/starsector-proxy/guarded-http-child.mjs')])&&step.cwd==='.'&&
  spool.schemaVersion===1&&spool.command==='verify'&&spool.ok===true&&spool.result?.verdict==='PASS'&&spool.result.steps?.length===1&&
  same(critical(spool.result.steps[0]),critical(step))&&ready.schemaVersion===1&&ready.runID===provider.runID&&ready.batch==='B1'&&
  ready.sourceSha256===e.source.sha256&&ready.startupTimeoutMs===20000&&ready.consentMode==='finite-explicit-consent'&&
  Number.isSafeInteger(t.verifierPid)&&ready.node?.executable===node&&ready.helper?.executable===powershell&&
  ready.node.parentPid===t.verifierPid&&ready.helper.parentPid===ready.node.pid&&
  ready.node.pid!==ready.helper.pid&&ready.node.sessionId===c.sessionId&&ready.helper.sessionId===c.sessionId&&
  ready.supervisorJobPids?.includes(ready.node.pid)&&ready.supervisorJobPids.includes(ready.helper.pid)&&
  ready.query==='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'&&Date.parse(t.at)>=Date.parse(ready.at));
 fail(closed(caller,['schemaVersion','kind','session','launchRaw','launchPolls','actual','terminalReaped'])&&caller.schemaVersion===1&&
  caller.kind==='ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL'&&caller.session===rootCall.value.raw.session_id&&caller.terminalReaped===true&&
  same(caller.launchRaw,rootCall.value.raw)&&Array.isArray(caller.launchPolls)&&caller.launchPolls.length>=1&&caller.launchPolls.length<=256);
 const chain=[caller.launchRaw,...caller.launchPolls];let bytes=0;
 for(const [i,raw] of chain.entries()){
  fail(typeof raw.output==='string'&&Number.isFinite(raw.wall_time_seconds)&&raw.wall_time_seconds>=0);
  const n=Buffer.byteLength(raw.output);bytes+=n;fail(n<=65536&&bytes<=131072);
  fail(i<chain.length-1?raw.session_id===caller.session&&(raw.exit_code===undefined||raw.exit_code===null):raw.exit_code===1&&!Object.hasOwn(raw,'session_id')&&same(raw,caller.actual));
 }
 const returned=callerDate?zeroProviderObject(chain.map(x=>x.output).join(''),fail):JSON.parse(chain.map(x=>x.output).join('')),ls=launch.result?.steps?.[0];
 fail(launch.schemaVersion===1&&launch.command==='verify'&&launch.ok===true&&launch.result.verdict==='FAIL'&&launch.result.steps.length===1&&
  ls.status===1&&ls.timedOut===false&&ls.processOwnership==='windows-job-object'&&typeof ls.stderr==='string'&&
  ls.stderr.includes('Driver requested owned cleanup; native success not established')&&returned.verdict==='FAIL'&&
  (!Array.isArray(returned.steps)||returned.steps.length===1)&&same(Array.isArray(returned.steps)?returned.steps[0]:returned.steps,returnedLaunchStep(ls)));
 fail(same(p.seedCreation?.preference,e.preference)&&same(p.seedCreation,lease.seedCreation)&&same(preference,readPreference(p,lease,scenario)));
 fail(protection.phase==='after-smoke'&&protection.protectedOriginalFiles===182&&protection.protectedSaveFiles===178&&
  protection.allOriginalAndBackupHashesMatch===true&&protection.saveCensusUnchanged===true&&protection.liveGameProcesses===0&&
  protection.originalWrites===false&&protection.privateAutosaveEnabled===false&&protection.privateSaveEntries===0&&
  protection.originalAutosave?.enabled===true&&protection.originalAutosave.maximumSlots===3&&same(protection.sharedPreference,preference));
 fail(owned.controllerPid===c.controllerPid&&owned.controllerCreationTime===c.controllerCreationTime&&
  owned.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&Array.isArray(owned.processes)&&
  Array.isArray(claim.binding.ownedPids)&&claim.binding.ownedPids.length>0&&c.installation===resolve(scenario,'installation'));
 const absence=a.pidAbsence,expected=[c.controllerPid,...c.supervisorJobPids,...owned.processes.flatMap((/** @type {any} */ x)=>[x.pid,...x.supervisorJobPids]),
  ...claim.binding.ownedPids,t.verifierPid,ready.node.pid,ready.helper.pid,...ready.supervisorJobPids];
 if(!callerDate)fail(absence.allAbsent===true&&same(absence.liveRows,[])&&Array.isArray(absence.queriedExactRecordedPids)&&
  expected.every(pid=>Number.isSafeInteger(pid)&&pid>0&&absence.queriedExactRecordedPids.includes(pid))&&Date.parse(absence.atUtc)>=Date.parse(t.at)&&
  Date.parse(protection.at)>=Date.parse(t.at));
 if(callerDate){assertCallerDateFailure({v,a,e,output,pinned:pinRead,fail,node,powershell});return v.actions;}
 if(linkage){assertCaptureLinkageFailure({v,a,e,output,pinned,fail,node,powershell});return v.actions;}
 if(wrongScene){assertWrongSceneFailure({v,a,e,output,pinned,fail,node,powershell});return v.actions;}
 if(known){assertKnownPlayFailure({v,a,e,output,pinned,fail,captureStop});return v.actions;}
 fail(Array.isArray(o.calls)&&o.calls.length===5&&o.schemaVersion===1&&
  o.kind==='ACTUAL_DRIVER_NO_INPUT_FOCUS_STOP'&&o.category==='FOCUS_BLOCKED'&&o.verdict==='FAIL'&&o.driver===driver&&
  o.scenarioId===p.scenarioId&&o.sourceHead===v.sourceHead&&o.inputActions===0&&o.activations===0&&o.providersStarted===1&&
  o.providersClosedKnown===1&&same(o.currentRunContext,r.actualRunContext)&&same(o.actualGuardHistory,l.history));
 const tools=['doctor','policy_status','get_window','get_frontmost_app','screenshot'];
 const attempts=readdirSync(output).filter(n=>/^call-\d+\.attempt\.private\.json$/.test(n)).sort();
 fail(same(attempts,tools.map((_,i)=>'call-'+String(i+1).padStart(3,'0')+'.attempt.private.json'))&&!readdirSync(scenario).some(n=>/^guard-E[12]-consumed\.private\.json$/.test(n)));
 /** @type {any[]} */const rawCalls=[];
 for(const [i,tool] of tools.entries()){
  const row=o.calls[i],prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));
  fail(closed(row,['completed','attempt','raw']));
  const post=pinned(row.completed,prefix+'.private.json'),attempt=pinned(row.attempt,prefix+'.attempt.private.json');
  const raw=pinned(row.raw,prefix+'.raw.private.json',8*1024*1024);rawCalls.push(raw);
  fail(post.tool===tool&&attempt.tool===tool&&post.runID===ready.runID&&attempt.runID===ready.runID&&post.batch==='B1'&&attempt.batch==='B1'&&
   attempt.sourceSha256===e.source.sha256&&same(post.args,attempt.args)&&same(post.precedingActions,[])&&post.isError===false&&raw.isError!==true&&
   Date.parse(post.callStarted)>=Date.parse(ready.at)&&Date.parse(post.callCompleted)>=Date.parse(post.callStarted)&&Date.parse(post.callCompleted)<=Date.parse(t.at));
 }
 const window=rawCalls[2].structuredContent,front=rawCalls[3].structuredContent?.app;
 const targets=owned.processes.filter((/** @type {any} */ proc)=>proc.pid===window?.pid);
 fail(targets.length===1&&targets[0].inSupervisorJob===true&&targets[0].sessionId===c.sessionId&&
  targets[0].executable===resolve(c.installation,'jre/bin/java.exe')&&targets[0].supervisorJobPids.includes(window.pid)&&
  c.supervisorJobPids.every((/** @type {number} */ pid)=>targets[0].supervisorJobPids.includes(pid))&&
  claim.binding.ownedPids.includes(window.pid)&&claim.binding.campaign?.windowId===window.windowId&&
  absence.queriedExactRecordedPids.includes(window.pid));
 fail(window?.isFocused===false&&window.windowId===o.capture?.windowId&&window.pid===o.capture.pid&&
  Number.isSafeInteger(front?.pid)&&front.pid!==window.pid&&front.pid===o.frontmost?.pid);
 return v.actions;
}
