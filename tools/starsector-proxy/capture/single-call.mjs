// @ts-check
import {createHash} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';
import {loadCaptureConfiguration} from './capture-adapter.mjs';
import {createCaptureIntentIO} from './capture-intent-io.mjs';
import {readCaptureCompletion} from './capture-completion-reader.mjs';
import {readFileSync,writeFileSync,existsSync,lstatSync} from 'node:fs';
import {resolve,relative,isAbsolute,dirname,basename} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {Client} from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import {StdioClientTransport,getDefaultEnvironment} from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/stdio.mjs';
const observations=new Set(['policy_status','list_windows','get_window','get_frontmost_app','get_display_size','get_tool_guide','cursor_position']);
const sha=(/** @type {Buffer|string} */ b)=>createHash('sha256').update(b).digest('hex');
/** @param {string} root @param {string} path */
function privatePath(root,path){
 if(typeof path!=='string'||!isAbsolute(path))throw Error('ABSOLUTE_PRIVATE_CALL_PATH_REQUIRED');
 const suffix=relative(resolve(root,'.tmp'),path);
 if(!suffix||suffix.startsWith('..')||isAbsolute(suffix))throw Error('PRIVATE_CALL_OUTPUT_REQUIRED');
 for(let part=dirname(path);;part=dirname(part)){if(lstatSync(part).isSymbolicLink())throw Error('ORDINARY_CALL_OUTPUT_REQUIRED');if(dirname(part)===part)break;}
}
/** Refresh observation evidence only; immutable caller authority and clocks stay unchanged.
 * @param {any} client @param {any} p @param {any} configuration @param {any} intent
 * @param {any} actual @param {()=>number} budget */
