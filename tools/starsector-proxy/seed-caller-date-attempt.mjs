// @ts-check
// Historical four-call stop after successful geometry, before any input.
import {resolve,basename} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,file} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';
import {assertCallerDateGeometry,callerDatePacket} from './seed-caller-date-geometry.mjs';

export const callerDateNames={observation:'driver-stop-observation.private.json',metadataCaller:'driver-metadata-calls.actual.private.json',
 initialOwned:'driver-B1-owned.private.json',frameOwned:'B1-PLAY-owned-snapshot.private.json',beforeFrame:'B1-PLAY-before-frame.private.json',
 sourceFrame:'B1-PLAY-source-before.private.json',geometry:'B1-PLAY-client-geometry.actual.private.json',
 geometryTerminal:'B1-PLAY-client-geometry-terminal.actual.private.json',point:'B1-PLAY-client-point.private.json',
 envelope:'B1-PLAY-coordinate-proof.private.json',geometryConfiguration:'geometry-owned-B1-PLAY/.agentic/project.json'};
export const callerDateProviderNames=Object.fromEntries([1,2,3,4].flatMap(i=>[
 ['call'+i+'Attempt','call-'+String(i).padStart(3,'0')+'.attempt.private.json'],
 ['call'+i+'Raw','call-'+String(i).padStart(3,'0')+'.raw.private.json']]));
/** Strict JSON object containers at this new custody boundary. @param {any} value */
const object=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
/** @param {any} controller @param {any} ready @param {(ok:any)=>void} fail */
export function assertCallerDateIdentityShapes(controller,ready,fail){
 /** @param {any} ids */const pids=ids=>Array.isArray(ids)&&ids.length>0&&ids.every(id=>Number.isSafeInteger(id)&&id>0)&&new Set(ids).size===ids.length;
 fail(pids(controller.supervisorJobPids)&&pids(ready.supervisorJobPids)&&typeof controller.controllerKernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(controller.controllerKernelCreationFileTime));
}

