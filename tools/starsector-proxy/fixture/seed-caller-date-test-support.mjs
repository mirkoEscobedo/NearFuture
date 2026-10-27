// @ts-check
// Generated terminal, OS and image boundary data; no game or native qualification.
import {resolve,basename} from 'node:path';
import {readFileSync,writeFileSync} from 'node:fs';
import {captureLinkageHistoryFixture} from './seed-capture-linkage-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json as writeJson,write,quote} from './seed-test-support.mjs';

export const callerDateKind='ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY';
/** @param {{change?:(name:string,value:any)=>void,campaign?:(value:any)=>void,capture?:(value:any)=>void,metadata?:(value:any)=>void,provider?:(value:any)=>void,spool?:(value:any)=>void,aliasBytes?:(bytes:Buffer)=>Buffer|string,controller?:(value:any)=>void,window?:(value:any)=>any}} [options] */
export function callerDateHistoryFixture(options={}){
 const json=(/** @type {string} */ path,/** @type {any} */ object)=>{options.change?.(basename(path),object);writeJson(path,object);};
 const f=/** @type {any} */(captureLinkageHistoryFixture()),e=f.evidencePaths,s=f.priorScenario;
 const iso=(/** @type {number} */ n)=>new Date(f.anchor+n).toISOString(),run=value(e.providerReady).runID;
 const output=resolve(f.root,'.tmp/starsector-proxy',run),oldObservation=e.observation;
 const put=(/** @type {string} */ name,/** @type {any} */ object)=>{const p=resolve(s,name);json(p,object);return p;};
 const original=f.original,windowRawPath=resolve(output,'call-003.raw.private.json'),windowRaw=value(windowRawPath),window=options.window?.(windowRaw.structuredContent)??windowRaw.structuredContent;
 windowRaw.structuredContent=window;write(windowRawPath,JSON.stringify(windowRaw)+'\n');const visualWindow=Array.isArray(window)?window[0]:window;
 const originalController=value(e.controller);options.controller?.(originalController);writeJson(e.controller,originalController);
 const initialCurrent=value(e.owned);for(const key of ['controllerPid','controllerCreationTime','controllerKernelCreationFileTime'])initialCurrent[key]=originalController[key];
 json(e.owned,initialCurrent);json(e.caller,value(e.caller));json(e.protection,value(e.protection));const currentOwned=value(e.owned),owner=currentOwned.processes[0];
 e.observation=resolve(s,'driver-stop-observation.private.json');
 e.initialOwned=resolve(s,'driver-B1-owned.private.json');
 // Model the helper's recorded projection before its source alias was overwritten.
 writeJson(e.owned,{...currentOwned,at:iso(550)});const observedSource=ref(e.owned);writeJson(e.owned,currentOwned);
 json(e.initialOwned,{at:iso(600),sourceSnapshot:observedSource,processes:currentOwned.processes});
 e.frameOwned=put('B1-PLAY-owned-snapshot.private.json',{...value(e.owned),at:iso(1800)});
 if(options.capture){const raw=value(original.raw.path);options.capture(raw);for(const path of [original.raw.path,f.calls[3].raw.path])write(path,JSON.stringify(raw)+'\n');original.raw=ref(original.raw.path);}
 const captureMetadata=value(original.metadata.path);captureMetadata.window=window;captureMetadata.rawSha256=ref(original.raw.path).sha256;options.metadata?.(captureMetadata);writeJson(original.metadata.path,captureMetadata);original.metadata=ref(original.metadata.path);
 const frame={pngPath:original.png.path,pngSha256:original.png.sha256,metadataPath:original.metadata.path,
  metadataSha256:original.metadata.sha256,capturedAtMs:Date.parse(value(original.metadata.path).callCompleted)};
 e.beforeFrame=put('B1-PLAY-before-frame.private.json',frame);
 e.geometry=put('B1-PLAY-client-geometry.actual.private.json',{schemaVersion:1,scope:'READ_ONLY_OWNED_HWND_CLIENT_GEOMETRY',
  windowId:19,pid:101,creationTime:owner.creationTime,controllerPid:value(e.controller).controllerPid,observedAtUtc:iso(2000),
  outerBounds:visualWindow.bounds,clientOriginScreen:{x:0,y:0},clientSize:{width:2,height:2},offsetFromOuter:{x:0,y:0},
  mcpOuterBoundsMatched:true,inputGranted:false,originMeaning:'Generated client origin'});
 e.geometryRunner=resolve(s,'read-client-geometry.owned.private.mjs');write(e.geometryRunner,'// GENERATED INERT GEOMETRY RUNNER; NEVER EXECUTE\n');
 const plan=value(e.plan);plan.preclockHelpers['read-client-geometry.owned.private.mjs']=ref(e.geometryRunner);json(e.plan,plan);
 const supervisor=resolve(f.root,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe');
 const support=value(e.supportPins);for(const path of [supervisor,resolve(f.root,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1')])if(!support.pins.some((/** @type {any} */ x)=>x.path===path))support.pins.push(ref(path));json(e.supportPins,support);
 e.sourceFrame=resolve(s,'B1-PLAY-source-before.private.json');
 e.geometryTerminal=resolve(s,'B1-PLAY-client-geometry-terminal.actual.private.json');
 const geometryProject=resolve(s,'geometry-owned-B1-PLAY'),ps=value(e.providerReady).helper.executable;
 const child={program:ps.replaceAll('\\','/'),args:['-NoProfile','-NonInteractive','-File',resolve(f.root,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1'),
  '-EvidenceRoot',s,'-WindowId','19','-ExpectedX','0','-ExpectedY','0','-ExpectedWidth','2','-ExpectedHeight','2'],cwd:'.'};
 const configuration=resolve(geometryProject,'.agentic/project.json');json(configuration,{version:2,overrides:{workspace:{commands:{'node:.':[child]}}}});
 e.point=resolve(s,'B1-PLAY-client-point.private.json');e.envelope=resolve(s,'B1-PLAY-coordinate-proof.private.json');
 const inspected=f.anchor+1400,computed=f.anchor+2300;
 const node=value(e.providerReady).node.executable;
 const cmd="$ErrorActionPreference='Stop';$h='"+e.helper.replaceAll('\\','/')+"';$n='"+node.replaceAll('\\','/')+"';"+
  '$gRaw=&$n $h geometry B1-PLAY '+run+' call-004 capture-001 '+inspected+";if($LASTEXITCODE-ne0){throw 'Geometry failed; no mapping/input'};$g=$gRaw|ConvertFrom-Json;"+
  '$mRaw=&$n $h play-point '+run+' call-004 capture-001 1 1 '+inspected+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'Mapping failed; no input'};$m=$mRaw|ConvertFrom-Json;"+
  "$cRaw=&$n $h point-check B1-PLAY $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw 'Point-check failed; no input'};$c=$cRaw|ConvertFrom-Json;"+
  '$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json;[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress';
 const d=new Date(f.anchor+2000),us=String(d.getUTCMonth()+1).padStart(2,'0')+'/'+String(d.getUTCDate()).padStart(2,'0')+'/'+d.getUTCFullYear()+' '+d.toISOString().slice(11,19);
 const error="String '"+us+"' was not recognized as a valid DateTime.";
 const metadata={schemaVersion:1,kind:'ACTUAL_DRIVER_METADATA_CALLS',driver:'/root/recovery_game_driver',scenario:s,
  entries:[{label:'B1-PLAY',args:{cmd,sandbox_permissions:'require_escalated',max_output_tokens:2000},
   raw:{chunk_id:'generated-date-failure',wall_time_seconds:1.5,exit_code:1,original_token_count:50,
    output:'ParentContainsErrorRecordException: Exception calling "Parse" with "1" argument(s): "'+error+'"'},polls:[]}]};json(e.metadataCaller,metadata);
 const generic={schemaVersion:1,kind:'ACTUAL_FINITE_DRIVER_FLOW_RESULT',scenarioId:basename(s),driver:'/root/recovery_game_driver',status:'CLOSED_FAIL',verdict:'FAIL',
  failureCategory:'CALLER_METADATA_SERIALIZATION_FAILED',sourceHead:plan.source.qualifiedHead,
  history:{guard:f.guard,actualContext:f.context,absoluteMaxReservedActions:f.cap,historicalUnsentReservations:2,addedReservations:0,addedSentActions:0,refunded:0},
  counts:{vendorCalls:4,originalPngs:1,providersStarted:1,actions:0,activations:0},
  steps:{B1:'STOPPED_BEFORE_INPUT_CALLER_DATE_SERIALIZATION',E1:'NOT_STARTED',E2:'NOT_STARTED'},
  originalInspection:{file:'capture-001.png',sha256:original.png.sha256,dimensions:{width:2,height:2},assertion:'Generated focused launcher Play',localPoint:{x:1,y:1},screenPointFromActualGeometry:[1,1],freshness:'UNKNOWN',inputSent:false},
  geometry:{childTerminal:'PASS_STATUS0_NO_TIMEOUT_WINDOWS_JOB_REAPED',mapping:'PASSED',pointCheck:'PASSED_BEFORE_FINAL_CALLER_SERIALIZATION',observedAtUtc:iso(2000),subsequentFreshFocusPair:'NOT_CALLED',strictClick:'NOT_CALLED'},
  failure:{outerExecChunk:'generated-date-failure',exitCode:1,literalError:error,boundary:'Generated final caller date conversion failure',sourceLocation:'Generated caller command',classification:'DRIVER_CALLER_ONLY_NOT_SOURCE_NOT_TIMEOUT',currentRetry:false,clockRenewed:false,proposedNextCallerOnlyFix:'Generated proposal: use ConvertFrom-Json -DateKind String in a future caller; preserve the failed bytes.'},
  closure:{providerB1:{verifierPid:value(e.providerTerminal).verifierPid,status:0,timedOut:false,terminalObserved:true,terminalReaped:true,at:iso(3000),verdict:'PASS'},
   rootCaller:{session:777,terminalChunk:'generated-terminal',exitCode:1,timedOut:false,terminalReaped:true,verdict:'FAIL',reason:'MAINTAINED_REQUESTED_OWNED_CLEANUP'},
   ownedStopAtUtc:iso(3500),ownedPidAbsence:[100,101,105],ordinaryExit:false,allOwnedTerminalReaped:true,originalCallerDeadlineMs:plan.callerAdmissionDeadlineMs},
  protection:{protectedOriginalFiles:182,protectedSaveFiles:178,originalAndBackupHashesMatch:true,saveCensusUnchanged:true,privateAutosaveEnabled:false,privateSaveEntries:0,liveGameProcesses:0,originalWrites:false,restoration:'UNNECESSARY_ORIGINALS_UNCHANGED',sharedPreference:'Registry64/String/true unchanged'},
  claims:{seed:false,manualSave:false,campaign:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,closedHistoryKindIssued:false},evidence:{},atUtc:iso(5000)};
 const observation={schemaVersion:1,kind:'ACTUAL_S8_DRIVER_PRE_INPUT_CALLER_SERIALIZATION_STOP',driver:generic.driver,scenario:s,atUtc:iso(3300),failureCategory:generic.failureCategory,
  assertion:'Generated focused launcher Play',inspection:{at:inspected,local:[1,1],assertion:'Generated Play enabled'},
  actualVendorCalls:f.calls.map((/** @type {any} */ row,/** @type {number} */ i)=>({ordinal:i+1,batch:'B1',name:value(row.post.path).tool,args:i===0?{}:value(row.post.path).args,atMs:Date.parse(value(row.post.path).callCompleted)})),
  inputs:0,reservations:0,begin:{args:{consentMode:'finite-explicit-consent',runID:run,source:e.source.replaceAll('\\','/')},raw:{content:[{type:'text',text:JSON.stringify({runID:run,output,deadline:iso(180000)})}]},at:f.anchor+1001},
  providerFinish:{raw:{content:[{type:'text',text:JSON.stringify(value(e.providerTerminal))}]},at:f.anchor+3001},
  metadataFailure:{outerChunk:'generated-date-failure',exitCode:1,boundary:'Generated final parser error following three successful metadata modes.',literalError:error},noRetry:true,rootCallerSession:777,originalCallerCap:plan.callerAdmissionDeadlineMs,
  historyBaseline:f.guard,actualContext:f.context,absoluteMaxReservedActions:f.cap,ordinaryExit:false,seed:false,rootProtection:'PENDING'};
 json(e.observation,observation);json(e.result,generic);
 const callerRaw=put('caller-terminal.actual.private.json',value(e.caller).actual),progress=put('progress.pre-native-preparation.original.private.json',{generated:true});
 e.geometryConfiguration=configuration;
 for(const [i,row]of f.calls.entries()){e['call'+(i+1)+'Attempt']=row.attempt.path;e['call'+(i+1)+'Raw']=row.raw.path;}
 function refresh(){
  // Refresh the generated acyclic original root/predecessor/claim after adding the runner inventory.
  const p=value(e.plan),lease=value(e.lease),c=value(e.controller),root=value(e.rootCall),claim=value(e.claim);
  const sourcePins=resolve(s,'source-pins.private.json'),supportBytes=readFileSync(e.supportPins);writeFileSync(sourcePins,options.aliasBytes?.(supportBytes)??supportBytes);
  p.source.supportPins=ref(sourcePins);json(e.plan,p);lease.planSha256=ref(e.plan).sha256;lease.supportPinsSha256=ref(e.supportPins).sha256;json(e.lease,lease);
  c.rootLeaseSha256=ref(e.lease).sha256;json(e.controller,c);
  const delegation=resolve(s,'root-postlaunch-mechanical-delegation.private.json'),deleg=value(delegation);
  Object.assign(deleg,{planSha256:ref(e.plan).sha256,leaseSha256:ref(e.lease).sha256});json(delegation,deleg);
  root.value.args.cmd='& '+quote(ps)+' -NoProfile -NonInteractive -File '+quote(p.preclockHelpers['launch-owned-caller.private.ps1'].path)+' -PlanSha256 '+quote(ref(e.plan).sha256)+' -LeaseSha256 '+quote(ref(e.lease).sha256)+' -DelegationSha256 '+quote(ref(delegation).sha256)+' -HelperSha256 '+quote(ref(e.helper).sha256)+' -HistoryAdmissionSha256 '+quote(p.historyAdmission.sha256);json(e.rootCall,root);
  const campaign=claim.binding.campaign,key='controller-start:'+ref(e.controller).sha256;
  options.campaign?.(campaign);
  campaign.rootCallRecord=root;campaign.rootClock.producerRecordKey=key;campaign.producerWitness.producerRecordKey=key;campaign.producerWitness.controllerKernelCreationFileTime=c.controllerKernelCreationFileTime;
  const predecessor=campaign.launchPredecessor.path,pre=value(predecessor);
  Object.assign(pre,{plan:ref(e.plan),lease:ref(e.lease),controller:ref(e.controller),rootCallRecord:root,rootClock:campaign.rootClock,producerWitness:campaign.producerWitness});json(predecessor,pre);campaign.launchPredecessor=ref(predecessor);
  const auth=value(e.source),bp={...p,campaign,batches:p.batches.filter((/** @type {any} */ b)=>b.id==='B1')};json(auth.planPath,bp);json(auth.leasePath,{...lease,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:ref(auth.planPath).sha256});
  Object.assign(auth,{planSha256:ref(auth.planPath).sha256,leaseSha256:ref(auth.leasePath).sha256,ownedReceiptSha256:ref(e.initialOwned).sha256});json(e.source,auth);
  Object.assign(claim,{sourceSha256:ref(e.source).sha256});Object.assign(claim.binding,{campaign,sourceSha256:ref(e.source).sha256,planSha256:auth.planSha256,originalPlanSha256:ref(e.plan).sha256,controllerSha256:ref(e.controller).sha256});json(e.claim,claim);
  const ready=value(e.providerReady);ready.sourceSha256=ref(e.source).sha256;json(e.providerReady,ready);
  {const terminal=value(e.providerTerminal),spool=value(e.providerSpool);options.provider?.(terminal);spool.result.steps=terminal.steps;options.spool?.(spool);json(e.providerTerminal,terminal);json(e.providerSpool,spool);}
  for(const row of f.calls){const a=value(row.attempt.path);a.sourceSha256=ref(e.source).sha256;json(row.attempt.path,a);Object.assign(row.attempt,ref(row.attempt.path));const raw=value(row.raw.path);options.change?.(basename(row.raw.path),raw);write(row.raw.path,JSON.stringify(raw)+'\n');json(row.post.path,value(row.post.path));}
  const ledger=value(e.ledger);Object.assign(ledger,{controllerSha256:ref(e.controller).sha256,b1ClaimSha256:ref(e.claim).sha256,provider:{batch:'B1',runID:run,sourceSha256:ref(e.source).sha256}});json(e.ledger,ledger);
  const abs={schemaVersion:1,kind:'ACTUAL_OWNED_TERMINAL_PID_ABSENCE',atUtc:iso(4001),sourceOwnedSnapshot:ref(e.initialOwned),rows:[100,101,105].map(pid=>({pid,absent:true})),allAbsent:true,zeroLiveOwnedGameProcesses:true,originalCallerSession:777,originalCallerTerminalReaped:true};json(e.pidAbsence,abs);
  const sourceFrame={schemaVersion:1,kind:'PRIVATE_FIXED_LATEST_SOURCE_CAPTURE',scenario:s,sourceHead:p.source.qualifiedHead,label:'B1-PLAY',runID:run,callLabel:'call-004',captureLabel:'capture-001',controller:ref(e.controller),source:ref(e.source),claim:ref(e.claim),owned:ref(e.initialOwned),
   post:ref(f.calls[3].post.path),attempt:ref(f.calls[3].attempt.path),raw:ref(f.calls[3].raw.path),captureRaw:original.raw,automatic:original.metadata,frame:ref(e.beforeFrame),closedFrame:frame,window,ownedSnapshot:ref(e.frameOwned),rootClock:campaign.rootClock,inspectedAtMs:inspected,inputGranted:false};json(e.sourceFrame,sourceFrame);
  const step={command:child.program,args:child.args,cwd:'.',module:'node:.',processOwnership:'windows-job-object',status:0,timedOut:false,stdout:JSON.stringify(value(e.geometry)),stderr:''};
  const report={schemaVersion:1,command:'verify',ok:true,result:{verdict:'PASS',steps:[step]}};
  json(e.geometryTerminal,{schemaVersion:1,kind:'ACTUAL_OWNED_GEOMETRY_COMMAND',sourceHead:p.source.qualifiedHead,scenario:s,label:'B1-PLAY',sourceCapture:ref(e.sourceFrame),controller:ref(e.controller),sourceFrame:ref(e.beforeFrame),program:supervisor,args:['verify',geometryProject,'--scope','module','--module','node:.','--concurrency','1','--timeout','5000'],cwd:geometryProject,childCommand:child,configuration:ref(configuration),startedAtMs:f.anchor+1900,completedAtMs:f.anchor+2200,childTimeoutMs:5000,cleanupAllowanceMs:10000,outerTimeoutMs:15000,status:0,signal:null,error:null,stdout:JSON.stringify(report)+'\n',stderr:'',terminalReaped:true,outputComplete:true,supervisorJSON:report,rootClock:campaign.rootClock,providerClock:ledger.providerClock,closedProvider:null,geometry:ref(e.geometry),inputGranted:false});
  json(e.point,{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_CLIENT_POINT',label:'B1-PLAY',frame,geometry:ref(e.geometry),terminal:ref(e.geometryTerminal),sourceCapture:ref(e.sourceFrame),ownedSnapshot:ref(e.frameOwned),localPoint:{x:1,y:1},coordinate:[1,1],inspectedAtMs:inspected,computedAtMs:computed,controllerSha256:ref(e.controller).sha256,inputGranted:false});
  json(e.envelope,{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_POINT_CANONICAL_BINDING',point:ref(e.point),canonical:ref(e.sourceFrame),frame,inputGranted:false});
  const o=value(e.observation);o.providerFinish.raw.content[0].text=JSON.stringify(value(e.providerTerminal));json(e.observation,o);
  const r=value(e.result),paths=[e.plan,e.lease,e.controller,e.source,e.claim,e.ledger,e.observation,e.metadataCaller,e.geometry,e.geometryTerminal,e.point,e.envelope,e.sourceFrame,callerRaw,e.caller,e.launch,e.protection,e.pidAbsence,progress,original.png.path,original.metadata.path,original.raw.path,e.providerReady,e.providerTerminal,e.providerSpool,e.providerStderr,...f.calls.map((/** @type {any} */ x)=>x.post.path)];r.evidence=Object.fromEntries(paths.map(p=>[basename(p),ref(p)]));json(e.result,r);
  f.closeout.kind=callerDateKind;f.closeout.evidence=Object.fromEntries(Object.entries(e).map(([k,p])=>[k,ref(/** @type {string} */(p))]));json(f.closeoutPath,f.closeout);
  json(f.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',historyBaseline:{...f.guard,...f.context},maxReservedActions:f.cap,previous:{result:ref(f.closeoutPath),ledger:ref(e.ledger)}});
  for(const a of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){a.historyBaseline={...f.guard,...f.context};a.absoluteMaxReservedActions=f.cap;a.historyAdmission=ref(f.grantPath);}f.campaign.historyBaseline={...f.guard,...f.context};f.campaign.maxReservedActions=f.cap;f.predecessor.historyBaseline={...f.guard,...f.context};f.predecessor.maxReservedActions=f.cap;f.refreshLaunch();
 }
 refresh();return {...f,evidencePaths:e,refresh,oldObservation};
}