async function refreshOwnedWitness(client,p,configuration,intent,actual,budget){
 if(configuration.admitted.refreshOwnedWitnessAfterConnect!==true)throw Error('CAPTURE_REFRESH_NOT_ADMITTED');
 const programs=(configuration.runtimePins??[]).filter((/** @type {any} */ pin)=>basename(pin.path).toLowerCase()==='pwsh.exe');
 if(programs.length!==1)throw Error('PINNED_PRODUCER_READER_REQUIRED');
 const path=resolve(configuration.admitted.diagnostics,'owned-processes-current.private.json');
 privatePath(p.workspaceRoot,path);
 const leaf=lstatSync(path);if(leaf.isSymbolicLink()||!leaf.isFile())throw Error('ORDINARY_CAPTURE_PRODUCER_REQUIRED');
 const command="$ErrorActionPreference='Stop';Get-Content -Raw -LiteralPath '"+path.replaceAll("'","''")+"'";
 const args=['-NoLogo','-NoProfile','-NonInteractive','-Command',command],timeout=Math.min(2000,budget()),startMs=Date.now();
 const read=spawnSync(programs[0].path,args,{cwd:p.workspaceRoot,encoding:'utf8',timeout,maxBuffer:262144,windowsHide:true});
 const source={kind:'CHILD_PROCESS_TERMINAL',startMs,endMs:Date.now(),
  args:{program:programs[0].path,args,cwd:p.workspaceRoot,timeoutMs:timeout},
  terminal:{pid:read.pid,exit_code:read.status,signal:read.signal,output:read.stdout,stderr:read.stderr,errorCode:read.error?/** @type {NodeJS.ErrnoException} */(read.error).code:null}};
 const producerPath=p.actualPath+'.producer.private.json';privatePath(p.workspaceRoot,producerPath);
 writeFileSync(producerPath,JSON.stringify(source)+'\n',{flag:'wx'});
 actual.refreshProof={producerPath,initialWitnessAgeMs:Date.now()-intent.ownedWitness.atMs,nativeObservations:0};
 if(read.status!==0||read.signal||read.error)throw Error('CURRENT_CAPTURE_PRODUCER_READ_REQUIRED');
 const sample=JSON.parse(read.stdout),grant=intent.producerWitness,prior=intent.ownedWitness.row;
 const rows=Array.isArray(sample.processes)?sample.processes.filter((/** @type {any} */ row)=>row.pid===prior.pid):[];
 const row=rows[0],birth=(/** @type {any} */ value)=>typeof value==='string'?Date.parse(value):NaN;
 const samePath=(/** @type {string} */ a,/** @type {string} */ b)=>typeof a==='string'&&typeof b==='string'&&resolve(a).toLowerCase()===resolve(b).toLowerCase();
 if(sample.controllerPid!==grant.controllerPid||sample.controllerKernelCreationFileTime!==grant.controllerKernelCreationFileTime||
  !Number.isFinite(birth(sample.controllerCreationTime))||birth(sample.controllerCreationTime)!==birth(grant.controllerCreationTime)||
  sample.conservativeOuterDeadlineUtc!==grant.conservativeOuterDeadlineUtc||sample.controllerDeadlineUtc!==grant.controllerDeadlineUtc||
  rows.length!==1||row.parentPid!==prior.parentPid||row.kernelCreationFileTime!==prior.kernelCreationFileTime||
  !Number.isFinite(birth(row.creationTime))||birth(row.creationTime)!==birth(prior.creationTime)||!samePath(row.executable,prior.executable)||
  row.sessionId!==prior.sessionId||row.inSupervisorJob!==true||!Array.isArray(row.supervisorJobPids)||
  !row.supervisorJobPids.includes(row.pid)||!row.supervisorJobPids.includes(grant.controllerPid)||
  row.mainWindowHandle!==String(intent.window.windowId))throw Error('CURRENT_CAPTURE_PRODUCER_IDENTITY_REQUIRED');
 const atMs=birth(sample.at);actual.witnessAgeMs=Date.now()-atMs;
 if(!Number.isSafeInteger(atMs)||atMs<intent.rootClock.startMs||actual.witnessAgeMs<0||actual.witnessAgeMs>5000)throw Error('EXACT_FRESH_CALLER_CAPTURE_REQUIRED');
 actual.refreshProof.nativeObservations=1;
 const windowStartMs=Date.now(),raw=await client.callTool({name:'get_window',arguments:{window_id:intent.window.windowId}},{timeout:budget()});
 const windowEndMs=Date.now(),windowPath=p.actualPath+'.window.private.json';privatePath(p.workspaceRoot,windowPath);
 const encoded=JSON.stringify({startMs:windowStartMs,endMs:windowEndMs,tool:'get_window',args:{window_id:intent.window.windowId},raw})+'\n';
 if(Buffer.byteLength(encoded)>262144)throw Error('BOUNDED_CURRENT_WINDOW_REQUIRED');
 writeFileSync(windowPath,encoded,{flag:'wx'});actual.refreshProof.windowPath=windowPath;
 const window=raw.structuredContent??JSON.parse(raw.content?.find((/** @type {any} */ b)=>b.type==='text')?.text??'null');
 if(raw.isError||window?.pid!==prior.pid||window.windowId!==intent.window.windowId||window.isOnScreen!==true||
  !isDeepStrictEqual(window.bounds,intent.window.bounds)||(intent.window.isFocused===true&&window.isFocused!==true))
  throw Error('CURRENT_CAPTURE_WINDOW_REQUIRED');
 return {...intent,createdMs:Date.now(),window,ownedWitness:{rootRecordKey:intent.rootClock.recordKey,atMs,row,source}};
}

/** One caller-authorized raw MCP operation. No game launch or input authority is created.
 * Caller must keep the original game clock, counters and inspected-image requirements.
 * @param {any} p */
