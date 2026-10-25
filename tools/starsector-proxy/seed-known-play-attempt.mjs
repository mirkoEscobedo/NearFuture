// @ts-check
// Historical one-Play failure only. This never resolves the action or grants input.
import {resolve} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {file,closed} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';

/** @param {{v:any,a:any,e:any,output:string,pinned:(pin:any,path:string,limit?:number)=>any,fail:(ok:any)=>void}} context */
export function assertKnownPlayFailure({v,a,e,output,pinned,fail}){
 const {result:r,ledger:l,plan:p,controller:c,owned,source,claim,observation:o,providerReady:ready,providerTerminal:terminal}=a;
 fail(Number.isSafeInteger(c.sessionId)&&c.sessionId>=0);
 const typedWindow=(/** @type {any} */ window)=>window&&Number.isSafeInteger(window.windowId)&&window.windowId>0&&
  Number.isSafeInteger(window.pid)&&window.pid>0&&window.bundleId==='java.exe'&&window.isFocused===true&&
  closed(window.bounds,['x','y','width','height'])&&Number.isSafeInteger(window.bounds.x)&&Number.isSafeInteger(window.bounds.y)&&
  Number.isSafeInteger(window.bounds.width)&&window.bounds.width>0&&Number.isSafeInteger(window.bounds.height)&&window.bounds.height>0;
 for(const value of [ready.at,terminal.at,a.protection.at,a.pidAbsence.atUtc])fail(typeof value==='string');
 const before=p.historyBaseline,action=l.pending;
 fail(closed(before,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&
  Object.values(before).every(n=>Number.isSafeInteger(n)&&n>=0)&&before.sentActions<=before.reservedActions&&
  same(l.history,{...before,reservedActions:before.reservedActions+1,sentActions:before.sentActions+1})&&
  same(r.inheritedGuardBaseline,before)&&same(v.actions.currentRunContext,{overallAttempts:before.overallAttempts+1,uiAttempts:before.uiAttempts+1})&&
  r.absoluteMaxReservedActions>before.reservedActions&&r.absoluteMaxReservedActions<=before.reservedActions+8);
 fail(closed(action,['batch','runID','sourceSha256','reservationID','tool','args','status','wizardStep','sentAtMs','knownResultAtMs'])&&
  action.batch==='B1'&&action.runID===ready.runID&&action.sourceSha256===e.source.sha256&&
  action.reservationID===e.controller.sha256+':'+l.history.reservedActions&&action.tool==='left_click'&&
  action.status==='AWAITING_AFTER'&&same(action.wizardStep,{ordinal:1,stage:'PLAY'})&&
  (closed(action.args,['coordinate','target_window_id','focus_strategy'])||
   (closed(action.args,['coordinate','target_window_id','focus_strategy','target_app'])&&action.args.target_app==='java.exe'))&&action.args.focus_strategy==='strict'&&
  Array.isArray(action.args.coordinate)&&action.args.coordinate.length===2&&action.args.coordinate.every(Number.isSafeInteger)&&
  Number.isSafeInteger(action.sentAtMs)&&Number.isSafeInteger(action.knownResultAtMs)&&action.knownResultAtMs>=action.sentAtMs&&
  !['resolvedB1','resolvedE1','resolvedE2'].some(key=>Object.hasOwn(l,key)));
 fail(same(r.providers,{started:1,B1:'TERMINAL_PASS_REAPED',E1:'NOT_STARTED',E2:'NOT_STARTED',calls:14})&&
  o.schemaVersion===1&&o.kind==='ACTUAL_DRIVER_HANDOFF_CUSTODY_STOP'&&o.category===r.failureCategory&&o.verdict==='FAIL'&&
  o.driver===v.driver&&o.scenarioId===p.scenarioId&&o.sourceHead===v.sourceHead&&
  same(o.actualRunContext,r.actualRunContext)&&same(o.guardHistory,l.history)&&o.E1==='NOT_STARTED'&&o.E2==='NOT_STARTED'&&
  o.seedClaim===false&&o.allHistoryRetained===true&&same(o.providerTerminal,e.providerTerminal)&&same(o.ledger,e.ledger)&&same(o.rootCall,e.rootCall));
 const tools=['doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click',
  'get_frontmost_app','list_windows','get_window','screenshot','get_frontmost_app','get_window','screenshot'];
 fail(Array.isArray(o.calls)&&o.calls.length===14&&Array.isArray(o.originals)&&o.originals.length===3&&
  same(readdirSync(output).filter(n=>/^call-\d+\.attempt\.private\.json$/.test(n)).sort(),tools.map((_,i)=>'call-'+String(i+1).padStart(3,'0')+'.attempt.private.json'))&&
  !readdirSync(v.scenario).some(n=>/^guard-E[12]-consumed\.private\.json$/.test(n)));
 /** @type {any[]} */const posts=[],raws=[];
 let previous=Date.parse(ready.at);
 for(const [i,tool] of tools.entries()){
  const row=o.calls[i],prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));
  fail(closed(row,['post','raw','attempt']));
  const post=pinned(row.post,prefix+'.private.json'),attempt=pinned(row.attempt,prefix+'.attempt.private.json');
  const raw=pinned(row.raw,prefix+'.raw.private.json',32*1024*1024);
  fail(post.tool===tool&&attempt.tool===tool&&post.runID===ready.runID&&attempt.runID===ready.runID&&
   post.batch==='B1'&&attempt.batch==='B1'&&attempt.sourceSha256===e.source.sha256&&same(post.args,attempt.args)&&
   attempt.callStarted===post.callStarted&&typeof post.callStarted==='string'&&typeof post.callCompleted==='string'&&post.isError===false&&
   (raw.isError===undefined||raw.isError===false)&&
   Number.isSafeInteger(Date.parse(post.callStarted))&&Date.parse(post.callStarted)>=previous&&
   Date.parse(post.callCompleted)>=Date.parse(post.callStarted)&&Date.parse(post.callCompleted)<=Date.parse(terminal.at)&&
   same(post.precedingActions,i<=6?[]:[posts[6]]));
  previous=Date.parse(post.callCompleted);posts.push(post);raws.push(raw);
 }
 const click=posts[6],ack=raws[6].content;
 fail(same(click.args,action.args)&&Date.parse(click.callStarted)<=action.sentAtMs&&
  action.knownResultAtMs<=Date.parse(click.callCompleted)&&Array.isArray(ack)&&ack.length===1&&
  same(ack[0],{type:'text',text:'Clicked ('+action.args.coordinate.join(', ')+')'})&&
  raws[0].structuredContent?.ok===true&&raws[1].structuredContent?.approval_required_for_all===true&&
  raws[1].structuredContent.approval_token_configured===false);
 const old=raws[2].structuredContent,current=raws[12].structuredContent;
 fail(typedWindow(old)&&typedWindow(current));
 fail(old?.windowId===claim.binding.campaign?.windowId&&action.args.target_window_id===old.windowId&&
  Number.isSafeInteger(current?.windowId)&&current.windowId>0&&current.pid===old.pid&&
  old.bundleId==='java.exe'&&current.bundleId==='java.exe'&&old.isFocused===true&&current.isFocused===true);
 for(const i of [4,7,11])fail(raws[i].structuredContent?.app?.pid===old.pid&&raws[i].structuredContent.app.bundleId==='java.exe');
 for(const [i,window] of [[2,old],[5,old],[9,current],[12,current]]){
  const actual=raws[i].structuredContent;
  fail(typedWindow(actual)&&same(posts[i].args,{window_id:window.windowId})&&actual?.windowId===window.windowId&&actual.pid===old.pid&&
   actual.bundleId==='java.exe'&&actual.isFocused===true&&same(actual.bounds,window.bounds));
 }
 const listed=raws[8].structuredContent?.windows;
 fail(Array.isArray(listed)&&listed.filter(x=>x.windowId===current.windowId&&x.pid===old.pid).length===1);
 fail(typedWindow(listed.find((/** @type {any} */ x)=>x.windowId===current.windowId&&x.pid===old.pid)));
 const originalOwnedPath=resolve(v.scenario,'driver-B1-owned.private.json');
 const initial=pinned({path:source.ownedReceiptPath,sha256:source.ownedReceiptSha256},originalOwnedPath);
 const initialRows=initial.processes?.filter((/** @type {any} */ x)=>x.pid===old.pid),rows=owned.processes.filter((/** @type {any} */ x)=>x.pid===old.pid);
 fail(initialRows?.length===1&&rows.length===1);const first=initialRows[0],last=rows[0];
 fail(Number(first.mainWindowHandle)===old.windowId&&Number(last.mainWindowHandle)===current.windowId&&
  Number.isSafeInteger(first.parentPid)&&first.parentPid>0&&Number.isSafeInteger(old.pid)&&claim.binding.ownedPids.includes(old.pid)&&
  typeof first.creationTime==='string'&&Number.isSafeInteger(Date.parse(first.creationTime))&&
  typeof first.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(first.kernelCreationFileTime)&&
  first.executable===resolve(c.installation,'jre/bin/java.exe')&&Number.isSafeInteger(first.sessionId)&&first.sessionId>=0&&first.sessionId===c.sessionId&&
  ['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(first[key],last[key])));
 for(const row of [first,last])fail(row.inSupervisorJob===true&&Array.isArray(row.supervisorJobPids)&&
  [c.controllerPid,old.pid,...c.supervisorJobPids].every(pid=>row.supervisorJobPids.includes(pid)));
 fail(a.pidAbsence.queriedExactRecordedPids.includes(old.pid)&&same(r.actions,{reservedThisRun:1,sentThisRun:1,clicks:1,activations:0,
  stage:'PLAY',coordinate:action.args.coordinate,originalWindow:old.windowId,replacementWindow:current.windowId,ownedPid:old.pid,status:'AWAITING_AFTER',uncertain:false})&&
  same(o.actions,{clicks:1,activations:0,coordinate:action.args.coordinate,targetWindow:old.windowId,pid:old.pid,
   replacementWindow:current.windowId,knownInput:true,ledgerStatus:'AWAITING_AFTER',uncertain:false}));
 fail(action.args.coordinate[0]>=old.bounds.x&&action.args.coordinate[0]<old.bounds.x+old.bounds.width&&
  action.args.coordinate[1]>=old.bounds.y&&action.args.coordinate[1]<old.bounds.y+old.bounds.height);
 for(const [index,callIndex] of [3,10,13].entries()){
  const row=o.originals[index],prefix=resolve(output,'capture-'+String(index+1).padStart(3,'0'));
  fail(closed(row,['png','metadata','raw','originalInspected','freshness'])&&row.originalInspected===true&&row.freshness==='UNKNOWN');
  const metadata=pinned(row.metadata,prefix+'.metadata.private.json'),raw=pinned(row.raw,prefix+'.raw.private.json',32*1024*1024);
  fail(closed(row.png,['path','sha256'])&&row.png.path===prefix+'.png');
  const png=file(row.png.path,32*1024*1024),dimensions=completePng(png.bytes),post=posts[callIndex],window=index===0?old:current;
  const images=raw.content?.filter((/** @type {any} */ x)=>x.type==='image');
  fail((raw.isError===undefined||raw.isError===false)&&png.sha256===row.png.sha256&&same(raw,raws[callIndex])&&images?.length===1&&images[0].mimeType==='image/png'&&
   images[0].data===png.bytes.toString('base64')&&metadata.captureReceipt===true&&metadata.pngPath===row.png.path&&
   metadata.pngSha256===png.sha256&&metadata.rawPath===row.raw.path&&metadata.rawSha256===row.raw.sha256&&
   metadata.tool==='screenshot'&&metadata.runID===ready.runID&&metadata.batch==='B1'&&
   metadata.callStarted===post.callStarted&&metadata.callCompleted===post.callCompleted&&same(metadata.precedingActions,post.precedingActions)&&
   same(metadata.args,post.args)&&post.args.target_window_id===window.windowId&&post.args.quality===0&&
   post.args.width===window.bounds.width&&metadata.window?.windowId===window.windowId&&metadata.window.pid===old.pid&&
   typedWindow(metadata.window)&&metadata.window.isFocused===true&&same(metadata.window.bounds,window.bounds)&&same(metadata.dimensions,dimensions)&&
   dimensions.width===window.bounds.width&&dimensions.height===window.bounds.height);
 }
}
