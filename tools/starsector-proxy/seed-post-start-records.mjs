// @ts-check
// Immutable post-Start records only. No OS query, recorded code execution or live grant.
import {resolve,isAbsolute,basename} from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,requireThat} from './campaign-proof.mjs';
import {preStartReader,preStartPacket,preStartOriginal} from './seed-pre-start-margin-records.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {readdirSync} from 'node:fs';

export const postStartNeed=(/** @type {any} */ value)=>requireThat(value,'post-Start verification-incomplete custody required');
export const postStartObject=(/** @type {any} */ v)=>v!==null&&typeof v==='object'&&!Array.isArray(v);
export const postStartInteger=(/** @type {any} */ v,minimum=0)=>Number.isSafeInteger(v)&&v>=minimum;
export const postStartTime=(/** @type {any} */ v)=>typeof v==='string'&&Number.isSafeInteger(Date.parse(v));
const need=postStartNeed,object=postStartObject,integer=postStartInteger,time=postStartTime;
export const postStartPreference={schemaVersion:1,kind:'NATIVE_SEED_SHARED_PREFERENCE_V1',view:'Registry64',node:'/com/fs/starfarer',
    valueName:'canSkipTutorial',registryValueName:'can/Skip/Tutorial',valueKind:'String',value:'true'};
export const postStartStages=['PLAY','NEW_GAME','GENERATE','CONTINUE','EXPLORER','NORMAL','SKIP','START_GAME'];