export async function executeSingleCall(p){
 if(p?.schemaVersion!==1||p.kind!=='NATIVE99_SINGLE_SDK_CALL_V1'||!(observations.has(p.call?.name)||p.call?.name==='screenshot'&&p.capture))
  throw Error('CLOSED_SINGLE_SDK_OBSERVATION_REQUIRED');
 for(const path of [p.actualPath,p.rawPath,p.stopPath])privatePath(p.workspaceRoot,path);
 if(existsSync(p.stopPath))throw Error('DRIVER_ALREADY_LATCHED');
 if(!Number.isSafeInteger(p.deadlineMs)||p.deadlineMs<=Date.now()||p.deadlineMs-Date.now()>480000)throw Error('ORIGINAL_BOUNDED_CALL_DEADLINE_REQUIRED');
 const actual={schemaVersion:1,kind:p.kind,startMs:Date.now(),call:p.call,captureProof:/** @type {any} */(null),toolStartedMs:/** @type {number|null} */(null),toolEndedMs:/** @type {number|null} */(null),serverPid:/** @type {number|null} */(null),
  closeReturned:false,serverPidAbsent:false,dispatchAttempted:false,serverStderr:'',serverStderrOverflow:false,
  refreshProof:/** @type {any} */(null),failure:/** @type {any} */(null),witnessAgeMs:/** @type {number|null} */(null),rawPath:p.rawPath,rawSha256:/** @type {string|null} */(null),resultScope:'SDK_TOOL_RESULT_NOT_JSONRPC_ENVELOPE'};
 writeFileSync(p.actualPath,JSON.stringify(actual)+'\n',{flag:'wx'});
 const transport=new StdioClientTransport({command:p.node,args:[p.entrypoint,p.configuration.path,p.configuration.sha256],
  env:{...getDefaultEnvironment(),...p.environment},cwd:p.workspaceRoot,stderr:'pipe'});
 const client=new Client({name:'near-future-single-sdk-call',version:'1'},{capabilities:{},supportedProtocolVersions:['2025-11-25']});
 /** @type {any} */let failure;let phase='CONNECT';
 /** @type {Promise<void>|undefined} */let closing;
 function close(){return closing??=client.close();}
 function budget(){
  if(failure)throw failure;
  const remaining=p.deadlineMs-Date.now();
  if(remaining<=0||existsSync(p.stopPath))throw Error('ORIGINAL_CALL_BOUNDARY_OR_DRIVER_STOP');
  return Math.min(20000,remaining);
 }
 transport.stderr?.on('data',chunk=>{
  if(actual.serverStderrOverflow)return;
  const text=String(chunk),remaining=65536-actual.serverStderr.length;actual.serverStderr+=text.slice(0,remaining);
  if(text.length>remaining){actual.serverStderrOverflow=true;actual.serverPid??=transport.pid;failure??=Error('BOUNDED_PROVIDER_STDERR_REQUIRED');void close().catch(error=>{failure??=error;});}
 });
 try{
  await client.connect(transport,{timeout:budget()});actual.serverPid=transport.pid;
  /** @type {any} */let capture;
  if(p.call.name==='screenshot'){
   let text=p.capture.intentText;
   if(typeof text!=='string'||Buffer.byteLength(text)>262144)throw Error('BOUNDED_CALLER_CAPTURE_INTENT_REQUIRED');
   let intent=JSON.parse(text);const configuration=loadCaptureConfiguration(p.configuration.path,p.configuration.sha256);
   if(p.capture.refreshOwnedWitnessAfterConnect===true){phase='REFRESH';intent=await refreshOwnedWitness(client,p,configuration,intent,actual,budget);text=JSON.stringify(intent)+'\n';}
   phase='INTENT_ADMISSION';
   const binding=configuration.admitted.bindings.find((/** @type {any} */ b)=>b.id===intent.bindingId&&b.batchId===intent.batchId);
   const index=binding?.frames.findIndex((/** @type {any} */ f)=>f.id===intent.frameId)??-1;
   const age=Date.now()-intent.ownedWitness?.atMs;actual.witnessAgeMs=age;
   if(index<0||!isDeepStrictEqual(intent.args,p.call.args)||age<0||age>5000)throw Error('EXACT_FRESH_CALLER_CAPTURE_REQUIRED');
   phase='PUBLISH';createCaptureIntentIO(configuration).publish({intentText:text,expectedPointerSHA256:p.capture.expectedPointerSHA256});
   capture={intent,intentText:text,slot:binding.frames[index],captureOrdinal:index+1};
  }else if(p.capture!==undefined)throw Error('CAPTURE_CONTEXT_ONLY_FOR_SCREENSHOT');
  if(capture){actual.witnessAgeMs=Date.now()-capture.intent.ownedWitness.atMs;if(actual.witnessAgeMs<0||actual.witnessAgeMs>5000)throw Error('EXACT_FRESH_CALLER_CAPTURE_REQUIRED');}
  phase='CAPTURE';const timeout=budget();actual.dispatchAttempted=true;
  writeFileSync(p.actualPath,JSON.stringify(actual)+'\n');
  actual.toolStartedMs=Date.now();
  const result=await client.callTool({name:p.call.name,arguments:p.call.args},{timeout});
  actual.toolEndedMs=Date.now();
  const raw=JSON.stringify(result)+'\n';
  if(Buffer.byteLength(raw)>32*1024*1024)throw Error('BOUNDED_SDK_RESULT_REQUIRED');
  writeFileSync(p.rawPath,raw,{flag:'wx'});actual.rawSha256=sha(raw);
  if(result.isError===true)throw Error('SDK_TOOL_REFUSED');
  if(capture){
   const images=result.content.filter(block=>block.type==='image');
   if(images.length!==1)throw Error('ONE_ORIGINAL_SDK_IMAGE_REQUIRED');
   actual.captureProof=readCaptureCompletion({...capture,captureStartMs:actual.toolStartedMs,captureEndMs:actual.toolEndedMs,
    inlineImageSHA256:sha(images[0].data),inlineImageCharacters:images[0].data.length,nonImageContent:result.content.filter(block=>block.type!=='image')});
  }
  budget();
 }catch(error){failure??=error;const code=error instanceof Error?error.message:'';actual.failure??={phase,code:new Set(["CAPTURE_REFRESH_NOT_ADMITTED","PINNED_PRODUCER_READER_REQUIRED","CURRENT_CAPTURE_PRODUCER_READ_REQUIRED","ORDINARY_CAPTURE_PRODUCER_REQUIRED","CURRENT_CAPTURE_PRODUCER_IDENTITY_REQUIRED","EXACT_FRESH_CALLER_CAPTURE_REQUIRED","BOUNDED_CURRENT_WINDOW_REQUIRED","CURRENT_CAPTURE_WINDOW_REQUIRED","EXACT_ADMITTED_CAPTURE_BINDING_REQUIRED","SDK_TOOL_REFUSED"]).has(code)?code:'UNCLASSIFIED_CALL_FAILURE'};}
 finally{
  actual.serverPid??=transport.pid;
  try{await close();actual.closeReturned=true;}catch(error){failure??=error;const code=error instanceof Error?error.message:'';actual.failure??={phase,code:new Set(["CAPTURE_REFRESH_NOT_ADMITTED","PINNED_PRODUCER_READER_REQUIRED","CURRENT_CAPTURE_PRODUCER_READ_REQUIRED","ORDINARY_CAPTURE_PRODUCER_REQUIRED","CURRENT_CAPTURE_PRODUCER_IDENTITY_REQUIRED","EXACT_FRESH_CALLER_CAPTURE_REQUIRED","BOUNDED_CURRENT_WINDOW_REQUIRED","CURRENT_CAPTURE_WINDOW_REQUIRED","EXACT_ADMITTED_CAPTURE_BINDING_REQUIRED","SDK_TOOL_REFUSED"]).has(code)?code:'UNCLASSIFIED_CALL_FAILURE'};}
  if(actual.serverPid){try{process.kill(actual.serverPid,0);}catch(error){if(/** @type {NodeJS.ErrnoException} */(error).code==='ESRCH')actual.serverPidAbsent=true;else failure??=error;}}
  if(!actual.serverPidAbsent||!actual.closeReturned)failure??=Error('OWNED_SDK_CLEANUP_UNCONFIRMED');
  if(failure&&!actual.failure)actual.failure={phase:'CLOSE',code:'UNCLASSIFIED_CALL_FAILURE'};
  const receipt={...actual,endMs:Date.now(),status:failure?'HOLD':'SDK_CALL_AND_CLEANUP_COMPLETED'};
  try{writeFileSync(p.actualPath,JSON.stringify(receipt)+'\n');}catch(error){failure??=error;receipt.status='HOLD';}
  if(failure&&!existsSync(p.stopPath))writeFileSync(p.stopPath,JSON.stringify({status:'NO_FURTHER_DISPATCH',actualPath:p.actualPath,atMs:Date.now()})+'\n',{flag:'wx'});
  if(failure)throw Error('SDK_SINGLE_CALL_HOLD');
  return receipt;
 }
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const [path,pin,...extra]=process.argv.slice(2),bytes=readFileSync(path);
 if(extra.length||bytes.length>262144||!/^([a-f0-9]{64})$/.test(pin??'')||sha(bytes)!==pin)throw Error('EXACT_SINGLE_CALL_INPUT_REQUIRED');
 const request=JSON.parse(bytes.toString('utf8'));
 try{const result=await executeSingleCall(request);console.log(JSON.stringify({status:result.status,actualPath:request.actualPath,rawPath:request.rawPath,serverPid:result.serverPid,serverPidAbsent:result.serverPidAbsent}));}
 catch{console.log(JSON.stringify({status:'HOLD',actualPath:request.actualPath,rawPath:request.rawPath}));process.exitCode=1;}
}
