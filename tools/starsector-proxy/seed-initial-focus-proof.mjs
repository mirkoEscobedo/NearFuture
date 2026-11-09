// @ts-check
// Machine custody of one initial activation. This supplies no visible Play decision.
import {resolve,isAbsolute} from 'node:path';
import {readdirSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,requireThat} from './campaign-proof.mjs';
import {preStartReader,preStartPacket,preStartOriginal} from './seed-pre-start-margin-records.mjs';
import {isInitialFocusCreation} from './seed-creation-profile.mjs';

/** Reconstruct the immutable transport sequence, also usable by historical consumers.
 * The focused window was observed BEFORE the final capture, not after it.
 * @param {any} binding @param {any} state @param {string} output @param {string} label @param {number} now @param {boolean} [historical] */
export function inspectInitialFocusResolution(binding,state,output,label,now,historical=false){
    const need=(/** @type {any} */ ok)=>requireThat(ok,'initial activation capture custody changed');
    const action=state.pending,reader=preStartReader(),{json,ref,bytes}=reader;
    need(isInitialFocusCreation(binding)&&binding.batch==='B1'&&state.uncertain===false&&
        !state.resolvedInitialActivation&&!(state.resolvedActions??[]).length&&
        closed(action,['batch','runID','sourceSha256','reservationID','tool','args','status','sentAtMs','knownResultAtMs'])&&action.tool==='activate_window'&&
        action.status==='AWAITING_AFTER'&&action.batch==='B1'&&action.runID===binding.runID&&action.sourceSha256===binding.sourceSha256&&
        action.reservationID===binding.controllerSha256+':'+(binding.campaign.historyBaseline.reservedActions+1)&&
        closed(action.args,['window_id','timeout_ms'])&&action.args.window_id===binding.campaign.windowId&&action.args.timeout_ms===2000&&
        Number.isSafeInteger(action.sentAtMs)&&Number.isSafeInteger(action.knownResultAtMs)&&action.sentAtMs<=action.knownResultAtMs&&
        output===resolve(binding.workspaceRoot,'.tmp/starsector-proxy',binding.runID)&&/^call-[0-9]{3}$/.test(label));
    const claim=ref(resolve(binding.scenario,'guard-B1-consumed.private.json'));
    const owner=json(claim.path);need(owner.sourceSha256===binding.sourceSha256&&same(owner.binding,binding));
    const paths=readdirSync(output).filter(n=>/^call-[0-9]{3}\.private\.json$/.test(n)).sort();
    need(paths.length<=32&&(historical||paths.at(-1)===label+'.private.json'));
    const rows=paths.map(n=>({label:n.replace('.private.json',''),post:json(resolve(output,n))}));
    const activations=rows.filter(r=>r.post.tool==='activate_window');
    need(activations.length===1);
    const index=rows.indexOf(activations[0]);need(index>=3&&rows[index+3]?.label===label&&
        !rows.slice(0,index).some(r=>['left_click','key','mouse_move'].includes(r.post.tool)));
    const selected=rows.slice(index-3,index+4),expected=['screenshot','get_frontmost_app','get_window','activate_window','get_frontmost_app','get_window','screenshot'];
    /** @type {any[]} */const calls=[];
    for(let i=0;i<selected.length;i++){
        const row=selected[i],post=row.post,base=resolve(output,row.label),attempt=json(base+'.attempt.private.json'),raw=json(base+'.raw.private.json',32*1024*1024);
        need(closed(post,['tool','args','outerProtocolID','innerProtocolID','callStarted','callCompleted','precedingActions','runID','batch','freshness','isError'])&&
            closed(attempt,['tool','args','outerProtocolID','innerProtocolID','callStarted','sourceSha256','runID','batch'])&&
            Array.isArray(post.precedingActions)&&post.tool===expected[i]&&post.isError===false&&post.batch==='B1'&&post.runID===binding.runID&&post.freshness==='UNKNOWN'&&
            attempt.tool===post.tool&&same(attempt.args,post.args)&&attempt.sourceSha256===binding.sourceSha256&&attempt.runID===binding.runID&&
            attempt.batch==='B1'&&attempt.callStarted===post.callStarted&&attempt.outerProtocolID===post.outerProtocolID&&attempt.innerProtocolID==='UNKNOWN'&&
            (typeof post.outerProtocolID==='string'||Number.isSafeInteger(post.outerProtocolID))&&
            (typeof post.innerProtocolID==='string'||Number.isSafeInteger(post.innerProtocolID))&&post.innerProtocolID!=='UNKNOWN'&&
            Number.isSafeInteger(Date.parse(post.callStarted))&&Number.isSafeInteger(Date.parse(post.callCompleted))&&
            Date.parse(post.callStarted)<=Date.parse(post.callCompleted)&&Date.parse(post.callCompleted)<=now&&
            Date.parse(post.callStarted)>=state.providerClock.startMs&&Date.parse(post.callCompleted)<=state.providerClock.deadlineMs&&
            (i===0||Date.parse(calls[i-1].post.callCompleted)<=Date.parse(post.callStarted))&&
            (!Object.hasOwn(raw,'isError')||raw.isError===false));
        calls.push({post,raw,pins:{post:ref(base+'.private.json'),attempt:ref(base+'.attempt.private.json'),raw:ref(base+'.raw.private.json')}});
    }
    const [before,preFg,preWindow,ack,postFg,postWindow,after]=calls;
    const oldWindow=preStartPacket(preWindow.raw),window=preStartPacket(postWindow.raw),oldApp=preStartPacket(preFg.raw).app,app=preStartPacket(postFg.raw).app;
    for(const w of [oldWindow,window])need(w.windowId===binding.campaign.windowId&&binding.ownedPids.includes(w.pid)&&w.bundleId==='java.exe'&&
        typeof w.title==='string'&&/starsector/i.test(w.title)&&typeof w.isFocused==='boolean'&&closed(w.bounds,['x','y','width','height'])&&
        Object.values(w.bounds).every(Number.isSafeInteger)&&w.bounds.width>0&&w.bounds.height>0);
    need(oldWindow.isFocused===false&&window.isFocused===true&&oldWindow.pid===window.pid&&same(oldWindow.bounds,window.bounds)&&
        closed(preWindow.post.args,['window_id'])&&preWindow.post.args.window_id===window.windowId&&
        closed(postWindow.post.args,['window_id'])&&postWindow.post.args.window_id===window.windowId&&
        closed(preFg.post.args,[])&&closed(postFg.post.args,[])&&Number.isSafeInteger(oldApp?.pid)&&oldApp.pid>0&&app?.pid===window.pid&&
        typeof oldApp.bundleId==='string'&&typeof oldApp.displayName==='string'&&typeof app.bundleId==='string'&&typeof app.displayName==='string'&&
        same(ack.post.args,action.args)&&preStartPacket(ack.raw).activated===true&&preStartPacket(ack.raw).windowId===window.windowId&&
        Date.parse(ack.post.callStarted)<=action.sentAtMs&&Date.parse(ack.post.callCompleted)>=action.knownResultAtMs);
    /** @param {any} call @param {any} w */
    function frame(call,w){
        const matches=readdirSync(output).filter(n=>/^capture-[0-9]{3}\.metadata\.private\.json$/.test(n)).map(n=>resolve(output,n))
            .filter(p=>{const m=json(p);return m.callStarted===call.post.callStarted&&m.callCompleted===call.post.callCompleted;});
        need(matches.length===1);const path=matches[0],m=json(path),stem=path.replace('.metadata.private.json','');
        const png=bytes(stem+'.png',32*1024*1024),raw=bytes(stem+'.raw.private.json',32*1024*1024);
        need(closed(m,['tool','args','outerProtocolID','innerProtocolID','callStarted','callCompleted','precedingActions','runID','batch','freshness',
            'window','dimensions','rawPath','pngPath','pngSha256','rawSha256','captureReceipt'])&&
            m.captureReceipt===true&&m.freshness==='UNKNOWN'&&m.runID===binding.runID&&m.batch==='B1'&&m.tool==='screenshot'&&
            same(m.precedingActions,call.post.precedingActions)&&closed(m.dimensions,['width','height'])&&
            m.dimensions.width===w.bounds.width&&m.dimensions.height===w.bounds.height&&
            same(m.window,w)&&same(m.args,call.post.args)&&m.outerProtocolID===call.post.outerProtocolID&&m.innerProtocolID===call.post.innerProtocolID&&
            m.rawPath===raw.path&&m.rawSha256===raw.sha256&&m.pngPath===png.path&&m.pngSha256===png.sha256&&
            same(json(raw.path,32*1024*1024),call.raw)&&png.bytes.equals(preStartOriginal(call.raw,w))&&
            call.post.args.target_window_id===w.windowId&&call.post.args.quality===0&&call.post.args.width===w.bounds.width);
        return{...call.pins,metadata:ref(path),png:ref(png.path),captureRaw:ref(raw.path)};
    }
    return{...action,custody:{kind:'INITIAL_ACTIVATION_PRE_CAPTURE_FOCUS_CUSTODY',claim,before:frame(before,oldWindow),
        preForeground:preFg.pins,preWindow:preWindow.pins,ack:ack.pins,postForeground:postFg.pins,postWindow:postWindow.pins,
        after:frame(after,window),lastCall:label,observedAtMs:Date.parse(postWindow.post.callCompleted),
        capturedAtMs:Date.parse(after.post.callCompleted),focusObservationPosition:'BEFORE_AFTER_CAPTURE',freshness:'UNKNOWN'}};
}

