// @ts-check
// Generated OS, protocol and terminal data. No provider, game, private trace or image is consumed.
import {resolve} from 'node:path';
import {mkdirSync,writeFileSync,readFileSync,realpathSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {syntheticPng} from '../direct-test-fixture.mjs';
import {persistDirectFrame} from '../direct-frames.mjs';
import {json} from './seed-test-support.mjs';
import {ref} from './seed-failed-history-test-support.mjs';
const callOrders={B1:['doctor','policy_status','get_window','get_frontmost_app','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','list_windows','get_window','screenshot'],
    E1:['doctor','policy_status','get_window','get_frontmost_app','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot']};
/** @param {any} data */
export const budgetPacket=data=>({content:[{type:'text',text:JSON.stringify(data)}],structuredContent:data});
/** @param {any} c @param {any} phase @param {number} start @param {number[]} times */
export function generatedBudgetProvider(c,phase,start,times){
    const {batch,runID,source}=phase,{root,s,anchor,iso,powershell}=c,output=resolve(root,'.tmp/starsector-proxy',runID);mkdirSync(output);c.outputs.push(output);
    const write=(/** @type {string} */ name,/** @type {any} */ v)=>{const path=resolve(output,name);json(path,v);c.indexed.add(path);return path;};
    const width=c.options.differentMappedMenuPoint?480:1,height=c.options.differentMappedMenuPoint?220:1;
    const png=c.options.differentMappedMenuPoint?readFileSync(new URL('../capture/fixtures/readiness.synthetic.png',import.meta.url)).toString('base64'):syntheticPng;
    const window={pid:101,windowId:20,bundleId:'java.exe',title:'Generated menu',isFocused:true,isOnScreen:true,bounds:{x:10,y:20,width,height}},launcher={...window,windowId:19,title:'Generated launcher'};
    /** @type {any[]} */const calls=[];/** @type {any[]} */const originals=[];
    for(const [i,name] of callOrders[/** @type {'B1'|'E1'} */(batch)].entries()){
        const windowNow=batch==='B1'&&i<8?launcher:window,normalized=name==='doctor'?{include_remediation:true}:name==='get_window'?{window_id:windowNow.windowId}:
            name==='screenshot'?{target_window_id:windowNow.windowId,width,quality:0}:name==='left_click'?{coordinate:[10,20],target_app:'java.exe',target_window_id:windowNow.windowId,focus_strategy:'strict'}:{};
        let raw=name==='screenshot'?{content:[{type:'image',mimeType:'image/png',data:png},{type:'text',text:width+'x'+height}]}:
            name==='left_click'?{content:[{type:'text',text:'Clicked (10, 20)'}]}:budgetPacket(name==='doctor'?{ok:true,checks:[]}:name==='policy_status'?{approval_required_for_all:true,approval_token_configured:false}:
                name==='get_window'?windowNow:name==='get_frontmost_app'?{app:{pid:101,bundleId:'java.exe'}}:{windows:[window]});
        const at=times[i],post={tool:name,args:normalized,outerProtocolID:++c.protocol,innerProtocolID:i+1,callStarted:iso(at),callCompleted:iso(at+20),
            precedingActions:calls.filter(x=>x.post.tool==='left_click').map(x=>x.post),runID,batch,freshness:'UNKNOWN',isError:false};
        const attempt={tool:name,args:normalized,outerProtocolID:post.outerProtocolID,innerProtocolID:'UNKNOWN',callStarted:post.callStarted,sourceSha256:source.sha256,runID,batch};
        const data={post,attempt,raw};c.options.call?.(data,batch,i+1);raw=data.raw;
        const label='call-'+String(i+1).padStart(3,'0'),prefix=resolve(output,label);
        write(label+'.private.json',post);write(label+'.attempt.private.json',attempt);write(label+'.raw.private.json',raw);
        const document=c.options.rawDocument?.(Buffer.from(JSON.stringify(raw)),batch,i+1);
        if(document!==undefined)writeFileSync(prefix+'.raw.private.json',document);
        const host={sequence:c.hostCalls.length+1,batchSequence:i+1,batch,name,args:name==='doctor'?{}:post.args,startedAtMs:anchor+at-1,completedAtMs:anchor+at+21,thrown:null,
            actualNonImageToolResponse:name==='screenshot'?null:raw,automaticOriginalRawReference:name==='screenshot'?{path:prefix+'.raw.private.json',sha256:null,status:'EXISTING_MAINTAINED_RAW_HASH_PENDING_FILESYSTEM_READ'}:null,
            actualCallPostPath:prefix+'.private.json',actualCallAttemptPath:prefix+'.attempt.private.json'};
        c.hostCalls.push(host);calls.push({post,attempt,raw,prefix,window:windowNow,host});
        if(name==='screenshot'){
            const cap='capture-'+String(originals.length+1).padStart(3,'0'),frame=persistDirectFrame({output,label:cap,result:raw,args:post.args,window:windowNow,ownedPids:[101],provenance:post});
            if(!frame.pngPath)throw Error('Generated complete original required');
            writeFileSync(prefix+'.raw.private.json',readFileSync(frame.rawPath));
            for(const path of [frame.pngPath,frame.metadataPath,frame.rawPath])c.indexed.add(path);
            originals.push({batch,captureOrdinal:originals.length+1,sourceCallOrdinal:i+1,png:ref(frame.pngPath),metadata:ref(frame.metadataPath),raw:ref(frame.rawPath)});
        }
    }
    const n=batch==='B1'?200:300,ready={schemaVersion:1,at:iso(start+20),runID,batch,sourceSha256:source.sha256,startupTimeoutMs:20000,consentMode:'finite-explicit-consent',
        node:{pid:n,parentPid:n+2,sessionId:1,creationTime:iso(start),executable:realpathSync.native(process.execPath)},helper:{pid:n+1,parentPid:n,sessionId:1,creationTime:iso(start+1),executable:powershell},
        supervisorJobPids:[n,n+1],query:'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'};
    write('guarded-http-ready.private.json',ready);
    const step={command:ready.node.executable,args:[resolve(root,'tools/starsector-proxy/guarded-http-child.mjs')],cwd:'.',processOwnership:'windows-job-object',status:0,timedOut:false};
    const terminal={at:iso(batch==='B1'?18000:204000),verifierPid:n+2,verifierStatus:batch==='B1'?0:null,signal:batch==='B1'?null:'SIGTERM',deadline:iso(start+179000),closeFailed:batch!=='B1',
        overflow:false,spawnFailed:false,terminalObserved:true,cleanupUnknown:false,steps:batch==='B1'?[step]:[],terminalBoundToChild:batch==='B1',verdict:batch==='B1'?'PASS':'UNKNOWN',terminalReceipt:resolve(output,'owned-provider-terminal.private.json')};
    if(batch==='E1')Object.assign(terminal,c.options.e1Terminal??{});
    write('owned-provider-terminal.private.json',terminal);
    const stdout=resolve(output,'owned-provider.stdout.private.txt'),stderr=resolve(output,'owned-provider.stderr.private.txt');
    if(batch==='B1')json(stdout,{schemaVersion:1,command:'verify',ok:true,result:{verdict:'PASS',steps:[{...step,module:'node:.',stdout:'',stderr:''}]}});else writeFileSync(stdout,c.options.e1Stdout??'');
    writeFileSync(stderr,'');c.indexed.add(stdout);c.indexed.add(stderr);
    const audit=calls.map(call=>({timestamp:call.post.callCompleted,duration_ms:10,tool:call.post.tool,mutates:call.post.tool==='left_click',args:call.post.args,
        policy:{allowed:true,approval:'approved',destructive:false},result:{isError:false,content:call.raw.content.map((/** @type {any} */ b)=>b.type==='text'?{type:'text',length:b.text.length,sha256:createHash('sha256').update(b.text).digest('hex')}:
            {type:'image',mimeType:b.mimeType,bytesBase64:b.data.length})}}));
    const dispatch=calls.flatMap((call,i)=>['before-dispatch','host-consent-decision','after-dispatch'].map(phase=>({at:call.post.callCompleted,runID,batch,phase,tool:call.post.tool,
        ...(phase==='host-consent-decision'?{approved:true}:{mutation:call.post.tool==='left_click',index:i+1})})));
    /** @type {Array<[string,any[]]>} */const auditFiles=[['native-audit.private.jsonl',audit],['batch-dispatch.private.jsonl',dispatch]];
    for(const [name,rows] of auditFiles){const path=resolve(output,name);writeFileSync(path,rows.map((/** @type {any} */ row)=>JSON.stringify(row)).join('\n')+'\n');c.indexed.add(path);}
    const begin={args:{consentMode:'finite-explicit-consent',runID,source:source.path},startedAtMs:anchor+start-10,completedAtMs:anchor+start+30,
        raw:{content:[{type:'text',text:JSON.stringify({runID,output,deadline:terminal.deadline})}]},...(batch==='B1'?{thrown:null}:{})};
    const finish={batch,args:{},startedAtMs:Date.parse(terminal.at)-1,completedAtMs:Date.parse(terminal.at)+1,
        raw:{content:[{type:'text',text:JSON.stringify(terminal)}],...(batch==='E1'?{isError:true}:{})},...(batch==='E1'?{thrown:null}:{})};
    c.options.providerResponse?.(begin,finish,batch);c.begins.push(begin);c.finishes.push(finish);
    return{...phase,output,calls,originals,window,launcher,ready,terminal,begin,finish,providerClock:{startMs:anchor+start,deadlineMs:anchor+start+180000}};
}
/** @param {any} c @param {any} provider @param {number} ordinal @param {string} stage @param {number} step @param {number} reserved @returns {any} */
export function generatedBudgetAction(c,provider,ordinal,stage,step,reserved){const call=provider.calls[ordinal-1];return{batch:provider.batch,runID:provider.runID,sourceSha256:provider.source.sha256,
    reservationID:c.controllerPin.sha256+':'+reserved,tool:'left_click',args:call.post.args,status:'AWAITING_AFTER',wizardStep:{ordinal:step,stage},sentAtMs:Date.parse(call.post.callStarted)+1,knownResultAtMs:Date.parse(call.post.callCompleted)-1};}