/** Bounded pins may retain an independently recorded byte count. @param {any} reader @param {any} pin @param {string} path @param {number} [limit] */
export function postStartPinned(reader,pin,path,limit=262144) {
    need(object(pin)&&typeof pin.path==='string'&&pin.path===path&&isAbsolute(path)&&resolve(path)===path&&
        typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256)&&
        (closed(pin,['path','sha256'])||closed(pin,['path','sha256','bytes'])));
    const f=reader.bytes(path,limit);need(f.sha256===pin.sha256&&(!Object.hasOwn(pin,'bytes')||(integer(pin.bytes)&&pin.bytes===f.bytes.length)));
    return f;
}
/** Strict reader shared with the earlier independent custody reader; its contract is unchanged. */
export const postStartReader=preStartReader;
/** @param {any} raw */
export function postStartTerminalPacket(raw) {
    need((closed(raw,['content'])||closed(raw,['content','isError']))&&(!Object.hasOwn(raw,'isError')||typeof raw.isError==='boolean')&&
        Array.isArray(raw.content)&&raw.content.length===1&&closed(raw.content[0],['type','text'])&&
        raw.content[0].type==='text'&&typeof raw.content[0].text==='string'&&Buffer.byteLength(raw.content[0].text)<=262144);
    const value=zeroProviderObject(raw.content[0].text,need);need(object(value));return value;
}
/** @param {any} ctx */
export function readPostStartProviders(ctx) {
    const {v,r,reader,phases,controller,clock,actions,support}=ctx,{json,ref}=reader;
    /** @type {Record<string,any>} */const providers={};
    /** @type {any[]} */const originals=[];
    for(const [batch,count] of /** @type {[string,number][]} */([['B1',11],['E1',22],['E2',28]])) {
        const phase=phases[batch],binding=phase.claim.binding,run=binding.runID,output=resolve(v.sourceWorkspace,'.tmp/starsector-proxy',run);
        need(typeof run==='string'&&/^[A-Za-z0-9_-]+$/.test(run)&&object(v.evidence.providers[batch]));
        const paths=v.evidence.providers[batch];
        need(same(readdirSync(output).filter(n=>/^call-[0-9]{3}\.private\.json$/.test(n)).sort(),
            Array.from({length:count},(_,i)=>'call-'+String(i+1).padStart(3,'0')+'.private.json')));
        need(closed(paths,['guarded-http-ready.private.json','owned-provider-terminal.private.json','owned-provider.stdout.private.txt',
            'owned-provider.stderr.private.txt','batch-dispatch.private.jsonl']));
        for(const name of Object.keys(paths))postStartPinned(reader,paths[name],resolve(output,name));
        const ready=json(resolve(output,'guarded-http-ready.private.json')),terminal=json(resolve(output,'owned-provider-terminal.private.json'));
        const providerClock=batch==='B1'?ctx.ledger.resolvedB1.providerClock:batch==='E1'?ctx.ledger.resolvedE1.providerClock:ctx.ledger.providerClock;
        need(object(providerClock)&&closed(providerClock,['startMs','deadlineMs'])&&integer(providerClock.startMs,clock.startMs)&&
            providerClock.deadlineMs===Math.min(providerClock.startMs+180000,clock.effectiveDeadlineMs-30000));
        need(ready.schemaVersion===1&&ready.runID===run&&ready.batch===batch&&ready.sourceSha256===phase.source.sha256&&
            ready.startupTimeoutMs===20000&&ready.consentMode==='finite-explicit-consent'&&time(ready.at)&&
            Date.parse(ready.at)>=providerClock.startMs&&Date.parse(ready.at)<=providerClock.startMs+20000&&
            Array.isArray(ready.supervisorJobPids)&&ready.supervisorJobPids.length===2&&new Set(ready.supervisorJobPids).size===2&&
            ready.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1))&&
            ready.query==='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)');
        for(const row of [ready.node,ready.helper])need(object(row)&&integer(row.pid,1)&&integer(row.parentPid,1)&&
            row.sessionId===controller.sessionId&&typeof row.executable==='string'&&time(row.creationTime)&&
            Date.parse(row.creationTime)>=providerClock.startMs&&Date.parse(row.creationTime)<=Date.parse(ready.at)&&
            ready.supervisorJobPids.includes(row.pid)&&support.pins.some((/** @type {any} */ pin)=>pin.path===row.executable));
        need(closed(terminal,['at','verifierPid','verifierStatus','signal','deadline','closeFailed','overflow','spawnFailed','terminalObserved',
            'cleanupUnknown','steps','terminalBoundToChild','verdict','terminalReceipt'])&&
            ready.node.executable===(providers.B1?.ready.node.executable??ready.node.executable)&&
            ready.helper.executable===(providers.B1?.ready.helper.executable??ready.helper.executable)&&
            ready.node.pid!==ready.helper.pid&&ready.node.parentPid===terminal.verifierPid&&ready.helper.parentPid===ready.node.pid&&
            integer(terminal.verifierPid,1)&&time(terminal.at)&&time(terminal.deadline)&&Date.parse(terminal.at)>=Date.parse(ready.at)&&
            Date.parse(terminal.deadline)<=providerClock.deadlineMs&&Date.parse(terminal.deadline)>Date.parse(ready.at)&&
            terminal.terminalObserved===true&&terminal.cleanupUnknown===false&&terminal.overflow===false&&terminal.spawnFailed===false&&
            terminal.terminalReceipt===resolve(output,'owned-provider-terminal.private.json')&&Array.isArray(terminal.steps));
        const packet=postStartTerminalPacket(r.providerClosure[batch]);need(same(packet,terminal));
        const lifecycle=ctx.a['driver-provider-calls.actual.private.json'];
        need(closed(lifecycle,['schemaVersion','kind','B1','E1','E2','sourceReadyTerminalPins','allProvidersPass'])&&
            lifecycle.schemaVersion===1&&lifecycle.kind==='ACTUAL_DRIVER_PROVIDER_LIFECYCLE_CALLS'&&lifecycle.allProvidersPass===false&&
            Array.isArray(lifecycle.sourceReadyTerminalPins)&&lifecycle.sourceReadyTerminalPins.length===3&&closed(lifecycle[batch],['begin','finish']));
        const begin=lifecycle[batch].begin;
        need(closed(begin,['args','raw'])&&(!Object.hasOwn(begin.raw,'isError')||begin.raw.isError===false)&&
            closed(begin.args,['consentMode','runID','source'])&&begin.args.consentMode==='finite-explicit-consent'&&
            begin.args.runID===run&&typeof begin.args.source==='string'&&resolve(begin.args.source)===phase.source.path&&
            same(postStartTerminalPacket(begin.raw),{runID:run,output,deadline:terminal.deadline})&&same(lifecycle[batch].finish,r.providerClosure[batch]));
        const lifecyclePin=lifecycle.sourceReadyTerminalPins[['B1','E1','E2'].indexOf(batch)];
        need(closed(lifecyclePin,['runID','ready','terminal','terminalValue'])&&lifecyclePin.runID===run&&
            same(lifecyclePin.ready,paths['guarded-http-ready.private.json'])&&same(lifecyclePin.terminal,paths['owned-provider-terminal.private.json'])&&
            same(lifecyclePin.terminalValue,terminal));
        if(batch==='E2')need(r.providerClosure[batch].isError===true&&terminal.verifierStatus===null&&terminal.signal==='SIGTERM'&&
            terminal.closeFailed===true&&terminal.terminalBoundToChild===false&&terminal.verdict==='UNKNOWN'&&terminal.steps.length===0&&
            Date.parse(terminal.at)>Date.parse(terminal.deadline));
        else {
            need((!Object.hasOwn(r.providerClosure[batch],'isError')||r.providerClosure[batch].isError===false)&&terminal.verifierStatus===0&&
                terminal.signal===null&&terminal.closeFailed===false&&terminal.terminalBoundToChild===true&&terminal.verdict==='PASS'&&
                terminal.steps.length===1&&Date.parse(terminal.at)<=Date.parse(terminal.deadline));
            const spool=json(resolve(output,'owned-provider.stdout.private.txt'));
            need(spool.schemaVersion===1&&spool.command==='verify'&&spool.ok===true&&spool.result?.verdict==='PASS'&&
                Array.isArray(spool.result.steps)&&spool.result.steps.length===1);
            for(const step of [terminal.steps[0],spool.result.steps[0]])need(object(step)&&step.status===0&&step.timedOut===false&&
                step.processOwnership==='windows-job-object'&&step.command===ready.node.executable&&step.cwd==='.'&&
                same(step.args,[resolve(v.sourceWorkspace,'tools/starsector-proxy/guarded-http-child.mjs')]));
        }
        const names=batch==='B1'?['doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click',
            'get_frontmost_app','list_windows','get_window','screenshot']:
            ['doctor','policy_status','get_window','screenshot',...Array.from({length:batch==='E2'?4:3},()=>[
                'get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot']).flat()];
        const rows=ctx.index.calls.filter((/** @type {any} */ x)=>x.batch===batch),before=originals.length;
        need(rows.length===count);let previous=Date.parse(ready.at),capture=0;
        /** @type {any[]} */const calls=[],windows=[];
        for(const [i,row] of rows.entries()) {
            const stem=resolve(output,'call-'+String(i+1).padStart(3,'0'));
            for(const [key,suffix] of [['post','.private.json'],['attempt','.attempt.private.json'],['raw','.raw.private.json']])
                postStartPinned(reader,row[key],stem+suffix,key==='raw'?32*1024*1024:262144);
            const post=json(stem+'.private.json'),attempt=json(stem+'.attempt.private.json'),raw=json(stem+'.raw.private.json',32*1024*1024),tool=names[i];
            need(row.label==='call-'+String(i+1).padStart(3,'0')&&row.runID===run&&row.tool===tool&&same(row.args,post.args)&&
                row.isError===false&&row.callStarted===post.callStarted&&row.callCompleted===post.callCompleted&&
                row.outerProtocolID===post.outerProtocolID&&row.innerProtocolID===post.innerProtocolID&&
                post.tool===tool&&post.runID===run&&post.batch===batch&&post.isError===false&&post.freshness==='UNKNOWN'&&
                attempt.tool===tool&&attempt.runID===run&&attempt.batch===batch&&attempt.sourceSha256===phase.source.sha256&&
                same(post.args,attempt.args)&&post.callStarted===attempt.callStarted&&integer(post.outerProtocolID,1)&&
                attempt.outerProtocolID===post.outerProtocolID&&attempt.innerProtocolID==='UNKNOWN'&&post.innerProtocolID===i+1&&
                time(post.callStarted)&&time(post.callCompleted)&&Date.parse(post.callStarted)>=previous&&
                Date.parse(post.callCompleted)>=Date.parse(post.callStarted)&&Date.parse(post.callCompleted)<=Date.parse(terminal.at)&&
                object(raw)&&(!Object.hasOwn(raw,'isError')||raw.isError===false)&&Array.isArray(post.precedingActions));
            previous=Date.parse(post.callCompleted);let data;
            need(same(post.precedingActions,calls.filter(row=>row.post.tool==='left_click').map(row=>row.post)));
            if(['doctor','policy_status','get_window','get_frontmost_app','list_windows'].includes(tool)){data=preStartPacket(raw);need(object(data));}
            if(tool==='doctor')need(same(post.args,{include_remediation:true})&&data.ok===true);
            if(tool==='policy_status')need(same(post.args,{})&&data.approval_required_for_all===true&&data.approval_token_configured===false);
            if(tool==='get_window'){need(object(data)&&same(post.args,{window_id:data.windowId})&&integer(data.windowId,1)&&
                data.pid===binding.ownedPids[0]&&data.bundleId==='java.exe'&&data.isFocused===true&&object(data.bounds));windows.push(data);}
            if(tool==='get_frontmost_app')need(same(post.args,{})&&object(data.app)&&data.app.pid===binding.ownedPids[0]&&data.app.bundleId==='java.exe');
            if(tool==='list_windows')need(same(post.args,{bundle_id:'java.exe'})&&Array.isArray(data.windows)&&data.windows.every(object));
            if(tool==='screenshot') {
                const window=windows.at(-1),png=preStartOriginal(raw,window);capture++;
                need(same(post.args,{target_window_id:window.windowId,quality:0,width:window.bounds.width}));
                const original=ctx.index.originals[originals.length],prefix=resolve(output,'capture-'+String(capture).padStart(3,'0'));
                need(object(original)&&original.batch===batch&&original.runID===run&&original.label==='capture-'+String(capture).padStart(3,'0'));
                const saved=postStartPinned(reader,original.png,prefix+'.png',32*1024*1024),captureRaw=postStartPinned(reader,original.raw,prefix+'.raw.private.json',32*1024*1024);
                postStartPinned(reader,original.automaticMetadata,prefix+'.metadata.private.json');const metadata=json(prefix+'.metadata.private.json');
                need(saved.bytes.equals(png)&&captureRaw.bytes.equals(reader.bytes(stem+'.raw.private.json',32*1024*1024).bytes)&&
                    metadata.captureReceipt===true&&metadata.freshness==='UNKNOWN'&&metadata.rawPath===captureRaw.path&&metadata.rawSha256===captureRaw.sha256&&
                    metadata.pngPath===saved.path&&metadata.pngSha256===saved.sha256&&same(metadata.window,window)&&same(original.window,window));
                for(const key of ['args','callStarted','callCompleted','outerProtocolID','innerProtocolID'])need(same(original[key],post[key]));
                for(const key of ['tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID'])need(same(metadata[key],post[key]));
                originals.push({batch,post,raw,metadata,png,window,ref:original});
            }
            if(tool==='left_click') {
                const action=actions.find((/** @type {any} */ a)=>a.batch===batch&&a.knownResultAtMs>=Date.parse(post.callStarted)&&a.knownResultAtMs<=Date.parse(post.callCompleted));
                need(action&&same(post.args,action.args)&&closed(post.args,['coordinate','target_app','target_window_id','focus_strategy'])&&
                    post.args.target_app==='java.exe'&&post.args.focus_strategy==='strict'&&post.args.target_window_id===windows.at(-1).windowId&&
                    Array.isArray(post.args.coordinate)&&post.args.coordinate.length===2&&post.args.coordinate.every(Number.isSafeInteger)&&
                    Array.isArray(raw.content)&&raw.content.length===1&&closed(raw.content[0],['type','text'])&&raw.content[0].type==='text'&&
                    raw.content[0].text==='Clicked ('+post.args.coordinate[0]+', '+post.args.coordinate[1]+')'&&
                    Date.parse(post.callStarted)<=action.sentAtMs&&action.sentAtMs<=action.knownResultAtMs&&
                    action.sentAtMs<=clock.effectiveDeadlineMs-50000&&action.sentAtMs<=providerClock.deadlineMs-20000&&
                    calls[i-3].post.tool==='screenshot'&&calls[i-2].post.tool==='get_frontmost_app'&&calls[i-1].post.tool==='get_window');
            }
            calls.push({post,attempt,raw,stem,window:windows.at(-1)});
        }
        need(originals.length-before===(batch==='B1'?2:batch==='E1'?4:5));
        providers[batch]={ready,terminal,providerClock,output,calls,source:phase.source,binding};
    }
    need(originals.length===11);return{providers,originals};
}
