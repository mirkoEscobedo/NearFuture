// @ts-check
// Generated provider boundary data. Nothing here launches or controls a game.
import {mkdirSync,writeFileSync,readFileSync,realpathSync} from 'node:fs';
import {resolve} from 'node:path';
import {persistDirectFrame} from '../direct-frames.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {syntheticPng} from '../direct-test-fixture.mjs';

export const preStartPreference={schemaVersion:1,kind:'NATIVE_SEED_SHARED_PREFERENCE_V1',view:'Registry64',node:'/com/fs/starfarer',
    valueName:'canSkipTutorial',registryValueName:'can/Skip/Tutorial',valueKind:'String',value:'true'};

/** @param {any} ctx @param {any} phase @param {number} start @param {number} terminalAt */
export function generatedPreStartProvider(ctx,phase,start,terminalAt) {
    const {s,anchor,iso,powershell,root}=ctx,{batch,runID,source,binding}=phase;
    const output=resolve(root,'.tmp/starsector-proxy',runID);mkdirSync(output);
    ctx.outputs.push(output);
    const window={pid:101,windowId:20,bundleId:'java.exe',title:'Synthetic game',isFocused:true,isOnScreen:true,
        bounds:{x:10,y:20,width:1,height:1}};
    const launcher={...window,windowId:19,title:'Synthetic launcher'};
    const args=(/** @type {any} */ w)=>({coordinate:[10,20],target_app:'java.exe',target_window_id:w.windowId,focus_strategy:'strict'});
    const packet=(/** @type {any} */ v)=>({content:[{type:'text',text:JSON.stringify(v)}],structuredContent:v});
    const names=batch==='B1'?['doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click',
        'get_frontmost_app','list_windows','get_window','screenshot']:
        ['doctor','policy_status','get_window','screenshot',...Array.from({length:3},()=>[
            'get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot']).flat()];
    /** @type {any[]} */const calls=[];
    /** @type {any[]} */const originals=[];
    for(const [i,tool] of names.entries()) {
        const w=batch==='B1'&&i<7?launcher:window;
        const at=start+1000+i*(batch==='B1'?100:1000)+(batch==='B1'&&i>=7?10000:0)+
            (batch==='E2'&&i>=18?(ctx.options.lastClickDelayMs??0):0);
        const a=tool==='doctor'?{include_remediation:true}:tool==='get_window'?{window_id:w.windowId}:
            tool==='screenshot'?{target_window_id:w.windowId,quality:0,width:1}:tool==='list_windows'?{bundle_id:'java.exe'}:
                tool==='left_click'?args(w):{};
        const raw=tool==='screenshot'?{content:[{type:'image',mimeType:'image/png',data:syntheticPng},{type:'text',text:'1x1'}]}:
            tool==='left_click'?{content:[{type:'text',text:'Clicked (10, 20)'}]}:
                packet(tool==='doctor'?{ok:true}:tool==='policy_status'?{approval_required_for_all:true,approval_token_configured:false}:
                    tool==='get_window'?w:tool==='get_frontmost_app'?{app:{pid:101,bundleId:'java.exe'}}:{windows:[window]});
        const post={tool,args:a,outerProtocolID:ctx.outer++,innerProtocolID:i+1,callStarted:iso(at),callCompleted:iso(at+20),
            precedingActions:calls.filter(row=>row.post.tool==='left_click').map(row=>row.post),runID,batch,freshness:'UNKNOWN',isError:false};
        const prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));
        json(prefix+'.private.json',post);json(prefix+'.raw.private.json',raw);
        json(prefix+'.attempt.private.json',{tool,args:a,callStarted:post.callStarted,outerProtocolID:post.outerProtocolID,
            innerProtocolID:'UNKNOWN',sourceSha256:source.sha256,runID,batch});
        calls.push({post,raw,prefix,window:w,refs:{post:ref(prefix+'.private.json'),attempt:ref(prefix+'.attempt.private.json'),raw:ref(prefix+'.raw.private.json')}});
        if(tool==='screenshot') {
            const frame=persistDirectFrame({output,label:'capture-'+String(originals.length+1).padStart(3,'0'),result:raw,args:a,window:w,
                ownedPids:[101],provenance:post});
            if(!frame.pngPath)throw Error('Synthetic complete PNG required');
            writeFileSync(prefix+'.raw.private.json',readFileSync(frame.rawPath));
            calls.at(-1).refs.raw=ref(prefix+'.raw.private.json');
            originals.push({raw:ref(frame.rawPath),original:ref(frame.pngPath),batch,metadata:ref(frame.metadataPath)});
        }
    }
    const n=batch==='B1'?200:batch==='E1'?300:400;
    const readyPath=resolve(output,'guarded-http-ready.private.json'),terminalPath=resolve(output,'owned-provider-terminal.private.json');
    const command=realpathSync.native(process.execPath),childArgs=[resolve(root,'tools/starsector-proxy/guarded-http-child.mjs')];
    json(readyPath,{schemaVersion:1,runID,batch,sourceSha256:source.sha256,startupTimeoutMs:20000,consentMode:'finite-explicit-consent',
        at:iso(start+20),node:{pid:n,parentPid:n+2,sessionId:1,executable:command,creationTime:iso(start)},
        helper:{pid:n+1,parentPid:n,sessionId:1,executable:batch==='E2'?(ctx.options.e2Helper??powershell):powershell,creationTime:iso(start+1)},supervisorJobPids:[n,n+1],
        query:batch==='E2'?(ctx.options.e2Query??'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'):'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'});
    const step={command,args:childArgs,cwd:'.',module:'node:.',processOwnership:'windows-job-object',status:0,timedOut:false,stdout:'',stderr:''};
    const providerClock={startMs:anchor+start,deadlineMs:Math.min(anchor+start+180000,ctx.clock.effectiveDeadlineMs-30000)};
    json(terminalPath,{verifierPid:n+2,verifierStatus:0,signal:null,closeFailed:false,overflow:false,spawnFailed:false,terminalObserved:true,
        cleanupUnknown:false,terminalBoundToChild:true,verdict:'PASS',terminalReceipt:terminalPath,at:iso(terminalAt),
        deadline:new Date(providerClock.deadlineMs+(batch==='E2'?(ctx.options.e2DeadlineOffsetMs??0):0)).toISOString(),steps:[step]});
    json(resolve(output,'owned-provider.stdout.private.txt'),{schemaVersion:1,command:'verify',ok:true,result:{verdict:'PASS',steps:[step]}});
    writeFileSync(resolve(output,'owned-provider.stderr.private.txt'),'');writeFileSync(resolve(output,'batch-dispatch.private.jsonl'),'');
    return {...phase,output,calls,originals,window,launcher,providerClock,ready:ref(readyPath),terminal:ref(terminalPath),
        closure:{calls:calls.length,verifierPid:n+2,at:iso(terminalAt),status:0,timedOut:false,terminalReaped:true,verdict:'PASS'},
        proof:{readySha256:ref(readyPath).sha256,terminalSha256:ref(terminalPath).sha256,observedClosedAtMs:anchor+terminalAt+200,providerReaped:true}};
}