/** @param {{v:any,a:any,e:any,output:string,pinned:(p:any,s:string,n?:number)=>any,fail:(v:any)=>void,node:string,powershell:string}} ctx */
export function assertCallerDateFailure(ctx){
 const {v,a,e,output,pinned,fail,node,powershell}=ctx;
 const {result:r,ledger:l,plan:p,lease,controller:c,observation:o,claim,providerReady:ready,providerTerminal:t}=a,owner=claim.binding;
 /** @param {any} x */const ms=x=>{fail(typeof x==='string'&&Number.isSafeInteger(Date.parse(x)));return Date.parse(x);};
 /** @param {any} x */const pos=x=>fail(Number.isSafeInteger(x)&&x>0);
 const before=p.historyBaseline,h=r.history,cap=v.actions.absoluteMaxReservedActions;
 fail(closed(before,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&Object.values(before).every(n=>Number.isSafeInteger(n)&&n>=0)&&before.sentActions<=before.reservedActions&&same(l.history,before)&&l.pending===null&&l.uncertain===false&&
  same(v.actions.currentRunContext,{overallAttempts:before.overallAttempts+1,uiAttempts:before.uiAttempts+1})&&Number.isSafeInteger(cap)&&cap>before.reservedActions&&cap<=before.reservedActions+8&&cap===p.absoluteMaxReservedActions&&cap===lease.absoluteMaxReservedActions&&
  same(h,{guard:before,actualContext:v.actions.currentRunContext,absoluteMaxReservedActions:cap,historicalUnsentReservations:before.reservedActions-before.sentActions,addedReservations:0,addedSentActions:0,refunded:0})&&
  same(r.counts,{vendorCalls:4,originalPngs:1,providersStarted:1,actions:0,activations:0})&&same(r.steps,{B1:'STOPPED_BEFORE_INPUT_CALLER_DATE_SERIALIZATION',E1:'NOT_STARTED',E2:'NOT_STARTED'})&&
  same(r.claims,{seed:false,manualSave:false,campaign:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,closedHistoryKindIssued:false}));
 fail(closed(r,['schemaVersion','kind','scenarioId','driver','status','verdict','failureCategory','sourceHead','history','counts','steps','originalInspection','geometry','failure','closure','protection','claims','evidence','atUtc']));
 fail(closed(o,['schemaVersion','kind','driver','scenario','atUtc','failureCategory','assertion','inspection','actualVendorCalls','inputs','reservations','begin','providerFinish','metadataFailure','noRetry','rootCallerSession','originalCallerCap','historyBaseline','actualContext','absoluteMaxReservedActions','ordinaryExit','seed','rootProtection'])&&o.schemaVersion===1&&o.kind==='ACTUAL_S8_DRIVER_PRE_INPUT_CALLER_SERIALIZATION_STOP'&&
  o.driver===v.driver&&typeof o.scenario==='string'&&resolve(o.scenario)===v.scenario&&o.failureCategory===r.failureCategory&&o.inputs===0&&o.reservations===0&&o.noRetry===true&&o.ordinaryExit===false&&o.seed===false&&o.rootProtection==='PENDING'&&
  same(o.historyBaseline,before)&&same(o.actualContext,v.actions.currentRunContext)&&o.absoluteMaxReservedActions===cap&&o.rootCallerSession===a.caller.session&&o.originalCallerCap===p.callerAdmissionDeadlineMs);
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
  phaseLease.maxActions===8&&phaseLease.maxProxyInvocations===3&&phaseLease.ownedGameHardTimeoutMs===480000&&phaseLease.campaignLoadSaveAllowed===false&&
  phaseLease.supportPinsPath===e.supportPins.path&&phaseLease.supportPinsSha256?.toLowerCase()===e.supportPins.sha256&&phaseLease.fixtureReceiptSha256===lease.fixtureReceiptSha256&&same(phaseLease.seedCreation,p.seedCreation)&&phaseLease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs);
 const clock=owner.campaign.rootClock,birth=ms(c.controllerCreationTime),published=ms(c.at),outer=ms(c.conservativeOuterDeadlineUtc),end=ms(c.controllerDeadlineUtc);
 fail([clock.startMs,clock.deadlineMs,clock.effectiveDeadlineMs,p.callerAdmissionDeadlineMs,l.providerClock?.startMs,l.providerClock?.deadlineMs].every(Number.isSafeInteger)&&
  clock.startMs===a.rootCall.value.startMs&&clock.recordKey===a.rootCall.key&&clock.deadlineMs===clock.startMs+480000&&clock.sessionWatchdogMs===480000&&clock.cleanupReserveMs===30000&&
  clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.originalCallerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&lease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,p.callerAdmissionDeadlineMs,outer,end)&&
  clock.startMs<=birth&&birth<=published&&published<end&&end<=outer&&outer<=birth+480000&&l.providerClock.startMs>=published&&l.providerClock.startMs>=clock.startMs&&l.providerClock.deadlineMs===Math.min(l.providerClock.startMs+180000,clock.effectiveDeadlineMs-30000)&&ms(ready.at)>=l.providerClock.startMs&&ms(t.at)<=l.providerClock.deadlineMs);
 const producer=owner.campaign.producerWitness;
 fail(clock.producerRecordKey==='controller-start:'+e.controller.sha256&&producer.producerRecordKey===clock.producerRecordKey&&producer.controllerPid===c.controllerPid&&producer.controllerCreationTime===c.controllerCreationTime&&producer.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&producer.conservativeOuterDeadlineUtc===c.conservativeOuterDeadlineUtc&&producer.controllerDeadlineUtc===c.controllerDeadlineUtc&&producer.effectiveDeadlineMs===clock.effectiveDeadlineMs&&producer.sessionWatchdogMs===480000);
 const predecessor=pinned(owner.campaign.launchPredecessor,resolve(v.scenario,'campaign-postlaunch.private.json'));
 fail(same(predecessor,{schemaVersion:1,kind:'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR',plan:e.plan,lease:e.lease,controller:e.controller,rootClock:clock,producerWitness:producer,rootCallRecord:a.rootCall,historyBaseline:before,maxReservedActions:cap}));
 for(const child of [ready.node,ready.helper]){pos(child.pid);pos(child.parentPid);fail(ms(child.creationTime)>=published&&ms(child.creationTime)<=ms(ready.at));}
 fail(ms(ready.helper.creationTime)>=ms(ready.node.creationTime));
 const tools=['doctor','policy_status','get_window','screenshot'],posts=[],raws=[];
 fail(Array.isArray(o.actualVendorCalls)&&o.actualVendorCalls.length===4&&same(readdirSync(output).filter(n=>/^call-\d+\.attempt\.private\.json$/.test(n)).sort(),tools.map((_,i)=>'call-'+String(i+1).padStart(3,'0')+'.attempt.private.json'))&&!readdirSync(v.scenario).some(n=>/^(?:guard-E[12]-consumed|E[12]-authorization)\.private\.json$/.test(n)));
 let previous=ms(ready.at);
 for(const [i,tool]of tools.entries()){
  const prefix=resolve(output,'call-'+String(i+1).padStart(3,'0')),row=o.actualVendorCalls[i];
  // The root receipt pins the result's post; post bytes in turn name the raw/attempt paths.
  const post=pinned(r.evidence[basename(prefix+'.private.json')],prefix+'.private.json');
  const attempt=a['call'+(i+1)+'Attempt'],raw=a['call'+(i+1)+'Raw'];
  fail(closed(row,['ordinal','batch','name','args','atMs'])&&row.ordinal===i+1&&row.batch==='B1'&&row.name===tool&&Number.isSafeInteger(row.atMs)&&row.atMs>=ms(post.callCompleted)&&row.atMs<=ms(t.at)&&same(row.args,i===0?{}:post.args)&&
   post.tool===tool&&attempt.tool===tool&&post.runID===ready.runID&&attempt.runID===ready.runID&&post.batch==='B1'&&attempt.batch==='B1'&&attempt.sourceSha256===e.source.sha256&&same(post.args,attempt.args)&&attempt.callStarted===post.callStarted&&same(post.precedingActions,[])&&post.isError===false&&(!Object.hasOwn(raw,'isError')||raw.isError===false)&&Number.isSafeInteger(post.outerProtocolID)&&post.outerProtocolID>0&&post.innerProtocolID===i+1&&attempt.outerProtocolID===post.outerProtocolID&&attempt.innerProtocolID==='UNKNOWN'&&post.freshness==='UNKNOWN'&&ms(post.callStarted)>=previous&&ms(post.callCompleted)>=ms(post.callStarted)&&ms(post.callCompleted)<=ms(t.at));
  previous=ms(post.callCompleted);posts.push(post);raws.push(raw);
 }
 fail(closed(o.begin,['args','raw','at'])&&same(o.begin.args,{consentMode:'finite-explicit-consent',runID:ready.runID,source:e.source.path.replaceAll('\\','/')})&&Number.isSafeInteger(o.begin.at)&&o.begin.at>=ms(ready.at)&&o.begin.at<=ms(posts[0].callStarted));
 const begun=callerDatePacket(o.begin.raw,fail);
 fail(closed(begun,['runID','output','deadline'])&&begun.runID===ready.runID&&begun.output===output&&ms(begun.deadline)>=ms(t.at)&&ms(begun.deadline)<=l.providerClock.deadlineMs&&Number.isSafeInteger(o.providerFinish.at)&&o.providerFinish.at>=ms(t.at)&&o.providerFinish.at<=ms(o.atUtc));
 fail(raws.slice(0,3).every(raw=>object(raw.structuredContent))&&same(posts[0].args,{include_remediation:true})&&same(posts[1].args,{})&&raws[0].structuredContent?.ok===true&&raws[1].structuredContent?.approval_required_for_all===true&&raws[1].structuredContent.approval_token_configured===false);
 const window=raws[2].structuredContent;pos(window?.pid);pos(window?.windowId);pos(c.sessionId);
 fail(window.bundleId==='java.exe'&&window.isFocused===true&&closed(window.bounds,['x','y','width','height'])&&Object.values(window.bounds).every(Number.isSafeInteger)&&window.bounds.width>0&&window.bounds.height>0&&same(posts[2].args,{window_id:window.windowId})&&same(posts[3].args,{target_window_id:window.windowId,quality:0,width:window.bounds.width})&&owner.campaign.windowId===window.windowId);
 fail(same(e.initialOwned,{path:source.ownedReceiptPath,sha256:source.ownedReceiptSha256}));
 const snapshots=[a.initialOwned,a.frameOwned,a.owned];
 fail(snapshots.every(s=>Array.isArray(s.processes)&&s.processes.every((/** @type {any} */ x)=>object(x)&&typeof x.executable==='string'&&Number.isSafeInteger(x.sessionId)&&x.sessionId>0)));
 const games=snapshots.map(s=>s.processes.filter((/** @type {any} */ x)=>x.executable===resolve(c.installation,'jre/bin/java.exe')));
 fail(closed(a.initialOwned,['at','sourceSnapshot','processes'])&&closed(a.initialOwned.sourceSnapshot,['path','sha256'])&&a.initialOwned.sourceSnapshot.path===e.owned.path&&typeof a.initialOwned.sourceSnapshot.sha256==='string'&&/^[a-f0-9]{64}$/.test(a.initialOwned.sourceSnapshot.sha256));
 fail(games.every(rows=>rows?.length===1));const first=games[0][0];pos(first.pid);pos(first.parentPid);
 fail(typeof first.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(first.kernelCreationFileTime)&&first.pid===window.pid&&same(owner.ownedPids,[first.pid])&&first.sessionId===c.sessionId&&ms(first.creationTime)>=published&&ms(first.creationTime)<=ms(ready.at));
 for(const [i,rows]of games.entries()){
  const x=rows[0],s=snapshots[i],handle=x.mainWindowHandle;
  fail((i===0||(s.controllerPid===c.controllerPid&&s.controllerCreationTime===c.controllerCreationTime&&s.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime))&&['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(k=>same(x[k],first[k]))&&x.inSupervisorJob===true&&Array.isArray(x.supervisorJobPids)&&x.supervisorJobPids.every((/** @type {any} */ id)=>Number.isSafeInteger(id)&&id>0)&&[first.pid,c.controllerPid,...c.supervisorJobPids,x.parentPid].every(id=>x.supervisorJobPids.includes(id))&&x.parentPid!==x.pid&&ms(x.creationTime)<=ms(s.at)&&((Number.isSafeInteger(handle)&&handle>0)||(typeof handle==='string'&&/^[1-9][0-9]*$/.test(handle)))&&Number(handle)===window.windowId);
 }
 const prefix=resolve(output,'capture-001'),image=r.originalInspection;
 fail(closed(image,['file','sha256','dimensions','assertion','localPoint','screenPointFromActualGeometry','freshness','inputSent'])&&image.file==='capture-001.png'&&image.freshness==='UNKNOWN'&&image.inputSent===false&&typeof image.assertion==='string'&&image.assertion.length>0);
 const png=file(prefix+'.png',32*1024*1024),metadata=pinned(r.evidence['capture-001.metadata.private.json'],prefix+'.metadata.private.json'),capture=pinned(r.evidence['capture-001.raw.private.json'],prefix+'.raw.private.json',32*1024*1024),dimensions=completePng(png.bytes),images=capture.content?.filter((/** @type {any} */ x)=>x.type==='image');
 fail(png.sha256===image.sha256&&same(r.evidence['capture-001.png'],{path:prefix+'.png',sha256:png.sha256})&&same(capture,raws[3])&&file(prefix+'.raw.private.json',32*1024*1024).bytes.equals(file(resolve(output,'call-004.raw.private.json'),32*1024*1024).bytes)&&images?.length===1&&object(images[0])&&images[0].mimeType==='image/png'&&images[0].data===png.bytes.toString('base64')&&metadata.captureReceipt===true&&metadata.freshness==='UNKNOWN'&&metadata.rawPath===prefix+'.raw.private.json'&&metadata.rawSha256===r.evidence['capture-001.raw.private.json'].sha256&&metadata.pngPath===prefix+'.png'&&metadata.pngSha256===png.sha256&&same(metadata.window,window)&&same(metadata.dimensions,dimensions)&&same(image.dimensions,dimensions)&&dimensions.width===window.bounds.width&&dimensions.height===window.bounds.height);
 for(const key of ['tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID'])fail(same(metadata[key],posts[3][key]));
 assertCallerDateGeometry({...ctx,window,dimensions,posts,raws});
 const absence=a.pidAbsence,ids=[...new Set([c.controllerPid,...c.supervisorJobPids,...snapshots.flatMap(s=>s.processes.flatMap((/** @type {any} */ x)=>[x.pid,...x.supervisorJobPids])),...owner.ownedPids])].sort((x,y)=>x-y);
 fail(closed(absence,['schemaVersion','kind','atUtc','sourceOwnedSnapshot','rows','allAbsent','zeroLiveOwnedGameProcesses','originalCallerSession','originalCallerTerminalReaped'])&&absence.schemaVersion===1&&absence.kind==='ACTUAL_OWNED_TERMINAL_PID_ABSENCE'&&same(absence.sourceOwnedSnapshot,e.initialOwned)&&Array.isArray(absence.rows)&&same(absence.rows.map((/** @type {any} */ x)=>x.pid).sort((/** @type {number} */ x,/** @type {number} */ y)=>x-y),ids)&&absence.rows.every((/** @type {any} */ x)=>closed(x,['pid','absent'])&&Number.isSafeInteger(x.pid)&&x.pid>0&&x.absent===true)&&absence.allAbsent===true&&absence.zeroLiveOwnedGameProcesses===true&&absence.originalCallerSession===a.caller.session&&absence.originalCallerTerminalReaped===true&&ms(absence.atUtc)>=ms(t.at)&&ms(a.protection.at)>=ms(t.at));
 const close=r.closure;
 fail(closed(close,['providerB1','rootCaller','ownedStopAtUtc','ownedPidAbsence','ordinaryExit','allOwnedTerminalReaped','originalCallerDeadlineMs'])&&same(close.providerB1,{verifierPid:t.verifierPid,status:0,timedOut:false,terminalObserved:true,terminalReaped:true,at:t.at,verdict:'PASS'})&&same(close.rootCaller,{session:a.caller.session,terminalChunk:a.caller.actual.chunk_id,exitCode:1,timedOut:false,terminalReaped:true,verdict:'FAIL',reason:'MAINTAINED_REQUESTED_OWNED_CLEANUP'})&&same([...close.ownedPidAbsence].sort((x,y)=>x-y),ids)&&close.ordinaryExit===false&&close.allOwnedTerminalReaped===true&&close.originalCallerDeadlineMs===p.callerAdmissionDeadlineMs&&ms(close.ownedStopAtUtc)>=ms(t.at)&&ms(close.ownedStopAtUtc)<=ms(absence.atUtc)&&ms(close.ownedStopAtUtc)<=clock.effectiveDeadlineMs&&ms(r.atUtc)>=ms(a.protection.at));
 fail(same(r.protection,{protectedOriginalFiles:182,protectedSaveFiles:178,originalAndBackupHashesMatch:true,saveCensusUnchanged:true,privateAutosaveEnabled:false,privateSaveEntries:0,liveGameProcesses:0,originalWrites:false,restoration:'UNNECESSARY_ORIGINALS_UNCHANGED',sharedPreference:'Registry64/String/true unchanged'}));
 const aliases=Object.keys(e).filter(k=>!['result','rootCall','preference','supportPins','owned','helper','geometryConfiguration','geometryRunner','initialOwned','frameOwned','beforeFrame',...Object.keys(callerDateProviderNames)].includes(k));
 for(const key of aliases)fail(same(r.evidence[basename(e[key].path)],e[key]));
 const expectedEvidence=[...aliases.map(k=>basename(e[k].path)),'caller-terminal.actual.private.json','progress.pre-native-preparation.original.private.json','capture-001.png','capture-001.metadata.private.json','capture-001.raw.private.json',...tools.map((_,i)=>'call-'+String(i+1).padStart(3,'0')+'.private.json')];
 fail(closed(r.evidence,expectedEvidence));
 for(const name of ['caller-terminal.actual.private.json','progress.pre-native-preparation.original.private.json'])pinned(r.evidence[name],resolve(v.scenario,name));
 fail(same(pinned(r.evidence['caller-terminal.actual.private.json'],resolve(v.scenario,'caller-terminal.actual.private.json')),a.caller.actual));
 assertPrivateCaller(ctx);
}

/** @param {any} ctx */
function assertPrivateCaller({v,a,e,pinned,fail,powershell}){
 const p=a.plan,c=a.controller;
 const historic=(/** @type {string} */ name)=>{const pin=p.preclockHelpers?.[name];fail(closed(pin,['path','sha256'])&&pin.path===resolve(v.scenario,name)&&file(pin.path,262144).sha256===pin.sha256);return pin;};
 const caller=historic('launch-owned-caller.private.ps1'),stager=historic('record-root-call-and-stage-B1.private.ps1'),orchestration=historic('launch-and-stage.orchestration.proposed.private.js');
 fail(same(historic('construct-actual-phase-inputs.private.mjs'),e.helper));
 for(const [pin,name]of [[p.historyAdmission,'root-history-baseline.admitted.private.json'],[p.fixtureBinding,'fixture-binding.private.json']])fail(closed(pin,['path','sha256'])&&pin.path===resolve(v.scenario,name)&&file(pin.path,262144).sha256===pin.sha256);
 fail(same(p.historyAdmission,a.lease.historyAdmission)&&p.rootCallContract?.path===e.rootCall.path&&p.rootCallContract.key===a.rootCall.key);
 const path=resolve(v.scenario,'root-postlaunch-mechanical-delegation.private.json'),pin={path,sha256:file(path).sha256},d=pinned(pin,path);
 fail(d.schemaVersion===1&&d.kind==='ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'&&d.issuer==='/root'&&d.driver===v.driver&&d.planSha256===e.plan.sha256&&d.leaseSha256===e.lease.sha256&&d.helperSha256===e.helper.sha256&&d.callerSha256===caller.sha256&&d.stagerSha256===stager.sha256&&d.orchestrationSha256===orchestration.sha256&&d.historyAdmissionSha256===p.historyAdmission.sha256&&same(d.allowedModes,['init','phase B1']));
 const q=(/** @type {string} */ s)=>"'"+s.replaceAll("'","''")+"'";
 fail(a.rootCall.value.args.cmd==='& '+q(powershell)+' -NoProfile -NonInteractive -File '+q(caller.path)+' -PlanSha256 '+q(e.plan.sha256)+' -LeaseSha256 '+q(e.lease.sha256)+' -DelegationSha256 '+q(pin.sha256)+' -HelperSha256 '+q(e.helper.sha256)+' -HistoryAdmissionSha256 '+q(p.historyAdmission.sha256));
 const outer=a.launch.result.steps[0];fail(outer.command===powershell&&same(outer.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',v.scenario,'-BindingPath',p.fixtureBinding.path,'-BindingSha256',p.fixtureBinding.sha256,'-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000'])&&outer.cwd==='.');
}
