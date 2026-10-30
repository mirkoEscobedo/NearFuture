// @ts-check
// Read-only custody of the four-call, pre-input capture-path refusal.
import {resolve} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,file} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';

/** @param {{v:any,a:any,e:any,output:string,pinned:(p:any,s:string,n?:number)=>any,fail:(v:any)=>void,node:string,powershell:string}} context */
export function assertCaptureLinkageFailure({v,a,e,output,pinned,fail,node,powershell}){
 const {result:r,ledger:l,plan:p,controller:c,observation:o,claim,providerReady:ready,providerTerminal:terminal}=a;
 const category='PRIVATE_CAPTURE_LINKAGE_REFUSAL_BEFORE_INPUT',before=p.historyBaseline,owner=claim.binding;
 fail([terminal.steps,a.providerSpool.result?.steps,a.launch.result?.steps].every(Array.isArray));
 /** @param {any} x */const millis=x=>{fail(typeof x==='string'&&Number.isSafeInteger(Date.parse(x)));return Date.parse(x);};
 /** @param {any} n */const integer=n=>fail(Number.isSafeInteger(n)&&n>0);
 fail(closed(before,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(before).every(n=>Number.isSafeInteger(n)&&n>=0)&&
  before.sentActions<=before.reservedActions&&same(l.history,before)&&l.pending===null&&l.uncertain===false&&
  same(v.actions.currentRunContext,{overallAttempts:before.overallAttempts+1,uiAttempts:before.uiAttempts+1})&&
  same(r.counts,{reservedThisRun:0,sentThisRun:0,clicks:0,activations:0,providers:1,vendorCalls:4,originals:1,geometryModeAttempts:1,geometryChildrenLaunched:0,mappingModes:0,pointChecks:0})&&
  same(r.providerStatus,{B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED'})&&
  same(r.seed,{created:false,modernCloseoutCreated:false,stabilityModeExecuted:false,semanticAcceptance:'NOT_RUN'}));
 fail(closed(o,['schemaVersion','kind','scenarioId','driver','atUtc','sourceHead','verdict','failureCategory','actualRunContext','actualGuardHistory','actions','B1','helperInvocation','smallestReadOnlyMismatch','E1','E2','ledger','rootCall','seedClaim','cleanup','allPriorFailuresRetained'])&&
  o.schemaVersion===1&&o.kind==='ACTUAL_DRIVER_PRE_INPUT_CAPTURE_LINKAGE_STOP'&&o.failureCategory===category&&o.verdict==='FAIL'&&
  o.scenarioId===p.scenarioId&&o.driver===v.driver&&o.sourceHead===v.sourceHead&&same(o.actualRunContext,r.actualRunContext)&&same(o.actualGuardHistory,before)&&
  same(o.actions,{clicks:0,activations:0,reservations:0,pending:null,uncertain:false})&&o.E1==='NOT_STARTED'&&o.E2==='NOT_STARTED'&&
  o.seedClaim===false&&o.allPriorFailuresRetained===true&&o.cleanup==='ROOT_OWNS_PENDING_ACTUAL_TERMINAL_PROTECTION'&&same(o.ledger,e.ledger)&&same(o.rootCall,e.rootCall)&&
  closed(o.B1,['runId','calls','ready','terminal','original'])&&o.B1.runId===ready.runID&&same(o.B1.ready,e.providerReady)&&same(o.B1.terminal,e.providerTerminal));
 const source=a.source;
 fail(closed(source,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256']));
 const phase=pinned({path:source.planPath,sha256:source.planSha256},resolve(v.scenario,'B1-plan.private.json'));
 const phaseLease=pinned({path:source.leasePath,sha256:source.leaseSha256},resolve(v.scenario,'B1-lease.private.json'));
 fail(same(phase.campaign,owner.campaign)&&same(phaseLease.campaign,owner.campaign)&&owner.planSha256===source.planSha256&&owner.sourcePath===e.source.path);
 fail(phase.scenarioId===p.scenarioId&&phase.paths?.scenario===v.scenario&&phase.source?.qualifiedHead===v.sourceHead&&
  same(phase.historyBaseline,before)&&phase.absoluteMaxReservedActions===p.absoluteMaxReservedActions&&same(phase.seedCreation,p.seedCreation)&&
  same(owner.campaign.historyBaseline,before)&&owner.campaign.maxReservedActions===p.absoluteMaxReservedActions&&same(owner.campaign.rootCallRecord,a.rootCall)&&
  phaseLease.schemaVersion===6&&phaseLease.authorized===true&&phaseLease.issuer==='/root'&&phaseLease.driver===v.driver&&phaseLease.phase==='DISPOSABLE_SEED_CREATION'&&
  phaseLease.planSha256===source.planSha256&&phaseLease.maxActions===8&&phaseLease.maxProxyInvocations===3&&phaseLease.ownedGameHardTimeoutMs===480000&&
  phaseLease.campaignLoadSaveAllowed===false&&phaseLease.supportPinsPath===e.supportPins.path&&phaseLease.supportPinsSha256?.toLowerCase()===e.supportPins.sha256&&
  phaseLease.fixtureReceiptSha256===a.lease.fixtureReceiptSha256&&same(phaseLease.seedCreation,p.seedCreation)&&phaseLease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs);
 const clock=owner.campaign.rootClock,birth=millis(c.controllerCreationTime),published=millis(c.at),outer=millis(c.conservativeOuterDeadlineUtc),end=millis(c.controllerDeadlineUtc);
 fail([clock.startMs,clock.deadlineMs,clock.effectiveDeadlineMs,p.callerAdmissionDeadlineMs,l.providerClock?.startMs,l.providerClock?.deadlineMs].every(Number.isSafeInteger)&&
  clock.startMs===a.rootCall.value.startMs&&clock.recordKey===a.rootCall.key&&clock.deadlineMs===clock.startMs+480000&&clock.sessionWatchdogMs===480000&&clock.cleanupReserveMs===30000&&
  clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.originalCallerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&a.lease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&
  clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,p.callerAdmissionDeadlineMs,outer,end)&&clock.startMs<=birth&&birth<=published&&published<end&&end<=outer&&outer<=birth+480000&&
  l.providerClock.startMs>=published&&l.providerClock.startMs>=clock.startMs&&l.providerClock.deadlineMs===Math.min(l.providerClock.startMs+180000,clock.effectiveDeadlineMs-30000)&&
  millis(ready.at)>=l.providerClock.startMs&&millis(terminal.at)<=l.providerClock.deadlineMs);
 const producer=owner.campaign.producerWitness;
 fail(clock.producerRecordKey==='controller-start:'+e.controller.sha256&&producer.producerRecordKey===clock.producerRecordKey&&
  producer.controllerPid===c.controllerPid&&producer.controllerCreationTime===c.controllerCreationTime&&producer.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&
  producer.conservativeOuterDeadlineUtc===c.conservativeOuterDeadlineUtc&&producer.controllerDeadlineUtc===c.controllerDeadlineUtc&&producer.effectiveDeadlineMs===clock.effectiveDeadlineMs&&producer.sessionWatchdogMs===480000);
 for(const child of [ready.node,ready.helper]){integer(child.pid);integer(child.parentPid);fail(millis(child.creationTime)>=published&&millis(child.creationTime)<=millis(ready.at));}
 const tools=['doctor','policy_status','get_window','screenshot'],posts=[],raws=[];
 fail(Array.isArray(o.B1.calls)&&o.B1.calls.length===4&&same(readdirSync(output).filter(n=>/^call-\d+\.attempt\.private\.json$/.test(n)).sort(),tools.map((_,i)=>'call-'+String(i+1).padStart(3,'0')+'.attempt.private.json'))&&
  !readdirSync(v.scenario).some(n=>/^(?:guard-E[12]-consumed|E[12]-authorization)\.private\.json$/.test(n)||/^B1-PLAY-(owned-snapshot|before-frame|source-before|client-point|client-geometry|coordinate-proof|point|geometry)/.test(n)||n==='geometry-owned-B1-PLAY'));
 let previous=millis(ready.at);
 for(const [i,tool]of tools.entries()){
  const row=o.B1.calls[i],prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));fail(closed(row,['post','raw','attempt']));
  const post=pinned(row.post,prefix+'.private.json'),attempt=pinned(row.attempt,prefix+'.attempt.private.json'),raw=pinned(row.raw,prefix+'.raw.private.json',32*1024*1024);
  fail(post.tool===tool&&attempt.tool===tool&&post.runID===ready.runID&&attempt.runID===ready.runID&&post.batch==='B1'&&attempt.batch==='B1'&&
   attempt.sourceSha256===e.source.sha256&&same(post.args,attempt.args)&&attempt.callStarted===post.callStarted&&same(post.precedingActions,[])&&post.isError===false&&(!Object.hasOwn(raw,'isError')||raw.isError===false)&&
   Number.isSafeInteger(post.outerProtocolID)&&post.outerProtocolID>0&&post.innerProtocolID===i+1&&attempt.outerProtocolID===post.outerProtocolID&&attempt.innerProtocolID==='UNKNOWN'&&post.freshness==='UNKNOWN'&&
   millis(post.callStarted)>=previous&&millis(post.callCompleted)>=millis(post.callStarted)&&millis(post.callCompleted)<=millis(terminal.at));
  previous=millis(post.callCompleted);posts.push(post);raws.push(raw);
 }
 fail(same(posts[0].args,{include_remediation:true})&&same(posts[1].args,{})&&raws[0].structuredContent?.ok===true&&
  raws[1].structuredContent?.approval_required_for_all===true&&raws[1].structuredContent.approval_token_configured===false);
 const window=raws[2].structuredContent;integer(window?.pid);integer(window?.windowId);integer(c.sessionId);
 fail(window.bundleId==='java.exe'&&window.isFocused===true&&closed(window.bounds,['x','y','width','height'])&&Object.values(window.bounds).every(Number.isSafeInteger)&&window.bounds.width>0&&window.bounds.height>0&&
  same(posts[2].args,{window_id:window.windowId})&&same(posts[3].args,{target_window_id:window.windowId,quality:0,width:window.bounds.width})&&owner.campaign.windowId===window.windowId);
 const initial=pinned({path:source.ownedReceiptPath,sha256:source.ownedReceiptSha256},resolve(v.scenario,'driver-B1-owned.private.json'));
 const games=[initial,a.owned].map(snapshot=>snapshot.processes?.filter((/** @type {any} */ row)=>row.executable===resolve(c.installation,'jre/bin/java.exe')));
 fail(games.every(rows=>rows?.length===1));const [first,last]=games.map(rows=>rows[0]);
 integer(first.pid);integer(first.parentPid);
 fail(typeof first.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(first.kernelCreationFileTime)&&first.pid===window.pid&&first.executable===resolve(c.installation,'jre/bin/java.exe')&&first.sessionId===c.sessionId&&millis(first.creationTime)>=published&&millis(first.creationTime)<=millis(ready.at));
 for(const row of [first,last])fail(['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(k=>same(row[k],first[k]))&&row.inSupervisorJob===true&&
  Array.isArray(row.supervisorJobPids)&&row.supervisorJobPids.every((/** @type {any} */ id)=>Number.isSafeInteger(id)&&id>0)&&[first.pid,c.controllerPid,...c.supervisorJobPids].every(id=>row.supervisorJobPids.includes(id))&&
  (Number.isSafeInteger(row.mainWindowHandle)||(typeof row.mainWindowHandle==='string'&&/^[1-9][0-9]*$/.test(row.mainWindowHandle)))&&Number(row.mainWindowHandle)===window.windowId);
 const original=o.B1.original;
 fail(closed(original,['png','metadata','raw','originalInspected','nativeDimensions','visibleAssertion','freshness','windowId','pid','isFocused'])&&original.originalInspected===true&&original.freshness==='UNKNOWN'&&original.isFocused===true&&original.windowId===window.windowId&&original.pid===window.pid&&original.visibleAssertion==='Launcher Play visible');
 const prefix=resolve(output,'capture-001'),m=pinned(original.metadata,prefix+'.metadata.private.json'),raw=pinned(original.raw,prefix+'.raw.private.json',32*1024*1024);
 fail(closed(original.png,['path','sha256'])&&original.png.path===prefix+'.png');const png=file(original.png.path,32*1024*1024),callRaw=file(o.B1.calls[3].raw.path,32*1024*1024),captureRaw=file(original.raw.path,32*1024*1024);
 const dimensions=completePng(png.bytes),images=raw.content?.filter((/** @type {any} */ x)=>x.type==='image');
 fail(png.sha256===original.png.sha256&&captureRaw.bytes.equals(callRaw.bytes)&&same(raw,raws[3])&&images?.length===1&&images[0].mimeType==='image/png'&&images[0].data===png.bytes.toString('base64')&&
  m.captureReceipt===true&&m.freshness==='UNKNOWN'&&m.rawPath===original.raw.path&&m.rawSha256===original.raw.sha256&&m.pngPath===original.png.path&&m.pngSha256===original.png.sha256&&
  same(m.window,window)&&same(m.dimensions,dimensions)&&same(original.nativeDimensions,[dimensions.width,dimensions.height])&&dimensions.width===window.bounds.width&&dimensions.height===window.bounds.height);
 for(const key of ['tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID'])fail(same(m[key],posts[3][key]));
 fail(same(r.original,{originalInspected:true,dimensions:original.nativeDimensions,pngSha256:original.png.sha256,visibleAssertion:original.visibleAssertion,isFocused:true,windowId:window.windowId,pid:window.pid,freshness:'UNKNOWN',inputSent:false}));
 const mismatch=o.smallestReadOnlyMismatch;
 fail(same(mismatch,{predicate:'m.rawPath===raw.path',automaticMetadataRawPath:original.raw.path,selectedCallRawPath:o.B1.calls[3].raw.path,automaticRaw:original.raw,callRaw:o.B1.calls[3].raw,noSourceOrHelperChange:true,noRetryOrTimeoutChange:true}));
 fail(closed(e.helper,['path','sha256'])&&e.helper.path===resolve(v.scenario,'construct-actual-phase-inputs.private.mjs')&&same(p.driverMechanicalScope?.metadataHelper,e.helper));
 const helper=file(e.helper.path,262144);fail(helper.sha256===e.helper.sha256&&helper.bytes.toString('utf8').includes('m.rawPath===raw.path'));
 const packet=a.metadataCaller;fail(Array.isArray(packet)&&packet.length===1);const entry=packet[0];
 fail(closed(entry,['label','args','raw','polls'])&&entry.label==='B1-PLAY'&&same(entry.polls,[])&&entry.raw.exit_code===1&&!Object.hasOwn(entry.raw,'session_id')&&
  typeof entry.raw.output==='string'&&Buffer.byteLength(entry.raw.output)<=8192&&Number.isFinite(entry.raw.wall_time_seconds)&&entry.raw.wall_time_seconds>=0&&
  entry.raw.output.includes('Error: Actual automatic/raw/source linkage required')&&entry.raw.output.includes('at selectedCapture')&&entry.raw.output.includes('Geometry failed; no mapping/input'));
 const cmd=entry.args?.cmd;fail(typeof cmd==='string'&&cmd.length<=8192);
 const suffix=cmd.match(/ capture-001 ([0-9]+);if\(\$LASTEXITCODE-ne0\)/),point=cmd.match(/ capture-001 ([0-9]+) ([0-9]+) ([0-9]+) \$g.geometry.path/);fail(suffix&&point);
 const inspected=Number(suffix[1]);fail(Number.isSafeInteger(inspected)&&inspected>=previous&&inspected<=millis(terminal.at)&&point[3]===suffix[1]&&Number(point[1])<dimensions.width&&Number(point[2])<dimensions.height);
 const h=e.helper.path.replaceAll('\\','/'),n=node.replaceAll('\\','/'),run=ready.runID;
 const expected="$ErrorActionPreference='Stop';$h='"+h+"';$n='"+n+"';$gRaw=& $n $h geometry B1-PLAY "+run+" call-004 capture-001 "+inspected+";if($LASTEXITCODE-ne0){throw 'Geometry failed; no mapping/input'};$g=$gRaw|ConvertFrom-Json;$mRaw=& $n $h play-point "+run+" call-004 capture-001 "+point[1]+" "+point[2]+" "+inspected+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'Play mapping failed; no input'};$m=$mRaw|ConvertFrom-Json;$cRaw=& $n $h point-check B1-PLAY $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw 'Point-check failed; no input'};$c=$cRaw|ConvertFrom-Json;$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json;[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress";
 fail(cmd===expected&&same(o.helperInvocation,{modeAttempted:'geometry B1-PLAY',toolChunkId:entry.raw.chunk_id,status:1,message:'Actual automatic/raw/source linkage required',actualCallerPacket:e.metadataCaller,helper:e.helper,geometryChildLaunched:false,mappingAttempted:false,pointCheckAttempted:false,nativeInputAttempted:false,timingFailure:false})&&
  same(r.helperFailure,{mode:'geometry B1-PLAY',actualToolChunk:entry.raw.chunk_id,actualExitCode:1,message:'Actual automatic/raw/source linkage required',predicate:'m.rawPath===raw.path',automaticRawBasename:'capture-001.raw.private.json',callRawBasename:'call-004.raw.private.json',bothRawSha256:original.raw.sha256,geometryChildLaunched:false,timingFailure:false,noRetryOrTimeoutChange:true,noCauseFixedClaim:true})&&millis(o.atUtc)>=millis(terminal.at)&&millis(o.atUtc)<=millis(a.protection.at));
// PRIVATE PROPOSED SNIPPET ONLY. Insert in NEW linkage case after common custody.
// Requires existing v,a,e,p,c,fail,pinned,closed,file,resolve,same and powershell.
// Pass historical powershell from readTerminalFailure context; do not select current runtime.
/** @param {string} name */ function historicalPrivatePin(name){
 const pin=p.preclockHelpers?.[name],path=resolve(v.scenario,name);
 fail(closed(pin,['path','sha256'])&&pin.path===path&&typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256));
 fail(file(path,262144).sha256===pin.sha256);return pin;
}
const callerPin=historicalPrivatePin('launch-owned-caller.private.ps1');
const stagerPin=historicalPrivatePin('record-root-call-and-stage-B1.private.ps1');
const orchestrationPin=historicalPrivatePin('launch-and-stage.orchestration.proposed.private.js');
fail(same(historicalPrivatePin('construct-actual-phase-inputs.private.mjs'),e.helper));
const history=p.historyAdmission,binding=p.fixtureBinding;
for(const [pin,name]of [[history,'root-history-baseline.admitted.private.json'],[binding,'fixture-binding.private.json']]){
 fail(closed(pin,['path','sha256'])&&pin.path===resolve(v.scenario,name)&&typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256)&&file(pin.path,262144).sha256===pin.sha256);
}
fail(same(history,a.lease.historyAdmission)&&p.rootCallContract?.path===e.rootCall.path&&p.rootCallContract.key===a.rootCall.key);
const delegationPath=resolve(v.scenario,'root-postlaunch-mechanical-delegation.private.json');
const delegationPin={path:delegationPath,sha256:file(delegationPath,262144).sha256};
const d=pinned(delegationPin,delegationPath);
fail(d.schemaVersion===1&&d.kind==='ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'&&d.issuer==='/root'&&d.driver===v.driver&&
 d.planSha256===e.plan.sha256&&d.leaseSha256===e.lease.sha256&&d.helperSha256===e.helper.sha256&&
 d.callerSha256===callerPin.sha256&&d.stagerSha256===stagerPin.sha256&&d.orchestrationSha256===orchestrationPin.sha256&&
 d.historyAdmissionSha256===history.sha256&&same(d.allowedModes,['init','phase B1']));
const quote=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
const expectedRootCmd='& '+quote(powershell)+' -NoProfile -NonInteractive -File '+quote(callerPin.path)+
 ' -PlanSha256 '+quote(e.plan.sha256)+' -LeaseSha256 '+quote(e.lease.sha256)+
 ' -DelegationSha256 '+quote(delegationPin.sha256)+' -HelperSha256 '+quote(e.helper.sha256)+
 ' -HistoryAdmissionSha256 '+quote(history.sha256);
fail(typeof a.rootCall.value.args.cmd==='string'&&a.rootCall.value.args.cmd===expectedRootCmd);
const expectedOuterArgs=['-NoProfile','-NonInteractive','-File',
 resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),
 '-EvidenceRoot',v.scenario,'-BindingPath',binding.path,'-BindingSha256',binding.sha256,
 '-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000'];
const outerStep=a.launch.result.steps[0];
fail(outerStep.command===powershell&&same(outerStep.args,expectedOuterArgs)&&outerStep.cwd==='.');
// Bind the actual result's explicitly named metadata packet as well as observation.
fail(same(a.result.evidence?.actualMetadataCalls,e.metadataCaller));

}
