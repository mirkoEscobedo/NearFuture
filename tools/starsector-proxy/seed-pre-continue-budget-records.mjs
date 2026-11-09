// @ts-check
// Replay only the two recorded providers and their closed 29-call, three-click prefix.
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {preStartOriginal,preStartPacket} from './seed-pre-start-margin-records.mjs';
import {budgetHistoryJson,budgetHistoryNeed as need,budgetHistoryInteger as integer,budgetHistoryObject as object,budgetHistoryTime as time} from './seed-pre-continue-budget-index.mjs';
const orders={B1:['doctor','policy_status','get_window','get_frontmost_app','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','list_windows','get_window','screenshot'],
    E1:['doctor','policy_status','get_window','get_frontmost_app','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot']};
/** @param {any} raw @param {boolean} failed */
function terminalPacket(raw,failed){need(closed(raw,failed?['content','isError']:['content'])&&(!failed||raw.isError===true)&&Array.isArray(raw.content)&&raw.content.length===1&&
    closed(raw.content[0],['type','text'])&&raw.content[0].type==='text'&&typeof raw.content[0].text==='string');
    return budgetHistoryJson(Buffer.from(raw.content[0].text));}
/** @param {any} ids */
const pids=ids=>Array.isArray(ids)&&ids.length>0&&ids.every((/** @type {any} */ id)=>integer(id,1))&&new Set(ids).size===ids.length;
/** @param {any} ctx */
export function assertPreContinueBudgetRecords(ctx){
    const {v,reader,index,phases,clock,c,node,powershell,g,actions}=ctx,{json,ref}=reader,s=v.scenario;
    const custody=json(resolve(s,'driver-call-metadata-custody.actual.private.json'));
    need(custody.schemaVersion===1&&custody.kind==='ACTUAL_S15_DRIVER_CALL_AND_METADATA_CUSTODY'&&custody.scenario===s&&custody.root===v.sourceWorkspace&&
        custody.rootGo===true&&custody.rootDeadline===clock.effectiveDeadlineMs&&custody.cleanupBy===clock.effectiveDeadlineMs-30000&&
        custody.acknowledgedActions===3&&custody.initialActivations===0&&custody.continueSent===false&&custody.e2Started===false&&custody.seedCreated===false&&custody.ordinaryExit===false&&
        Array.isArray(custody.begins)&&custody.begins.length===2&&Array.isArray(custody.finishes)&&custody.finishes.length===2&&
        Array.isArray(custody.calls)&&custody.calls.length===29&&Array.isArray(index.calls)&&index.calls.length===29&&Array.isArray(index.originals)&&index.originals.length===5);
    /** @type {Record<string,any>} */const providers={};let global=0,actionIndex=0;
    for(const [bi,batch] of ['B1','E1'].entries()){
        const phase=phases[batch],binding=phase.claim.binding,output=phase.output,ready=json(resolve(output,'guarded-http-ready.private.json')),
            terminal=json(resolve(output,'owned-provider-terminal.private.json')),begin=custody.begins[bi],finish=custody.finishes[bi];
        const pc=batch==='B1'?g.resolvedB1.providerClock:g.providerClock;
        need(closed(pc,['startMs','deadlineMs'])&&integer(pc.startMs,clock.startMs)&&pc.deadlineMs===Math.min(pc.startMs+180000,clock.effectiveDeadlineMs-30000)&&
            ready.schemaVersion===1&&ready.runID===binding.runID&&ready.batch===batch&&ready.sourceSha256===phase.source.sha256&&ready.startupTimeoutMs===20000&&
            ready.consentMode==='finite-explicit-consent'&&time(ready.at)&&Date.parse(ready.at)>=pc.startMs&&Date.parse(ready.at)<=pc.startMs+20000&&
            pids(ready.supervisorJobPids)&&ready.supervisorJobPids.length===2&&ready.query==='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)');
        for(const [row,exe] of [[ready.node,node],[ready.helper,powershell]])need(object(row)&&integer(row.pid,1)&&integer(row.parentPid,1)&&row.sessionId===c.sessionId&&row.executable===exe&&
            time(row.creationTime)&&Date.parse(row.creationTime)>=pc.startMs&&Date.parse(row.creationTime)<=Date.parse(ready.at)&&ready.supervisorJobPids.includes(row.pid));
        need(closed(terminal,['at','verifierPid','verifierStatus','signal','deadline','closeFailed','overflow','spawnFailed','terminalObserved','cleanupUnknown','steps','terminalBoundToChild','verdict','terminalReceipt'])&&
            integer(terminal.verifierPid,1)&&ready.node.parentPid===terminal.verifierPid&&ready.helper.parentPid===ready.node.pid&&ready.node.pid!==ready.helper.pid&&
            terminal.overflow===false&&terminal.spawnFailed===false&&terminal.terminalObserved===true&&terminal.cleanupUnknown===false&&
            terminal.terminalReceipt===resolve(output,'owned-provider-terminal.private.json')&&time(terminal.at)&&time(terminal.deadline)&&
            Date.parse(terminal.deadline)<=pc.deadlineMs&&Date.parse(terminal.at)>=Date.parse(ready.at));
        if(batch==='B1'){
            const spool=json(resolve(output,'owned-provider.stdout.private.txt'));
            const step={command:node,args:[resolve(v.sourceWorkspace,'tools/starsector-proxy/guarded-http-child.mjs')],cwd:'.',processOwnership:'windows-job-object',status:0,timedOut:false};
            need(terminal.verifierStatus===0&&terminal.signal===null&&terminal.closeFailed===false&&terminal.terminalBoundToChild===true&&terminal.verdict==='PASS'&&
                Date.parse(terminal.at)<=Date.parse(terminal.deadline)&&same(terminal.steps,[step])&&spool.schemaVersion===1&&spool.command==='verify'&&spool.ok===true&&
                spool.result?.verdict==='PASS'&&same(spool.result.steps,[{...step,module:'node:.',stdout:'',stderr:''}])&&g.resolvedB1.providerClosedAtMs===Date.parse(terminal.at));
        }else need(terminal.verifierStatus===null&&terminal.signal==='SIGTERM'&&terminal.closeFailed===true&&terminal.terminalBoundToChild===false&&terminal.verdict==='UNKNOWN'&&
            same(terminal.steps,[])&&reader.bytes(resolve(output,'owned-provider.stdout.private.txt')).bytes.length===0);
        need(reader.bytes(resolve(output,'owned-provider.stderr.private.txt')).bytes.length===0);
        need(closed(begin,['args','startedAtMs','completedAtMs','raw',...(batch==='B1'?['thrown']:[])])&&(batch!=='B1'||begin.thrown===null)&&closed(begin.args,['consentMode','runID','source'])&&begin.args.consentMode==='finite-explicit-consent'&&
            begin.args.runID===binding.runID&&typeof begin.args.source==='string'&&resolve(begin.args.source)===phase.source.path&&
            same(terminalPacket(begin.raw,false),{runID:binding.runID,output,deadline:terminal.deadline})&&integer(begin.startedAtMs,clock.startMs)&&integer(begin.completedAtMs,begin.startedAtMs)&&
            begin.startedAtMs<=pc.startMs&&Date.parse(ready.at)<=begin.completedAtMs&&
            closed(finish,['batch','args','startedAtMs','completedAtMs','raw',...(batch==='E1'?['thrown']:[])])&&(batch!=='E1'||finish.thrown===null)&&finish.batch===batch&&same(finish.args,{})&&same(terminalPacket(finish.raw,batch==='E1'),terminal)&&
            integer(finish.startedAtMs,begin.completedAtMs)&&integer(finish.completedAtMs,finish.startedAtMs)&&finish.startedAtMs<=Date.parse(terminal.at)&&Date.parse(terminal.at)<=finish.completedAtMs);
        /** @type {any[]} */const calls=[];let previous=begin.completedAtMs,protocol=0,window=null,foreground=null,capture=0;
        const names=orders[/** @type {'B1'|'E1'} */(batch)];
        for(const [i,name] of names.entries()){
            const item=index.calls[global],host=custody.calls[global],prefix=resolve(output,'call-'+String(i+1).padStart(3,'0'));
            need(closed(item,['batch','ordinal','tool','args','callerTimes','post','attempt','raw'])&&item.batch===batch&&item.ordinal===i+1&&item.tool===name&&
                closed(item.callerTimes,['startedAtMs','completedAtMs']));
            const post=reader.pinJson({path:item.post.path,sha256:item.post.sha256},prefix+'.private.json'),attempt=reader.pinJson({path:item.attempt.path,sha256:item.attempt.sha256},prefix+'.attempt.private.json'),
                raw=reader.pinJson({path:item.raw.path,sha256:item.raw.sha256},prefix+'.raw.private.json');
            need(closed(post,['tool','args','outerProtocolID','innerProtocolID','callStarted','callCompleted','precedingActions','runID','batch','freshness','isError'])&&
                closed(attempt,['tool','args','outerProtocolID','innerProtocolID','callStarted','sourceSha256','runID','batch'])&&post.tool===name&&attempt.tool===name&&
                same(post.args,attempt.args)&&same(post.args,name==='doctor'?{include_remediation:true}:item.args)&&
                post.runID===binding.runID&&attempt.runID===binding.runID&&post.batch===batch&&attempt.batch===batch&&attempt.sourceSha256===phase.source.sha256&&
                post.isError===false&&post.freshness==='UNKNOWN'&&same(post.precedingActions,calls.filter(x=>x.post.tool==='left_click').map(x=>x.post))&&
                integer(post.outerProtocolID,protocol+1)&&(i===0||post.outerProtocolID===protocol+1)&&attempt.outerProtocolID===post.outerProtocolID&&post.innerProtocolID===i+1&&attempt.innerProtocolID==='UNKNOWN'&&
                post.callStarted===attempt.callStarted&&time(post.callStarted)&&time(post.callCompleted)&&Date.parse(post.callStarted)>=previous&&
                Date.parse(post.callCompleted)>=Date.parse(post.callStarted)&&Date.parse(post.callCompleted)<=Math.min(pc.deadlineMs,Date.parse(terminal.deadline))&&
                host.sequence===global+1&&host.batchSequence===i+1&&host.batch===batch&&host.name===name&&same(host.args,item.args)&&host.thrown===null&&
                integer(host.startedAtMs,previous)&&integer(host.completedAtMs,host.startedAtMs)&&host.startedAtMs<=Date.parse(post.callStarted)&&Date.parse(post.callCompleted)<=host.completedAtMs&&
                same(item.callerTimes,{startedAtMs:host.startedAtMs,completedAtMs:host.completedAtMs})&&host.actualCallPostPath===prefix+'.private.json'&&host.actualCallAttemptPath===prefix+'.attempt.private.json');
            if(name==='screenshot')need(host.actualNonImageToolResponse===null&&host.automaticOriginalRawReference?.path===prefix+'.raw.private.json');
            else need(same(host.actualNonImageToolResponse,raw)&&host.automaticOriginalRawReference===null);
            let payload;
            if(name==='left_click'){
                const action=actions[actionIndex++];
                need(same(post.args,action.args)&&action.batch===batch&&action.runID===binding.runID&&action.sourceSha256===phase.source.sha256&&
                    action.sentAtMs>=Date.parse(post.callStarted)&&action.knownResultAtMs<=Date.parse(post.callCompleted)&&
                    same(raw,{content:[{type:'text',text:'Clicked ('+action.args.coordinate.join(', ')+')'}]})&&
                    object(window)&&window.isFocused===true&&foreground?.pid===window.pid&&window.windowId===action.args.target_window_id&&
                    calls.at(-1)?.post.tool==='get_window'&&calls.at(-2)?.post.tool==='get_frontmost_app');
            }else if(name==='screenshot'){
                need(object(window)&&window.isFocused===true&&foreground?.pid===window.pid&&same(post.args,{target_window_id:window.windowId,width:window.bounds.width,quality:0}));
                const png=preStartOriginal(raw,window),base=resolve(output,'capture-'+String(++capture).padStart(3,'0')),m=json(base+'.metadata.private.json');
                need(reader.bytes(base+'.png').bytes.equals(png)&&reader.bytes(base+'.raw.private.json').bytes.equals(reader.bytes(prefix+'.raw.private.json').bytes)&&
                    m.captureReceipt===true&&m.pngPath===base+'.png'&&m.rawPath===base+'.raw.private.json'&&m.pngSha256===ref(base+'.png').sha256&&m.rawSha256===ref(base+'.raw.private.json').sha256&&
                    same(m.window,window)&&same(m.dimensions,{width:window.bounds.width,height:window.bounds.height})&&
                    ['tool','args','outerProtocolID','innerProtocolID','callStarted','callCompleted','precedingActions','runID','batch','freshness'].every(k=>same(m[k],post[k])));
            }else{
                payload=preStartPacket(raw);
                if(name==='get_window'){
                    need(object(payload)&&integer(payload.pid,1)&&payload.pid===binding.ownedPids[0]&&integer(payload.windowId,1)&&payload.bundleId==='java.exe'&&
                        payload.isFocused===true&&payload.isOnScreen===true&&closed(payload.bounds,['x','y','width','height'])&&Object.values(payload.bounds).every(Number.isSafeInteger)&&
                        payload.bounds.width>0&&payload.bounds.height>0&&same(post.args,{window_id:payload.windowId}));window=payload;
                }else if(name==='get_frontmost_app'){need(closed(payload,['app'])&&object(payload.app)&&payload.app.pid===binding.ownedPids[0]&&payload.app.bundleId==='java.exe'&&same(post.args,{}));foreground=payload.app;}
                else if(name==='doctor')need(payload.ok===true&&Array.isArray(payload.checks)&&same(item.args,{}));
                else if(name==='policy_status')need(payload.approval_required_for_all===true&&payload.approval_token_configured===false&&same(post.args,{}));
                else need(name==='list_windows'&&same(post.args,{}));
            }
            calls.push({post,attempt,raw,payload,window});protocol=post.outerProtocolID;previous=host.completedAtMs;global++;
        }
        need(capture===(batch==='B1'?2:3)&&previous<=finish.startedAtMs);
        const decodeLines=(/** @type {string} */ path)=>reader.bytes(path).bytes.toString('utf8').trim().split(/\r?\n/).map((/** @type {string} */ line)=>budgetHistoryJson(Buffer.from(line)));
        const audit=decodeLines(resolve(output,'native-audit.private.jsonl')),dispatch=decodeLines(resolve(output,'batch-dispatch.private.jsonl'));
        need(audit.length===calls.length);
        for(const [i,row] of audit.entries()){
            const call=calls[i];need(row.tool===call.post.tool&&row.mutates===(row.tool==='left_click')&&same(row.args,call.post.args)&&row.policy?.allowed===true&&
                row.policy.approval==='approved'&&row.policy.destructive===false&&row.result?.isError===false&&time(row.timestamp)&&integer(row.duration_ms)&&
                Date.parse(row.timestamp)>=Date.parse(call.post.callStarted)&&Date.parse(row.timestamp)<=Date.parse(call.post.callCompleted)&&Array.isArray(row.result.content)&&row.result.content.length===call.raw.content.length);
            for(const [j,block] of call.raw.content.entries())need(same(row.result.content[j],block.type==='text'?{type:'text',length:block.text.length,sha256:createHash('sha256').update(block.text).digest('hex')}:
                {type:'image',mimeType:block.mimeType,bytesBase64:block.data.length}));
        }
        for(const phaseName of ['before-dispatch','after-dispatch','host-consent-decision']){
            const rows=dispatch.filter((/** @type {any} */ row)=>row.phase===phaseName);need(rows.length===calls.length);
            for(const [i,row] of rows.entries())need(row.runID===binding.runID&&row.batch===batch&&row.tool===calls[i].post.tool&&time(row.at)&&
                (phaseName==='host-consent-decision'?row.approved===true:row.mutation===(row.tool==='left_click')));
        }
        providers[batch]={output,ready,terminal,providerClock:pc,calls,begin,finish};
    }
    need(global===29&&actionIndex===3&&same(g.resolvedB1.sourceSha256,phases.B1.source.sha256)&&g.resolvedB1.resolvedAtMs===g.providerClock.startMs);
    return{...ctx,custody,providers};
}
