// @ts-check
// Recorded client mapping and the original pre-child budget refusal, without fresh geometry authority.
import {resolve} from 'node:path';
import {existsSync} from 'node:fs';
import {isDeepStrictEqual as same} from 'node:util';
import {closed,file} from './campaign-proof.mjs';
import {clientImagePoint} from './client-coordinate.mjs';
import {budgetHistoryJson,budgetHistoryNeed as need,budgetHistoryInteger as integer,budgetHistoryObject as object,budgetHistoryTime as time} from './seed-pre-continue-budget-index.mjs';
const stages=[['B1-PLAY','B1',5,0],['B1-NEW_GAME','B1',12,-1],['E1-NEW_GAME','E1',5,1],['E1-GENERATE','E1',11,2],['E1-CONTINUE','E1',17,-2]];
/** @param {any} ctx @param {string} label @param {string} batch @param {number} ordinal */
function sourceCapture(ctx,label,batch,ordinal){
    const {reader,v,phases,providers,controllerPin,clock,c}=ctx,{json,ref}=reader,s=v.scenario,phase=phases[batch],provider=providers[batch];
    const call=provider.calls[ordinal-1],capture=provider.calls.slice(0,ordinal).filter((/** @type {any} */ row)=>row.post.tool==='screenshot').length,
        prefix=resolve(provider.output,'capture-'+String(capture).padStart(3,'0')),path=resolve(s,label+'-source-before.private.json'),source=json(path),frame=json(resolve(s,label+'-before-frame.private.json'));
    need(closed(source,['schemaVersion','kind','scenario','sourceHead','label','runID','callLabel','captureLabel','controller','source','claim','owned','post','attempt','raw','captureRaw','automatic','frame','closedFrame','window','ownedSnapshot','rootClock','inspectedAtMs','inputGranted'])&&
        source.schemaVersion===1&&source.kind==='PRIVATE_FIXED_LATEST_SOURCE_CAPTURE'&&source.scenario===s&&source.sourceHead===v.sourceHead&&source.label===label&&
        source.runID===phase.claim.binding.runID&&source.callLabel==='call-'+String(ordinal).padStart(3,'0')&&source.captureLabel==='capture-'+String(capture).padStart(3,'0')&&
        same(source.controller,controllerPin)&&same(source.source,phase.source)&&same(source.claim,phase.claimPin)&&same(source.owned,{path:phase.auth.ownedReceiptPath,sha256:phase.auth.ownedReceiptSha256})&&
        same(source.post,ref(resolve(provider.output,source.callLabel+'.private.json')))&&same(source.attempt,ref(resolve(provider.output,source.callLabel+'.attempt.private.json')))&&
        same(source.raw,ref(resolve(provider.output,source.callLabel+'.raw.private.json')))&&same(source.captureRaw,ref(prefix+'.raw.private.json'))&&
        same(source.automatic,ref(prefix+'.metadata.private.json'))&&same(source.frame,ref(resolve(s,label+'-before-frame.private.json')))&&same(source.rootClock,clock)&&
        source.inputGranted===false&&integer(source.inspectedAtMs,Date.parse(call.post.callCompleted))&&same(source.window,call.window)&&same(source.closedFrame,frame)&&
        same(frame,{pngPath:prefix+'.png',pngSha256:ref(prefix+'.png').sha256,metadataPath:prefix+'.metadata.private.json',metadataSha256:ref(prefix+'.metadata.private.json').sha256,capturedAtMs:Date.parse(call.post.callCompleted)})&&
        same(source.ownedSnapshot,ref(resolve(s,label+'-owned-snapshot.private.json'))));
    const snapshot=json(source.ownedSnapshot.path);need(object(snapshot)&&Array.isArray(snapshot.processes)&&time(snapshot.at)&&snapshot.controllerPid===c.controllerPid&&
        snapshot.controllerCreationTime===c.controllerCreationTime&&snapshot.controllerKernelCreationFileTime===c.controllerKernelCreationFileTime);
    for(const row of snapshot.processes)need(object(row)&&typeof row.executable==='string'&&integer(row.sessionId,1));
    const games=snapshot.processes.filter((/** @type {any} */ row)=>row.executable===resolve(s,'installation/jre/bin/java.exe'));need(games.length===1);
    const game=games[0],first=phases.B1.owned.processes[0],handle=game.mainWindowHandle;
    need(((typeof handle==='number'&&integer(handle,1))||(typeof handle==='string'&&/^[1-9][0-9]*$/.test(handle)&&integer(Number(handle),1)))&&Number(handle)===source.window.windowId&&
        game.inSupervisorJob===true&&game.pid===source.window.pid&&game.sessionId===c.sessionId&&integer(game.parentPid,1)&&game.parentPid!==game.pid&&
        time(game.creationTime)&&Date.parse(game.creationTime)>=Date.parse(c.controllerCreationTime)&&Date.parse(game.creationTime)>=Date.parse(c.at)&&Date.parse(game.creationTime)<=Date.parse(snapshot.at)&&
        typeof game.kernelCreationFileTime==='string'&&/^[1-9][0-9]*$/.test(game.kernelCreationFileTime)&&
        ['pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId'].every(key=>same(game[key],first[key]))&&
        Array.isArray(game.supervisorJobPids)&&new Set(game.supervisorJobPids).size===game.supervisorJobPids.length&&game.supervisorJobPids.every((/** @type {any} */ x)=>integer(x,1))&&
        [game.pid,game.parentPid,c.controllerPid,...c.supervisorJobPids].every(id=>game.supervisorJobPids.includes(id)));
    return{source,frame,path,snapshot,game,provider,call};
}
/** @param {any} ctx */
export function assertPreContinueBudgetGeometry(ctx){
    const {reader,v,clock,c,controllerPin,providers,actions,custody}=ctx,{json,ref}=reader,s=v.scenario;
    need(Array.isArray(custody.metadataCommands)&&custody.metadataCommands.length===5);
    assertRepublishedFrames(ctx);
    /** @type {any[]} */const owned=[];
    /** @type {any[]} */const items=[];
    for(const [i,[label,batch,ordinal,actionIndex]] of stages.entries()){
        const name=String(label),b=String(batch),item=sourceCapture(ctx,name,b,Number(ordinal));items.push(item);owned.push(item.game);
        const row=custody.metadataCommands[i];need(closed(row,['stage','cmd','startedAtMs','raw','completedAtMs'])&&typeof row.stage==='string'&&
            integer(row.startedAtMs,item.source.inspectedAtMs)&&integer(row.completedAtMs,row.startedAtMs)&&object(row.raw)&&row.raw.exit_code===(i===4?1:0)&&
            !Object.hasOwn(row.raw,'session_id')&&typeof row.raw.output==='string'&&Buffer.byteLength(row.raw.output)<=262144&&typeof row.raw.chunk_id==='string');
        if(i===4)continue;
        const gp=ref(resolve(s,name+'-client-geometry.actual.private.json')),tp=ref(resolve(s,name+'-client-geometry-terminal.actual.private.json')),
            pp=ref(resolve(s,name+'-client-point.private.json')),g=json(gp.path),t=json(tp.path),point=json(pp.path),pc=providers[b].providerClock;
        need(closed(t,['schemaVersion','kind','sourceHead','scenario','label','sourceCapture','controller','sourceFrame','program','args','cwd','childCommand','configuration','startedAtMs','completedAtMs',
            'childTimeoutMs','cleanupAllowanceMs','outerTimeoutMs','status','signal','error','stdout','stderr','terminalReaped','outputComplete','supervisorJSON','rootClock','providerClock','closedProvider','geometry','inputGranted'])&&
            t.schemaVersion===1&&t.kind==='ACTUAL_OWNED_GEOMETRY_COMMAND'&&t.sourceHead===v.sourceHead&&t.scenario===s&&t.label===name&&same(t.sourceCapture,ref(item.path))&&
            same(t.controller,controllerPin)&&same(t.sourceFrame,item.source.frame)&&same(t.rootClock,clock)&&same(t.geometry,gp)&&t.status===0&&t.signal===null&&t.error===null&&t.stderr===''&&
            t.terminalReaped===true&&t.outputComplete===true&&t.inputGranted===false&&integer(t.startedAtMs,row.startedAtMs)&&integer(t.completedAtMs,t.startedAtMs)&&t.completedAtMs<=row.completedAtMs&&Date.parse(item.snapshot.at)<=t.startedAtMs);
        const remaining=clock.effectiveDeadlineMs-30000-20000-t.startedAtMs,budget=Math.floor(name==='B1-NEW_GAME'?remaining:Math.min(remaining,pc.deadlineMs-t.startedAtMs-20000-10000)),cleanup=Math.min(10000,Math.floor(budget/2)),child=Math.min(5000,budget-cleanup);
        need(budget>=2&&t.cleanupAllowanceMs===cleanup&&t.childTimeoutMs===child&&t.outerTimeoutMs===child+cleanup&&t.completedAtMs-t.startedAtMs<=t.outerTimeoutMs&&
            same(t.providerClock,name==='B1-NEW_GAME'?null:pc)&&same(t.closedProvider,name==='B1-NEW_GAME'?ref(resolve(providers.B1.output,'owned-provider-terminal.private.json')):null));
        const project=resolve(s,'geometry-owned-'+name),w=item.source.window;
        need(closed(t.childCommand,['program','args','cwd'])&&typeof t.childCommand.program==='string'&&resolve(t.childCommand.program)===resolve(ctx.powershell)&&t.childCommand.cwd==='.'&&
            same(t.childCommand.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1'),'-EvidenceRoot',s,'-WindowId',String(w.windowId),
                '-ExpectedX',String(w.bounds.x),'-ExpectedY',String(w.bounds.y),'-ExpectedWidth',String(w.bounds.width),'-ExpectedHeight',String(w.bounds.height)])&&
            t.program===resolve(v.sourceWorkspace,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe')&&t.cwd===project&&same(t.args,['verify',project,'--scope','module','--module','node:.','--concurrency','1','--timeout',String(child)])&&
            closed(t.configuration,['path','sha256'])&&t.configuration.path===resolve(project,'.agentic/project.json')&&typeof t.configuration.sha256==='string');
        const configuration=file(t.configuration.path,262144);need(configuration.sha256===t.configuration.sha256&&
            same(budgetHistoryJson(configuration.bytes),{version:2,overrides:{workspace:{commands:{'node:.':[t.childCommand]}}}}));
        need(typeof t.stdout==='string'&&Buffer.byteLength(t.stdout)<=262144&&same(budgetHistoryJson(Buffer.from(t.stdout)),t.supervisorJSON));
        const report=t.supervisorJSON,step=report.result?.steps?.[0];
        need(report.schemaVersion===1&&report.command==='verify'&&report.ok===true&&report.result.verdict==='PASS'&&same(report.result.blocked,[])&&
            Array.isArray(report.result.steps)&&report.result.steps.length===1&&object(step)&&step.command===t.childCommand.program&&same(step.args,t.childCommand.args)&&step.cwd==='.'&&step.module==='node:.'&&
            step.processOwnership==='windows-job-object'&&step.status===0&&step.timedOut===false&&step.stderr===''&&typeof step.stdout==='string'&&
            Buffer.from(step.stdout).equals(reader.bytes(gp.path).bytes)&&same(budgetHistoryJson(Buffer.from(step.stdout)),g)&&time(g.observedAtUtc)&&Date.parse(g.observedAtUtc)>=t.startedAtMs&&Date.parse(g.observedAtUtc)<=t.completedAtMs);
        need(closed(point,['schemaVersion','kind','label','frame','geometry','terminal','sourceCapture','ownedSnapshot','localPoint','coordinate','inspectedAtMs','computedAtMs','controllerSha256','inputGranted'])&&
            point.schemaVersion===1&&point.kind==='PRIVATE_FIXED_SEED_CLIENT_POINT'&&point.label===name&&same(point.geometry,gp)&&same(point.terminal,tp)&&same(point.sourceCapture,ref(item.path))&&
            same(point.ownedSnapshot,item.source.ownedSnapshot)&&point.inspectedAtMs===item.source.inspectedAtMs&&point.inspectedAtMs<=t.startedAtMs&&integer(point.computedAtMs,t.completedAtMs)&&
            point.computedAtMs<=row.completedAtMs&&point.controllerSha256===controllerPin.sha256&&point.inputGranted===false&&
            same(point.frame,name==='B1-NEW_GAME'?json(resolve(s,'B1-frame-custody.private.json')):name==='E1-GENERATE'?json(resolve(s,'E1-wizard-2-frame-custody.private.json')):item.frame));
        need(same(point.coordinate,clientImagePoint({geometry:g,window:w,owned:{...item.game,controllerPid:c.controllerPid},dimensions:{width:w.bounds.width,height:w.bounds.height},point:point.localPoint,nowMs:point.computedAtMs})));
        const canonical=name==='B1-PLAY'?ref(item.path):ref(resolve(s,name==='E1-GENERATE'?'E1-wizard-2-inspected.private.json':'B1-menu-inspected.private.json'));
        need(same(json(resolve(s,name+'-coordinate-proof.private.json')),{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_POINT_CANONICAL_BINDING',point:pp,canonical,frame:point.frame,inputGranted:false}));
        if(name==='B1-NEW_GAME')need(same(point.coordinate,json(canonical.path).inspection.newGameCoordinate)&&
            Date.parse(providers.B1.terminal.at)<=t.startedAtMs&&point.computedAtMs<=providers.E1.providerClock.startMs);
        else{const a=actions[Number(actionIndex)],matches=providers[b].calls.filter((/** @type {any} */ x)=>x.post.tool==='left_click'&&
                Date.parse(x.post.callStarted)<=a.sentAtMs&&a.knownResultAtMs<=Date.parse(x.post.callCompleted));
            need(matches.length===1);const call=matches[0];
            need(same(call.post.args,a.args)&&same(call.window.bounds,w.bounds)&&same(g.outerBounds,w.bounds)&&call.window.windowId===w.windowId&&same(point.coordinate,a.args.coordinate)&&
                point.computedAtMs<=a.sentAtMs);}
    }
    assertMetadataCommands(ctx,items);
    return owned;
}
/** @param {any} ctx */
function assertRepublishedFrames(ctx){
    const {reader,providers,phases,actions,v,clock,controllerPin}=ctx,{json,ref}=reader,s=v.scenario;
    for(const [i,[batch,ordinal,capture,label,frameName]] of [['B1',12,2,'B1-after','B1-frame-custody.private.json'],['E1',11,2,'E1-wizard-2','E1-wizard-2-frame-custody.private.json'],['E1',17,3,'E1-wizard-3','E1-wizard-3-frame-custody.private.json']].entries()){
        const b=String(batch),provider=providers[b],phase=phases[b],call=provider.calls[Number(ordinal)-1],a=actions[i],folder=resolve(s,String(label)),
            original=resolve(provider.output,'capture-'+String(capture).padStart(3,'0')),post=resolve(provider.output,'call-'+String(ordinal).padStart(3,'0')),
            png=resolve(folder,'F1.original.png'),raw=resolve(folder,'capture-001.raw.private.json'),metadata=resolve(folder,'capture-001.metadata.private.json'),frame=json(resolve(s,String(frameName)));
        need(reader.bytes(png).bytes.equals(reader.bytes(original+'.png').bytes)&&reader.bytes(raw).bytes.equals(reader.bytes(original+'.raw.private.json').bytes)&&
            same(json(metadata),{...call.post,sourceSha256:phase.source.sha256,reservationID:a.reservationID,capturedAtMs:Date.parse(call.post.callCompleted),window:call.window,
                dimensions:{width:call.window.bounds.width,height:call.window.bounds.height},rawPath:raw,pngPath:png,pngSha256:ref(png).sha256,rawSha256:ref(raw).sha256,captureReceipt:true})&&
            same(frame,{pngPath:png,pngSha256:ref(png).sha256,metadataPath:metadata,metadataSha256:ref(metadata).sha256,capturedAtMs:Date.parse(call.post.callCompleted)}));
        const source=json(resolve(s,i===0?'B1-after-source-capture.private.json':String(label)+'-source-capture.private.json'));
        need(same(source,{schemaVersion:1,source:phase.source,claim:phase.claimPin,sourceCall:ref(post+'.private.json'),attempt:ref(post+'.attempt.private.json'),raw:ref(post+'.raw.private.json'),
            automaticMetadata:ref(original+'.metadata.private.json'),reservationID:a.reservationID,capturedAtMs:frame.capturedAtMs,frame:ref(resolve(s,String(frameName)))}));
        if(i===0){const menu=json(resolve(s,'B1-menu-inspected.private.json'));
            need(menu.schemaVersion===1&&menu.kind==='STARSECTOR_B1_MAIN_MENU_INSPECTION'&&menu.scenario===s&&menu.flow==='DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'&&
                menu.rootRecordKey===clock.recordKey&&menu.controllerSha256===controllerPin.sha256&&same(menu.b1,{sourceSha256:phase.source.sha256,claimSha256:phase.claimPin.sha256,runID:phase.claim.binding.runID,reservationID:a.reservationID})&&
                same(menu.frame,frame)&&menu.inspection?.issuer==='/root'&&menu.inspection.driver===v.driver&&menu.inspection.state==='STARSECTOR_MAIN_MENU'&&
                integer(menu.inspection.inspectedAtMs,frame.capturedAtMs)&&menu.inspection.inspectedAtMs===json(resolve(s,'B1-NEW_GAME-source-before.private.json')).inspectedAtMs&&
                menu.inspection.inspectedAtMs<=providers.E1.providerClock.startMs&&same(menu.inspection.newGameCoordinate,actions[1].args.coordinate)&&same(givenProvider(menu.provider),{
                    readySha256:ref(resolve(provider.output,'guarded-http-ready.private.json')).sha256,terminalSha256:ref(resolve(provider.output,'owned-provider-terminal.private.json')).sha256,
                    observedClosedAtMs:menu.provider.observedClosedAtMs,providerReaped:true})&&integer(menu.provider.observedClosedAtMs,Date.parse(provider.terminal.at))&&menu.provider.observedClosedAtMs<=providers.E1.providerClock.startMs);
        }else if(i===1){const control=json(resolve(s,'E1-wizard-2-inspected.private.json'));
            need(same(control,{schemaVersion:1,kind:'STARSECTOR_SEED_WIZARD_CONTROL',scenario:s,rootRecordKey:clock.recordKey,controllerSha256:controllerPin.sha256,
                sourceSha256:phase.source.sha256,claimSha256:phase.claimPin.sha256,runID:phase.claim.binding.runID,reservationID:a.reservationID,frame,
                inspection:{issuer:'/root',driver:v.driver,inspectedAtMs:control.inspection?.inspectedAtMs,state:'IDENTITY_EMPTY',controlLabel:'GENERATE',enabled:true,coordinate:actions[2].args.coordinate,generatedName:''}})&&
                integer(control.inspection.inspectedAtMs,frame.capturedAtMs)&&control.inspection.inspectedAtMs===json(resolve(s,'E1-GENERATE-source-before.private.json')).inspectedAtMs&&
                control.inspection.inspectedAtMs<=actions[2].sentAtMs&&actions[2].wizardStep.inspectionSha256===ref(resolve(s,'E1-wizard-2-inspected.private.json')).sha256);
        }
    }
}
/** @param {any} value */
function givenProvider(value){need(closed(value,['readySha256','terminalSha256','observedClosedAtMs','providerReaped']));return value;}
/** @param {any} ctx @param {any[]} items */
function assertMetadataCommands(ctx,items){
    const {v,reader,custody,providers,powershell,node,clock}=ctx,{json,ref}=reader,s=v.scenario,q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    const helper=resolve(s,'construct-actual-phase-inputs.private.mjs'),n=q(node.replaceAll('\\','/')),prefix="$ErrorActionPreference='Stop';$env:PATH='"+
        resolve(node,'..').replaceAll('\\','/')+';'+resolve(powershell,'..').replaceAll('\\','/')+";'+$env:PATH;$h="+q(helper)+';';
    for(const [i,item] of items.entries()){
        const row=custody.metadataCommands[i],label=String(stages[i][0]),run=ctx.phases[String(stages[i][1])].claim.binding.runID,
            call=item.source.callLabel,cap=item.source.captureLabel,at=item.source.inspectedAtMs;
        const point=i===4?custody.inspections.at(-1).localPoint:Object.values(json(resolve(s,label+'-client-point.private.json')).localPoint);
        const [x,y]=point;
        const geometry='$gRaw=& '+n+' $h geometry '+label+' '+q(run)+' '+call+' '+cap+' '+at+";if($LASTEXITCODE-ne0){throw 'geometry failed'};$g=$gRaw|ConvertFrom-Json -DateKind String;";
        let command;
        if(i===1)command=prefix+'$gRaw=& '+n+' $h geometry '+label+' '+run+' '+call+' '+cap+' '+at+";if($LASTEXITCODE-ne0){throw 'menu geometry failed'};$g=$gRaw|ConvertFrom-Json -DateKind String;& "+
            q(resolve(s,'frame-menu-phase.failfast.proposed.private.ps1'))+' -RootDispatchedWholeFlow -HelperSha256 '+ctx.p.preclockHelpers['construct-actual-phase-inputs.private.mjs'].sha256+
            ' -CallLabel '+call+' -CaptureLabel '+cap+' -NewGameX '+x+' -NewGameY '+y+' -GeometryPath $g.geometry.path -GeometrySha256 $g.geometry.sha256 -GeometryTerminalSha256 $g.terminal.sha256 -ActualInspectedAtMs '+at+' -ActualGameHwnd '+item.source.window.windowId;
        else{
            command=prefix+(i>=3?'& '+n+' $h frame E1 '+q(run)+' '+call+' '+cap+' E1-wizard-'+(i-1)+";if($LASTEXITCODE-ne0){throw 'frame failed'};":'')+geometry;
            const mapping=i===0?'play-point '+q(run)+' '+call+' '+cap+' '+x+' '+y+' '+at:i===2?'new-game-point '+q(run)+' '+call+' '+cap+' '+x+' '+y+' '+at:
                'control E1 '+(i-1)+' '+q(i===4?custody.observedGeneratedName:'')+' '+x+' '+y+' '+at+' true';
            command+='$mRaw=& '+n+' $h '+mapping+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw '"+(i>=3?'control':'mapping')+" failed'};$m=$mRaw|ConvertFrom-Json -DateKind String;$cRaw=& "+n+' $h point-check '+label+
                " $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw '"+(i===0?'point check':'pointcheck')+" failed'};$cRaw";
        }
        need(row.cmd===command);
        if(i!==1&&i!==4){const checked=budgetHistoryJson(Buffer.from(row.raw.output.trim().split(/\r?\n/).at(-1)));
            need(same(checked.args,ctx.actions[i===0?0:i-1].args)&&same(checked.point,ref(resolve(s,label+'-client-point.private.json')))&&
                same(checked.envelope,ref(resolve(s,label+'-coordinate-proof.private.json')))&&integer(checked.checkedAtMs,json(checked.point.path).computedAtMs)&&checked.checkedAtMs<=row.completedAtMs&&checked.inputGranted===false);}
    }
    const last=custody.metadataCommands[4],failed=items[4],outer=Date.parse(providers.E1.terminal.deadline),pc=providers.E1.providerClock;
    need(typeof custody.observedGeneratedName==='string'&&custody.observedGeneratedName.length>0&&custody.observedGeneratedName.length<=128&&
        last.raw.output.includes('Error: No original budget for geometry plus bounded cleanup')&&last.raw.output.includes('runOwnedClientGeometry')&&last.raw.output.includes('geometry failed')&&
        last.startedAtMs<outer&&outer-last.startedAtMs<30000&&last.startedAtMs<pc.deadlineMs&&pc.deadlineMs-last.startedAtMs<30000&&
        clock.effectiveDeadlineMs-last.startedAtMs>50000&&Date.parse(failed.snapshot.at)<=last.completedAtMs);
    for(const name of ['E1-CONTINUE-client-geometry.actual.private.json','E1-CONTINUE-client-geometry-terminal.actual.private.json','E1-CONTINUE-client-point.private.json','E1-CONTINUE-coordinate-proof.private.json',
        'E1-wizard-3-inspected.private.json','guard-E2-consumed.private.json','E2-authorization.private.json','seed-save-stability.actual.private.json','E2-created-seed-inspected.private.json','seed-creation-closeout.private.json'])need(!existsSync(resolve(s,name)));
}
