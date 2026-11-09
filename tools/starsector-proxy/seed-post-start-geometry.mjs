// @ts-check
// Measured historical geometry for all eight ACKs. Never executes recorded commands.
import {resolve} from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';
import {clientImagePoint} from './client-coordinate.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
import {preStartSourceCapture} from './seed-pre-start-margin-geometry.mjs';
import {postStartNeed as need,postStartObject as object,postStartInteger as integer,postStartTime as time} from './seed-post-start-records.mjs';
const stages=[['B1-PLAY','B1',4,0],['B1-NEW_GAME','B1',11,-1],['E1-NEW_GAME','E1',4,1],
 ['E1-GENERATE','E1',10,2],['E1-CONTINUE','E1',16,3],['E2-EXPLORER','E2',4,4],['E2-NORMAL','E2',10,5],
 ['E2-SKIP','E2',16,6],['E2-START_GAME','E2',22,7]];
/** @param {any} ctx */
export function assertPostStartGeometry(ctx) {
    const {v,r,reader,controller,controllerPin,clock}=ctx,{json,pinJson,pinned,ref}=reader,s=v.scenario;
    /** @type {any[]} */const ownedSnapshots=[];
    need(Array.isArray(r.successfulGeometry)&&r.successfulGeometry.length===9);
    for(const [i,[label,batch,index,actionIndex]] of stages.entries()) {
        const name=String(label),b=String(batch),sample=r.successfulGeometry[i];
        need(closed(sample,['terminal','geometry','point','stage'])&&sample.stage===name);
        // Reject coercing HWND values before the unchanged earlier source helper consumes one.
        const snapshot=json(resolve(s,name+'-owned-snapshot.private.json'));
        need(object(snapshot)&&Array.isArray(snapshot.processes));
        const selected=snapshot.processes.filter((/** @type {any} */ p)=>object(p)&&p.executable===resolve(s,'installation/jre/bin/java.exe'));
        need(selected.length===1);
        const handle=selected[0].mainWindowHandle;
        need((typeof handle==='number'&&integer(handle,1))||
            (typeof handle==='string'&&/^[1-9][0-9]*$/.test(handle)&&integer(Number(handle),1)));
        const item=preStartSourceCapture(ctx,name,b,Number(index));
        need(Number(handle)===item.source.window.windowId);
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
        need(time(snapshot.at)&&Date.parse(snapshot.at)<=t.startedAtMs);
        const providerClock=ctx.providers[b].providerClock;
        const rootBudget=clock.effectiveDeadlineMs-30000-20000-t.startedAtMs;
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
        if(name==='B1-NEW_GAME')need(same(point.coordinate,json(canonical.path).inspection.newGameCoordinate)&&
            Date.parse(ctx.providers.B1.terminal.at)<=t.startedAtMs&&point.computedAtMs<=ctx.providers.E1.providerClock.startMs);
        else {
            const action=ctx.actions[Number(actionIndex)];
            const clicks=ctx.providers[b].calls.filter((/** @type {any} */ c)=>c.post.tool==='left_click'&&
                Date.parse(c.post.callStarted)<=action.knownResultAtMs&&action.knownResultAtMs<=Date.parse(c.post.callCompleted));
            need(clicks.length===1&&clicks[0].post.args.target_window_id===w.windowId&&
                clicks[0].window.windowId===w.windowId&&same(clicks[0].window.bounds,w.bounds)&&same(clicks[0].window.bounds,g.outerBounds));
            need(same(point.coordinate,action.args.coordinate)&&point.computedAtMs<=action.sentAtMs&&
                action.sentAtMs-Date.parse(g.observedAtUtc)<=5000);
            if(name==='E1-NEW_GAME')need(same(point.coordinate,json(canonical.path).inspection.newGameCoordinate));
        }
    }
    return ownedSnapshots;
}
