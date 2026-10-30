// @ts-check
// Generated terminal/OS/observation boundaries only. No game or native evidence.
import {mkdtempSync,rmSync,readFileSync,realpathSync} from 'node:fs';
import {resolve,dirname,basename} from 'node:path';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {json,filePin,write,powershell,quote} from './seed-test-support.mjs';

/** @param {string} p */
export const ref=p=>({path:p,sha256:filePin(p).sha256.toLowerCase()});
/** @param {string} p */
export const value=p=>JSON.parse(readFileSync(p,'utf8'));
/** @type {{path:string,sha256:string}[]|undefined} */let runtimePins;
function providerPins(){
 if(!runtimePins){
  const shell=process.env.NF_TEST_POWERSHELL??powershell(['-Command','(Get-Process -Id $PID).Path']).stdout.trim();
  runtimePins=[process.execPath,shell].map(p=>ref(realpathSync.native(p)));
 }
 return runtimePins;
}

/** @param {{watchdogMs?:number}} [options] */
export function failedSeedHistoryFixture(options={}){
 const f=seedCreationFixture(options),scenario=mkdtempSync(resolve(dirname(f.scenario),'failed-history-prior-'));
 const runID=basename(scenario)+'-B1',output=resolve(f.root,'.tmp/starsector-proxy',runID);
 const guard={overallAttempts:40,uiAttempts:37,reservedActions:72,sentActions:70};
 const context={overallAttempts:41,uiAttempts:38},head='1'.repeat(40),session=777;
 const put=(/** @type {string} */ name,/** @type {any} */ v)=>{const p=resolve(scenario,name);json(p,v);return p;};
 const preference=put('seed-creation-preference.private.json',value(f.preferencePath));
 const seedCreation={mode:'CREATE_ONE_PRIVATE_DISPOSABLE_SEED',preference:ref(preference)};
 const supportPins=put('frozen-support-pins.private.json',{pins:[...value(f.launchLease.supportPinsPath).pins,...providerPins()]});
 const plan=put('plan.json',{...f.plan,scenarioId:basename(scenario),paths:{scenario},source:{qualifiedHead:head,supportPins:ref(supportPins)},historyBaseline:guard,seedCreation});
 const lease=put('native-lane-lease.private.json',{...f.launchLease,scenarioId:basename(scenario),planSha256:ref(plan).sha256,seedCreation,supportPinsPath:supportPins,supportPinsSha256:ref(supportPins).sha256});
 const originalController={...f.controller,installation:resolve(scenario,'installation'),rootLeaseSha256:ref(lease).sha256,supervisorJobPids:[100,105]};
 const controller=put('controller-start.private.json',originalController);
 const owned=put('owned-processes-current.private.json',{controllerPid:100,controllerCreationTime:originalController.controllerCreationTime,controllerKernelCreationFileTime:originalController.controllerKernelCreationFileTime,processes:[{pid:101,inSupervisorJob:true,sessionId:1,executable:resolve(scenario,'installation/jre/bin/java.exe'),supervisorJobPids:[100,105,101]}]});
 const raw={chunk_id:'generated-launch',wall_time_seconds:1,session_id:session,output:''};
 const rootCall=put('root-call.actual.private.json',{key:'generated-closed-attempt',value:{startMs:Date.now()-3000,args:{workdir:f.root,cmd:'GENERATED_NOT_RUN'},raw}});
 const source=put('B1-authorization.private.json',{schemaVersion:6,scenario,batch:'B1'});
 const claim=put('guard-B1-consumed.private.json',{sourcePath:source,sourceSha256:ref(source).sha256,binding:{schemaVersion:6,batch:'B1',scenario,workspaceRoot:f.root,runID,sourceSha256:ref(source).sha256,originalPlanSha256:ref(plan).sha256,controllerSha256:ref(controller).sha256,ownedPids:[101],campaign:{windowId:19}}});
 const ledger=put('guard-launch-history.private.json',{schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',rootRecordKey:'generated-closed-attempt',controllerSha256:ref(controller).sha256,history:guard,pending:null,uncertain:false,provider:{batch:'B1',runID,sourceSha256:ref(source).sha256},b1ClaimSha256:ref(claim).sha256});
 const readyPath=resolve(output,'guarded-http-ready.private.json');
 const terminalPath=resolve(output,'owned-provider-terminal.private.json');
 const at=new Date(Date.now()-2000).toISOString(),closedAt=new Date(Date.now()-1000).toISOString();
 json(readyPath,{schemaVersion:1,at,runID,batch:'B1',sourceSha256:ref(source).sha256,startupTimeoutMs:20000,consentMode:'finite-explicit-consent',node:{pid:102,parentPid:104,sessionId:1,executable:providerPins()[0].path},helper:{pid:103,parentPid:102,sessionId:1,executable:providerPins()[1].path},supervisorJobPids:[102,103],query:'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'});
 const providerStep={command:process.execPath,args:[resolve(f.root,'tools/starsector-proxy/guarded-http-child.mjs')],cwd:'.',processOwnership:'windows-job-object',status:0,timedOut:false};
 json(terminalPath,{at:closedAt,verifierPid:104,verifierStatus:0,signal:null,closeFailed:false,overflow:false,spawnFailed:false,terminalObserved:true,cleanupUnknown:false,terminalBoundToChild:true,verdict:'PASS',terminalReceipt:terminalPath,steps:[providerStep]});
 const providerSpool=resolve(output,'owned-provider.stdout.private.txt');json(providerSpool,{schemaVersion:1,command:'verify',ok:true,result:{verdict:'PASS',steps:[providerStep]}});
 const providerStderr=resolve(output,'owned-provider.stderr.private.txt');write(providerStderr,'');
 const returnedStep={status:1,timedOut:false,processOwnership:'windows-job-object',stdout:'',stderr:'Driver requested owned cleanup; native success not established'};
 const launchStep={...returnedStep,command:'generated-controller',args:['generated'],cwd:'.',module:'node:.'};
 const launch=put('launch-supervisor.private.json',{schemaVersion:1,command:'verify',ok:true,result:{verdict:'FAIL',steps:[launchStep]}});
 const terminalRaw={chunk_id:'generated-terminal',wall_time_seconds:1,exit_code:1,output:JSON.stringify({verdict:'FAIL',blocked:[],steps:returnedStep})};
 const caller=put('launch-caller-terminal.actual.private.json',{schemaVersion:1,kind:'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL',session,launchRaw:raw,launchPolls:[terminalRaw],actual:terminalRaw,terminalReaped:true});
 const protection=put('after-smoke-protection.private.json',{at:new Date().toISOString(),phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,liveGameProcesses:0,originalWrites:false,privateAutosaveEnabled:false,privateSaveEntries:0,originalAutosave:{enabled:true,maximumSlots:3},sharedPreference:value(preference)});
 const pidAbsence=put('owned-terminal-pid-absence.actual.private.json',{atUtc:new Date().toISOString(),queriedExactRecordedPids:[100,105,101,102,103,104],allAbsent:true,liveRows:[]});
 const tools=['doctor','policy_status','get_window','get_frontmost_app','screenshot'];
 const calls=tools.map((tool,i)=>{
  const path=resolve(output,'call-'+String(i+1).padStart(3,'0'));
  const args=tool==='get_window'?{window_id:19}:{};
  const post={tool,args,runID,batch:'B1',callStarted:at,callCompleted:at,precedingActions:[],isError:false};
  json(path+'.private.json',post);json(path+'.attempt.private.json',{...post,sourceSha256:ref(source).sha256});
  json(path+'.raw.private.json',{structuredContent:tool==='get_window'?{windowId:19,pid:101,isFocused:false}:tool==='get_frontmost_app'?{app:{pid:200,bundleId:'generated.exe'}}:{}});
  return {completed:ref(path+'.private.json'),attempt:ref(path+'.attempt.private.json'),raw:ref(path+'.raw.private.json')};
 });
 const observation=put('driver-observation-stop.private.json',{schemaVersion:1,kind:'ACTUAL_DRIVER_NO_INPUT_FOCUS_STOP',scenarioId:basename(scenario),driver:'/root/recovery_game_driver',verdict:'FAIL',category:'FOCUS_BLOCKED',sourceHead:head,currentRunContext:context,actualGuardHistory:guard,inputActions:0,activations:0,providersStarted:1,providersClosedKnown:1,calls,frontmost:{pid:200},capture:{windowId:19,pid:101,isFocused:false,freshness:'UNKNOWN'}});
 const result=put('driver-flow-result.private.json',{schemaVersion:1,kind:'ACTUAL_FINITE_DRIVER_FLOW_RESULT',scenarioId:basename(scenario),driver:'/root/recovery_game_driver',status:'CLOSED_FAIL',verdict:'FAIL',failureCategory:'FOCUS_BLOCKED',sourceHead:head,inheritedGuardBaseline:guard,actualGuardHistory:guard,actualRunContext:context,absoluteMaxReservedActions:80,historicalUnsentReservations:2,actions:{reservedThisRun:0,sentThisRun:0,clicks:0,activations:0},providers:{started:1,B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED',calls:5},evidence:{}});
 const evidencePaths={result,ledger,plan,lease,controller,owned,rootCall,caller,launch,protection,pidAbsence,preference,supportPins,source,claim,providerReady:readyPath,providerTerminal:terminalPath,providerSpool,providerStderr,observation};
 const closeout={schemaVersion:1,kind:'ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',scenario,sourceWorkspace:f.root,sourceHead:head,allOwnedHandlesTerminalReaped:true,actions:{actualGuard:guard,currentRunContext:context,absoluteMaxReservedActions:80},evidence:{}};
 const closeoutPath=resolve(scenario,'failed-seed-attempt-history.private.json');
 function refresh(){
  const r=value(result);r.evidence=Object.fromEntries(Object.entries({observation:evidencePaths.observation,callerTerminal:caller,launchSupervisor:launch,protection,pidAbsence,ledger,plan,lease}).map(([k,p])=>[k,ref(p)]));json(result,r);
  closeout.evidence=Object.fromEntries(Object.entries(evidencePaths).map(([k,p])=>[k,ref(p)]));json(closeoutPath,closeout);
  const baseline={...guard,...context};json(f.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',historyBaseline:baseline,maxReservedActions:80,previous:{result:ref(closeoutPath),ledger:ref(ledger)}});
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){v.historyBaseline={...baseline};v.historyAdmission=ref(f.grantPath);}
  f.campaign.historyBaseline={...baseline};f.predecessor.historyBaseline={...baseline};f.refreshLaunch();
 }
 refresh();
 return {...f,closeout,closeoutPath,evidencePaths,priorScenario:scenario,refresh,
  admit(){return powershell(['-Command',"$ErrorView='DetailedView'; & "+quote(resolve(f.root,'tools/starsector-proxy/fixture/protection-check.ps1'))+' -BindingPath '+quote(f.bindingPath)+' -BindingSha256 '+quote(f.bindingPin.sha256)+' -Phase admit-launch -EvidenceRoot '+quote(f.scenario)]);},
  close(){f.close();rmSync(scenario,{recursive:true,force:true});rmSync(output,{recursive:true,force:true});}};
}