/** Exact optional activation delta before the first Play; no lazy pending resolution.
 * Historical consumers re-run the same archived proof with its original provider clock.
 * @param {any} binding @param {any} state @param {boolean} [completed] */
export function assertInitialFocusArchive(binding,state,completed=false){
    const need=(/** @type {any} */ ok)=>requireThat(ok,'initial activation archive or spent delta changed');
    need(isInitialFocusCreation(binding));
    const sourceBinding=preStartReader().json(resolve(binding.scenario,'guard-B1-consumed.private.json')).binding;
    const output=resolve(binding.workspaceRoot,'.tmp/starsector-proxy',sourceBinding.runID);
    const activation=state.resolvedInitialActivation,reader=preStartReader();
    const calls=readdirSync(output).filter(n=>/^call-[0-9]{3}\.private\.json$/.test(n)).map(n=>reader.json(resolve(output,n)));
    const count=calls.filter(r=>r.tool==='activate_window').length;need(count===(activation?1:0));
    if(activation){
        const {custody,...pending}=activation;
        need(custody&&typeof custody.lastCall==='string'&&Number.isSafeInteger(custody.capturedAtMs));
        const historicalState={...state,pending,resolvedInitialActivation:undefined,resolvedActions:[],providerClock:completed?state.resolvedB1.providerClock:state.providerClock};
        const proof=inspectInitialFocusResolution(sourceBinding,historicalState,output,custody.lastCall,Date.now(),true);
        need(same(proof,activation));
    }
    if(!completed){
        need(binding.batch==='B1'&&!state.pending&&!(state.resolvedActions??[]).length&&
            state.history.reservedActions===binding.campaign.historyBaseline.reservedActions+count&&
            state.history.sentActions===binding.campaign.historyBaseline.sentActions+count);
    }
    return count;
}

