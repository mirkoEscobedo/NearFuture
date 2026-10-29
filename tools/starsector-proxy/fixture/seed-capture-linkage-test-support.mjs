// @ts-check
// Generated historical boundary data only; no actual desktop, screenshot or authority.
import {rmSync} from 'node:fs';
import {resolve,basename} from 'node:path';
import {crc32,deflateSync} from 'node:zlib';
import {failedSeedHistoryFixture,ref,value} from './seed-failed-history-test-support.mjs';
import {json,write} from './seed-test-support.mjs';
import {persistDirectFrame} from '../direct-frames.mjs';

/** @param {string} type @param {Buffer} data */
function chunk(type,data){
 const bytes=Buffer.alloc(data.length+12);bytes.writeUInt32BE(data.length);bytes.write(type,4,'ascii');
 data.copy(bytes,8);bytes.writeUInt32BE(crc32(bytes.subarray(4,-4)),bytes.length-4);return bytes;
}
const png=Buffer.concat([Buffer.from('89504e470d0a1a0a','hex'),
 chunk('IHDR',Buffer.from('00000002000000020806000000','hex')),
 chunk('IDAT',deflateSync(Buffer.alloc(18))),chunk('IEND',Buffer.alloc(0))]);
const category='PRIVATE_CAPTURE_LINKAGE_REFUSAL_BEFORE_INPUT';
const message='Actual automatic/raw/source linkage required';

