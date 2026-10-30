// @ts-check
// Generated tool/OS boundary data only. No native or source qualification.
import {mkdtempSync,rmSync,realpathSync} from 'node:fs';
import {resolve,dirname,basename} from 'node:path';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {json,write,quote,powershell} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

export const zeroKind='ROOT_CLOSED_ZERO_PROVIDER_CONTROLLER_CLOCK_FAILED_SEED_ATTEMPT_HISTORY';
/** @param {{divergentHistory?:boolean}} [options] */
export function zeroProviderHistoryFixture(options={}){
 const f=seedCreationFixture(),s=mkdtempSync(resolve(dirname(f.scenario),'zero-provider-prior-'));
 const guard={overallAttempts:46,uiAttempts:43,reservedActions:76,sentActions:74},cap=84,runID=basename(s)+'-B1';
 const priorOutput=resolve(f.root,'.tmp/starsector-proxy',runID);
 const anchor=Date.parse('2026-10-27T07:50:38.458Z'),ps=realpathSync.native(process.env.NF_TEST_POWERSHELL??powershell(['-Command','(Get-Process -Id $PID).Path']).stdout.trim());
 const put=(/** @type {string} */ name,/** @type {any} */ v)=>{const p=resolve(s,name);json(p,v);return p;};
 const preference=put('seed-creation-preference.private.json',value(f.preferencePath));
 const supportPins=put('frozen-support-pins.private.json',{pins:[ref(ps),ref(realpathSync.native(process.execPath))]});
 const callerScript=resolve(s,'launch-owned-caller.private.ps1'),stager=resolve(s,'record-root-call-and-stage-B1.private.ps1');
 write(callerScript,'# generated never executed');write(stager,'# generated never executed');
 const helper=resolve(s,'construct-actual-phase-inputs.private.mjs');write(helper,'// generated never executed');
 const freezeScript=resolve(s,'root-freeze-and-preflight.seed.proposed.private.ps1');write(freezeScript,'# generated never executed');
 const preclockIndex=put('seed-graph-preclock-status.proposed.private.json',{kind:'UNISSUED_SEED_GRAPH_PRECLOCK_PROPOSAL_INDEX',scenario:s});
 const loaded=put('loaded-frontend-witness.private.json',{sourceHead:'1'.repeat(40),generated:true});
 const delegation=put('root-postlaunch-mechanical-delegation.private.json',{helperSha256:ref(helper).sha256});
 const history=put('root-history-baseline.admitted.private.json',{retained:true});
 const plan=put('plan.json',{...f.plan,scenarioId:basename(s),paths:{scenario:s},source:{qualifiedHead:'1'.repeat(40),supportPins:ref(supportPins),loadedFrontendWitness:ref(loaded)},
  historyBaseline:guard,absoluteMaxReservedActions:cap,historyAdmission:ref(history),callerAdmissionDeadlineMs:anchor+472550,
  batches:f.plan.batches.map((/** @type {any} */ b)=>b.id==='B1'?{...b,runID}:b),
  seedCreation:{mode:'CREATE_ONE_PRIVATE_DISPOSABLE_SEED',preference:ref(preference)},preclockHelpers:{
   'launch-owned-caller.private.ps1':ref(callerScript),'record-root-call-and-stage-B1.private.ps1':ref(stager),
   'construct-actual-phase-inputs.private.mjs':ref(helper),'root-freeze-and-preflight.seed.proposed.private.ps1':ref(freezeScript)}});
 const lease=put('native-lane-lease.private.json',{...f.launchLease,scenarioId:basename(s),planSha256:ref(plan).sha256,
  seedCreation:value(plan).seedCreation,historyAdmission:options.divergentHistory?{path:resolve(s,'other-history.private.json'),sha256:'9'.repeat(64)}:ref(history),absoluteMaxReservedActions:cap,
  supportPinsPath:supportPins,supportPinsSha256:ref(supportPins).sha256,callerAdmissionDeadlineMs:anchor+472550});
 const controller=put('controller-start.private.json',{...f.controller,at:'2026-10-27T07:50:59.4028786Z',controllerCreationTime:'2026-10-27T07:50:48.7199730Z',
  conservativeOuterDeadlineUtc:'2026-10-27T07:58:48.5837603Z',controllerDeadlineUtc:'2026-10-27T07:58:49.2838289Z',
  installation:resolve(s,'installation'),rootLeaseSha256:ref(lease).sha256,supervisorJobPids:[100,105]});
 const proc={pid:101,parentPid:106,creationTime:'2026-10-27T07:50:59.622869Z',kernelCreationFileTime:'134375610596228693',executable:resolve(s,'installation/jre/bin/java.exe'),sessionId:1,inSupervisorJob:true,supervisorJobPids:[100,105,106,101],mainWindowHandle:'19'};
 const owned=put('owned-processes-current.private.json',{at:'2026-10-27T07:51:05.171Z',controllerPid:100,controllerCreationTime:value(controller).controllerCreationTime,controllerKernelCreationFileTime:f.controller.controllerKernelCreationFileTime,processes:[proc]});
 const initialOwned=put('driver-B1-owned.private.json',{at:'2026-10-27T07:51:05.582Z',sourceSnapshot:{path:owned,sha256:'a'.repeat(64)},processes:[proc]});
 const raw={chunk_id:'generated-launch',wall_time_seconds:10,session_id:777,output:''};
 const frozen=put('root-final-prelaunch-actual.private.json',{schemaVersion:1,issuer:'/root',atUtc:'2026-10-27T07:50:38.3214709Z',planSha256:ref(plan).sha256,leaseSha256:ref(lease).sha256,delegationSha256:ref(delegation).sha256,helperSha256:ref(helper).sha256,historyAdmissionSha256:ref(history).sha256,callerAdmissionDeadlineMs:anchor+472550,cleanupStartNoLaterMs:anchor+442550,actualWatchdogMs:480000,actualPsAdmissionExit:0,actualPsAdmissionOutput:[],gameStarted:false,providerStarted:false,next:'Immediate announced exact combined dispatch; no postfreeze construction/review/reissue'});
 const fr=value(frozen),q=quote;
 const args={cmd:'& '+q(ps)+' -NoProfile -NonInteractive -File '+q(callerScript)+' -PlanSha256 '+q(fr.planSha256)+' -LeaseSha256 '+q(fr.leaseSha256)+' -DelegationSha256 '+q(fr.delegationSha256)+' -HelperSha256 '+q(fr.helperSha256)+' -HistoryAdmissionSha256 '+q(fr.historyAdmissionSha256),workdir:f.root};
 const rootCall=put('root-call.actual.private.json',{key:'generated-zero-provider-root',value:{startMs:anchor,args,raw}});
 const rootClock={recordKey:value(rootCall).key,startMs:anchor,deadlineMs:anchor+480000,sessionWatchdogMs:480000,cleanupReserveMs:30000,callerAdmissionDeadlineMs:fr.callerAdmissionDeadlineMs,originalCallerAdmissionDeadlineMs:fr.callerAdmissionDeadlineMs,producerRecordKey:'controller-start:'+ref(controller).sha256,effectiveDeadlineMs:fr.callerAdmissionDeadlineMs};
 const producerWitness={producerRecordKey:rootClock.producerRecordKey,controllerPid:100,controllerCreationTime:value(controller).controllerCreationTime,controllerKernelCreationFileTime:f.controller.controllerKernelCreationFileTime,conservativeOuterDeadlineUtc:value(controller).conservativeOuterDeadlineUtc,controllerDeadlineUtc:value(controller).controllerDeadlineUtc,effectiveDeadlineMs:fr.callerAdmissionDeadlineMs,sessionWatchdogMs:480000};
 const predecessor=put('campaign-postlaunch.private.json',{schemaVersion:1,kind:'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR',plan:ref(plan),lease:ref(lease),controller:ref(controller),rootClock,producerWitness,rootCallRecord:value(rootCall),historyBaseline:guard,maxReservedActions:cap});
 const campaign={...f.campaign,flow:'DISPOSABLE_SEED_CREATION',windowId:19,rootClock,producerWitness,rootCallRecord:value(rootCall),historyBaseline:guard,maxReservedActions:cap,launchPredecessor:ref(predecessor)};
 const phasePlan=put('B1-plan.private.json',{...value(plan),campaign,batches:value(plan).batches.filter((/** @type {any} */ b)=>b.id==='B1')});
 const phaseLease=put('B1-lease.private.json',{...value(lease),schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:ref(phasePlan).sha256});
 const source=put('B1-authorization.private.json',{schemaVersion:6,scenario:s,batch:'B1',planPath:phasePlan,planSha256:ref(phasePlan).sha256,leasePath:phaseLease,leaseSha256:ref(phaseLease).sha256,ownedReceiptPath:initialOwned,ownedReceiptSha256:ref(initialOwned).sha256});
 const ledger=put('guard-launch-history.private.json',{schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',rootRecordKey:value(rootCall).key,controllerSha256:ref(controller).sha256,history:guard,pending:null,uncertain:false});
 const returnedStep={status:1,timedOut:false,processOwnership:'windows-job-object',stdout:'',stderr:'Driver requested owned cleanup; native success not established'};
 const launch=put('launch-supervisor.private.json',{schemaVersion:1,command:'verify',ok:true,result:{verdict:'FAIL',steps:[{...returnedStep,command:ps,args:['-NoProfile','-NonInteractive','-File',resolve(f.root,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',s,'-BindingPath',value(plan).fixtureBinding.path,'-BindingSha256',value(plan).fixtureBinding.sha256,'-OuterDeadlineUtc',value(controller).conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000'],cwd:'.'}]}});
 const terminalRaw={chunk_id:'generated-terminal',wall_time_seconds:1,exit_code:1,output:JSON.stringify({verdict:'FAIL',blocked:[],steps:returnedStep})};
 const caller=put('launch-caller-terminal.actual.private.json',{schemaVersion:1,kind:'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL',session:777,launchRaw:raw,launchPolls:[terminalRaw],actual:terminalRaw,terminalReaped:true});
 const stageArgs={cmd:'& '+q(ps)+' -NoProfile -NonInteractive -File '+q(stager)+' -RootCallJson '+q(JSON.stringify(value(rootCall)))+' -HelperSha256 '+q(fr.helperSha256)+' -HistoryAdmissionSha256 '+q(fr.historyAdmissionSha256)+' -DelegationSha256 '+q(fr.delegationSha256),workdir:f.root};
 const combined=put('combined-launch-stage.actual.private.json',{launch:value(rootCall),stage:{args:stageArgs,raw:{chunk_id:'generated-stage',wall_time_seconds:10,session_id:778,output:''}},stagePolls:[{chunk_id:'generated-stage-terminal',wall_time_seconds:1,exit_code:1,output:'Error: Campaign authority refused: actual controller clock and identity required\nActual unclaimed schema6 B1 admission refused; no retry'}],stop:{chunk_id:'generated-stop',wall_time_seconds:0,exit_code:0,output:''},stopPolls:[],launchPolls:[terminalRaw],error:{name:'Error',message:'Actual B1 stager not known terminal exit0; no provider'},b1Admit:null,remainingStageSession:null,remainingLaunchSession:null});
 const freezeStep={command:ps.replaceAll('\\','/'),args:['-NoProfile','-NonInteractive','-File',freezeScript.replaceAll('\\','/'),'-RootAdmitted','-ProcedureSha256',ref(freezeScript).sha256,'-PreclockIndexSha256',ref(preclockIndex).sha256,'-LoadedFrontendWitnessPath',loaded.replaceAll('\\','/'),'-LoadedFrontendWitnessSha256',ref(loaded).sha256],cwd:'.',status:0,timedOut:false,processOwnership:'windows-job-object',stdout:JSON.stringify(fr),stderr:''};
 const freezeSupervisor=put('root-freeze-supervisor.actual.private.json',{args:{},raw:{chunk_id:'generated-freeze',wall_time_seconds:9,exit_code:0,output:JSON.stringify({schemaVersion:1,command:'verify',ok:true,result:{verdict:'PASS',steps:[freezeStep]}})},polls:[],priorRejectedCall:{retained:true}});
 const at='2026-10-27T07:52:38.5246497Z';
 const protection=put('after-smoke-protection.private.json',{at,phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,liveGameProcesses:0,originalWrites:false,privateAutosaveEnabled:false,privateSaveEntries:0,originalAutosave:{enabled:true,maximumSlots:3},sharedPreference:value(preference)});
 const pidAbsence=put('poststop-pid-absence.actual.private.json',{atUtc:at,expectedPids:[100,105,106,101],absentPids:[100,105,106,101],noLiveOwnedPids:true,providerStarted:false,nativeInputs:0,forcedOwnedCleanup:true,ordinaryExitAccepted:false});
 const evidencePaths={plan,lease,controller,owned,rootCall,caller,launch,protection,pidAbsence,preference,supportPins,source,phasePlan,phaseLease,initialOwned,predecessor,frozen,freezeSupervisor,combined,ledger,preclockIndex};
 const closeout={schemaVersion:1,kind:zeroKind,issuer:'/root',driver:'/root/recovery_game_driver',scenario:s,sourceWorkspace:f.root,sourceHead:'1'.repeat(40),allOwnedHandlesTerminalReaped:true,actions:{actualGuard:guard,currentRunContext:{overallAttempts:47,uiAttempts:44},absoluteMaxReservedActions:cap},evidence:{}};
 const closeoutPath=resolve(s,'failed-zero-provider-history.private.json');
 function refresh(){
  closeout.evidence=Object.fromEntries(Object.entries(evidencePaths).map(([k,p])=>[k,ref(p)]));json(closeoutPath,closeout);
  const baseline={...guard,...closeout.actions.currentRunContext};json(f.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',historyBaseline:baseline,maxReservedActions:cap,previous:{result:ref(closeoutPath),ledger:ref(ledger)}});
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){v.historyBaseline={...baseline};v.historyAdmission=ref(f.grantPath);v.absoluteMaxReservedActions=cap;}
  f.campaign.historyBaseline={...baseline};f.campaign.maxReservedActions=cap;f.predecessor.historyBaseline={...baseline};f.predecessor.maxReservedActions=cap;f.refreshLaunch();
 }
 refresh();return {...f,priorScenario:s,priorOutput,evidencePaths,closeout,closeoutPath,refresh,close(){
  if(dirname(priorOutput)!==resolve(f.root,'.tmp/starsector-proxy')||dirname(s)!==dirname(f.scenario))throw Error('Generated cleanup containment required');
  f.close();rmSync(s,{recursive:true,force:true});rmSync(priorOutput,{recursive:true,force:true});
 }};
}
