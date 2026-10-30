// @ts-check
// Generated boundary data only. No native game or private screenshot evidence.
import {readFileSync} from 'node:fs';
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
const category='PRIVATE_FRAME_CUSTODY_REFUSAL_AFTER_KNOWN_PLAY';

/** Existing no-input fixture remains untouched; override only generated prior-attempt data. */
/** @param {{targetApp?:string,originX?:number,originY?:number,sessionId?:unknown,anchorMs?:number,historyBaseline?:{overallAttempts:number,uiAttempts:number,reservedActions:number,sentActions:number}}} [options] */
export function knownPlayHistoryFixture(options={}){
 const f=/** @type {any} */(failedSeedHistoryFixture()),paths=f.evidencePaths,scenario=f.priorScenario;
 const before=options.historyBaseline??{overallAttempts:40,uiAttempts:37,reservedActions:72,sentActions:70};
 const guard={...before,reservedActions:before.reservedActions+1,sentActions:before.sentActions+1};
 const context={overallAttempts:before.overallAttempts+1,uiAttempts:before.uiAttempts+1};
 const future={...guard,...context},previousCap=before.reservedActions+8,nextCap=guard.reservedActions+8;
 const runID=value(paths.ledger).provider.runID,output=resolve(f.root,'.tmp/starsector-proxy',runID);
 const anchor=options.anchorMs??Date.now()-5000,iso=(/** @type {number} */ offset)=>new Date(anchor+offset).toISOString();
 const priorPlan=value(paths.plan);priorPlan.historyBaseline={...before};priorPlan.absoluteMaxReservedActions=previousCap;json(paths.plan,priorPlan);
 const priorLease=value(paths.lease);priorLease.planSha256=ref(paths.plan).sha256;priorLease.absoluteMaxReservedActions=previousCap;json(paths.lease,priorLease);
 const c=value(paths.controller);
 Object.assign(c,{at:iso(100),controllerCreationTime:iso(100),sessionId:options.sessionId??1,rootLeaseSha256:ref(paths.lease).sha256,
  conservativeOuterDeadlineUtc:iso(480000),controllerDeadlineUtc:iso(480000)});
 json(paths.controller,c);
 const call=value(paths.rootCall);call.value.startMs=anchor;json(paths.rootCall,call);
 const originalOwner={pid:101,parentPid:105,creationTime:iso(500),kernelCreationFileTime:'134371266963297183',
  inSupervisorJob:true,sessionId:c.sessionId,executable:resolve(scenario,'installation/jre/bin/java.exe'),
  supervisorJobPids:[100,105,101],mainWindowHandle:19};
 const initialOwned=resolve(scenario,'driver-B1-owned.private.json');
 json(initialOwned,{at:iso(600),processes:[originalOwner]});
 json(paths.owned,{at:iso(2900),controllerPid:c.controllerPid,controllerCreationTime:c.controllerCreationTime,
  controllerKernelCreationFileTime:c.controllerKernelCreationFileTime,
  processes:[{...originalOwner,mainWindowHandle:20}]});
 const ready=value(paths.providerReady);ready.at=iso(1000);
 ready.node.sessionId=ready.helper.sessionId=c.sessionId;
 ready.node.creationTime=ready.helper.creationTime=iso(1000);json(paths.providerReady,ready);
 const terminal=value(paths.providerTerminal);terminal.at=iso(3000);json(paths.providerTerminal,terminal);
 const protection=value(paths.protection);protection.at=iso(4000);json(paths.protection,protection);
 const absence=value(paths.pidAbsence);absence.atUtc=iso(4001);json(paths.pidAbsence,absence);
 const campaign={...structuredClone(f.campaign),windowId:19,historyBaseline:{...before},maxReservedActions:previousCap,
  rootCallRecord:call,rootClock:{...f.campaign.rootClock,recordKey:call.key,startMs:anchor,
   deadlineMs:anchor+480000,effectiveDeadlineMs:anchor+420000,callerAdmissionDeadlineMs:anchor+420000,
   originalCallerAdmissionDeadlineMs:anchor+420000},
  producerWitness:{...f.campaign.producerWitness,controllerCreationTime:c.controllerCreationTime,
   controllerKernelCreationFileTime:c.controllerKernelCreationFileTime}};
 const phasePlanPath=resolve(scenario,'B1-plan.private.json'),phaseLeasePath=resolve(scenario,'B1-lease.private.json');
 json(phasePlanPath,{...priorPlan,campaign,batches:[{id:'B1'}]});
 json(phaseLeasePath,{...priorLease,schemaVersion:6,phase:'DISPOSABLE_SEED_CREATION',campaign,
  planSha256:ref(phasePlanPath).sha256});
 json(paths.source,{schemaVersion:6,scenario,batch:'B1',planPath:phasePlanPath,planSha256:ref(phasePlanPath).sha256,
  leasePath:phaseLeasePath,leaseSha256:ref(phaseLeasePath).sha256,
  ownedReceiptPath:initialOwned,ownedReceiptSha256:ref(initialOwned).sha256});
 const claim=value(paths.claim);Object.assign(claim,{sourcePath:paths.source,sourceSha256:ref(paths.source).sha256});
 Object.assign(claim.binding,{schemaVersion:6,batch:'B1',scenario,workspaceRoot:f.root,runID,
  sourcePath:paths.source,sourceSha256:ref(paths.source).sha256,planSha256:ref(phasePlanPath).sha256,
  originalPlanSha256:ref(paths.plan).sha256,controllerSha256:ref(paths.controller).sha256,
  ownedPids:[101],campaign});json(paths.claim,claim);
 ready.sourceSha256=ref(paths.source).sha256;json(paths.providerReady,ready);
 const original={pid:101,windowId:19,bundleId:'java.exe',title:'Generated launcher',
  isFocused:true,isOnScreen:true,bounds:{x:options.originX??0,y:options.originY??0,width:2,height:2}};
 const replacement={...original,windowId:20,title:'Generated main menu'};
 const clickArgs={coordinate:[original.bounds.x+1,original.bounds.y+1],target_window_id:19,focus_strategy:'strict',
  ...(options.targetApp===undefined?{}:{target_app:options.targetApp})};
 const rows=[
  ['doctor',{include_remediation:true}],['policy_status',{}],['get_window',{window_id:19}],
  ['screenshot',{target_window_id:19,quality:0,width:2}],['get_frontmost_app',{}],
  ['get_window',{window_id:19}],['left_click',clickArgs],['get_frontmost_app',{}],
  ['list_windows',{bundle_id:'java.exe'}],['get_window',{window_id:20}],
  ['screenshot',{target_window_id:20,quality:0,width:2}],['get_frontmost_app',{}],
  ['get_window',{window_id:20}],['screenshot',{target_window_id:20,quality:0,width:2}]
 ];
 /** @type {any[]} */const posts=[];/** @type {any[]} */const calls=[];/** @type {any[]} */const originals=[];
 for(const [i,row] of rows.entries()){
  const ordinal=i+1,tool=/** @type {string} */(row[0]),args=row[1],window=ordinal<=7?original:replacement;
  const post={tool,args,outerProtocolID:ordinal+9,innerProtocolID:ordinal,
   callStarted:iso(1100+i*50),callCompleted:iso(1120+i*50),
   precedingActions:i<7?[]:[structuredClone(posts[6])],runID,batch:'B1',freshness:'UNKNOWN',isError:false};
  posts.push(post);
  const prefix=resolve(output,'call-'+String(ordinal).padStart(3,'0'));
  json(prefix+'.private.json',post);
  json(prefix+'.attempt.private.json',{tool,args,callStarted:post.callStarted,outerProtocolID:post.outerProtocolID,
   innerProtocolID:'UNKNOWN',sourceSha256:ref(paths.source).sha256,runID,batch:'B1'});
  const raw=tool==='screenshot'?{content:[{type:'image',mimeType:'image/png',data:png.toString('base64')}]}
   :tool==='get_window'?{structuredContent:window}:tool==='get_frontmost_app'?{structuredContent:{app:{pid:101,bundleId:'java.exe'}}}
   :tool==='list_windows'?{structuredContent:{windows:[replacement]}}
   :tool==='left_click'?{content:[{type:'text',text:'Clicked ('+clickArgs.coordinate.join(', ')+')'}]}
   :tool==='doctor'?{structuredContent:{ok:true,summary:{failed:0}}}
   :tool==='policy_status'?{structuredContent:{approval_required_for_all:true,approval_token_configured:false,audit:{enabled:true},profile:'core'}}:{structuredContent:{}};
  write(prefix+'.raw.private.json',JSON.stringify(raw)+'\n');
  calls.push({post:ref(prefix+'.private.json'),raw:ref(prefix+'.raw.private.json'),attempt:ref(prefix+'.attempt.private.json')});
  if(tool==='screenshot'){
   const {isError,...provenance}=post;
   const label='capture-'+String(originals.length+1).padStart(3,'0');
   const frame=persistDirectFrame({output,label,result:raw,args,window,ownedPids:[101],provenance});
   if(!frame.pngPath)throw Error('Generated positive fixture must produce an original');
   originals.push({png:ref(frame.pngPath),metadata:ref(frame.metadataPath),raw:ref(frame.rawPath),
    originalInspected:true,freshness:'UNKNOWN'});
  }
 }
 const pending={batch:'B1',runID,sourceSha256:ref(paths.source).sha256,
  reservationID:ref(paths.controller).sha256+':'+guard.reservedActions,tool:'left_click',args:clickArgs,status:'AWAITING_AFTER',
  wizardStep:{ordinal:1,stage:'PLAY'},sentAtMs:Date.parse(posts[6].callStarted)+1,
  knownResultAtMs:Date.parse(posts[6].callCompleted)-1};
 const ledger=value(paths.ledger);Object.assign(ledger,{controllerSha256:ref(paths.controller).sha256,
  history:{...guard},pending,uncertain:false,provider:{batch:'B1',runID,sourceSha256:ref(paths.source).sha256},
  b1ClaimSha256:ref(paths.claim).sha256,providerClock:{startMs:anchor+1000,deadlineMs:anchor+181000}});
 json(paths.ledger,ledger);
 const observation={schemaVersion:1,kind:'ACTUAL_DRIVER_HANDOFF_CUSTODY_STOP',scenarioId:basename(scenario),
  driver:'/root/recovery_game_driver',atUtc:iso(3100),sourceHead:priorPlan.source.qualifiedHead,
  verdict:'FAIL',category,visibleOutcome:'Generated inspected replacement main-menu original; no E1 input',
  actualRunContext:context,guardHistory:guard,
  actions:{clicks:1,activations:0,coordinate:clickArgs.coordinate,targetWindow:19,pid:101,replacementWindow:20,
   knownInput:true,ledgerStatus:'AWAITING_AFTER',uncertain:false},
  calls,originals,helperRefusal:{exitCode:1,predicate:'window.windowId===binding.campaign.windowId'},
  callerError:{exitCode:1,message:'ENOENT B1-frame-custody.private.json'},
  providerTerminal:ref(paths.providerTerminal),ledger:ref(paths.ledger),rootCall:ref(paths.rootCall),
  E1:'NOT_STARTED',E2:'NOT_STARTED',seedClaim:false,cleanup:'ROOT_OWNS_PENDING_ACTUAL_TERMINAL_PROTECTION',allHistoryRetained:true};
 json(paths.observation,observation);
 const result=value(paths.result);Object.assign(result,{failureCategory:category,inheritedGuardBaseline:before,
  actualGuardHistory:guard,actualRunContext:context,absoluteMaxReservedActions:previousCap,historicalUnsentReservations:guard.reservedActions-guard.sentActions,
  actions:{reservedThisRun:1,sentThisRun:1,clicks:1,activations:0,stage:'PLAY',coordinate:clickArgs.coordinate,
   originalWindow:19,replacementWindow:20,ownedPid:101,status:'AWAITING_AFTER',uncertain:false},
  providers:{started:1,B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED',calls:14}});
 json(paths.result,result);
 const closeout=f.closeout;closeout.kind='ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY';
 closeout.actions={actualGuard:guard,currentRunContext:context,absoluteMaxReservedActions:previousCap};
 function refresh(){
  const r=value(paths.result);
  r.evidence=Object.fromEntries(Object.entries({observation:paths.observation,callerTerminal:paths.caller,
   launchSupervisor:paths.launch,protection:paths.protection,pidAbsence:paths.pidAbsence,
   ledger:paths.ledger,plan:paths.plan,lease:paths.lease}).map(([k,p])=>[k,ref(p)]));json(paths.result,r);
  closeout.evidence=Object.fromEntries(Object.entries(paths).map(([k,p])=>[k,ref(/** @type {string} */(p))]));
  json(f.closeoutPath,closeout);
  json(f.grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',
   driver:'/root/recovery_game_driver',historyBaseline:{...future},maxReservedActions:nextCap,
   previous:{result:ref(f.closeoutPath),ledger:ref(paths.ledger)}});
  for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){
   v.historyBaseline={...future};v.historyAdmission=ref(f.grantPath);v.absoluteMaxReservedActions=nextCap;
  }
  f.campaign.historyBaseline={...future};f.campaign.maxReservedActions=nextCap;
  f.predecessor.historyBaseline={...future};f.predecessor.maxReservedActions=nextCap;f.refreshLaunch();
 }
 refresh();
 return {...f,closeout,refresh,knownPlay:{before,guard,context,future,pending,original,replacement,posts,calls,originals}};
}