/** @param {any} ctx @param {any} phase @param {number} callIndex @param {string} prefix @param {any} action */
export function generatedPreStartCopy(ctx,phase,callIndex,prefix,action) {
    const call=phase.calls[callIndex-1],output=resolve(ctx.s,prefix);mkdirSync(output);
    const frame=persistDirectFrame({output,label:'capture-001',pngName:'F1.original.png',result:call.raw,args:call.post.args,
        window:call.window,ownedPids:[101],provenance:{...call.post,sourceSha256:phase.source.sha256,
            reservationID:action.reservationID,capturedAtMs:Date.parse(call.post.callCompleted)}});
    if(!frame.pngPath)throw Error('Synthetic copied original required');
    return {pngPath:frame.pngPath,pngSha256:ref(frame.pngPath).sha256,metadataPath:frame.metadataPath,
        metadataSha256:ref(frame.metadataPath).sha256,capturedAtMs:Date.parse(call.post.callCompleted)};
}

/** @param {any} ctx @param {any} phase @param {number} index @param {string} stage @param {number} ordinal @param {number} reservation */
export function generatedPreStartAction(ctx,phase,index,stage,ordinal,reservation) {
    const post=phase.calls[index-1].post;
    return {status:'AWAITING_AFTER',tool:'left_click',batch:phase.batch,runID:phase.runID,sourceSha256:phase.source.sha256,
        reservationID:ctx.controllerPin.sha256+':'+reservation,args:post.args,wizardStep:{ordinal,stage},
        sentAtMs:Date.parse(post.callStarted)+1,knownResultAtMs:Date.parse(post.callCompleted)-1};
}
