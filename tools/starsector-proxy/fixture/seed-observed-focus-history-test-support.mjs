// @ts-check
// Generated external-boundary records. No native game, process or save evidence.
import {resolve,dirname,basename} from 'node:path';
import {deflateSync,crc32} from 'node:zlib';
import {failedSeedHistoryFixture,ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

function generatedPng(){
 const chunk=(/** @type {string} */ type,/** @type {Buffer} */ data)=>{
  const bytes=Buffer.alloc(data.length+12);bytes.writeUInt32BE(data.length);bytes.write(type,4);data.copy(bytes,8);
  bytes.writeUInt32BE(crc32(bytes.subarray(4,-4)),bytes.length-4);return bytes;
 };
 const header=Buffer.alloc(13);header.writeUInt32BE(686);header.writeUInt32BE(428,4);header[8]=8;header[9]=2;
 return Buffer.concat([Buffer.from('89504e470d0a1a0a','hex'),chunk('IHDR',header),
  chunk('IDAT',deflateSync(Buffer.alloc((686*3+1)*428))),chunk('IEND',Buffer.alloc(0))]).toString('base64');
}

/** @param {{order?:string[]}} [options] */
export function observedFocusHistoryFixture(options={}){
 const f=failedSeedHistoryFixture({watchdogMs:1800000}),paths=f.evidencePaths;
 const old=value(paths.observation),r=value(paths.result),ready=value(paths.providerReady),terminal=value(paths.providerTerminal);
 const source=value(paths.source),caller=value(paths.caller),absence=value(paths.pidAbsence),protection=value(paths.protection);
 const originals=old.calls.map((/** @type {any} */ row)=>({post:value(row.completed.path),attempt:value(row.attempt.path),raw:value(row.raw.path)}));
 const output=dirname(paths.providerReady),start=Date.parse(ready.at),end=Date.parse(terminal.at);
 const window={bounds:{height:428,width:686,x:10,y:20},bundleId:'java.exe',displayId:1,displayName:'java.exe',
  isFocused:false,isOnScreen:true,pid:101,title:'Generated launcher',windowId:19};
 const front={app:{bundleId:'generated.exe',displayName:'Generated',pid:200}};
 const frontRaw={content:[{type:'text',text:JSON.stringify(front)}],structuredContent:front};
 /** @type {Array<{ordinal:number,name:string,args:any,atMs:number}>} */const audit=[];
 const order=options.order??['doctor','policy_status','get_window','screenshot','get_frontmost_app'];
 const calls=order.map((tool,i)=>{
  const row=originals.find((/** @type {any} */ entry)=>entry.post.tool===tool);
  if(!row)throw Error('Generated tool fixture missing');
  const callerArgs=tool==='screenshot'?{target_window_id:19,width:686,quality:0}:row.post.args;
  const args=tool==='doctor'?{include_remediation:true}:callerArgs;
  const at=start+100*(i+1),post={...row.post,args,callStarted:new Date(at).toISOString(),callCompleted:new Date(at+5).toISOString()};
  const prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));
  const raw=tool==='get_window'?{content:[{type:'text',text:JSON.stringify(window)}],structuredContent:window}:
   tool==='get_frontmost_app'?frontRaw:tool==='screenshot'?{content:[{type:'image',mimeType:'image/png',data:generatedPng()}]}:row.raw;
  json(prefix+'.private.json',post);json(prefix+'.attempt.private.json',{...row.attempt,args,callStarted:post.callStarted,callCompleted:post.callCompleted});json(prefix+'.raw.private.json',raw);
  audit.push({ordinal:i+1,name:tool,args:callerArgs,atMs:at+10});
  return {completed:ref(prefix+'.private.json'),attempt:ref(prefix+'.attempt.private.json'),raw:ref(prefix+'.raw.private.json')};
 });
 terminal.deadline=new Date(start+180000).toISOString();json(paths.providerTerminal,terminal);
 protection.restoration='UNNECESSARY_ORIGINALS_UNCHANGED';json(paths.protection,protection);
 const observed=resolve(f.priorScenario,'driver-stop-observation.private.json');
 json(observed,{schemaVersion:1,kind:'ACTUAL_S11_DRIVER_FOCUS_BLOCKED_STOP',driver:r.driver,scenario:f.priorScenario.replaceAll('\\','/'),
  atUtc:new Date(end+2).toISOString(),failureCategory:'FOCUS_BLOCKED',assertion:'Generated owned launcher is unfocused; no input authorized.',
  window,foreground:{raw:frontRaw,at:audit.find(row=>row.name==='get_frontmost_app')?.atMs,reason:'Generated focus stop.'},callCount:5,originalImages:1,inputCount:0,
  reservationsAdded:0,activations:0,geometryModes:0,metadataModes:0,actualVendorCalls:audit,
  begin:{args:{consentMode:'finite-explicit-consent',runID:ready.runID,source:paths.source.replaceAll('\\','/')},
   raw:{content:[{type:'text',text:JSON.stringify({runID:ready.runID,output,deadline:terminal.deadline})}]},at:start+20},
  providerFinish:{raw:{content:[{type:'text',text:JSON.stringify(terminal)}]},at:end+1},historyBaseline:r.inheritedGuardBaseline,
  actualContext:r.actualRunContext,absoluteMaxReservedActions:r.absoluteMaxReservedActions,originalCallerSession:caller.session,
  originalCallerDeadlineMs:f.plan.callerAdmissionDeadlineMs,retry:false,ordinaryExit:false,campaign:false,seed:false,rootProtection:'PENDING'});
 Object.assign(r,{counts:{originalPngs:1,metadataModes:0,geometryModes:0},actualCallOrder:audit.map(row=>row.name),
  focus:{ownedPid:101,ownedHwnd:19,isFocused:false,foregroundApp:'generated.exe',foregroundPid:200},
  visibleAssertion:{launcherPlayVisible:true,inputPrecondition:false,originalNativeDimensions:{width:686,height:428},captureReceipt:true,freshness:'UNKNOWN',activationAllowed:false},
  steps:{B1:'FOCUS_BLOCKED_BEFORE_INPUT',E1:'NOT_STARTED',E2:'NOT_STARTED'},
  timing:{rootWatchdogMs:1800000,inputMarginMs:20000,cleanupReserveMs:30000,originalCallerDeadlineMs:f.plan.callerAdmissionDeadlineMs,clockRenewal:false,retry:false},
  closure:{provider:{verifierPid:terminal.verifierPid,status:0,timedOut:false,verdict:'PASS',terminalReaped:true,at:terminal.at},
   rootCaller:{session:caller.session,originalRawChunk:caller.launchRaw.chunk_id,terminalRawChunk:caller.actual.chunk_id,status:1,timedOut:false,verdict:'FAIL',terminalReaped:true,kind:caller.kind,pollCount:1},
   maintainedStopToolChunk:'generated-stop',forcedOwnedCleanup:true,ordinaryExit:false,allOwnedHandlesTerminalReaped:true,knownPidsAbsent:absence.queriedExactRecordedPids},
  protection:{protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,originalWrites:false,
   originalAutosave:protection.originalAutosave,privateAutosaveEnabled:false,privateSaveEntries:0,liveGameProcesses:0,
   sharedPreference:'Registry64/String/true unchanged',restoration:'UNNECESSARY_ORIGINALS_UNCHANGED'},
  readerCompatibility:{currentNoInputFocusReader:'NOT_COMPATIBLE_WITH_ACTUAL_ORDER_OR_OBSERVATION_SHAPE',
   expectedCallOrder:['doctor','policy_status','get_window','get_frontmost_app','screenshot'],actualOrderPreserved:true,
   currentReaderSource:'tools/starsector-proxy/seed-failed-attempt.mjs:191-202',observationActualKind:'ACTUAL_S11_DRIVER_FOCUS_BLOCKED_STOP',
   canonicalObservationAliasCreated:false,reason:'Generated immutable alternate focus-stop representation.'},
  claims:{campaign:false,seed:false,stability:false,ordinaryExit:false,native99Acceptance:false,alphaAcceptance:false,newTypedHistoryKind:false},calls,atUtc:new Date(end+3).toISOString()});
 json(paths.result,r);paths.observation=observed;f.refresh();
 return {...f,observedPath:observed,output,source,scenarioId:basename(f.priorScenario)};
}
