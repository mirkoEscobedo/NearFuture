// @ts-check
// Generated wrong-scene boundaries only; never a native run or usable seed.
import {mkdirSync,unlinkSync,writeFileSync} from 'node:fs';
import {resolve,basename} from 'node:path';
import {knownPlayHistoryFixture} from './seed-known-play-history-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {persistDirectFrame} from '../direct-frames.mjs';

/** @param {{phaseCallerDeadlineDelta?:number,sessionId?:unknown,changeB1Owned?:(v:any)=>void,changeE1Owned?:(v:any)=>void,
 * b1InspectedOffsetMs?:number,b1ClosedObservedOffsetMs?:number,gameBirthOffsetMs?:number,e1NodeBirthOffsetMs?:number,e1HelperBirthOffsetMs?:number,
 * changeE1Args?:(v:any)=>void,changeE1Front?:(v:any)=>void,changeCampaign?:(v:any)=>void,changeRootCall?:(v:any)=>void,
 * changeE1Plan?:(v:any)=>void,changeE1Lease?:(v:any)=>void,changeCopiedFrame?:(v:any)=>void,
 * changeOwner?:(v:any)=>void,b1ProviderStartOffsetMs?:number,observedPid?:number}} [options] */
export function wrongSceneHistoryFixture(options={}){
 const anchor=Date.now()-30000,iso=(/** @type {number} */ n)=>new Date(anchor+n).toISOString();
 const before={overallAttempts:44,uiAttempts:41,reservedActions:74,sentActions:72};
 const f=knownPlayHistoryFixture({targetApp:'java.exe',historyBaseline:before,anchorMs:anchor,sessionId:options.sessionId});
 const paths=f.evidencePaths,s=f.priorScenario,prior=value(paths.plan),b1=value(paths.source),b1Claim=value(paths.claim);
 const initialOwned=value(b1.ownedReceiptPath);options.changeB1Owned?.(initialOwned);json(b1.ownedReceiptPath,initialOwned);
 if(options.changeOwner){
  options.changeOwner(initialOwned.processes[0]);json(b1.ownedReceiptPath,initialOwned);
  const owned=value(paths.owned);options.changeOwner(owned.processes[0]);json(paths.owned,owned);
 }
 if(options.gameBirthOffsetMs!==undefined){
  initialOwned.processes[0].creationTime=iso(options.gameBirthOffsetMs);json(b1.ownedReceiptPath,initialOwned);
  const owned=value(paths.owned);owned.processes[0].creationTime=iso(options.gameBirthOffsetMs);json(paths.owned,owned);
 }
 b1.ownedReceiptSha256=ref(b1.ownedReceiptPath).sha256;
 const phase=value(b1.planPath),campaign=phase.campaign,controller=value(paths.controller);
 prior.callerAdmissionDeadlineMs=campaign.rootClock.originalCallerAdmissionDeadlineMs;
 json(paths.plan,prior);const lease=value(paths.lease);lease.planSha256=ref(paths.plan).sha256;
 lease.callerAdmissionDeadlineMs=prior.callerAdmissionDeadlineMs;json(paths.lease,lease);
 controller.rootLeaseSha256=ref(paths.lease).sha256;json(paths.controller,controller);
 const producerKey='controller-start:'+ref(paths.controller).sha256;
 campaign.rootClock.producerRecordKey=producerKey;
 Object.assign(campaign.producerWitness,{producerRecordKey:producerKey,controllerPid:controller.controllerPid,
  conservativeOuterDeadlineUtc:controller.conservativeOuterDeadlineUtc,controllerDeadlineUtc:controller.controllerDeadlineUtc,
  effectiveDeadlineMs:campaign.rootClock.effectiveDeadlineMs,sessionWatchdogMs:480000});
 const historicalCall=value(paths.rootCall);options.changeRootCall?.(historicalCall);json(paths.rootCall,historicalCall);
 campaign.rootCallRecord=historicalCall;options.changeCampaign?.(campaign);
 campaign.rootClock.callerAdmissionDeadlineMs+=options.phaseCallerDeadlineDelta??0;
 campaign.rootClock.originalCallerAdmissionDeadlineMs+=options.phaseCallerDeadlineDelta??0;
 const predecessorPath=resolve(s,'campaign-postlaunch.private.json');
 json(predecessorPath,{...f.predecessor,plan:ref(paths.plan),lease:ref(paths.lease),controller:ref(paths.controller),
  rootClock:campaign.rootClock,producerWitness:campaign.producerWitness,rootCallRecord:campaign.rootCallRecord,
  historyBaseline:before,maxReservedActions:82});
 campaign.launchPredecessor=ref(predecessorPath);json(b1.planPath,phase);
 const b1Lease=value(b1.leasePath);b1Lease.campaign=campaign;b1Lease.planSha256=ref(b1.planPath).sha256;json(b1.leasePath,b1Lease);
 b1.planSha256=ref(b1.planPath).sha256;b1.leaseSha256=ref(b1.leasePath).sha256;json(paths.source,b1);
 b1Claim.sourceSha256=b1Claim.binding.sourceSha256=ref(paths.source).sha256;
 b1Claim.binding.originalPlanSha256=ref(paths.plan).sha256;b1Claim.binding.controllerSha256=ref(paths.controller).sha256;
 b1Claim.binding.planSha256=b1.planSha256;b1Claim.binding.campaign=campaign;json(paths.claim,b1Claim);
 const b1Run=b1Claim.binding.runID,b1Out=resolve(f.root,'.tmp/starsector-proxy',b1Run);
 const b1Ready=value(paths.providerReady);b1Ready.sourceSha256=ref(paths.source).sha256;json(paths.providerReady,b1Ready);
 const b1Terminal=value(paths.providerTerminal);b1Terminal.at=iso(15000);b1Terminal.deadline=iso(181000);json(paths.providerTerminal,b1Terminal);
 for(const row of f.knownPlay.originals)for(const key of ['png','metadata','raw'])unlinkSync(row[key].path);
 for(let i=12;i<=14;i++)for(const suffix of ['.private.json','.attempt.private.json','.raw.private.json'])
  unlinkSync(resolve(b1Out,'call-'+String(i).padStart(3,'0')+suffix));
 /** @param {string} batch @param {string} runID @param {string} sourcePath @param {any[]} rows @param {number} start */
 function calls(batch,runID,sourcePath,rows,start){
  const output=resolve(f.root,'.tmp/starsector-proxy',runID),posts=[],originals=[],refs=[];
  for(const [i,[tool,args,raw,window]]of rows.entries()){
   const ms=start+i*50+(batch==='B1'&&i>=7?10000:0);
   /** @type {any} */const post={tool,args,outerProtocolID:i+1,innerProtocolID:i+1,callStarted:iso(ms),callCompleted:iso(ms+20),
    precedingActions:i<=6?[]:[posts[6]],runID,batch,freshness:'UNKNOWN',isError:false};posts.push(post);
   const prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));
   json(prefix+'.private.json',post);json(prefix+'.raw.private.json',raw);
   json(prefix+'.attempt.private.json',{tool,args,callStarted:post.callStarted,outerProtocolID:i+1,
    innerProtocolID:'UNKNOWN',sourceSha256:ref(sourcePath).sha256,runID,batch});
   refs.push({post:ref(prefix+'.private.json'),raw:ref(prefix+'.raw.private.json'),attempt:ref(prefix+'.attempt.private.json')});
   if(tool==='screenshot'){
    const frame=persistDirectFrame({output,label:'capture-'+String(originals.length+1).padStart(3,'0'),result:raw,args,window,
     ownedPids:[window.pid],provenance:post});
    if(!frame.pngPath)throw Error('Generated original required');
    originals.push({png:ref(frame.pngPath),metadata:ref(frame.metadataPath),raw:ref(frame.rawPath),originalInspected:true,freshness:'UNKNOWN'});
   }
  }
  return {posts,originals,calls:refs};
 }
 const observedPid=options.observedPid??101;
 f.knownPlay.original.pid=f.knownPlay.replacement.pid=observedPid;
 const b1Rows=f.knownPlay.posts.slice(0,11).map((/** @type {any} */ post,/** @type {number} */ i)=>{
  const raw=value(resolve(b1Out,'call-'+String(i+1).padStart(3,'0')+'.raw.private.json')),data=raw.structuredContent;
  if(data?.windowId)data.pid=observedPid;if(data?.app)data.app.pid=observedPid;
  for(const w of data?.windows??[])w.pid=observedPid;
  return [post.tool,post.args,raw,i<7?f.knownPlay.original:f.knownPlay.replacement];
 });
 const first=calls('B1',b1Run,paths.source,b1Rows,1100),play={...f.knownPlay.pending,sourceSha256:ref(paths.source).sha256,
  reservationID:ref(paths.controller).sha256+':75'};
 const b1FrameOut=resolve(s,'B1-after');mkdirSync(b1FrameOut);
 const last=first.posts[10],raw=b1Rows[10][2],current=f.knownPlay.replacement;
 const frame=persistDirectFrame({output:b1FrameOut,label:'capture-001',pngName:'F1.original.png',result:raw,args:last.args,
  window:current,ownedPids:[observedPid],provenance:{...last,sourceSha256:ref(paths.source).sha256,
   reservationID:play.reservationID,capturedAtMs:Date.parse(last.callCompleted)}});
 if(!frame.pngPath)throw Error('Generated B1 handoff original required');
 if(options.changeCopiedFrame){const metadata=value(frame.metadataPath);options.changeCopiedFrame(metadata);json(frame.metadataPath,metadata);}
 const menuPath=resolve(s,'B1-menu-inspected.private.json');
 json(menuPath,{schemaVersion:1,kind:'STARSECTOR_B1_MAIN_MENU_INSPECTION',scenario:s,flow:'DISPOSABLE_SEED_CREATION',
  rootRecordKey:campaign.rootClock.recordKey,controllerSha256:ref(paths.controller).sha256,
  b1:{sourceSha256:ref(paths.source).sha256,claimSha256:ref(paths.claim).sha256,runID:b1Run,reservationID:play.reservationID},
  frame:{pngPath:frame.pngPath,pngSha256:ref(frame.pngPath).sha256,metadataPath:frame.metadataPath,
   metadataSha256:ref(frame.metadataPath).sha256,capturedAtMs:Date.parse(last.callCompleted)},
  inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:anchor+(options.b1InspectedOffsetMs??14000),
   state:'STARSECTOR_MAIN_MENU',newGameCoordinate:[1,1]},
  provider:{readySha256:ref(paths.providerReady).sha256,terminalSha256:ref(paths.providerTerminal).sha256,
   observedClosedAtMs:anchor+(options.b1ClosedObservedOffsetMs??15001),providerReaped:true}});
 const e1Run=basename(s)+'-E1',e1Out=resolve(f.root,'.tmp/starsector-proxy',e1Run),e1Campaign={...campaign,windowId:20};
 paths.e1Plan=resolve(s,'E1-plan.private.json');paths.e1Lease=resolve(s,'E1-lease.private.json');
 paths.e1Owned=resolve(s,'driver-E1-owned.private.json');paths.e1Source=resolve(s,'E1-authorization.private.json');
 paths.e1Claim=resolve(s,'guard-E1-consumed.private.json');paths.b1Menu=menuPath;
 const menuPin=ref(menuPath),e1Plan={...phase,campaign:e1Campaign,batches:[{id:'E1'}],b1MenuReceipt:menuPin};
 options.changeE1Plan?.(e1Plan);json(paths.e1Plan,e1Plan);
 const e1Lease={...b1Lease,planSha256:ref(paths.e1Plan).sha256,campaign:e1Campaign,b1MenuReceipt:menuPin};
 options.changeE1Lease?.(e1Lease);json(paths.e1Lease,e1Lease);
 const e1Owned={...value(paths.owned),at:iso(15900)};options.changeE1Owned?.(e1Owned);json(paths.e1Owned,e1Owned);
 json(paths.e1Source,{schemaVersion:6,scenario:s,batch:'E1',planPath:paths.e1Plan,planSha256:ref(paths.e1Plan).sha256,
  leasePath:paths.e1Lease,leaseSha256:ref(paths.e1Lease).sha256,ownedReceiptPath:paths.e1Owned,ownedReceiptSha256:ref(paths.e1Owned).sha256});
 const binding={...b1Claim.binding,batch:'E1',runID:e1Run,planSha256:ref(paths.e1Plan).sha256,campaign:e1Campaign,
  sourcePath:paths.e1Source,sourceSha256:ref(paths.e1Source).sha256,b1MenuReceipt:{...menuPin,newGameCoordinate:[1,1]}};
 json(paths.e1Claim,{sourcePath:paths.e1Source,sourceSha256:ref(paths.e1Source).sha256,binding});
 for(const [key,name]of Object.entries({e1ProviderReady:'guarded-http-ready.private.json',e1ProviderTerminal:'owned-provider-terminal.private.json',
  e1ProviderSpool:'owned-provider.stdout.private.txt',e1ProviderStderr:'owned-provider.stderr.private.txt'}))paths[key]=resolve(e1Out,name);
 json(paths.e1ProviderReady,{...b1Ready,at:iso(16020),runID:e1Run,batch:'E1',sourceSha256:ref(paths.e1Source).sha256,
  node:{...b1Ready.node,pid:202,parentPid:204,creationTime:iso(options.e1NodeBirthOffsetMs??16000)},
  helper:{...b1Ready.helper,pid:203,parentPid:202,creationTime:iso(options.e1HelperBirthOffsetMs??16001)},supervisorJobPids:[202,203]});
 json(paths.e1ProviderTerminal,{...b1Terminal,at:iso(18000),deadline:iso(196000),verifierPid:204,terminalReceipt:paths.e1ProviderTerminal});
 json(paths.e1ProviderSpool,value(paths.providerSpool));writeFileSync(paths.e1ProviderStderr,'',{flag:'wx'});
 const imageRaw=b1Rows[10][2],args={coordinate:[1,1],target_app:'java.exe',target_window_id:20,focus_strategy:'strict'};
 options.changeE1Args?.(args);
 const windowRaw={structuredContent:current},frontRaw={structuredContent:{app:{pid:observedPid,bundleId:'java.exe'}}};
 options.changeE1Front?.(frontRaw);
 const e1Rows=[['doctor',{include_remediation:true},b1Rows[0][2]],['policy_status',{},b1Rows[1][2]],
  ['get_window',{window_id:20},windowRaw],['screenshot',{target_window_id:20,quality:0,width:2},imageRaw],
  ['get_frontmost_app',{},frontRaw],['get_window',{window_id:20},windowRaw],['left_click',args,{content:[{type:'text',text:'Clicked (1, 1)'}]}],
  ['get_frontmost_app',{},frontRaw],['get_window',{window_id:20},windowRaw],['screenshot',{target_window_id:20,quality:0,width:2},imageRaw]]
  .map(row=>[...row,current]);
 const second=calls('E1',e1Run,paths.e1Source,e1Rows,16100);
 const guard={...before,reservedActions:76,sentActions:74},context={overallAttempts:45,uiAttempts:42};
 const ledger=value(paths.ledger);Object.assign(ledger,{history:guard,controllerSha256:ref(paths.controller).sha256,b1ClaimSha256:ref(paths.claim).sha256,
  provider:{batch:'E1',runID:e1Run,sourceSha256:ref(paths.e1Source).sha256},providerClock:{startMs:anchor+16000,deadlineMs:anchor+196000},
  resolvedB1:{...play,menuReceiptSha256:menuPin.sha256,providerClock:{startMs:anchor+(options.b1ProviderStartOffsetMs??1000),deadlineMs:Math.min(anchor+(options.b1ProviderStartOffsetMs??1000)+180000,campaign.rootClock.effectiveDeadlineMs-30000)}},
  pending:{...play,batch:'E1',runID:e1Run,sourceSha256:ref(paths.e1Source).sha256,reservationID:ref(paths.controller).sha256+':76',
   args,wizardStep:{ordinal:1,stage:'NEW_GAME'},sentAtMs:Date.parse(second.posts[6].callStarted)+1,knownResultAtMs:Date.parse(second.posts[6].callCompleted)-1}});
 json(paths.ledger,ledger);
 const protection=value(paths.protection);protection.at=iso(19000);json(paths.protection,protection);
 const absence=value(paths.pidAbsence);absence.atUtc=iso(19001);absence.queriedExactRecordedPids.push(202,203,204);json(paths.pidAbsence,absence);
 const actions=[{batch:'B1',guardStage:'PLAY',reservationOrdinal:75,args:play.args,acknowledged:true,visibleAfter:'Readable mainmenu',assertion:'PASS'},
  {batch:'E1',guardStage:'NEW_GAME',reservationOrdinal:76,args,acknowledged:true,visibleAfter:'Missions',expectedAfter:'Identity wizard',assertion:'FAIL',ledgerStatus:'AWAITING_AFTER',uncertain:false}];
 json(paths.observation,{schemaVersion:1,kind:'ACTUAL_DRIVER_WRONG_SCENE_STOP',scenarioId:basename(s),driver:'/root/recovery_game_driver',
  atUtc:iso(18100),sourceHead:prior.source.qualifiedHead,verdict:'FAIL',category:'POSTCONDITION_FAILED',actualRunContext:context,actualGuardHistory:guard,
  actions:[{batch:'B1',stage:'PLAY',args:play.args,acknowledged:true,visibleAfter:'Readable mainmenu',reservationOrdinal:75},
   {batch:'E1',guardStage:'NEW_GAME',intent:'New Game',args,acknowledged:true,visibleAfter:'Missions',expectedAfter:'Identity wizard',expectedAssertion:'FAIL',reservationOrdinal:76}],
  ownedPid:observedPid,launcherWindow:19,gameWindow:20,activations:0,
  providers:[{batch:'B1',ready:ref(paths.providerReady),terminal:ref(paths.providerTerminal),calls:first.calls,originals:first.originals},
   {batch:'E1',ready:ref(paths.e1ProviderReady),terminal:ref(paths.e1ProviderTerminal),calls:second.calls,originals:second.originals}],
  totalCalls:21,totalOriginals:4,B1LoadingWaitMs:10000,B1OptionalRecaptureUsed:0,B1MenuProof:menuPin,
  ledger:ref(paths.ledger),rootCall:ref(paths.rootCall),E2:'NOT_STARTED',seedClaim:false,allPriorFailuresRetained:true});
 const result=value(paths.result);Object.assign(result,{failureCategory:'POSTCONDITION_FAILED',assertionResult:'FAIL',actualGuardHistory:guard,
  actualRunContext:context,actions,counts:{reservedThisRun:2,sentThisRun:2,clicks:2,activations:0,providers:2,B1Calls:11,E1Calls:10,totalCalls:21,originals:4,B1WaitMs:10000,B1OptionalRecaptureUsed:0},
  providerStatus:{B1:'TERMINAL_PASS_REAPED',E1:'TERMINAL_PASS_REAPED',E2:'NOT_STARTED'},
  originals:{allFourInspected:true,nativeLauncherWidth:2,nativeGameWidth:2,captureReceipt:true,freshness:'UNKNOWN',wrongScenePngSha256:second.originals[1].png.sha256,actualVisualTransitionProven:true,scenarioSuccess:false},
  seed:{created:false,modernCloseoutCreated:false,stabilityModeExecuted:false,semanticAcceptance:'NOT_RUN'}});
 delete result.providers;json(paths.result,result);
 f.closeout.kind='ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY';
 f.closeout.actions={actualGuard:guard,currentRunContext:context,absoluteMaxReservedActions:82};
 function refresh(){
  const observation=value(paths.observation);observation.ledger=ref(paths.ledger);observation.rootCall=ref(paths.rootCall);
  observation.B1MenuProof=ref(paths.b1Menu);
  for(const [i,keys]of [['providerReady','providerTerminal'],['e1ProviderReady','e1ProviderTerminal']].entries()){
   const row=observation.providers[i];row.ready=ref(paths[keys[0]]);row.terminal=ref(paths[keys[1]]);
   for(const call of row.calls)for(const key of ['post','attempt','raw'])call[key]=ref(call[key].path);
   for(const original of row.originals)for(const key of ['png','metadata','raw'])original[key]=ref(original[key].path);
  }
  json(paths.observation,observation);
  const r=value(paths.result);r.evidence=Object.fromEntries(Object.entries({observation:paths.observation,callerTerminal:paths.caller,
   launchSupervisor:paths.launch,protection:paths.protection,pidAbsence:paths.pidAbsence,ledger:paths.ledger,plan:paths.plan,lease:paths.lease}).map(([k,p])=>[k,ref(p)]));json(paths.result,r);
  f.closeout.evidence=Object.fromEntries(Object.entries(paths).map(([k,p])=>[k,ref(/** @type {string} */(p))]));json(f.closeoutPath,f.closeout);
  const future={...guard,...context};json(f.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',
   historyBaseline:future,maxReservedActions:84,previous:{result:ref(f.closeoutPath),ledger:ref(paths.ledger)}});
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){v.historyBaseline=future;v.historyAdmission=ref(f.grantPath);v.absoluteMaxReservedActions=84;}
  f.campaign.historyBaseline=future;f.campaign.maxReservedActions=84;f.predecessor.historyBaseline=future;f.predecessor.maxReservedActions=84;f.refreshLaunch();
 }
 refresh();return {...f,refresh,wrongScene:{first,second,guard,context,binding}};
}
