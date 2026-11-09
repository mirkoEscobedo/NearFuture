// @ts-check
// Generated external-boundary data only; no game, source qualification or authority.
import {resolve,dirname} from 'node:path';
import {callerDateHistoryFixture} from './seed-caller-date-test-support.mjs';
import {value,ref} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

/** @param {{change?:(name:string,value:any)=>void}} [options] */
export function snapshotBusyHistoryFixture(options={}){
 const f=/** @type {any} */(callerDateHistoryFixture({watchdogMs:1800000,supportAlias:false,canonicalEnvironment:true,
  change:(name,v)=>{
   if(name==='B1-PLAY-client-geometry.actual.private.json'){v.clientOriginScreen={x:1,y:1};v.clientSize={width:1,height:1};v.offsetFromOuter={x:1,y:1};}
   if(name==='B1-PLAY-client-point.private.json'){v.localPoint={x:0,y:0};v.coordinate=[1,1];}
   options.change?.(name,v);
  }}));
 const e=f.evidencePaths,s=f.priorScenario,baseRefresh=f.refresh;
 const old=value(e.result),oldObservation=value(e.observation),window=value(f.calls[2].raw.path).structuredContent;
 const category='PRIVATE_POINT_CHECK_LIVE_SNAPSHOT_READ_EBUSY',point=value(e.point),ready=value(e.providerReady),ps=ready.helper.executable,node=ready.node.executable;
 const target=resolve(s,'owned-processes-current.private.json').replaceAll('\\','/');
 const fail={outerChunk:'generated-busy',exitCode:1,code:'EBUSY',errno:-4082,syscall:'open',path:target,
  location:'construct-actual-phase-inputs.private.mjs:262 pointCheck -> read():20 -> readFileSync',
  geometry:'PASS_NESTED_WINDOWS_JOB_STATUS0_REAPED',mapping:'PASS',pointCheck:'FAILED_READING_CURRENT_OWNED_SNAPSHOT',
  freshFocusPairAfterPointCheck:'NOT_CALLED',strictClick:'NOT_CALLED',timingFailure:false,focusFailure:false,dateFailure:false,
  cause:'Generated busy open; lock holder unknown.',retry:false,clockRenewal:false};
 const run=ready.runID,h=e.helper.replaceAll('\\','/'),n=node.replaceAll('\\','/'),inspected=point.inspectedAtMs,x=point.localPoint.x,y=point.localPoint.y;
 const cmd="$ErrorActionPreference='Stop';$env:PATH='"+dirname(node).replaceAll('\\','/')+';'+dirname(ps).replaceAll('\\','/')+";'+$env:PATH;$h='"+h+"';$n='"+n+"';"+
  '$gr=&$n $h geometry B1-PLAY '+run+' call-004 capture-001 '+inspected+";if($LASTEXITCODE-ne0){throw 'geometry failed'};$g=$gr|ConvertFrom-Json -DateKind String;"+
  '$mr=&$n $h play-point '+run+' call-004 capture-001 '+x+' '+y+' '+inspected+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'mapping failed'};$m=$mr|ConvertFrom-Json -DateKind String;"+
  "$cr=&$n $h point-check B1-PLAY $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw 'pointcheck failed'};$c=$cr|ConvertFrom-Json -DateKind String;"+
  '$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json -DateKind String;[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress';
 const metadata=value(e.metadataCaller);metadata.entries=[{label:'B1-PLAY',args:{cmd,shell:ps.replaceAll('\\','/'),sandbox_permissions:'require_escalated',max_output_tokens:1500},
  raw:{chunk_id:'generated-busy',wall_time_seconds:2,exit_code:1,original_token_count:50,
   output:"Error: EBUSY: resource busy or locked, open '"+resolve(s,'owned-processes-current.private.json')+"'\n    at readFileSync (node:fs:444:35)\n    at read (file:///"+h+":20:63)\n    at pointCheck (file:///"+h+":262:17)\nerrno: -4082,\ncode: 'EBUSY',\nsyscall: 'open'\npointcheck failed\n"},polls:[]}];json(e.metadataCaller,metadata);
 const o={schemaVersion:1,kind:'ACTUAL_S12_DRIVER_PRE_INPUT_POINTCHECK_EBUSY_STOP',driver:old.driver,scenario:s.replaceAll('\\','/'),sourceHead:old.sourceHead,
  atUtc:oldObservation.atUtc,failureCategory:category,assertion:'Generated focused Play; geometry/map pass then busy snapshot read.',
  inspection:[{stage:'PLAY',at:inspected,local:[0,0],assertion:'Generated Play enabled'}],window,
  vendorCallCount:4,originalImages:1,inputs:0,reservationsAdded:0,activations:0,metadataModes:{geometry:'PASS',playPoint:'PASS',pointCheck:'FAIL_EBUSY'},
  actualVendorCalls:oldObservation.actualVendorCalls.map((/** @type {any} */ row)=>({ordinal:row.ordinal,name:row.name,args:row.args,atMs:row.atMs})),
  providerFinish:oldObservation.providerFinish,failure:{chunk:'generated-busy',exitCode:1,code:'EBUSY',errno:-4082,syscall:'open',path:target,
   helperLocation:'construct-actual-phase-inputs.private.mjs:262 pointCheck -> readFileSync:20',noFreshFocusPair:true,noNativeInput:true,noRetry:true},
  historyBaseline:f.guard,actualContext:f.context,absoluteMaxReservedActions:f.cap,rootCallerSession:value(e.caller).session,
  originalCallerDeadlineMs:value(e.plan).callerAdmissionDeadlineMs,timingFailure:false,focusFailure:false,dateFailure:false,ordinaryExit:false,seed:false,rootProtection:'PENDING'};
 json(e.observation,o);
 const r={schemaVersion:1,kind:old.kind,scenarioId:old.scenarioId,driver:old.driver,sourceHead:old.sourceHead,status:'CLOSED_FAIL',verdict:'FAIL',failureCategory:category,
  inheritedGuardBaseline:f.guard,actualGuardHistory:f.guard,actualRunContext:f.context,absoluteMaxReservedActions:f.cap,historicalUnsentReservations:2,
  actions:{reservedThisRun:0,sentThisRun:0,clicks:0,activations:0},providers:{started:1,B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED',calls:4},
  counts:{originalPngs:1,geometryModes:1,mappingModes:1,pointCheckAttempts:1},actualCallOrder:['doctor','policy_status','get_window','screenshot'],
  steps:{B1:'STOPPED_BEFORE_INPUT_POINTCHECK_LIVE_SNAPSHOT_EBUSY',E1:'NOT_STARTED',E2:'NOT_STARTED'},
  visibleAssertion:{ownedPid:window.pid,ownedHwnd:window.windowId,isFocused:true,launcherPlayVisible:true,originalNativeDimensions:{width:2,height:2},localPoint:[0,0],mappedScreenPoint:[1,1],inputSent:false,captureReceipt:true,freshness:'UNKNOWN'},
  failure:fail,timing:{rootWatchdogMs:1800000,inputMarginMs:20000,cleanupReserveMs:30000,originalCallerDeadlineMs:value(e.plan).callerAdmissionDeadlineMs},
  closure:{},protection:{},claims:{campaign:false,seed:false,stability:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,newTypedHistoryKind:false},
  readerDisposition:'Generated generic failure; no other history kind granted.',evidence:{},calls:[],atUtc:new Date(f.anchor+6000).toISOString()};json(e.result,r);
 function refresh(){
  baseRefresh();
  const terminal=value(e.providerTerminal),caller=value(e.caller),controller=value(e.controller),owned=value(e.owned),protection=value(e.protection);
  const ids=[...new Set([controller.controllerPid,...controller.supervisorJobPids,...owned.processes.flatMap((/** @type {any} */ x)=>[x.pid,...x.supervisorJobPids]),terminal.verifierPid,ready.node.pid,ready.helper.pid,...ready.supervisorJobPids])].sort((/** @type {number} */ a,/** @type {number} */ b)=>a-b);
  json(e.pidAbsence,{schemaVersion:1,kind:'ACTUAL_CURRENT_OWNED_PID_ABSENCE_AFTER_ORIGINAL_CALLER_TERMINAL',atUtc:new Date(f.anchor+5001).toISOString(),allAbsent:true,queriedExactRecordedPids:ids,liveRows:[],controllerSha256:ref(e.controller).sha256,ownedSha256:ref(e.owned).sha256,providerReadySha256:ref(e.providerReady).sha256,providerTerminalSha256:ref(e.providerTerminal).sha256,originalCallerSha256:ref(e.caller).sha256});
  const result=value(e.result);
  result.closure={provider:{verifierPid:terminal.verifierPid,status:0,timedOut:false,verdict:'PASS',terminalReaped:true,at:terminal.at},rootCaller:{session:caller.session,originalRawChunk:caller.launchRaw.chunk_id,terminalRawChunk:caller.actual.chunk_id,status:1,timedOut:false,verdict:'FAIL',terminalReaped:true,pollCount:caller.launchPolls.length},maintainedStopToolChunk:'generated-stop',forcedOwnedCleanup:true,ordinaryExit:false,allOwnedHandlesTerminalReaped:true,knownPidsAbsent:ids};
  result.protection={protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,originalWrites:false,originalAutosave:protection.originalAutosave,privateAutosaveEnabled:false,privateSaveEntries:0,liveGameProcesses:0,sharedPreference:'Registry64/String/true unchanged',restoration:'UNNECESSARY_ORIGINALS_UNCHANGED'};
  result.calls=f.calls.map((/** @type {any} */ row)=>({completed:ref(row.post.path),attempt:ref(row.attempt.path),raw:ref(row.raw.path)}));
  const aliases={observation:e.observation,metadataCaller:e.metadataCaller,callerTerminal:e.caller,launchSupervisor:e.launch,protection:e.protection,pidAbsence:e.pidAbsence,ledger:e.ledger,plan:e.plan,lease:e.lease,controller:e.controller,owned:e.owned,ownedB1:e.initialOwned,rootCall:e.rootCall,source:e.source,claim:e.claim,preference:e.preference,supportPins:e.supportPins,geometry:e.geometry,geometryTerminal:e.geometryTerminal,point:e.point,coordinateProof:e.envelope,sourceBefore:e.sourceFrame,geometryOwned:e.frameOwned,helper:e.helper,geometryRunner:e.geometryRunner,preparationProgress:resolve(s,'progress.pre-native-preparation.original.private.json'),providerReady:e.providerReady,providerTerminal:e.providerTerminal,providerSpool:e.providerSpool,providerStderr:e.providerStderr,originalPng:f.original.png.path,originalMetadata:f.original.metadata.path,originalRaw:f.original.raw.path};
  result.evidence=Object.fromEntries(Object.entries(aliases).map(([k,p])=>[k,ref(p)]));json(e.result,result);
  f.closeout.kind='ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY';f.closeout.evidence=Object.fromEntries(Object.entries(e).map(([k,p])=>[k,ref(/** @type {string} */(p))]));json(f.closeoutPath,f.closeout);
  const grant=value(f.grantPath);grant.previous.result=ref(f.closeoutPath);json(f.grantPath,grant);
  for(const authority of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])authority.historyAdmission=ref(f.grantPath);f.refreshLaunch();
 }
 function repin(){
  const result=value(e.result);
  for(const [key,p]of Object.entries(e)){
   const alias={caller:'callerTerminal',launch:'launchSupervisor',initialOwned:'ownedB1',envelope:'coordinateProof',sourceFrame:'sourceBefore',frameOwned:'geometryOwned'}[key]??key;
   if(Object.hasOwn(result.evidence,alias))result.evidence[alias]=ref(/** @type {string} */(p));
  }
  result.calls=result.calls.map((/** @type {any} */ row)=>({...row,...Object.fromEntries(Object.entries(row).map(([key,p])=>[key,ref(/** @type {{path:string}} */(p).path)]))}));
  json(e.result,result);
  repinOuter();
 }
 function repinOuter(){
  Object.assign(f.closeout.evidence,Object.fromEntries(Object.entries(e).map(([k,p])=>[k,ref(/** @type {string} */(p))])));json(f.closeoutPath,f.closeout);
  const grant=value(f.grantPath);grant.previous={result:ref(f.closeoutPath),ledger:ref(e.ledger)};json(f.grantPath,grant);
  for(const authority of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])authority.historyAdmission=ref(f.grantPath);f.refreshLaunch();
 }
 refresh();return {...f,refresh,repin,repinOuter};
}
