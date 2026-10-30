// @ts-check
// Historical measured geometry only. Never executes the recorded helper or renews a clock.
import {resolve} from 'node:path';
import {existsSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {clientImagePoint} from './client-coordinate.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {preStartNeed as need,preStartObject as object,preStartInteger as integer,preStartTime as time} from './seed-pre-start-margin-records.mjs';

const stages=[
    ['B1-PLAY','B1',4,0],['B1-NEW_GAME','B1',11,-1],['E1-NEW_GAME','E1',4,1],
    ['E1-GENERATE','E1',10,2],['E1-CONTINUE','E1',16,3],
    ['E2-EXPLORER','E2',4,4],['E2-NORMAL','E2',10,5],['E2-SKIP','E2',16,6],
];

/** @param {any} ctx @param {string} label @param {string} batch @param {number} index */
export function preStartSourceCapture(ctx,label,batch,index) {
    const {v,reader,phases,providers,controllerPin,clock}=ctx,{json,pinJson,pinned,ref}=reader,s=v.scenario;
    const path=resolve(s,label+'-source-before.private.json'),source=json(path),phase=phases[batch],provider=providers[batch];
    const call=provider.calls[index-1],capture=provider.calls.slice(0,index).filter((/** @type {any} */ row)=>row.post.tool==='screenshot').length;
    const prefix=resolve(provider.output,'capture-'+String(capture).padStart(3,'0'));
    need(closed(source,['schemaVersion','kind','scenario','sourceHead','label','runID','callLabel','captureLabel','controller','source','claim',
        'owned','post','attempt','raw','captureRaw','automatic','frame','closedFrame','window','ownedSnapshot','rootClock','inspectedAtMs','inputGranted'])&&
        source.schemaVersion===1&&source.kind==='PRIVATE_FIXED_LATEST_SOURCE_CAPTURE'&&source.scenario===s&&source.sourceHead===v.sourceHead&&
        source.label===label&&source.runID===phase.claim.binding.runID&&source.callLabel==='call-'+String(index).padStart(3,'0')&&
        source.captureLabel==='capture-'+String(capture).padStart(3,'0')&&same(source.controller,controllerPin)&&same(source.source,phase.source)&&
        same(source.claim,phase.claimPin)&&same(source.owned,{path:phase.auth.ownedReceiptPath,sha256:phase.auth.ownedReceiptSha256})&&
        same(source.post,v.evidence.calls[batch][index-1].post)&&same(source.attempt,v.evidence.calls[batch][index-1].attempt)&&
        same(source.raw,v.evidence.calls[batch][index-1].raw)&&same(source.captureRaw,ref(prefix+'.raw.private.json'))&&
        same(source.automatic,ref(prefix+'.metadata.private.json'))&&same(source.rootClock,clock)&&source.inputGranted===false&&integer(source.inspectedAtMs,1));
    need(same(source.window,call.window)&&same(source.frame,ref(resolve(s,label+'-before-frame.private.json'))));
    const frame=pinJson(source.frame,resolve(s,label+'-before-frame.private.json'));
    need(closed(frame,['pngPath','pngSha256','metadataPath','metadataSha256','capturedAtMs'])&&same(frame,source.closedFrame)&&
        frame.pngPath===prefix+'.png'&&frame.pngSha256===ref(prefix+'.png').sha256&&frame.metadataPath===prefix+'.metadata.private.json'&&
        frame.metadataSha256===ref(frame.metadataPath).sha256&&frame.capturedAtMs===Date.parse(call.post.callCompleted)&&source.inspectedAtMs>=frame.capturedAtMs);
    need(same(source.ownedSnapshot,ref(resolve(s,label+'-owned-snapshot.private.json'))));
    const owned=pinJson(source.ownedSnapshot,resolve(s,label+'-owned-snapshot.private.json'));
    need(object(owned)&&Array.isArray(owned.processes)&&owned.controllerPid===ctx.controller.controllerPid&&
        owned.controllerCreationTime===ctx.controller.controllerCreationTime&&
        owned.controllerKernelCreationFileTime===ctx.controller.controllerKernelCreationFileTime&&time(owned.at));
    const games=owned.processes.filter((/** @type {any} */ row)=>row?.executable===resolve(s,'installation/jre/bin/java.exe'));
    need(games.length===1);const game=games[0],first=phases.B1.owned.processes.find((/** @type {any} */ row)=>row.pid===game.pid);
    need(first&&game.inSupervisorJob===true&&game.pid===call.window.pid&&game.sessionId===ctx.controller.sessionId&&
        ['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(game[key],first[key]))&&
        Array.isArray(game.supervisorJobPids)&&new Set(game.supervisorJobPids).size===game.supervisorJobPids.length&&
        game.supervisorJobPids.every((/** @type {any} */ id)=>integer(id,1))&&
        [game.pid,game.parentPid,ctx.controller.controllerPid,...ctx.controller.supervisorJobPids].every(id=>game.supervisorJobPids.includes(id))&&
        Number(game.mainWindowHandle)===call.window.windowId&&Date.parse(game.creationTime)<=Date.parse(owned.at));
    return {path,source,frame,game,provider,call};
}

/** @param {any} ctx */
export function assertPreStartGeometry(ctx) {
    const {v,r,reader,controller,controllerPin,clock}=ctx,{json,pinJson,pinned,ref}=reader,s=v.scenario;
    /** @type {any[]} */const ownedSnapshots=[];
    need(Array.isArray(r.successfulGeometry)&&r.successfulGeometry.length===8);
    for(const [i,[label,batch,index,actionIndex]] of stages.entries()) {
        const name=String(label),b=String(batch),sample=r.successfulGeometry[i];
        need(closed(sample,['terminal','geometry','point','stage'])&&sample.stage===name);
        const item=preStartSourceCapture(ctx,name,b,Number(index));
        ownedSnapshots.push(item.game);
        const t=pinJson(sample.terminal,resolve(s,name+'-client-geometry-terminal.actual.private.json'));
        const g=pinJson(sample.geometry,resolve(s,name+'-client-geometry.actual.private.json'));
        const point=pinJson(sample.point,resolve(s,name+'-client-point.private.json'));
        need(closed(t,['schemaVersion','kind','sourceHead','scenario','label','sourceCapture','controller','sourceFrame','program','args','cwd',
            'childCommand','configuration','startedAtMs','completedAtMs','childTimeoutMs','cleanupAllowanceMs','outerTimeoutMs','status','signal',
            'error','stdout','stderr','terminalReaped','outputComplete','supervisorJSON','rootClock','providerClock','closedProvider','geometry','inputGranted'])&&
            t.schemaVersion===1&&t.kind==='ACTUAL_OWNED_GEOMETRY_COMMAND'&&t.sourceHead===v.sourceHead&&t.scenario===s&&t.label===name&&
            same(t.sourceCapture,ref(item.path))&&same(t.controller,controllerPin)&&same(t.sourceFrame,item.source.frame)&&same(t.rootClock,clock)&&
            t.program===resolve(v.sourceWorkspace,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe')&&t.status===0&&
            t.signal===null&&t.error===null&&t.terminalReaped===true&&t.outputComplete===true&&t.stderr===''&&t.inputGranted===false&&same(t.geometry,sample.geometry));
        const providerClock=ctx.providers[b].providerClock;
        const rootBudget=clock.effectiveDeadlineMs-30000-120000-t.startedAtMs;
        const budget=Math.floor(name==='B1-NEW_GAME'?rootBudget:Math.min(rootBudget,providerClock.deadlineMs-t.startedAtMs-20000-10000));
        const cleanup=Math.min(10000,Math.floor(budget/2)),child=Math.min(5000,budget-cleanup);
        need(integer(t.startedAtMs,1)&&integer(t.completedAtMs,t.startedAtMs)&&budget>=2&&t.cleanupAllowanceMs===cleanup&&
            t.childTimeoutMs===child&&t.outerTimeoutMs===child+cleanup&&t.completedAtMs-t.startedAtMs<=t.outerTimeoutMs&&
            same(t.providerClock,name==='B1-NEW_GAME'?null:providerClock)&&
            (name==='B1-NEW_GAME'?same(t.closedProvider,r.evidence['B1/owned-provider-terminal.private.json']):t.closedProvider===null));
        const project=resolve(s,'geometry-owned-'+name),w=item.source.window;
        need(closed(t.childCommand,['program','args','cwd'])&&typeof t.childCommand.program==='string'&&
            resolve(t.childCommand.program)===resolve(ctx.powershell)&&t.childCommand.cwd==='.'&&
            same(t.childCommand.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1'),
                '-EvidenceRoot',s,'-WindowId',String(w.windowId),'-ExpectedX',String(w.bounds.x),'-ExpectedY',String(w.bounds.y),
                '-ExpectedWidth',String(w.bounds.width),'-ExpectedHeight',String(w.bounds.height)])&&
            t.cwd===project&&same(t.args,['verify',project,'--scope','module','--module','node:.','--concurrency','1','--timeout',String(child)]));
        const configuration=pinJson(t.configuration,resolve(project,'.agentic/project.json'));
        need(same(configuration,{version:2,overrides:{workspace:{commands:{'node:.':[t.childCommand]}}}}));
        need(typeof t.stdout==='string'&&Buffer.byteLength(t.stdout)<=262144&&same(zeroProviderObject(t.stdout,need),t.supervisorJSON));
        const report=t.supervisorJSON,step=report.result?.steps?.[0];
        need(report.schemaVersion===1&&report.command==='verify'&&report.ok===true&&report.result.verdict==='PASS'&&
            Array.isArray(report.result.blocked)&&report.result.blocked.length===0&&Array.isArray(report.result.steps)&&report.result.steps.length===1&&
            object(step)&&step.command===t.childCommand.program&&same(step.args,t.childCommand.args)&&step.cwd==='.'&&step.module==='node:.'&&
            step.processOwnership==='windows-job-object'&&step.status===0&&step.timedOut===false&&step.stderr===''&&
            typeof step.stdout==='string'&&Buffer.from(step.stdout).equals(reader.bytes(sample.geometry.path).bytes)&&
            same(zeroProviderObject(step.stdout,need),g));
        need(time(g.observedAtUtc)&&Date.parse(g.observedAtUtc)>=t.startedAtMs&&Date.parse(g.observedAtUtc)<=t.completedAtMs);
        need(closed(point,['schemaVersion','kind','label','frame','geometry','terminal','sourceCapture','ownedSnapshot','localPoint','coordinate',
            'inspectedAtMs','computedAtMs','controllerSha256','inputGranted'])&&point.schemaVersion===1&&point.kind==='PRIVATE_FIXED_SEED_CLIENT_POINT'&&
            point.label===name&&same(point.geometry,sample.geometry)&&same(point.terminal,sample.terminal)&&same(point.sourceCapture,ref(item.path))&&
            same(point.ownedSnapshot,item.source.ownedSnapshot)&&point.inspectedAtMs===item.source.inspectedAtMs&&
            point.inspectedAtMs<=t.startedAtMs&&integer(point.computedAtMs,t.completedAtMs)&&
            point.controllerSha256===controllerPin.sha256&&point.inputGranted===false);
        const frame=point.frame,metadata=json(frame.metadataPath);
        need(closed(frame,['pngPath','pngSha256','metadataPath','metadataSha256','capturedAtMs'])&&
            ref(frame.pngPath).sha256===frame.pngSha256&&ref(frame.metadataPath).sha256===frame.metadataSha256&&
            reader.bytes(frame.pngPath,32*1024*1024).bytes.equals(reader.bytes(item.frame.pngPath,32*1024*1024).bytes)&&
            frame.capturedAtMs===item.frame.capturedAtMs&&same(metadata.window,w));
        const pointValue=clientImagePoint({geometry:g,window:w,owned:{...item.game,controllerPid:controller.controllerPid},
            dimensions:{width:w.bounds.width,height:w.bounds.height},point:point.localPoint,nowMs:point.computedAtMs});
        need(same(point.coordinate,pointValue));
        const canonical=name==='B1-PLAY'?ref(item.path):ref(resolve(s,name==='B1-NEW_GAME'||name==='E1-NEW_GAME'?'B1-menu-inspected.private.json':
            name==='E1-GENERATE'?'E1-wizard-2-inspected.private.json':name==='E1-CONTINUE'?'E1-wizard-3-inspected.private.json':
                'E2-wizard-'+(Number(actionIndex)-3)+'-inspected.private.json'));
        need(same(json(resolve(s,name+'-coordinate-proof.private.json')),{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_POINT_CANONICAL_BINDING',
            point:sample.point,canonical,frame,inputGranted:false}));
        if(name!=='B1-PLAY'&&name!=='E1-NEW_GAME') {
            const copied=json(canonical.path).frame;
            need(object(copied)&&copied.capturedAtMs===item.frame.capturedAtMs&&copied.pngSha256===item.frame.pngSha256&&
                ref(copied.pngPath).sha256===copied.pngSha256&&ref(copied.metadataPath).sha256===copied.metadataSha256&&
                reader.bytes(copied.pngPath,32*1024*1024).bytes.equals(reader.bytes(item.frame.pngPath,32*1024*1024).bytes)&&
                same(json(copied.metadataPath).window,w));
        }
        if(name==='B1-NEW_GAME')need(same(point.coordinate,json(canonical.path).inspection.newGameCoordinate));
        else {
            const action=ctx.actions[Number(actionIndex)];
            need(same(point.coordinate,action.args.coordinate)&&point.computedAtMs<=action.sentAtMs&&
                action.sentAtMs-Date.parse(g.observedAtUtc)<=5000);
            if(name==='E1-NEW_GAME')need(same(point.coordinate,json(canonical.path).inspection.newGameCoordinate));
        }
    }
    const final=preStartSourceCapture(ctx,'E2-START_GAME','E2',22);
    ownedSnapshots.push(final.game);
    need(same(v.evidence.finalSourceCapture,ref(final.path)));
    const failure=r.failure,metadata=ctx.metadata,entry=metadata.entries.at(-1),raw=entry?.raw;
    const expectedLabels=['B1-PLAY','B1-NEW_GAME-ONCE-WRAPPER','E1-NEW_GAME','E1-GENERATE','E1-CONTINUE',
        'E1-OCCUPATION-FRAME','E1-OCCUPATION-E2-PHASE','E2-EXPLORER','E2-NORMAL','E2-SKIP','E2-START_GAME'];
    need(closed(metadata,['schemaVersion','kind','driver','scenario','entries'])&&metadata.schemaVersion===1&&
        metadata.kind==='ACTUAL_DRIVER_METADATA_CALLS'&&metadata.driver===v.driver&&resolve(metadata.scenario)===s&&
        Array.isArray(metadata.entries)&&same(metadata.entries.map((/** @type {any} */ row)=>row.label),expectedLabels));
    for(const [i,row] of metadata.entries.entries())need(closed(row,['label','args','raw','polls'])&&object(row.args)&&object(row.raw)&&
        row.raw.exit_code===(i===10?1:0)&&!Object.hasOwn(row.raw,'session_id')&&same(row.polls,[])&&
        typeof row.raw.output==='string'&&Number.isFinite(row.raw.wall_time_seconds)&&row.raw.wall_time_seconds>=0);
    const helper=resolve(s,'construct-actual-phase-inputs.private.mjs'),q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    const inspected=final.source.inspectedAtMs,run=ctx.phases.E2.claim.binding.runID,[x,y]=r.visibleAssertion.localPoint;
    const command="$ErrorActionPreference='Stop';$h="+q(helper.replaceAll('\\','/'))+";$n="+q(ctx.node.replaceAll('\\','/'))+
        ";&$n $h frame E2 "+run+" call-022 capture-004 E2-wizard-4|Out-Null;if($LASTEXITCODE-ne0){throw 'frame failed'};"+
        "$gr=&$n $h geometry E2-START_GAME "+run+" call-022 capture-004 "+inspected+";if($LASTEXITCODE-ne0){throw 'geometry failed'};"+
        "$g=$gr|ConvertFrom-Json -DateKind String;$mr=&$n $h control E2 4 "+q(r.generatedName)+" "+x+" "+y+" "+inspected+
        " true $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'control failed'};"+
        "$m=$mr|ConvertFrom-Json -DateKind String;$cr=&$n $h point-check E2-START_GAME $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;"+
        "if($LASTEXITCODE-ne0){throw 'pointcheck failed'};$c=$cr|ConvertFrom-Json -DateKind String;"+
        "$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json -DateKind String;"+
        "[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress";
    need(same(entry.args,{cmd:command,shell:ctx.powershell.replaceAll('\\','/'),sandbox_permissions:'require_escalated',max_output_tokens:1500})&&
        typeof raw.chunk_id==='string'&&raw.chunk_id===failure.packetChunk&&raw.output.includes('Error: No original budget for geometry plus bounded cleanup')&&
        raw.output.includes('runOwnedClientGeometry')&&raw.output.includes('geometry failed')&&
        !raw.output.includes('ETIMEDOUT')&&failure.exitCode===1&&failure.message==='No original budget for geometry plus bounded cleanup'&&
        failure.inspectionAtMs===inspected&&inspected>=clock.effectiveDeadlineMs-150000&&inspected<clock.effectiveDeadlineMs&&
        failure.originalCapMs===clock.effectiveDeadlineMs&&failure.approxRemainingMsAtInspection===clock.effectiveDeadlineMs-inspected&&
        same(failure.requiredUnchangedMargins,{input:120000,cleanup:30000})&&failure.clockRenewal===false&&failure.retry===false);
    for(const name of ['geometry-owned-E2-START_GAME','E2-START_GAME-client-geometry.actual.private.json','E2-START_GAME-client-geometry-terminal.actual.private.json',
        'E2-wizard-4-inspected.private.json','E2-START_GAME-client-point.private.json','E2-START_GAME-coordinate-proof.private.json',
        'E2-created-seed-inspected.private.json','seed-save-stability.actual.private.json','controller-terminal.private.json'])need(!existsSync(resolve(s,name)));
    need(same(r.absentFinalArtifacts,{startGeometry:true,startGeometryTerminal:true,startControl:true,created:true,stability:true}));
    return ownedSnapshots;
}
