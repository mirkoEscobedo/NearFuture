// Closed snapshot-read failure custody only; never current input or seed authority.
// @ts-check
import {resolve,dirname,basename} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,file} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';
import {clientImagePoint} from './client-coordinate.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {callerDateNames,callerDateProviderNames,assertCallerDateIdentityShapes} from './seed-caller-date-attempt.mjs';
import {callerDatePacket} from './seed-caller-date-geometry.mjs';

export const snapshotBusyNames=Object.freeze({...callerDateNames});
export const snapshotBusyProviderNames=Object.freeze({...callerDateProviderNames});
const category='PRIVATE_POINT_CHECK_LIVE_SNAPSHOT_READ_EBUSY';
const object=(/** @type {any} */ v)=>v!==null&&typeof v==='object'&&!Array.isArray(v);
const note=(/** @type {any} */ v)=>typeof v==='string'&&v.length>0&&Buffer.byteLength(v)<=8192;
const positive=(/** @type {any} */ v)=>Number.isSafeInteger(v)&&v>0;
/** @param {any} v @param {(ok:any)=>void} fail */
function milliseconds(v,fail){fail(typeof v==='string'&&Number.isSafeInteger(Date.parse(v)));return Date.parse(v);}

/** Caller already ran strict byte pinning and common source/provider/caller/protection gates.
 * No historical geometry is returned as current input authority. @param {any} ctx */