/** Typed documentary fields of the NEW completed kind only. Old provenance keeps
 * its existing predicates. Strict JSON reads also reject duplicate source fields.
 * @param {any} context */
export function assertInitialFocusCloseoutTypes({scenario,closeout,plan,lease,controller,ledger,binding,limit}){
    const need=(/** @type {any} */ ok)=>requireThat(ok,'typed initial-focus closeout custody required');
    const integer=(/** @type {any} */ n,minimum=0)=>Number.isSafeInteger(n)&&n>=minimum;
    const reader=preStartReader(),c=reader.json(resolve(scenario,'seed-creation-closeout.private.json'));
    need(same(c,closeout)&&closed(c,['schemaVersion','kind','scenarioId','toolingHead','rootSessionId','allOwnedHandlesTerminalReaped',
        'actions','cleanup','protection','files','evidence'])&&c.schemaVersion===1&&integer(c.rootSessionId,1)&&
        c.rootSessionId===binding.campaign.rootCallRecord.value.raw.session_id&&c.allOwnedHandlesTerminalReaped===true&&
        typeof c.scenarioId==='string'&&c.scenarioId===plan.scenarioId&&typeof c.toolingHead==='string'&&
        /^[a-f0-9]{40}$/.test(c.toolingHead)&&c.toolingHead===plan.source.qualifiedHead);
    const a=c.actions,baseline=plan.historyBaseline,keys=['overallAttempts','uiAttempts','reservedActions','sentActions'];
    need(closed(a,['inheritedBaseline','actualGuard','currentRunContext','absoluteMaxReservedActions','clicks','activations','providers'])&&
        closed(a.inheritedBaseline,keys)&&closed(a.actualGuard,keys)&&closed(a.currentRunContext,['overallAttempts','uiAttempts'])&&
        Object.values(a.inheritedBaseline).every(n=>integer(n))&&Object.values(a.actualGuard).every(n=>integer(n))&&
        Object.values(a.currentRunContext).every(n=>integer(n))&&same(a.inheritedBaseline,baseline)&&same(a.actualGuard,ledger.history)&&
        a.currentRunContext.overallAttempts===baseline.overallAttempts+1&&a.currentRunContext.uiAttempts===baseline.uiAttempts+1&&
        integer(a.absoluteMaxReservedActions)&&a.absoluteMaxReservedActions===limit&&a.clicks===8&&a.providers===3&&
        (a.activations===0||a.activations===1));
    const cleanup=c.cleanup;
    need(cleanup&&typeof cleanup==='object'&&!Array.isArray(cleanup)&&(cleanup.status===0||cleanup.status===1)&&
        cleanup.verdict===(cleanup.status===0?'PASS':'FAIL')&&cleanup.timedOut===false&&
        cleanup.processOwnership==='windows-job-object'&&cleanup.zeroLiveGameProcesses===true);
    need(closed(c.evidence,['fixtureReceipt','rootCall','protection','launch','plan','lease','controller','ledger','preference','historyAdmission']));
    /** @type {Record<string,any>} */const records={};
    for(const[name,pin]of Object.entries(/** @type {Record<string,any>} */(c.evidence))){
        need(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&isAbsolute(pin.path)&&resolve(pin.path)===pin.path&&
            typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/i.test(pin.sha256));
        need(reader.bytes(pin.path).sha256===pin.sha256.toLowerCase());records[name]=reader.json(pin.path);
    }
    for(const[name,expected]of [['plan',plan],['lease',lease],['controller',controller],['ledger',ledger]])need(same(records[name],expected));
    need(same(records.rootCall,binding.campaign.rootCallRecord));
    const p=records.protection;
    need(p.phase==='after-smoke'&&p.originalAutosave?.enabled===true&&p.originalAutosave.maximumSlots===3);
    for(const[name,value]of Object.entries({protectedOriginalFiles:182,protectedSaveFiles:178,liveGameProcesses:0,privateSaveEntries:2,
        allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,originalWrites:false,privateAutosaveEnabled:false}))
        need(p[name]===value&&c.protection?.[name]===value);
    need(same(p.sharedPreference,c.protection.sharedPreference)&&same(p.sharedPreference,records.preference));
    need(Array.isArray(c.files)&&c.files.length===2);const names=new Set();let total=0;
    for(const row of c.files){
        need(closed(row,['relative','bytes','sha256'])&&['campaign.xml','descriptor.xml'].includes(row.relative)&&!names.has(row.relative)&&
            integer(row.bytes,1)&&row.bytes<=16*1024*1024&&typeof row.sha256==='string'&&/^[a-f0-9]{64}$/i.test(row.sha256));
        names.add(row.relative);total+=row.bytes;
    }
    need(total<=16*1024*1024);
}
