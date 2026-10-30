// @ts-check
// Closed historical provider/capture records; no process or input effects.
import {resolve,isAbsolute,basename} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {file,closed,requireThat} from './campaign-proof.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {completePng} from './direct-frames.mjs';

export const preStartNeed=(/** @type {any} */ ok)=>requireThat(ok,'closed pre-Start margin failure custody required');
export const preStartObject=(/** @type {any} */ v)=>v!==null&&typeof v==='object'&&!Array.isArray(v);
export const preStartInteger=(/** @type {any} */ v,minimum=0)=>Number.isSafeInteger(v)&&v>=minimum;
export const preStartTime=(/** @type {any} */ v)=>typeof v==='string'&&Number.isSafeInteger(Date.parse(v));
const need=preStartNeed,object=preStartObject,integer=preStartInteger;

/** Strict UTF-8 and duplicate-free JSON across every file and embedded packet. */
export function preStartReader() {
    /** @type {Map<string,{path:string,sha256:string,bytes:Buffer}>} */const files=new Map();
    /** @param {string} path @param {number} [limit] */
    function bytes(path,limit=262144) {
        need(typeof path==='string'&&isAbsolute(path)&&resolve(path)===path);
        const found=files.get(path)??{path,...file(path,limit)};need(found.bytes.length<=limit);files.set(path,found);return found;
    }
    /** @param {string} path @param {number} [limit] */
    function json(path,limit=262144) {return zeroProviderObject(new TextDecoder('utf-8',{fatal:true}).decode(bytes(path,limit).bytes),need);}
    /** @param {any} pin @param {string} expected @param {number} [limit] */
    function pinned(pin,expected,limit=262144) {
        need(closed(pin,['path','sha256'])&&pin.path===expected&&typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256));
        const found=bytes(expected,limit);need(found.sha256===pin.sha256);return found;
    }
    /** @param {any} pin @param {string} expected @param {number} [limit] */
    function pinJson(pin,expected,limit=262144) {pinned(pin,expected,limit);return json(expected,limit);}
    /** @param {string} path */
    function ref(path) {const found=bytes(path,32*1024*1024);return {path,sha256:found.sha256};}
    return {bytes,json,pinned,pinJson,ref};
}

/** @param {any} raw */
export function preStartPacket(raw) {
    need(object(raw)&&(!Object.hasOwn(raw,'isError')||raw.isError===false)&&Array.isArray(raw.content)&&
        raw.content.length===1&&closed(raw.content[0],['type','text'])&&raw.content[0].type==='text'&&
        typeof raw.content[0].text==='string'&&Buffer.byteLength(raw.content[0].text)<=65536);
    const value=zeroProviderObject(raw.content[0].text,need);
    if(Object.hasOwn(raw,'structuredContent'))need(object(raw.structuredContent)&&same(raw.structuredContent,value));
    return value;
}

/** @param {any} raw @param {any} window */
export function preStartOriginal(raw,window) {
    need(object(raw)&&(!Object.hasOwn(raw,'isError')||raw.isError===false)&&Array.isArray(raw.content)&&
        (raw.content.length===1||raw.content.length===2));
    const image=raw.content[0];
    need(object(image)&&image.type==='image'&&image.mimeType==='image/png'&&typeof image.data==='string'&&image.data.length<=44*1024*1024);
    const png=Buffer.from(image.data,'base64');need(png.toString('base64')===image.data);
    const dimensions=completePng(png);
    if(raw.content.length===2)need(closed(raw.content[1],['type','text'])&&raw.content[1].type==='text'&&
        raw.content[1].text===dimensions.width+'x'+dimensions.height);
    need(object(window)&&closed(window.bounds,['x','y','width','height'])&&Object.values(window.bounds).every(Number.isSafeInteger)&&
        window.bounds.width>0&&window.bounds.height>0&&dimensions.width===window.bounds.width&&dimensions.height===window.bounds.height);
    return png;
}

/** All55 calls are pinned by the root receipt, not inferred from a directory alone.
 * @param {any} context */