/** Extend the generated zero-input fixture, preserving its real public PS/JS entry points. */
/** @param {{initialJobPids?:number[],changePhaseLease?:(v:any)=>void,changeRootCommand?:(v:string)=>string,watchdogMs?:number}} [options] */
export function captureLinkageHistoryFixture(options={}){
 const f=/** @type {any} */(failedSeedHistoryFixture({watchdogMs:options.watchdogMs})),e=f.evidencePaths,scenario=f.priorScenario;
 const guard={overallAttempts:45,uiAttempts:42,reservedActions:76,sentActions:74};
 const context={overallAttempts:46,uiAttempts:43},future={...guard,...context},cap=84;
 const watchdog=options.watchdogMs??480000,deadline=watchdog-60000;
 const anchor=Date.now()-10000,iso=(/** @type {number} */ offset)=>new Date(anchor+offset).toISOString();
 const runID=value(e.ledger).provider.runID,output=resolve(f.root,'.tmp/starsector-proxy',runID);
 const helper=resolve(scenario,'construct-actual-phase-inputs.private.mjs');
 // This harmless fixture text is hashed as evidence, never imported or executed.
 write(helper,"// GENERATED HISTORICAL DATA ONLY; NEVER EXECUTE\n"+
  "const selectedCapture=(m,raw)=>{if(!(m.rawPath===raw.path))throw Error('Actual automatic/raw/source linkage required');};\n");
 e.helper=helper;e.metadataCaller=resolve(scenario,'driver-metadata-calls.actual.private.json');
 const privatePins=Object.fromEntries(['launch-owned-caller.private.ps1','record-root-call-and-stage-B1.private.ps1','launch-and-stage.orchestration.proposed.private.js'].map(name=>{const path=resolve(scenario,name);write(path,'// GENERATED INERT HISTORICAL DATA; NEVER EXECUTE\n');return [name,ref(path)];}));
 privatePins['construct-actual-phase-inputs.private.mjs']=ref(helper);
 const history=resolve(scenario,'root-history-baseline.admitted.private.json'),binding=resolve(scenario,'fixture-binding.private.json');
 json(history,{kind:'GENERATED_PRIOR_HISTORY_DATA_ONLY'});json(binding,{kind:'GENERATED_PRIOR_BINDING_DATA_ONLY'});
 const p=value(e.plan);Object.assign(p,{historyBaseline:{...guard},absoluteMaxReservedActions:cap,
  callerAdmissionDeadlineMs:anchor+deadline,sessionWatchdogMs:watchdog,
  historyAdmission:ref(history),fixtureBinding:ref(binding),rootCallContract:{key:value(e.rootCall).key,path:e.rootCall},
  preclockHelpers:privatePins,driverMechanicalScope:{metadataHelper:ref(helper)}});json(e.plan,p);
 const lease=value(e.lease);Object.assign(lease,{planSha256:ref(e.plan).sha256,
  absoluteMaxReservedActions:cap,callerAdmissionDeadlineMs:anchor+deadline,sessionWatchdogMs:watchdog,historyAdmission:p.historyAdmission});json(e.lease,lease);
 const c=value(e.controller);Object.assign(c,{at:iso(100),controllerCreationTime:iso(100),sessionId:1,
  rootLeaseSha256:ref(e.lease).sha256,conservativeOuterDeadlineUtc:iso(watchdog),controllerDeadlineUtc:iso(watchdog)});json(e.controller,c);
 const delegation=resolve(scenario,'root-postlaunch-mechanical-delegation.private.json');
 json(delegation,{schemaVersion:1,kind:'ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION',issuer:'/root',driver:'/root/recovery_game_driver',planSha256:ref(e.plan).sha256,leaseSha256:ref(e.lease).sha256,helperSha256:ref(helper).sha256,callerSha256:privatePins['launch-owned-caller.private.ps1'].sha256,stagerSha256:privatePins['record-root-call-and-stage-B1.private.ps1'].sha256,orchestrationSha256:privatePins['launch-and-stage.orchestration.proposed.private.js'].sha256,historyAdmissionSha256:p.historyAdmission.sha256,allowedModes:['init','phase B1']});
 const rootCall=value(e.rootCall),ps=value(e.providerReady).helper.executable,q=(/** @type {string} */ s)=>"'"+s.replaceAll("'","''")+"'";
 rootCall.value.startMs=anchor;rootCall.value.args.cmd='& '+q(ps)+' -NoProfile -NonInteractive -File '+q(privatePins['launch-owned-caller.private.ps1'].path)+' -PlanSha256 '+q(ref(e.plan).sha256)+' -LeaseSha256 '+q(ref(e.lease).sha256)+' -DelegationSha256 '+q(ref(delegation).sha256)+' -HelperSha256 '+q(ref(helper).sha256)+' -HistoryAdmissionSha256 '+q(p.historyAdmission.sha256);
 if(options.changeRootCommand)rootCall.value.args.cmd=options.changeRootCommand(rootCall.value.args.cmd);json(e.rootCall,rootCall);
 const launch=value(e.launch);Object.assign(launch.result.steps[0],{command:ps,cwd:'.',args:['-NoProfile','-NonInteractive','-File',resolve(f.root,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',scenario,'-BindingPath',binding,'-BindingSha256',p.fixtureBinding.sha256,'-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs',String(watchdog)]});json(e.launch,launch);
 const owner={pid:101,parentPid:105,creationTime:iso(500),kernelCreationFileTime:'134371266963297183',
  inSupervisorJob:true,sessionId:1,executable:resolve(scenario,'installation/jre/bin/java.exe'),
  supervisorJobPids:[100,105,101],mainWindowHandle:19};
 const initialOwned=resolve(scenario,'driver-B1-owned.private.json');
 const owned={at:iso(600),controllerPid:c.controllerPid,controllerCreationTime:c.controllerCreationTime,
  controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,processes:[owner]};
 json(initialOwned,{...owned,processes:[{...owner,supervisorJobPids:options.initialJobPids??owner.supervisorJobPids}]});json(e.owned,{...owned,at:iso(2900)});
 const producerRecordKey='controller-start:'+ref(e.controller).sha256;
 const campaign={...structuredClone(f.campaign),windowId:19,historyBaseline:{...guard},maxReservedActions:cap,
  rootCallRecord:rootCall,rootClock:{...f.campaign.rootClock,recordKey:rootCall.key,producerRecordKey,
   startMs:anchor,deadlineMs:anchor+watchdog,effectiveDeadlineMs:anchor+deadline,
   callerAdmissionDeadlineMs:anchor+deadline,originalCallerAdmissionDeadlineMs:anchor+deadline},
  producerWitness:{...f.campaign.producerWitness,producerRecordKey,controllerPid:c.controllerPid,
   controllerCreationTime:c.controllerCreationTime,controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,
   conservativeOuterDeadlineUtc:c.conservativeOuterDeadlineUtc,controllerDeadlineUtc:c.controllerDeadlineUtc,
   effectiveDeadlineMs:anchor+deadline,sessionWatchdogMs:watchdog}};
 const predecessor=resolve(scenario,'campaign-postlaunch.private.json');
 json(predecessor,{...structuredClone(f.predecessor),plan:ref(e.plan),lease:ref(e.lease),controller:ref(e.controller),
  rootClock:campaign.rootClock,producerWitness:campaign.producerWitness,rootCallRecord:rootCall,
  historyBaseline:{...guard},maxReservedActions:cap});campaign.launchPredecessor=ref(predecessor);
 const phasePlan=resolve(scenario,'B1-plan.private.json'),phaseLease=resolve(scenario,'B1-lease.private.json');
 json(phasePlan,{...p,campaign,batches:[{id:'B1'}]});
 const phaseLeaseValue={...lease,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,planSha256:ref(phasePlan).sha256};
 options.changePhaseLease?.(phaseLeaseValue);json(phaseLease,phaseLeaseValue);
 json(e.source,{schemaVersion:6,scenario,batch:'B1',planPath:phasePlan,planSha256:ref(phasePlan).sha256,
  leasePath:phaseLease,leaseSha256:ref(phaseLease).sha256,ownedReceiptPath:initialOwned,ownedReceiptSha256:ref(initialOwned).sha256});
 const claim=value(e.claim);Object.assign(claim,{sourcePath:e.source,sourceSha256:ref(e.source).sha256});
 Object.assign(claim.binding,{schemaVersion:6,batch:'B1',scenario,workspaceRoot:f.root,runID,sourcePath:e.source,
  sourceSha256:ref(e.source).sha256,planSha256:ref(phasePlan).sha256,originalPlanSha256:ref(e.plan).sha256,
  controllerSha256:ref(e.controller).sha256,ownedPids:[101],campaign});json(e.claim,claim);
 const ready=value(e.providerReady);Object.assign(ready,{at:iso(1000),sourceSha256:ref(e.source).sha256});
 ready.node.creationTime=iso(980);ready.helper.creationTime=iso(990);json(e.providerReady,ready);
 const terminal=value(e.providerTerminal);Object.assign(terminal,{at:iso(3000),deadlineMs:anchor+181000});json(e.providerTerminal,terminal);
 const protection=value(e.protection);protection.at=iso(4000);json(e.protection,protection);
 const absence=value(e.pidAbsence);absence.atUtc=iso(4001);json(e.pidAbsence,absence);
 const ledger=value(e.ledger);Object.assign(ledger,{history:{...guard},pending:null,uncertain:false,
  controllerSha256:ref(e.controller).sha256,b1ClaimSha256:ref(e.claim).sha256,
  provider:{batch:'B1',runID,sourceSha256:ref(e.source).sha256},
  providerClock:{startMs:anchor+1000,deadlineMs:anchor+181000}});json(e.ledger,ledger);
 // Remove the old fifth call, not merely its observation reference.
 for(const suffix of ['.private.json','.attempt.private.json','.raw.private.json'])rmSync(resolve(output,'call-005'+suffix));
 const window={pid:101,windowId:19,bundleId:'java.exe',title:'Generated launcher',isFocused:true,isOnScreen:true,
  bounds:{x:0,y:0,width:2,height:2}};
 const rows=[['doctor',{include_remediation:true}],['policy_status',{}],['get_window',{window_id:19}],
  ['screenshot',{target_window_id:19,quality:0,width:2}]];
 /** @type {any[]} */const calls=[];/** @type {any} */let original;
 for(const [i,row] of rows.entries()){
  const tool=/** @type {string} */(row[0]),args=row[1],ordinal=i+1;
  const post={tool,args,outerProtocolID:ordinal+9,innerProtocolID:ordinal,
   callStarted:iso(1100+i*50),callCompleted:iso(1120+i*50),precedingActions:[],runID,batch:'B1',freshness:'UNKNOWN',isError:false};
  const prefix=resolve(output,'call-'+String(ordinal).padStart(3,'0'));json(prefix+'.private.json',post);
  json(prefix+'.attempt.private.json',{tool,args,outerProtocolID:post.outerProtocolID,innerProtocolID:'UNKNOWN',
   callStarted:post.callStarted,sourceSha256:ref(e.source).sha256,runID,batch:'B1'});
  const raw=tool==='doctor'?{structuredContent:{ok:true,summary:{failed:0}}}
   :tool==='policy_status'?{structuredContent:{approval_required_for_all:true,approval_token_configured:false,audit:{enabled:true},profile:'core'}}
   :tool==='get_window'?{structuredContent:window}:{content:[{type:'image',mimeType:'image/png',data:png.toString('base64')}]};
  // Byte-identical serialization to persistDirectFrame's automatic raw, at a DISTINCT path.
  write(prefix+'.raw.private.json',JSON.stringify(raw)+'\n');
  calls.push({post:ref(prefix+'.private.json'),raw:ref(prefix+'.raw.private.json'),attempt:ref(prefix+'.attempt.private.json')});
  if(tool==='screenshot'){
   const {isError,...provenance}=post;
   const frame=persistDirectFrame({output,label:'capture-001',result:raw,args,window,ownedPids:[101],provenance});
   if(!frame.pngPath)throw Error('Generated fixture must persist a complete original');
   original={png:ref(frame.pngPath),metadata:ref(frame.metadataPath),raw:ref(frame.rawPath),originalInspected:true,
    nativeDimensions:[2,2],visibleAssertion:'Launcher Play visible',freshness:'UNKNOWN',windowId:19,pid:101,isFocused:true};
  }
 }
 if(!original)throw Error('Generated screenshot original required');
 const node=ready.node.executable,inspectionMs=anchor+1400;
 // Recorded command DATA only. These strings are never evaluated by this fixture.
 const cmd="$ErrorActionPreference='Stop';$h='"+helper.replaceAll('\\','/').replaceAll("'","''")+"';$n='"+node.replaceAll('\\','/').replaceAll("'","''")+"';"+
  '$gRaw=& $n $h geometry B1-PLAY '+runID+' call-004 capture-001 '+inspectionMs+";if($LASTEXITCODE-ne0){throw 'Geometry failed; no mapping/input'};$g=$gRaw|ConvertFrom-Json;"+
  '$mRaw=& $n $h play-point '+runID+' call-004 capture-001 1 1 '+inspectionMs+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'Play mapping failed; no input'};$m=$mRaw|ConvertFrom-Json;"+
  "$cRaw=& $n $h point-check B1-PLAY $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw 'Point-check failed; no input'};$c=$cRaw|ConvertFrom-Json;"+
  '$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json;[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress';
 const metadataCaller=[{label:'B1-PLAY',args:{cmd,sandbox_permissions:'require_escalated',
  justification:'Generated historical command data; never executed',yield_time_ms:1000,max_output_tokens:2300},
  raw:{chunk_id:'generated-helper-failure',wall_time_seconds:0.1,exit_code:1,original_token_count:50,
   output:'Error: '+message+'\n    at selectedCapture ('+helper+':2:1)\nGeometry failed; no mapping/input\n'},polls:[]}];json(e.metadataCaller,metadataCaller);
 const observation={schemaVersion:1,kind:'ACTUAL_DRIVER_PRE_INPUT_CAPTURE_LINKAGE_STOP',scenarioId:basename(scenario),
  driver:'/root/recovery_game_driver',atUtc:iso(3300),sourceHead:p.source.qualifiedHead,verdict:'FAIL',failureCategory:category,
  actualRunContext:context,actualGuardHistory:guard,actions:{clicks:0,activations:0,reservations:0,pending:null,uncertain:false},
  B1:{runId:runID,calls,ready:ref(e.providerReady),terminal:ref(e.providerTerminal),original},
  helperInvocation:{modeAttempted:'geometry B1-PLAY',toolChunkId:'generated-helper-failure',status:1,message,
   actualCallerPacket:ref(e.metadataCaller),helper:ref(helper),geometryChildLaunched:false,mappingAttempted:false,
   pointCheckAttempted:false,nativeInputAttempted:false,timingFailure:false},
  smallestReadOnlyMismatch:{predicate:'m.rawPath===raw.path',automaticMetadataRawPath:original.raw.path,
   selectedCallRawPath:calls[3].raw.path,automaticRaw:original.raw,callRaw:calls[3].raw,
   noSourceOrHelperChange:true,noRetryOrTimeoutChange:true},E1:'NOT_STARTED',E2:'NOT_STARTED',
  ledger:ref(e.ledger),rootCall:ref(e.rootCall),seedClaim:false,cleanup:'ROOT_OWNS_PENDING_ACTUAL_TERMINAL_PROTECTION',allPriorFailuresRetained:true};
 json(e.observation,observation);
 const r=value(e.result);delete r.actions;delete r.providers;
 Object.assign(r,{sealedAtUtc:iso(5000),failureCategory:category,reason:message,inheritedGuardBaseline:guard,
  actualGuardHistory:guard,actualRunContext:context,absoluteMaxReservedActions:cap,
  counts:{reservedThisRun:0,sentThisRun:0,clicks:0,activations:0,providers:1,vendorCalls:4,originals:1,
   geometryModeAttempts:1,geometryChildrenLaunched:0,mappingModes:0,pointChecks:0},
  providerStatus:{B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED'},
  original:{originalInspected:true,dimensions:[2,2],pngSha256:original.png.sha256,visibleAssertion:original.visibleAssertion,
   isFocused:true,windowId:19,pid:101,freshness:'UNKNOWN',inputSent:false},
  helperFailure:{mode:'geometry B1-PLAY',actualToolChunk:'generated-helper-failure',actualExitCode:1,message,
   predicate:'m.rawPath===raw.path',automaticRawBasename:'capture-001.raw.private.json',callRawBasename:'call-004.raw.private.json',
   bothRawSha256:original.raw.sha256,geometryChildLaunched:false,timingFailure:false,noRetryOrTimeoutChange:true,noCauseFixedClaim:true},
  cleanup:{originalSession:777,actualCallerTerminalReaped:true,rawLaunchVerdict:'FAIL',rawLaunchStatus:1,
   rawLaunchTimedOut:false,ordinaryExit:false},protection,
  seed:{created:false,modernCloseoutCreated:false,stabilityModeExecuted:false,semanticAcceptance:'NOT_RUN'},
  allPriorFailuresAndAllowancesRetained:true,noNewHistoryKindGrantRetryOrAuthority:true});json(e.result,r);
 const closeout=f.closeout;Object.assign(closeout,{kind:'ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY',
  actions:{actualGuard:guard,currentRunContext:context,absoluteMaxReservedActions:cap}});
 function refresh(){
  const o=value(e.observation);o.ledger=ref(e.ledger);o.rootCall=ref(e.rootCall);
  o.B1.ready=ref(e.providerReady);o.B1.terminal=ref(e.providerTerminal);
  o.helperInvocation.actualCallerPacket=ref(e.metadataCaller);o.helperInvocation.helper=ref(helper);json(e.observation,o);
  const result=value(e.result);result.evidence=Object.fromEntries(Object.entries({observation:e.observation,
   callerTerminal:e.caller,launchSupervisor:e.launch,protection:e.protection,pidAbsence:e.pidAbsence,
   ledger:e.ledger,plan:e.plan,lease:e.lease,actualMetadataCalls:e.metadataCaller}).map(([k,path])=>[k,ref(path)]));json(e.result,result);
  closeout.evidence=Object.fromEntries(Object.entries(e).map(([k,path])=>[k,ref(/** @type {string} */(path))]));json(f.closeoutPath,closeout);
  json(f.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',
   historyBaseline:future,maxReservedActions:cap,previous:{result:ref(f.closeoutPath),ledger:ref(e.ledger)}});
  for(const authority of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){
   authority.historyBaseline={...future};authority.absoluteMaxReservedActions=cap;authority.historyAdmission=ref(f.grantPath);
  }
  f.campaign.historyBaseline={...future};f.campaign.maxReservedActions=cap;
  f.predecessor.historyBaseline={...future};f.predecessor.maxReservedActions=cap;f.refreshLaunch();
 }
 refresh();return {...f,closeout,evidencePaths:e,refresh,calls,original,guard,context,cap,anchor,metadataCaller};
}