export function assertSnapshotBusyFailure(ctx){
 const {v,a,e,output,pinned,fail,node,powershell}=ctx;
 const {result:r,ledger:l,plan:p,controller:c,observation:o,claim,providerReady:ready,providerTerminal:t}=a;
 const before=p.historyBaseline,owner=claim.binding,cap=v.actions.absoluteMaxReservedActions;
 const ms=(/** @type {any} */ x)=>milliseconds(x,fail);
 assertCallerDateIdentityShapes(c,ready,fail); // Pure identity/array checks, no date/focus normalization.
 fail(closed(r,['schemaVersion','kind','scenarioId','driver','sourceHead','status','verdict','failureCategory','inheritedGuardBaseline','actualGuardHistory','actualRunContext','absoluteMaxReservedActions','historicalUnsentReservations','actions','providers','counts','actualCallOrder','steps','visibleAssertion','failure','timing','closure','protection','claims','readerDisposition','evidence','calls','atUtc'])&&r.schemaVersion===1&&r.kind==='ACTUAL_FINITE_DRIVER_FLOW_RESULT'&&r.scenarioId===p.scenarioId&&r.driver===v.driver&&r.sourceHead===v.sourceHead&&r.status==='CLOSED_FAIL'&&r.verdict==='FAIL'&&r.failureCategory===category&&note(r.readerDisposition));
 fail(closed(before,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(before).every(x=>Number.isSafeInteger(x)&&x>=0)&&before.sentActions<=before.reservedActions&&Object.values(r.actualRunContext).every(x=>Number.isSafeInteger(x)&&x>=0)&&same(l.history,before)&&l.pending===null&&l.uncertain===false&&same(r.inheritedGuardBaseline,before)&&same(r.actualGuardHistory,before)&&same(r.actualRunContext,{overallAttempts:before.overallAttempts+1,uiAttempts:before.uiAttempts+1})&&same(r.actualRunContext,v.actions.currentRunContext)&&r.absoluteMaxReservedActions===cap&&cap===p.absoluteMaxReservedActions&&cap===a.lease.absoluteMaxReservedActions&&Number.isSafeInteger(cap)&&cap>before.reservedActions&&cap<=before.reservedActions+8&&r.historicalUnsentReservations===before.reservedActions-before.sentActions);
 fail(same(r.actions,{reservedThisRun:0,sentThisRun:0,clicks:0,activations:0})&&same(r.providers,{started:1,B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED',calls:4})&&same(r.counts,{originalPngs:1,geometryModes:1,mappingModes:1,pointCheckAttempts:1})&&same(r.actualCallOrder,['doctor','policy_status','get_window','screenshot'])&&same(r.steps,{B1:'STOPPED_BEFORE_INPUT_POINTCHECK_LIVE_SNAPSHOT_EBUSY',E1:'NOT_STARTED',E2:'NOT_STARTED'})&&same(r.claims,{campaign:false,seed:false,stability:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,newTypedHistoryKind:false}));
 fail(same(r.timing,{rootWatchdogMs:1800000,inputMarginMs:20000,cleanupReserveMs:30000,originalCallerDeadlineMs:p.callerAdmissionDeadlineMs}));
 fail(closed(o,['schemaVersion','kind','driver','scenario','sourceHead','atUtc','failureCategory','assertion','inspection','window','vendorCallCount','originalImages','inputs','reservationsAdded','activations','metadataModes','actualVendorCalls','providerFinish','failure','historyBaseline','actualContext','absoluteMaxReservedActions','rootCallerSession','originalCallerDeadlineMs','timingFailure','focusFailure','dateFailure','ordinaryExit','seed','rootProtection'])&&o.schemaVersion===1&&o.kind==='ACTUAL_S12_DRIVER_PRE_INPUT_POINTCHECK_EBUSY_STOP'&&o.driver===v.driver&&typeof o.scenario==='string'&&resolve(o.scenario)===v.scenario&&o.sourceHead===v.sourceHead&&o.failureCategory===category&&note(o.assertion)&&o.vendorCallCount===4&&o.originalImages===1&&o.inputs===0&&o.reservationsAdded===0&&o.activations===0&&same(o.metadataModes,{geometry:'PASS',playPoint:'PASS',pointCheck:'FAIL_EBUSY'})&&same(o.historyBaseline,before)&&same(o.actualContext,r.actualRunContext)&&o.absoluteMaxReservedActions===cap&&o.rootCallerSession===a.caller.session&&o.originalCallerDeadlineMs===p.callerAdmissionDeadlineMs&&o.timingFailure===false&&o.focusFailure===false&&o.dateFailure===false&&o.ordinaryExit===false&&o.seed===false&&o.rootProtection==='PENDING');
 fail(Array.isArray(o.inspection)&&o.inspection.length===1&&closed(o.inspection[0],['stage','at','local','assertion'])&&o.inspection[0].stage==='PLAY'&&positive(o.inspection[0].at)&&note(o.inspection[0].assertion));
 assertSnapshotBusyAuthority(ctx);
 const tools=['doctor','policy_status','get_window','screenshot'],posts=[],raws=[];
 fail(Array.isArray(r.calls)&&r.calls.length===4&&Array.isArray(o.actualVendorCalls)&&o.actualVendorCalls.length===4&&same(readdirSync(output).filter(n=>/^call-\d+\.(?:attempt\.private|raw\.private|private)\.json$/.test(n)).sort(),tools.flatMap((_,i)=>{const n='call-'+String(i+1).padStart(3,'0');return[n+'.attempt.private.json',n+'.raw.private.json',n+'.private.json'];}).sort())&&!readdirSync(v.scenario).some(n=>/^(?:guard-E[12]-consumed|E[12]-authorization)\.private\.json$/.test(n)));
 let completed=ms(ready.at),protocol=0,auditAt=0;
 for(const [i,tool]of tools.entries()){
  const row=r.calls[i],prefix=resolve(output,'call-'+String(i+1).padStart(3,'0')),observed=o.actualVendorCalls[i];
  fail(closed(row,['completed','attempt','raw'])&&same(row.attempt,e['call'+(i+1)+'Attempt'])&&same(row.raw,e['call'+(i+1)+'Raw']));
  const post=pinned(row.completed,prefix+'.private.json'),attempt=a['call'+(i+1)+'Attempt'],raw=a['call'+(i+1)+'Raw'];
  fail(closed(observed,['ordinal','name','args','atMs'])&&observed.ordinal===i+1&&observed.name===tool&&positive(observed.atMs)&&observed.atMs>auditAt&&observed.atMs>=ms(post.callCompleted)&&observed.atMs<=ms(t.at)&&same(observed.args,i===0?{}:post.args)&&post.tool===tool&&attempt.tool===tool&&post.runID===ready.runID&&attempt.runID===ready.runID&&post.batch==='B1'&&attempt.batch==='B1'&&attempt.sourceSha256===e.source.sha256&&same(post.args,attempt.args)&&attempt.callStarted===post.callStarted&&same(post.precedingActions,[])&&post.isError===false&&(!Object.hasOwn(raw,'isError')||raw.isError===false)&&positive(post.outerProtocolID)&&post.outerProtocolID>protocol&&post.innerProtocolID===i+1&&attempt.outerProtocolID===post.outerProtocolID&&attempt.innerProtocolID==='UNKNOWN'&&post.freshness==='UNKNOWN'&&ms(post.callStarted)>=completed&&ms(post.callCompleted)>=ms(post.callStarted)&&ms(post.callCompleted)<=ms(t.at));
  completed=ms(post.callCompleted);protocol=post.outerProtocolID;auditAt=observed.atMs;posts.push(post);raws.push(raw);
 }
 fail(raws.slice(0,3).every(x=>object(x.structuredContent))&&same(posts[0].args,{include_remediation:true})&&same(posts[1].args,{})&&raws[0].structuredContent.ok===true&&raws[1].structuredContent.approval_required_for_all===true&&raws[1].structuredContent.approval_token_configured===false);
 const window=raws[2].structuredContent;fail(positive(window.pid)&&positive(window.windowId)&&window.bundleId==='java.exe'&&window.isFocused===true&&closed(window.bounds,['x','y','width','height'])&&Object.values(window.bounds).every(Number.isSafeInteger)&&positive(window.bounds.width)&&positive(window.bounds.height)&&same(o.window,window)&&same(posts[2].args,{window_id:window.windowId})&&same(posts[3].args,{target_window_id:window.windowId,quality:0,width:window.bounds.width})&&owner.campaign.windowId===window.windowId);
 assertSnapshotBusyOwned(ctx,window);
 const aliases={observation:e.observation,metadataCaller:e.metadataCaller,callerTerminal:e.caller,launchSupervisor:e.launch,protection:e.protection,pidAbsence:e.pidAbsence,ledger:e.ledger,plan:e.plan,lease:e.lease,controller:e.controller,owned:e.owned,ownedB1:e.initialOwned,rootCall:e.rootCall,source:e.source,claim:e.claim,preference:e.preference,supportPins:e.supportPins,geometry:e.geometry,geometryTerminal:e.geometryTerminal,point:e.point,coordinateProof:e.envelope,sourceBefore:e.sourceFrame,geometryOwned:e.frameOwned,helper:e.helper,geometryRunner:e.geometryRunner,providerReady:e.providerReady,providerTerminal:e.providerTerminal,providerSpool:e.providerSpool,providerStderr:e.providerStderr};
 fail(closed(r.evidence,[...Object.keys(aliases),'preparationProgress','originalPng','originalMetadata','originalRaw']));for(const [key,pin]of Object.entries(aliases))fail(same(r.evidence[key],pin));
 pinned(r.evidence.preparationProgress,resolve(v.scenario,'progress.pre-native-preparation.original.private.json'));
 const prefix=resolve(output,'capture-001'),png=file(prefix+'.png',32*1024*1024),metadata=pinned(r.evidence.originalMetadata,prefix+'.metadata.private.json'),capture=pinned(r.evidence.originalRaw,prefix+'.raw.private.json',32*1024*1024),dimensions=completePng(png.bytes);
 const images=capture.content?.filter((/** @type {any} */ x)=>object(x)&&x.type==='image');
 fail(Array.isArray(capture.content)&&capture.content.every(object)&&images?.length===1&&images[0].mimeType==='image/png'&&images[0].data===png.bytes.toString('base64')&&same(r.evidence.originalPng,{path:prefix+'.png',sha256:png.sha256})&&same(capture,raws[3])&&file(prefix+'.raw.private.json',32*1024*1024).bytes.equals(file(e.call4Raw.path,32*1024*1024).bytes)&&metadata.captureReceipt===true&&metadata.freshness==='UNKNOWN'&&metadata.rawPath===prefix+'.raw.private.json'&&metadata.rawSha256===r.evidence.originalRaw.sha256&&metadata.pngPath===prefix+'.png'&&metadata.pngSha256===png.sha256&&same(metadata.window,window)&&same(metadata.dimensions,dimensions)&&dimensions.width===window.bounds.width&&dimensions.height===window.bounds.height);
 for(const key of ['tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID'])fail(same(metadata[key],posts[3][key]));
 assertSnapshotBusyGeometry({...ctx,window,dimensions,posts});
 fail(same(r.visibleAssertion,{ownedPid:window.pid,ownedHwnd:window.windowId,isFocused:true,launcherPlayVisible:true,originalNativeDimensions:dimensions,localPoint:[a.point.localPoint.x,a.point.localPoint.y],mappedScreenPoint:a.point.coordinate,inputSent:false,captureReceipt:true,freshness:'UNKNOWN'}));
 assertSnapshotBusyMetadata(ctx);
 fail(closed(o.providerFinish,['raw','at'])&&positive(o.providerFinish.at)&&o.providerFinish.at>=ms(t.at)&&o.providerFinish.at<=ms(o.atUtc)&&same(callerDatePacket(o.providerFinish.raw,fail),t)&&ms(o.atUtc)<=ms(a.protection.at)&&ms(r.atUtc)>=ms(a.protection.at));
 const ids=[...new Set([c.controllerPid,...c.supervisorJobPids,...a.owned.processes.flatMap((/** @type {any} */ x)=>[x.pid,...x.supervisorJobPids]),...owner.ownedPids,t.verifierPid,ready.node.pid,ready.helper.pid,...ready.supervisorJobPids])].sort((x,y)=>x-y),absence=a.pidAbsence;
 fail(ids.every(positive)&&same(absence,{schemaVersion:1,kind:'ACTUAL_CURRENT_OWNED_PID_ABSENCE_AFTER_ORIGINAL_CALLER_TERMINAL',atUtc:absence.atUtc,allAbsent:true,queriedExactRecordedPids:ids,liveRows:[],controllerSha256:e.controller.sha256,ownedSha256:e.owned.sha256,providerReadySha256:e.providerReady.sha256,providerTerminalSha256:e.providerTerminal.sha256,originalCallerSha256:e.caller.sha256})&&ms(absence.atUtc)>=ms(t.at)&&ms(a.protection.at)>=ms(t.at)&&ms(r.atUtc)>=ms(absence.atUtc));
 const close=r.closure;fail(closed(close,['provider','rootCaller','maintainedStopToolChunk','forcedOwnedCleanup','ordinaryExit','allOwnedHandlesTerminalReaped','knownPidsAbsent'])&&same(close.provider,{verifierPid:t.verifierPid,status:0,timedOut:false,verdict:'PASS',terminalReaped:true,at:t.at})&&same(close.rootCaller,{session:a.caller.session,originalRawChunk:a.caller.launchRaw.chunk_id,terminalRawChunk:a.caller.actual.chunk_id,status:1,timedOut:false,verdict:'FAIL',terminalReaped:true,pollCount:a.caller.launchPolls.length})&&typeof close.maintainedStopToolChunk==='string'&&/^[A-Za-z0-9_-]{1,128}$/.test(close.maintainedStopToolChunk)&&close.forcedOwnedCleanup===true&&close.ordinaryExit===false&&close.allOwnedHandlesTerminalReaped===true&&same(close.knownPidsAbsent,ids));
 fail(same(r.protection,{protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,originalWrites:false,originalAutosave:{enabled:true,maximumSlots:3},privateAutosaveEnabled:false,privateSaveEntries:0,liveGameProcesses:0,sharedPreference:'Registry64/String/true unchanged',restoration:'UNNECESSARY_ORIGINALS_UNCHANGED'}));
 assertSnapshotBusyCaller(ctx);
}

/** @param {any} ctx @param {any} window */
function assertSnapshotBusyOwned({v,a,e,fail},window){
 const c=a.controller,ms=(/** @type {any} */ x)=>milliseconds(x,fail),snapshots=[a.initialOwned,a.frameOwned,a.owned];
 fail(same(e.initialOwned,{path:a.source.ownedReceiptPath,sha256:a.source.ownedReceiptSha256}));
 fail(closed(a.initialOwned,['at','sourceSnapshot','processes'])&&closed(a.initialOwned.sourceSnapshot,['path','sha256'])&&a.initialOwned.sourceSnapshot.path===e.owned.path&&typeof a.initialOwned.sourceSnapshot.sha256==='string'&&/^[a-f0-9]{64}$/.test(a.initialOwned.sourceSnapshot.sha256)); // Recorded overwritten alias; do not rehash it now.
 const games=snapshots.map(s=>{fail(object(s)&&Array.isArray(s.processes)&&s.processes.every((/** @type {any} */ p)=>object(p)&&typeof p.executable==='string'&&positive(p.sessionId)));return s.processes.filter((/** @type {any} */ p)=>p.executable===resolve(c.installation,'jre/bin/java.exe'));});
 fail(games.every(x=>x.length===1));const first=games[0][0];fail(positive(first.pid)&&positive(first.parentPid)&&first.pid===window.pid&&same(a.claim.binding.ownedPids,[first.pid])&&typeof first.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(first.kernelCreationFileTime)&&ms(first.creationTime)>=ms(c.at)&&ms(first.creationTime)<=ms(a.providerReady.at));
 for(const [i,rows]of games.entries()){const x=rows[0],s=snapshots[i],handle=x.mainWindowHandle;
  fail((i===0||(s.controllerPid===c.controllerPid&&s.controllerCreationTime===c.controllerCreationTime&&s.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime))&&['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(k=>same(x[k],first[k]))&&x.sessionId===c.sessionId&&x.inSupervisorJob===true&&Array.isArray(x.supervisorJobPids)&&x.supervisorJobPids.every(positive)&&new Set(x.supervisorJobPids).size===x.supervisorJobPids.length&&[x.pid,x.parentPid,c.controllerPid,...c.supervisorJobPids].every(id=>x.supervisorJobPids.includes(id))&&x.pid!==x.parentPid&&ms(x.creationTime)<=ms(s.at)&&((positive(handle))||(typeof handle==='string'&&/^[1-9][0-9]*$/.test(handle)))&&Number(handle)===window.windowId);
 }
 return first;
}

/** @param {any} ctx */
function assertSnapshotBusyMetadata({v,a,e,fail,node,powershell}){
 const packet=a.metadataCaller,r=a.result,o=a.observation,point=a.point,run=a.providerReady.runID,h=e.helper.path.replaceAll('\\','/'),n=node.replaceAll('\\','/'),ps=powershell.replaceAll('\\','/'),x=point.localPoint.x,y=point.localPoint.y,inspected=point.inspectedAtMs;
 fail(closed(packet,['schemaVersion','kind','driver','scenario','entries'])&&packet.schemaVersion===1&&packet.kind==='ACTUAL_DRIVER_METADATA_CALLS'&&packet.driver===v.driver&&typeof packet.scenario==='string'&&resolve(packet.scenario)===v.scenario&&Array.isArray(packet.entries)&&packet.entries.length===1);
 const cmd="$ErrorActionPreference='Stop';$env:PATH='"+dirname(node).replaceAll('\\','/')+';'+dirname(powershell).replaceAll('\\','/')+";'+$env:PATH;$h='"+h+"';$n='"+n+"';$gr=&$n $h geometry B1-PLAY "+run+' call-004 capture-001 '+inspected+";if($LASTEXITCODE-ne0){throw 'geometry failed'};$g=$gr|ConvertFrom-Json -DateKind String;$mr=&$n $h play-point "+run+' call-004 capture-001 '+x+' '+y+' '+inspected+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'mapping failed'};$m=$mr|ConvertFrom-Json -DateKind String;$cr=&$n $h point-check B1-PLAY $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw 'pointcheck failed'};$c=$cr|ConvertFrom-Json -DateKind String;$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json -DateKind String;[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress";
 const entry=packet.entries[0],raw=entry.raw,path=resolve(v.scenario,'owned-processes-current.private.json'),forward=path.replaceAll('\\','/');
 fail(closed(entry,['label','args','raw','polls'])&&entry.label==='B1-PLAY'&&same(entry.args,{cmd,shell:ps,sandbox_permissions:'require_escalated',max_output_tokens:1500})&&same(entry.polls,[])&&raw.exit_code===1&&!Object.hasOwn(raw,'session_id')&&Number.isFinite(raw.wall_time_seconds)&&raw.wall_time_seconds>=0&&typeof raw.output==='string'&&Buffer.byteLength(raw.output)<=8192&&typeof raw.chunk_id==='string'&&/^[A-Za-z0-9_-]{1,128}$/.test(raw.chunk_id));
 fail(raw.output.includes("Error: EBUSY: resource busy or locked, open '"+path+"'")&&raw.output.includes('at readFileSync (node:fs:444:35)')&&raw.output.includes('at read (file:///'+h+':20:63)')&&raw.output.includes('at pointCheck (file:///'+h+':262:17)')&&/errno:\s*-4082\s*,/.test(raw.output)&&/code:\s*'EBUSY'\s*,/.test(raw.output)&&/syscall:\s*'open'/.test(raw.output)&&raw.output.includes('pointcheck failed'));
 fail(same(r.failure,{outerChunk:raw.chunk_id,exitCode:1,code:'EBUSY',errno:-4082,syscall:'open',path:forward,location:'construct-actual-phase-inputs.private.mjs:262 pointCheck -> read():20 -> readFileSync',geometry:'PASS_NESTED_WINDOWS_JOB_STATUS0_REAPED',mapping:'PASS',pointCheck:'FAILED_READING_CURRENT_OWNED_SNAPSHOT',freshFocusPairAfterPointCheck:'NOT_CALLED',strictClick:'NOT_CALLED',timingFailure:false,focusFailure:false,dateFailure:false,cause:r.failure.cause,retry:false,clockRenewal:false})&&note(r.failure.cause)&&same(o.failure,{chunk:raw.chunk_id,exitCode:1,code:'EBUSY',errno:-4082,syscall:'open',path:forward,helperLocation:'construct-actual-phase-inputs.private.mjs:262 pointCheck -> readFileSync:20',noFreshFocusPair:true,noNativeInput:true,noRetry:true}));
}

/** @param {any} ctx */
function assertSnapshotBusyAuthority({v,a,e,pinned,fail}){
 const {plan:p,lease,controller:c,ledger:l,claim,providerReady:ready,providerTerminal:t}=a,owner=claim.binding,before=p.historyBaseline,cap=v.actions.absoluteMaxReservedActions;
 const ms=(/** @type {any} */ x)=>milliseconds(x,fail),pos=(/** @type {any} */ x)=>fail(positive(x));
 fail(p.sessionWatchdogMs===1800000&&lease.sessionWatchdogMs===1800000);
 const source=a.source;
 fail(closed(source,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256']));
 const phase=pinned({path:source.planPath,sha256:source.planSha256},resolve(v.scenario,'B1-plan.private.json'));
 const phaseLease=pinned({path:source.leasePath,sha256:source.leaseSha256},resolve(v.scenario,'B1-lease.private.json'));
 fail(Array.isArray(p.batches)&&p.batches.filter((/** @type {any} */ x)=>x.id==='B1').length===1&&
  same(phase,{...p,campaign:owner.campaign,batches:p.batches.filter((/** @type {any} */ x)=>x.id==='B1')})&&
  same(phaseLease,{...lease,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign:owner.campaign,planSha256:source.planSha256})&&
  closed(owner.campaign,['flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor'])&&owner.campaign.flow==='DISPOSABLE_SEED_CREATION');
 fail(same(phase.campaign,owner.campaign)&&same(phaseLease.campaign,owner.campaign)&&owner.planSha256===source.planSha256&&owner.sourcePath===e.source.path&&
  phase.scenarioId===p.scenarioId&&phase.paths?.scenario===v.scenario&&phase.source?.qualifiedHead===v.sourceHead&&same(phase.historyBaseline,before)&&phase.absoluteMaxReservedActions===cap&&same(phase.seedCreation,p.seedCreation)&&
  same(owner.campaign.historyBaseline,before)&&owner.campaign.maxReservedActions===cap&&same(owner.campaign.rootCallRecord,a.rootCall)&&
  phaseLease.schemaVersion===6&&phaseLease.authorized===true&&phaseLease.issuer==='/root'&&phaseLease.driver===v.driver&&phaseLease.phase==='DISPOSABLE_SEED_CREATION'&&phaseLease.planSha256===source.planSha256&&
  phaseLease.maxActions===8&&phaseLease.maxProxyInvocations===3&&phaseLease.ownedGameHardTimeoutMs===1800000&&phaseLease.campaignLoadSaveAllowed===false&&
  phaseLease.supportPinsPath===e.supportPins.path&&phaseLease.supportPinsSha256?.toLowerCase()===e.supportPins.sha256&&phaseLease.fixtureReceiptSha256===lease.fixtureReceiptSha256&&same(phaseLease.seedCreation,p.seedCreation)&&phaseLease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs);
 const clock=owner.campaign.rootClock,birth=ms(c.controllerCreationTime),published=ms(c.at),outer=ms(c.conservativeOuterDeadlineUtc),end=ms(c.controllerDeadlineUtc);
 fail(positive(clock.startMs)&&positive(p.callerAdmissionDeadlineMs)&&p.callerAdmissionDeadlineMs>clock.startMs&&p.callerAdmissionDeadlineMs-clock.startMs<=1800000);
 fail([clock.startMs,clock.deadlineMs,clock.effectiveDeadlineMs,p.callerAdmissionDeadlineMs,l.providerClock?.startMs,l.providerClock?.deadlineMs].every(Number.isSafeInteger)&&
  clock.startMs===a.rootCall.value.startMs&&clock.recordKey===a.rootCall.key&&clock.deadlineMs===clock.startMs+1800000&&clock.sessionWatchdogMs===1800000&&clock.cleanupReserveMs===30000&&
  clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.originalCallerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&lease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,p.callerAdmissionDeadlineMs,outer,end)&&
  clock.startMs<=birth&&birth<=published&&published<end&&end<=outer&&outer<=birth+1800000&&l.providerClock.startMs>=published&&l.providerClock.startMs>=clock.startMs&&l.providerClock.deadlineMs===Math.min(l.providerClock.startMs+180000,clock.effectiveDeadlineMs-30000)&&ms(ready.at)>=l.providerClock.startMs&&ms(t.at)<=l.providerClock.deadlineMs);
 const producer=owner.campaign.producerWitness;
 fail(clock.producerRecordKey==='controller-start:'+e.controller.sha256&&producer.producerRecordKey===clock.producerRecordKey&&producer.controllerPid===c.controllerPid&&producer.controllerCreationTime===c.controllerCreationTime&&producer.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&producer.conservativeOuterDeadlineUtc===c.conservativeOuterDeadlineUtc&&producer.controllerDeadlineUtc===c.controllerDeadlineUtc&&producer.effectiveDeadlineMs===clock.effectiveDeadlineMs&&producer.sessionWatchdogMs===1800000);
 const predecessor=pinned(owner.campaign.launchPredecessor,resolve(v.scenario,'campaign-postlaunch.private.json'));
 fail(same(predecessor,{schemaVersion:1,kind:'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR',plan:e.plan,lease:e.lease,controller:e.controller,rootClock:clock,producerWitness:producer,rootCallRecord:a.rootCall,historyBaseline:before,maxReservedActions:cap}));
 for(const child of [ready.node,ready.helper]){pos(child.pid);pos(child.parentPid);fail(ms(child.creationTime)>=published&&ms(child.creationTime)<=ms(ready.at));}
 fail(ms(ready.helper.creationTime)>=ms(ready.node.creationTime));

}


/** Original recorded timings prove only this historical mapping; never current freshness.
 * @param {any} ctx */
function assertSnapshotBusyGeometry({v,a,e,output,fail,node,powershell,window,dimensions,posts}){
 const {result:r,plan:p,controller:c,sourceFrame:s,beforeFrame:frame,geometry:g,geometryTerminal:t,point,metadataCaller:packet,observation:o}=a,clock=a.claim.binding.campaign.rootClock;
 fail(note(g.originMeaning));
 const parse=(/** @type {string} */ text)=>zeroProviderObject(text,fail);
 const frameExpected={pngPath:resolve(output,'capture-001.png'),pngSha256:r.evidence.originalPng.sha256,metadataPath:resolve(output,'capture-001.metadata.private.json'),metadataSha256:r.evidence.originalMetadata.sha256,capturedAtMs:Date.parse(posts[3].callCompleted)};
 fail(same(frame,frameExpected)&&same(s,{schemaVersion:1,kind:'PRIVATE_FIXED_LATEST_SOURCE_CAPTURE',scenario:v.scenario,sourceHead:v.sourceHead,label:'B1-PLAY',runID:a.providerReady.runID,callLabel:'call-004',captureLabel:'capture-001',controller:e.controller,source:e.source,claim:e.claim,owned:e.initialOwned,post:r.calls[3].completed,attempt:e.call4Attempt,raw:e.call4Raw,captureRaw:r.evidence.originalRaw,automatic:r.evidence.originalMetadata,frame:e.beforeFrame,closedFrame:frame,window,ownedSnapshot:e.frameOwned,rootClock:clock,inspectedAtMs:point.inspectedAtMs,inputGranted:false}));
 fail(closed(point,['schemaVersion','kind','label','frame','geometry','terminal','sourceCapture','ownedSnapshot','localPoint','coordinate','inspectedAtMs','computedAtMs','controllerSha256','inputGranted'])&&point.schemaVersion===1&&point.kind==='PRIVATE_FIXED_SEED_CLIENT_POINT'&&point.label==='B1-PLAY'&&same(point.frame,frame)&&same(point.geometry,e.geometry)&&same(point.terminal,e.geometryTerminal)&&same(point.sourceCapture,e.sourceFrame)&&same(point.ownedSnapshot,e.frameOwned)&&point.controllerSha256===e.controller.sha256&&point.inputGranted===false&&Number.isSafeInteger(point.inspectedAtMs)&&Number.isSafeInteger(point.computedAtMs)&&point.inspectedAtMs>=frame.capturedAtMs&&point.inspectedAtMs<=t.startedAtMs&&point.computedAtMs>=t.completedAtMs&&point.computedAtMs<=Date.parse(a.providerTerminal.at));
 const target=a.frameOwned.processes.find((/** @type {any} */ x)=>x.pid===window.pid),coordinates=clientImagePoint({geometry:g,window,owned:{...target,controllerPid:c.controllerPid},dimensions,point:point.localPoint,nowMs:point.computedAtMs});
 fail(same(point.coordinate,coordinates)&&same(o.inspection[0].local,[point.localPoint.x,point.localPoint.y])&&o.inspection[0].at===point.inspectedAtMs);
 fail(same(a.envelope,{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_POINT_CANONICAL_BINDING',point:e.point,canonical:e.sourceFrame,frame,inputGranted:false}));
 const project=resolve(v.scenario,'geometry-owned-B1-PLAY'),bounds=window.bounds;
 const program=t.childCommand?.program;fail(typeof program==='string'&&(program===powershell||program===powershell.replaceAll('\\','/')));
 const child={program,args:['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1'),'-EvidenceRoot',v.scenario,'-WindowId',String(window.windowId),'-ExpectedX',String(bounds.x),'-ExpectedY',String(bounds.y),'-ExpectedWidth',String(bounds.width),'-ExpectedHeight',String(bounds.height)],cwd:'.'};
 fail(same(a.geometryConfiguration,{version:2,overrides:{workspace:{commands:{'node:.':[child]}}}})&&closed(t,['schemaVersion','kind','sourceHead','scenario','label','sourceCapture','controller','sourceFrame','program','args','cwd','childCommand','configuration','startedAtMs','completedAtMs','childTimeoutMs','cleanupAllowanceMs','outerTimeoutMs','status','signal','error','stdout','stderr','terminalReaped','outputComplete','supervisorJSON','rootClock','providerClock','closedProvider','geometry','inputGranted'])&&t.schemaVersion===1&&t.kind==='ACTUAL_OWNED_GEOMETRY_COMMAND'&&t.sourceHead===v.sourceHead&&t.scenario===v.scenario&&t.label==='B1-PLAY'&&same(t.sourceCapture,e.sourceFrame)&&same(t.controller,e.controller)&&same(t.sourceFrame,e.beforeFrame)&&same(t.childCommand,child)&&same(t.configuration,e.geometryConfiguration)&&t.program===resolve(v.sourceWorkspace,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe')&&same(t.args,['verify',project,'--scope','module','--module','node:.','--concurrency','1','--timeout','5000'])&&t.cwd===project&&t.childTimeoutMs===5000&&t.cleanupAllowanceMs===10000&&t.outerTimeoutMs===15000&&t.status===0&&t.signal===null&&t.error===null&&t.stderr===''&&t.terminalReaped===true&&t.outputComplete===true&&t.closedProvider===null&&t.inputGranted===false&&same(t.rootClock,clock)&&same(t.providerClock,a.ledger.providerClock)&&same(t.geometry,e.geometry));
 const verifierPins=a.supportPins.pins.filter((/** @type {any} */ x)=>x.path===t.program);fail(verifierPins.length===1&&typeof verifierPins[0].sha256==='string'&&/^[a-fA-F0-9]{64}$/.test(verifierPins[0].sha256)&&file(t.program,128*1024*1024).sha256===verifierPins[0].sha256.toLowerCase());
 const childPins=a.supportPins.pins.filter((/** @type {any} */ x)=>x.path===child.args[3]);fail(childPins.length===1&&typeof childPins[0].sha256==='string'&&/^[a-fA-F0-9]{64}$/.test(childPins[0].sha256)&&file(child.args[3]).sha256===childPins[0].sha256.toLowerCase());
 fail(Number.isSafeInteger(t.startedAtMs)&&Number.isSafeInteger(t.completedAtMs)&&t.startedAtMs<=Date.parse(g.observedAtUtc)&&Date.parse(g.observedAtUtc)<=t.completedAtMs&&t.completedAtMs-t.startedAtMs<=15000&&t.completedAtMs<clock.effectiveDeadlineMs-50000&&t.completedAtMs<a.ledger.providerClock.deadlineMs-30000&&t.completedAtMs-Date.parse(g.observedAtUtc)<=5000&&milliseconds(a.frameOwned.at,fail)<=t.startedAtMs&&t.startedAtMs>=a.ledger.providerClock.startMs);
 // Replay the pinned producer's original clipping, rather than trusting the captured timeout labels.
 const provider=a.ledger.providerClock,budget=Math.floor(Math.min(clock.effectiveDeadlineMs-t.startedAtMs-50000,provider.deadlineMs-t.startedAtMs-30000));
 const cleanup=Math.min(10000,Math.floor(budget/2)),childTimeout=Math.min(5000,budget-cleanup);
 fail(provider.deadlineMs===provider.startMs+180000&&budget>=2&&t.cleanupAllowanceMs===cleanup&&t.childTimeoutMs===childTimeout&&t.outerTimeoutMs===childTimeout+cleanup);
 fail(typeof t.stdout==='string'&&Buffer.byteLength(t.stdout)<=65536&&same(parse(t.stdout),t.supervisorJSON));
 const report=t.supervisorJSON,step=report.result?.steps?.[0];
 fail(report.schemaVersion===1&&report.command==='verify'&&report.ok===true&&report.result.verdict==='PASS'&&Array.isArray(report.result.steps)&&report.result.steps.length===1&&step.command===program&&same(step.args,child.args)&&step.cwd==='.'&&step.module==='node:.'&&step.processOwnership==='windows-job-object'&&step.status===0&&step.timedOut===false&&step.stderr===''&&typeof step.stdout==='string'&&Buffer.byteLength(step.stdout)<=8192&&same(parse(step.stdout),g));
 for(const [name,pin]of [['read-client-geometry.owned.private.mjs',e.geometryRunner],['construct-actual-phase-inputs.private.mjs',e.helper]])fail(same(p.preclockHelpers?.[name],pin)&&closed(pin,['path','sha256'])&&pin.path===resolve(v.scenario,name)&&file(pin.path,262144).sha256===pin.sha256);

}

/** @param {any} ctx */
function assertSnapshotBusyCaller({v,a,e,pinned,fail,node,powershell}){
 const p=a.plan,c=a.controller;
 const historic=(/** @type {string} */ name)=>{const pin=p.preclockHelpers?.[name];fail(closed(pin,['path','sha256'])&&pin.path===resolve(v.scenario,name)&&file(pin.path,262144).sha256===pin.sha256);return pin;};
 const caller=historic('launch-owned-caller.private.ps1'),stager=historic('record-root-call-and-stage-B1.private.ps1'),orchestration=historic('launch-and-stage.orchestration.proposed.private.js');
 fail(same(historic('construct-actual-phase-inputs.private.mjs'),e.helper));
 for(const [pin,name]of [[p.historyAdmission,'root-history-baseline.admitted.private.json'],[p.fixtureBinding,'fixture-binding.private.json']])fail(closed(pin,['path','sha256'])&&pin.path===resolve(v.scenario,name)&&file(pin.path,262144).sha256===pin.sha256);
 fail(same(p.historyAdmission,a.lease.historyAdmission)&&p.rootCallContract?.path===e.rootCall.path&&p.rootCallContract.key===a.rootCall.key);
 const path=resolve(v.scenario,'root-postlaunch-mechanical-delegation.private.json'),pin={path,sha256:file(path).sha256},d=pinned(pin,path);
 fail(d.schemaVersion===1&&d.kind==='ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'&&d.issuer==='/root'&&d.driver===v.driver&&d.planSha256===e.plan.sha256&&d.leaseSha256===e.lease.sha256&&d.helperSha256===e.helper.sha256&&d.callerSha256===caller.sha256&&d.stagerSha256===stager.sha256&&d.orchestrationSha256===orchestration.sha256&&d.historyAdmissionSha256===p.historyAdmission.sha256&&same(d.allowedModes,['init','phase B1']));
 const q=(/** @type {string} */ s)=>"'"+s.replaceAll("'","''")+"'";
 const prefix="$env:PATH='"+dirname(node).replaceAll('\\','/')+';'+dirname(powershell).replaceAll('\\','/')+";'+$env:PATH; ";
 fail(a.rootCall.value.args.shell===powershell.replaceAll('\\','/')&&a.rootCall.value.args.cmd===prefix+'& '+q(powershell)+' -NoProfile -NonInteractive -File '+q(caller.path)+' -PlanSha256 '+q(e.plan.sha256)+' -LeaseSha256 '+q(e.lease.sha256)+' -DelegationSha256 '+q(pin.sha256)+' -HelperSha256 '+q(e.helper.sha256)+' -HistoryAdmissionSha256 '+q(p.historyAdmission.sha256));
 const outer=a.launch.result.steps[0];fail(outer.command===powershell&&same(outer.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',v.scenario,'-BindingPath',p.fixtureBinding.path,'-BindingSha256',p.fixtureBinding.sha256,'-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','1800000'])&&outer.cwd==='.');
}