export function readPreStartProviders(context) {
    const {v,r,reader,phases,controller,clock}=context;
    const {json,pinJson,pinned,ref}=reader;
    /** @type {Record<string,any>} */const providers={};
    /** @type {any[]} */const originals=[];
    for(const [batch,count] of /** @type {[string,number][]} */([['B1',11],['E1',22],['E2',22]])) {
        const phase=phases[batch],binding=phase.claim.binding,run=binding.runID;
        need(typeof run==='string'&&/^[A-Za-z0-9_-]+$/.test(run));
        const output=resolve(v.sourceWorkspace,'.tmp/starsector-proxy',run);
        const expectedNames=Array.from({length:count},(_,i)=>'call-'+String(i+1).padStart(3,'0')+'.private.json');
        need(same(readdirSync(output).filter((/** @type {string} */ n)=>/^call-[0-9]{3}\.private\.json$/.test(n)).sort(),expectedNames));
        const ready=pinJson(r.evidence[batch+'/guarded-http-ready.private.json'],resolve(output,'guarded-http-ready.private.json'));
        const terminal=pinJson(r.evidence[batch+'/owned-provider-terminal.private.json'],resolve(output,'owned-provider-terminal.private.json'));
        const spool=pinJson(r.evidence[batch+'/owned-provider.stdout.private.txt'],resolve(output,'owned-provider.stdout.private.txt'));
        const stderr=pinned(r.evidence[batch+'/owned-provider.stderr.private.txt'],resolve(output,'owned-provider.stderr.private.txt'));
        pinned(r.evidence[batch+'/batch-dispatch.private.jsonl'],resolve(output,'batch-dispatch.private.jsonl'));
        const actions=context.actions.filter((/** @type {any} */ a)=>a.batch===batch);
        const providerClock=batch==='B1'?r.closure.providers.B1&&context.ledger.resolvedB1.providerClock:
            batch==='E1'?context.ledger.resolvedE1.providerClock:context.ledger.providerClock;
        need(integer(providerClock.startMs,clock.startMs)&&providerClock.deadlineMs===Math.min(providerClock.startMs+180000,clock.effectiveDeadlineMs-30000));
        need(ready.schemaVersion===1&&ready.runID===run&&ready.batch===batch&&ready.sourceSha256===ref(phase.source.path).sha256&&
            ready.startupTimeoutMs===20000&&ready.consentMode==='finite-explicit-consent'&&preStartTime(ready.at)&&
            Date.parse(ready.at)>=providerClock.startMs&&Date.parse(ready.at)<=providerClock.startMs+20000&&
            Array.isArray(ready.supervisorJobPids)&&ready.supervisorJobPids.length===2&&new Set(ready.supervisorJobPids).size===2&&
            ready.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1)));
        for(const row of [ready.node,ready.helper])need(object(row)&&integer(row.pid,1)&&integer(row.parentPid,1)&&
            row.sessionId===controller.sessionId&&typeof row.executable==='string'&&preStartTime(row.creationTime)&&
            Date.parse(row.creationTime)>=providerClock.startMs&&Date.parse(row.creationTime)<=Date.parse(ready.at)&&ready.supervisorJobPids.includes(row.pid));
        const runtime=providers.B1?.ready??ready;
        need(ready.node.executable===runtime.node.executable&&ready.helper.executable===runtime.helper.executable&&
            ready.query==='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'&&
            [ready.node.executable,ready.helper.executable].every(path=>context.support.pins.some((/** @type {any} */ pin)=>pin.path===path)));
        need(ready.node.parentPid===terminal.verifierPid&&ready.helper.parentPid===ready.node.pid&&ready.node.pid!==ready.helper.pid);
        const step=terminal.steps?.[0],body=spool.result?.steps?.[0];
        need(object(step)&&object(body)&&terminal.verifierStatus===0&&integer(terminal.verifierPid,1)&&terminal.signal===null&&
            terminal.closeFailed===false&&terminal.overflow===false&&terminal.spawnFailed===false&&terminal.terminalObserved===true&&
            terminal.cleanupUnknown===false&&terminal.terminalBoundToChild===true&&terminal.verdict==='PASS'&&
            terminal.terminalReceipt===resolve(output,'owned-provider-terminal.private.json')&&preStartTime(terminal.at)&&
            Date.parse(terminal.at)>=Date.parse(ready.at)&&Date.parse(terminal.at)<=providerClock.deadlineMs&&
            preStartTime(terminal.deadline)&&Date.parse(terminal.deadline)>=Date.parse(terminal.at)&&Date.parse(terminal.deadline)<=providerClock.deadlineMs&&
            Array.isArray(terminal.steps)&&terminal.steps.length===1&&spool.schemaVersion===1&&spool.command==='verify'&&spool.ok===true&&
            spool.result.verdict==='PASS'&&Array.isArray(spool.result.steps)&&spool.result.steps.length===1&&stderr.bytes.length===0);
        for(const row of [step,body])need(row.command===ready.node.executable&&
            same(row.args,[resolve(v.sourceWorkspace,'tools/starsector-proxy/guarded-http-child.mjs')])&&row.cwd==='.'&&
            row.processOwnership==='windows-job-object'&&row.status===0&&row.timedOut===false);
        need(same(r.closure.providers[batch],{calls:count,verifierPid:terminal.verifierPid,at:terminal.at,status:0,
            timedOut:false,terminalReaped:true,verdict:'PASS'}));
        const expectedTools=batch==='B1'?['doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window',
            'left_click','get_frontmost_app','list_windows','get_window','screenshot']:
            ['doctor','policy_status','get_window','screenshot',...Array.from({length:3},()=>[
                'get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot']).flat()];
        const rows=v.evidence.calls[batch];need(Array.isArray(rows)&&rows.length===count);
        let previous=Date.parse(ready.at),capture=0;const calls=[],windows=[];
        for(const [i,name] of expectedNames.entries()) {
            const stem=resolve(output,name.slice(0,-13)),pin=rows[i];need(closed(pin,['post','attempt','raw']));
            const post=pinJson(pin.post,resolve(output,name)),attempt=pinJson(pin.attempt,stem+'.attempt.private.json');
            const raw=pinJson(pin.raw,stem+'.raw.private.json',32*1024*1024),tool=expectedTools[i];
            need(post.tool===tool&&attempt.tool===tool&&post.runID===run&&attempt.runID===run&&post.batch===batch&&attempt.batch===batch&&
                attempt.sourceSha256===phase.source.sha256&&same(post.args,attempt.args)&&post.callStarted===attempt.callStarted&&
                integer(post.outerProtocolID,1)&&attempt.outerProtocolID===post.outerProtocolID&&attempt.innerProtocolID==='UNKNOWN'&&
                post.innerProtocolID===i+1&&post.freshness==='UNKNOWN'&&post.isError===false&&Array.isArray(post.precedingActions)&&
                post.precedingActions.every(object)&&preStartTime(post.callStarted)&&preStartTime(post.callCompleted)&&
                Date.parse(post.callStarted)>=previous&&Date.parse(post.callCompleted)>=Date.parse(post.callStarted)&&
                Date.parse(post.callCompleted)<=Date.parse(terminal.at)&&object(raw)&&(!Object.hasOwn(raw,'isError')||raw.isError===false));
            previous=Date.parse(post.callCompleted);let data;
            if(['doctor','policy_status','get_window','get_frontmost_app','list_windows'].includes(tool))data=preStartPacket(raw);
            if(tool==='doctor')need(same(post.args,{include_remediation:true})&&data.ok===true);
            if(tool==='policy_status')need(same(post.args,{})&&data.approval_required_for_all===true&&data.approval_token_configured===false);
            if(tool==='get_window') {
                need(same(post.args,{window_id:data.windowId})&&integer(data.windowId,1)&&data.pid===binding.ownedPids[0]&&
                    data.bundleId==='java.exe'&&data.isFocused===true&&object(data.bounds));
                windows.push(data);
            }
            if(tool==='get_frontmost_app')need(same(post.args,{})&&object(data.app)&&
                data.app.pid===binding.ownedPids[0]&&data.app.bundleId==='java.exe');
            if(tool==='list_windows')need(same(post.args,{bundle_id:'java.exe'})&&Array.isArray(data.windows)&&data.windows.every(object));
            if(tool==='screenshot') {
                const window=windows.at(-1),png=preStartOriginal(raw,window);capture++;
                need(same(post.args,{target_window_id:window.windowId,quality:0,width:window.bounds.width}));
                const prefix=resolve(output,'capture-'+String(capture).padStart(3,'0'));
                /** @type {any} */const originalPin=r.originals[originals.length];need(closed(originalPin,['raw','original','batch','metadata'])&&originalPin.batch===batch);
                /** @type {any} */const metadata=pinJson(originalPin.metadata,prefix+'.metadata.private.json');
                const saved=pinned(originalPin.original,prefix+'.png',32*1024*1024);
                const captureRaw=pinned(originalPin.raw,prefix+'.raw.private.json',32*1024*1024);
                need(saved.bytes.equals(png)&&captureRaw.bytes.equals(reader.bytes(stem+'.raw.private.json',32*1024*1024).bytes)&&
                    metadata.captureReceipt===true&&metadata.freshness==='UNKNOWN'&&metadata.rawPath===captureRaw.path&&metadata.rawSha256===captureRaw.sha256&&
                    metadata.pngPath===saved.path&&metadata.pngSha256===saved.sha256&&same(metadata.window,window)&&
                    same(metadata.dimensions,completePng(png)));
                for(const key of ['tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID'])need(same(metadata[key],post[key]));
                originals.push({batch,post,raw,metadata,png,window,ref:originalPin});
            }
            if(tool==='left_click') {
                const action=actions[calls.filter((/** @type {any} */ row)=>row.post.tool==='left_click').length],window=windows.at(-1);
                need(action&&closed(post.args,['coordinate','target_app','target_window_id','focus_strategy'])&&same(action.args,post.args)&&
                    post.args.target_app==='java.exe'&&post.args.focus_strategy==='strict'&&post.args.target_window_id===window.windowId&&
                    Array.isArray(post.args.coordinate)&&post.args.coordinate.length===2&&post.args.coordinate.every(Number.isSafeInteger)&&
                    Array.isArray(raw.content)&&raw.content.length===1&&closed(raw.content[0],['type','text'])&&raw.content[0].type==='text'&&
                    raw.content[0].text==='Clicked ('+post.args.coordinate[0]+', '+post.args.coordinate[1]+')'&&
                    Date.parse(post.callStarted)<=action.sentAtMs&&action.sentAtMs<=action.knownResultAtMs&&action.knownResultAtMs<=Date.parse(post.callCompleted)&&
                    action.sentAtMs<=clock.effectiveDeadlineMs-30000-120000);
                need(calls[i-2].post.tool==='get_frontmost_app'&&calls[i-1].post.tool==='get_window'&&calls[i-3].post.tool==='screenshot');
            }
            calls.push({post,attempt,raw,stem,window:windows.at(-1)});
        }
        need(capture===(batch==='B1'?2:4));
        providers[batch]={ready,terminal,providerClock,output,calls,source:phase.source,binding};
    }
    need(originals.length===10);return {providers,originals};
}
