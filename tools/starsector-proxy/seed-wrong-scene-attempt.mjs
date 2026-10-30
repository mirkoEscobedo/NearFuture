// @ts-check
// Closed historical Play -> New Game wrong-scene custody; no input or seed authority.
import {resolve} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,file} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';
import {inspectSeedCreationLauncher} from './campaign-authority.mjs';

/** @param {{v:any,a:any,e:any,output:string,pinned:(p:any,s:string,n?:number)=>any,fail:(v:any)=>void,node:string,powershell:string}} context */
export function assertWrongSceneFailure({v,a,e,output,pinned,fail,node,powershell}){
 const {result:r,ledger:l,plan:p,lease,controller:c,observation:o,e1Source:source,e1Claim:claim,e1Plan:phase,e1Lease:phaseLease}=a;
 const before=p.historyBaseline,owner=claim.binding,batch='E1',pending=l.pending;
 fail(Number.isSafeInteger(l.providerClock?.startMs));
 fail(Number.isSafeInteger(c.sessionId)&&c.sessionId>0&&typeof c.controllerKernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(c.controllerKernelCreationFileTime));
 fail(closed(before,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&
  Object.values(before).every(n=>Number.isSafeInteger(n)&&n>=0)&&before.sentActions<=before.reservedActions&&
  same(l.history,{...before,reservedActions:before.reservedActions+2,sentActions:before.sentActions+2})&&
  same(v.actions.currentRunContext,{overallAttempts:before.overallAttempts+1,uiAttempts:before.uiAttempts+1})&&
  r.assertionResult==='FAIL'&&same(r.counts,{reservedThisRun:2,sentThisRun:2,clicks:2,activations:0,providers:2,
   B1Calls:11,E1Calls:10,totalCalls:21,originals:4,B1WaitMs:10000,B1OptionalRecaptureUsed:0})&&
  same(r.providerStatus,{B1:'TERMINAL_PASS_REAPED',E1:'TERMINAL_PASS_REAPED',E2:'NOT_STARTED'})&&
  same(r.seed,{created:false,modernCloseoutCreated:false,stabilityModeExecuted:false,semanticAcceptance:'NOT_RUN'}));
 fail(closed(source,['schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256'])&&
  source.schemaVersion===6&&source.scenario===v.scenario&&source.batch===batch&&
  source.planPath===e.e1Plan.path&&source.planSha256===e.e1Plan.sha256&&source.leasePath===e.e1Lease.path&&source.leaseSha256===e.e1Lease.sha256&&
  source.ownedReceiptPath===e.e1Owned.path&&source.ownedReceiptSha256===e.e1Owned.sha256&&
  closed(claim,['sourcePath','sourceSha256','binding'])&&claim.sourcePath===e.e1Source.path&&claim.sourceSha256===e.e1Source.sha256&&
  owner?.schemaVersion===6&&owner.batch===batch&&owner.sourcePath===e.e1Source.path&&owner.sourceSha256===e.e1Source.sha256&&
  owner.scenario===v.scenario&&owner.workspaceRoot===v.sourceWorkspace&&owner.planSha256===e.e1Plan.sha256&&
  owner.originalPlanSha256===e.plan.sha256&&owner.controllerSha256===e.controller.sha256&&/^[A-Za-z0-9_-]+$/.test(owner.runID)&&
  same(l.provider,{batch,runID:owner.runID,sourceSha256:e.e1Source.sha256}));
 fail(phase.scenarioId===p.scenarioId&&phase.paths?.scenario===v.scenario&&phase.source?.qualifiedHead===v.sourceHead&&
  same(phase.historyBaseline,before)&&phase.absoluteMaxReservedActions===p.absoluteMaxReservedActions&&
  same(phase.seedCreation,p.seedCreation)&&same(phase.campaign,owner.campaign)&&same(phase.campaign,phaseLease.campaign)&&
  same(phase.campaign.historyBaseline,before)&&phase.campaign.maxReservedActions===p.absoluteMaxReservedActions&&
  same(phase.campaign.rootCallRecord,a.rootCall)&&phase.campaign.rootClock.sessionWatchdogMs===480000&&
  phaseLease.schemaVersion===6&&phaseLease.authorized===true&&phaseLease.issuer==='/root'&&phaseLease.driver===v.driver&&
  phaseLease.phase==='DISPOSABLE_SEED_CREATION'&&phaseLease.planSha256===e.e1Plan.sha256&&
  phaseLease.maxActions===8&&phaseLease.maxProxyInvocations===3&&phaseLease.ownedGameHardTimeoutMs===480000&&
  phaseLease.campaignLoadSaveAllowed===false&&phaseLease.supportPinsPath===e.supportPins.path&&phaseLease.supportPinsSha256.toLowerCase()===e.supportPins.sha256&&
  phaseLease.fixtureReceiptSha256===lease.fixtureReceiptSha256&&same(phaseLease.seedCreation,p.seedCreation)&&
  same(phase.b1MenuReceipt,{path:e.b1Menu.path,sha256:e.b1Menu.sha256})&&same(phase.b1MenuReceipt,phaseLease.b1MenuReceipt));
 const clock=owner.campaign.rootClock,producer=owner.campaign.producerWitness,
  birth=Date.parse(c.controllerCreationTime),published=Date.parse(c.at),outer=Date.parse(c.conservativeOuterDeadlineUtc),ended=Date.parse(c.controllerDeadlineUtc);
 fail([clock.startMs,clock.deadlineMs,clock.effectiveDeadlineMs,p.callerAdmissionDeadlineMs,birth,published,outer,ended].every(Number.isSafeInteger)&&
  clock.recordKey===a.rootCall.key&&clock.startMs===a.rootCall.value.startMs&&clock.deadlineMs===clock.startMs+480000&&
  clock.sessionWatchdogMs===480000&&clock.cleanupReserveMs===30000&&clock.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&
  clock.originalCallerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&lease.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&
  clock.effectiveDeadlineMs===Math.min(clock.deadlineMs,p.callerAdmissionDeadlineMs,outer,ended)&&
  clock.startMs<=birth&&birth<=published&&published<ended&&ended<=outer&&outer<=birth+480000&&
  clock.producerRecordKey==='controller-start:'+e.controller.sha256&&producer.producerRecordKey===clock.producerRecordKey&&
  producer.controllerPid===c.controllerPid&&producer.controllerCreationTime===c.controllerCreationTime&&
  producer.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime&&producer.conservativeOuterDeadlineUtc===c.conservativeOuterDeadlineUtc&&
  producer.controllerDeadlineUtc===c.controllerDeadlineUtc&&producer.effectiveDeadlineMs===clock.effectiveDeadlineMs&&producer.sessionWatchdogMs===480000);
 const initialOwned=pinned({path:a.source.ownedReceiptPath,sha256:a.source.ownedReceiptSha256},resolve(v.scenario,'driver-B1-owned.private.json'));
 const rows=[initialOwned,a.e1Owned,a.owned].map(data=>data.processes?.filter((/** @type {any} */ row)=>row.pid===o.ownedPid));
 fail(rows.every(list=>list?.length===1));const [initial,current,last]=rows.map(list=>list[0]);
 fail(Number.isSafeInteger(initial.pid)&&initial.pid>0&&Number.isSafeInteger(initial.parentPid)&&initial.parentPid>0&&
  typeof initial.creationTime==='string'&&Number.isSafeInteger(Date.parse(initial.creationTime))&&
  typeof initial.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(initial.kernelCreationFileTime)&&
  initial.executable===resolve(c.installation,'jre/bin/java.exe')&&initial.sessionId===c.sessionId);
 for(const [i,row]of [initial,current,last].entries()){
  fail(['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(row[key],initial[key]))&&
   row.inSupervisorJob===true&&Array.isArray(row.supervisorJobPids)&&row.supervisorJobPids.every(Number.isSafeInteger)&&
   [c.controllerPid,initial.pid,...c.supervisorJobPids].every(id=>row.supervisorJobPids.includes(id))&&
   (Number.isSafeInteger(row.mainWindowHandle)||(typeof row.mainWindowHandle==='string'&&/^[1-9][0-9]*$/.test(row.mainWindowHandle)))&&
   Number(row.mainWindowHandle)===(i===0?a.claim.binding.campaign.windowId:owner.campaign.windowId));
 }
 // This public historical seam checks complete B1 original/claim/phase/owner/clock and real provider closure.
 const handoff=inspectSeedCreationLauncher(owner),play=handoff.action;
 fail(a.b1Menu.inspection.inspectedAtMs<=l.providerClock.startMs&&a.b1Menu.provider.observedClosedAtMs<=l.providerClock.startMs&&
  Date.parse(initial.creationTime)>=published&&Date.parse(initial.creationTime)<=play.knownResultAtMs&&play.providerClock.startMs>=clock.startMs);
 fail(handoff.receiptSha256===e.b1Menu.sha256&&same(owner.b1MenuReceipt,{...e.b1Menu,newGameCoordinate:handoff.loadCoordinate})&&
  same(l.resolvedB1,play)&&play.reservationID===e.controller.sha256+':'+(before.reservedActions+1)&&
  same(play.wizardStep,{ordinal:1,stage:'PLAY'})&&
  closed(pending,['batch','runID','sourceSha256','reservationID','tool','args','status','wizardStep','sentAtMs','knownResultAtMs'])&&
  pending.batch===batch&&pending.runID===owner.runID&&pending.sourceSha256===e.e1Source.sha256&&
  pending.reservationID===e.controller.sha256+':'+l.history.reservedActions&&pending.tool==='left_click'&&pending.status==='AWAITING_AFTER'&&
  same(pending.wizardStep,{ordinal:1,stage:'NEW_GAME'})&&same(pending.args.coordinate,handoff.loadCoordinate)&&
  !Object.hasOwn(l,'resolvedE1')&&!Object.hasOwn(l,'resolvedE2')&&(!l.resolvedActions||same(l.resolvedActions,[])));
 const secondOutput=resolve(v.sourceWorkspace,'.tmp/starsector-proxy',owner.runID);
 const ready=pinned(e.e1ProviderReady,resolve(secondOutput,'guarded-http-ready.private.json'));
 const terminal=pinned(e.e1ProviderTerminal,resolve(secondOutput,'owned-provider-terminal.private.json'));
 const spool=pinned(e.e1ProviderSpool,resolve(secondOutput,'owned-provider.stdout.private.txt'));
 fail(closed(e.e1ProviderStderr,['path','sha256'])&&e.e1ProviderStderr.path===resolve(secondOutput,'owned-provider.stderr.private.txt'));
 const stderr=file(e.e1ProviderStderr.path),step=terminal.steps?.[0];
 const critical=(/** @type {any} */ x)=>({command:x?.command,args:x?.args,cwd:x?.cwd,processOwnership:x?.processOwnership,status:x?.status,timedOut:x?.timedOut});
 fail(stderr.sha256===e.e1ProviderStderr.sha256&&stderr.bytes.length===0&&terminal.terminalObserved===true&&terminal.cleanupUnknown===false&&
  terminal.terminalBoundToChild===true&&terminal.closeFailed===false&&terminal.overflow===false&&terminal.spawnFailed===false&&
  terminal.signal===null&&terminal.verifierStatus===0&&terminal.verdict==='PASS'&&terminal.terminalReceipt===e.e1ProviderTerminal.path&&
  terminal.steps?.length===1&&step.command===node&&same(step.args,[resolve(v.sourceWorkspace,'tools/starsector-proxy/guarded-http-child.mjs')])&&
  step.cwd==='.'&&step.processOwnership==='windows-job-object'&&step.status===0&&step.timedOut===false&&
  spool.schemaVersion===1&&spool.command==='verify'&&spool.ok===true&&spool.result?.verdict==='PASS'&&spool.result.steps?.length===1&&
  same(critical(spool.result.steps[0]),critical(step))&&ready.schemaVersion===1&&ready.batch===batch&&ready.runID===owner.runID&&
  ready.sourceSha256===e.e1Source.sha256&&ready.startupTimeoutMs===20000&&ready.consentMode==='finite-explicit-consent'&&
  Number.isSafeInteger(terminal.verifierPid)&&terminal.verifierPid>0&&ready.node?.executable===node&&ready.helper?.executable===powershell&&
  ready.node.parentPid===terminal.verifierPid&&ready.helper.parentPid===ready.node.pid&&ready.node.pid!==ready.helper.pid&&
  ready.node.sessionId===c.sessionId&&ready.helper.sessionId===c.sessionId&&ready.supervisorJobPids?.includes(ready.node.pid)&&
  ready.supervisorJobPids.includes(ready.helper.pid)&&ready.query==='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)');
 const time=(/** @type {any} */ x)=>{fail(typeof x==='string'&&Number.isSafeInteger(Date.parse(x)));return Date.parse(x);};
 fail(time(ready.at)>=time(a.providerTerminal.at)&&time(terminal.at)>=time(ready.at)&&
  time(o.atUtc)>=time(terminal.at)&&time(o.atUtc)<=time(a.protection.at)&&time(a.pidAbsence.atUtc)>=time(terminal.at)&&
  time(ready.node.creationTime)>=l.providerClock.startMs&&time(ready.helper.creationTime)>=time(ready.node.creationTime)&&
  time(ready.helper.creationTime)<=time(ready.at)&&time(terminal.at)<=time(terminal.deadline)&&
  time(terminal.deadline)<=l.providerClock.deadlineMs&&
  l.providerClock.deadlineMs===Math.min(l.providerClock.startMs+180000,owner.campaign.rootClock.effectiveDeadlineMs-30000));
 for(const id of [terminal.verifierPid,ready.node.pid,ready.helper.pid,...ready.supervisorJobPids,...owner.ownedPids])
  fail(Number.isSafeInteger(id)&&id>0&&a.pidAbsence.queriedExactRecordedPids.includes(id));
 fail(o.schemaVersion===1&&o.kind==='ACTUAL_DRIVER_WRONG_SCENE_STOP'&&o.category==='POSTCONDITION_FAILED'&&o.verdict==='FAIL'&&
  o.scenarioId===p.scenarioId&&o.driver===v.driver&&o.sourceHead===v.sourceHead&&same(o.actualRunContext,r.actualRunContext)&&
  same(o.actualGuardHistory,l.history)&&o.activations===0&&o.totalCalls===21&&o.totalOriginals===4&&o.B1LoadingWaitMs===10000&&
  o.B1OptionalRecaptureUsed===0&&o.E2==='NOT_STARTED'&&o.seedClaim===false&&o.allPriorFailuresRetained===true&&
  same(o.ledger,e.ledger)&&same(o.rootCall,e.rootCall)&&same(o.B1MenuProof,e.b1Menu)&&Array.isArray(o.providers)&&o.providers.length===2);
 const toolsB1=['doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','list_windows','get_window','screenshot'];
 const toolsE1=['doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot'];
 const typedWindow=(/** @type {any} */ w)=>w&&Number.isSafeInteger(w.windowId)&&w.windowId>0&&Number.isSafeInteger(w.pid)&&w.pid>0&&
  w.bundleId==='java.exe'&&w.isFocused===true&&closed(w.bounds,['x','y','width','height'])&&Object.values(w.bounds).every(Number.isSafeInteger)&&w.bounds.width>0&&w.bounds.height>0;
 /** @param {any} row @param {string[]} tools @param {string} out @param {any} provider @param {any} ended @param {any} sourcePin @param {any} action */
 function calls(row,tools,out,provider,ended,sourcePin,action){
  fail(closed(row,['batch','ready','terminal','calls','originals'])&&row.batch===provider.batch&&same(row.ready,provider.batch==='B1'?e.providerReady:e.e1ProviderReady)&&
   same(row.terminal,provider.batch==='B1'?e.providerTerminal:e.e1ProviderTerminal)&&row.calls?.length===tools.length&&row.originals?.length===2&&
   same(readdirSync(out).filter(n=>/^call-\d+\.attempt\.private\.json$/.test(n)).sort(),tools.map((_,i)=>'call-'+String(i+1).padStart(3,'0')+'.attempt.private.json')));
  /** @type {any[]} */const posts=[],raws=[];let last=time(provider.at);
  for(const [i,tool]of tools.entries()){
   const ref=row.calls[i],prefix=resolve(out,'call-'+String(i+1).padStart(3,'0'));fail(closed(ref,['post','raw','attempt']));
   const post=pinned(ref.post,prefix+'.private.json'),attempt=pinned(ref.attempt,prefix+'.attempt.private.json'),raw=pinned(ref.raw,prefix+'.raw.private.json',32*1024*1024);
   fail(post.tool===tool&&attempt.tool===tool&&post.runID===provider.runID&&attempt.runID===provider.runID&&post.batch===row.batch&&attempt.batch===row.batch&&
    attempt.sourceSha256===sourcePin.sha256&&same(attempt.args,post.args)&&attempt.callStarted===post.callStarted&&post.isError===false&&
    (raw.isError===undefined||raw.isError===false)&&time(post.callStarted)>=last&&time(post.callCompleted)>=time(post.callStarted)&&
    time(post.callCompleted)<=time(ended.at)&&same(post.precedingActions,i<=6?[]:[posts[6]]));last=time(post.callCompleted);posts.push(post);raws.push(raw);
  }
  fail(raws[0].structuredContent?.ok===true&&raws[1].structuredContent?.approval_required_for_all===true&&raws[1].structuredContent.approval_token_configured===false&&
   closed(action.args,['coordinate','target_app','target_window_id','focus_strategy'])&&action.args.target_app==='java.exe'&&action.args.focus_strategy==='strict'&&
   same(posts[6].args,action.args)&&same(raws[6].content,[{type:'text',text:'Clicked ('+action.args.coordinate.join(', ')+')'}])&&
   Number.isSafeInteger(action.sentAtMs)&&Number.isSafeInteger(action.knownResultAtMs)&&time(posts[6].callStarted)<=action.sentAtMs&&
   action.knownResultAtMs>=action.sentAtMs&&action.knownResultAtMs<=time(posts[6].callCompleted));
  const original=raws[2].structuredContent,current=raws[row.batch==='B1'?9:8].structuredContent;
  fail(typedWindow(original)&&typedWindow(current)&&original.pid===current.pid&&original.pid===o.ownedPid&&action.args.target_window_id===original.windowId);
  for(const i of [4,7])fail(raws[i].structuredContent?.app?.pid===original.pid&&raws[i].structuredContent.app.bundleId==='java.exe');
  for(const [i,w]of [[2,original],[5,original],[row.batch==='B1'?9:8,current]])fail(typedWindow(raws[i].structuredContent)&&same(raws[i].structuredContent,w)&&same(posts[i].args,{window_id:w.windowId}));
  if(row.batch==='B1')fail(raws[8].structuredContent?.windows?.filter((/** @type {any} */ w)=>same(w,current)).length===1&&time(posts[10].callStarted)-time(posts[6].callCompleted)>=10000);
  else fail(same(original,current));
  fail(action.args.coordinate.length===2&&action.args.coordinate.every(Number.isSafeInteger)&&action.args.coordinate[0]>=original.bounds.x&&
   action.args.coordinate[0]<original.bounds.x+original.bounds.width&&action.args.coordinate[1]>=original.bounds.y&&action.args.coordinate[1]<original.bounds.y+original.bounds.height);
  for(const [i,index]of [3,tools.length-1].entries()){
   const ref=row.originals[i],prefix=resolve(out,'capture-'+String(i+1).padStart(3,'0')),w=i===0?original:current;
   fail(closed(ref,['png','metadata','raw','originalInspected','freshness'])&&ref.originalInspected===true&&ref.freshness==='UNKNOWN'&&closed(ref.png,['path','sha256'])&&ref.png.path===prefix+'.png');
   const png=file(ref.png.path,32*1024*1024),dimensions=completePng(png.bytes),meta=pinned(ref.metadata,prefix+'.metadata.private.json'),raw=pinned(ref.raw,prefix+'.raw.private.json',32*1024*1024);
   fail(png.sha256===ref.png.sha256&&same(raw,raws[index])&&raw.content?.filter((/** @type {any} */ x)=>x.type==='image').length===1&&
    raw.content.find((/** @type {any} */ x)=>x.type==='image').mimeType==='image/png'&&raw.content.find((/** @type {any} */ x)=>x.type==='image').data===png.bytes.toString('base64')&&
    meta.captureReceipt===true&&meta.pngPath===ref.png.path&&meta.pngSha256===ref.png.sha256&&meta.rawPath===ref.raw.path&&meta.rawSha256===ref.raw.sha256&&
    meta.runID===provider.runID&&meta.batch===row.batch&&meta.tool==='screenshot'&&meta.callStarted===posts[index].callStarted&&meta.callCompleted===posts[index].callCompleted&&
    same(meta.args,posts[index].args)&&same(meta.precedingActions,posts[index].precedingActions)&&same(meta.window,w)&&same(meta.dimensions,dimensions)&&
    same(posts[index].args,{target_window_id:w.windowId,quality:0,width:w.bounds.width})&&dimensions.width===w.bounds.width&&dimensions.height===w.bounds.height);
  }
  return {original,current};
 }
 const first=calls(o.providers[0],toolsB1,output,a.providerReady,a.providerTerminal,e.source,play);
 const second=calls(o.providers[1],toolsE1,secondOutput,ready,terminal,e.e1Source,pending);
 fail(same(first.current,second.original)&&o.launcherWindow===first.original.windowId&&o.gameWindow===second.current.windowId&&
  r.originals?.allFourInspected===true&&r.originals.nativeLauncherWidth===first.original.bounds.width&&r.originals.nativeGameWidth===second.current.bounds.width&&
  r.originals.captureReceipt===true&&r.originals.freshness==='UNKNOWN'&&r.originals.wrongScenePngSha256===o.providers[1].originals[1].png.sha256&&
  r.originals.actualVisualTransitionProven===true&&r.originals.scenarioSuccess===false&&!readdirSync(v.scenario).some(n=>/^guard-E2-consumed\.private\.json$/.test(n)));
 fail(Array.isArray(r.actions)&&r.actions.length===2&&Array.isArray(o.actions)&&o.actions.length===2);
 for(const [i,action]of [play,pending].entries()){
  const result=r.actions[i],observed=o.actions[i];
  fail(result.batch===action.batch&&result.guardStage===(i===0?'PLAY':'NEW_GAME')&&result.reservationOrdinal===before.reservedActions+i+1&&
   same(result.args,action.args)&&result.acknowledged===true&&result.assertion===(i===0?'PASS':'FAIL')&&
   observed.batch===action.batch&&same(observed.args,action.args)&&observed.acknowledged===true&&observed.reservationOrdinal===result.reservationOrdinal);
 }
 fail(r.actions[1].ledgerStatus==='AWAITING_AFTER'&&r.actions[1].uncertain===false&&o.actions[1].expectedAssertion==='FAIL');
}
