// @ts-check
// Declarative measured-client and terminal boundary data, never native execution.
import {resolve} from 'node:path';
import {mkdirSync,writeFileSync} from 'node:fs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

/** @param {any} ctx */
export function generatedPreStartGeometry(ctx) {
    const {s,root,phases,actions,clock,controllerPin,old,anchor,iso,game,powershell,supervisor}=ctx;
    const rows=[['B1-PLAY','B1',4,0],['B1-NEW_GAME','B1',11,-1],['E1-NEW_GAME','E1',4,1],
        ['E1-GENERATE','E1',10,2],['E1-CONTINUE','E1',16,3],['E2-EXPLORER','E2',4,4],['E2-NORMAL','E2',10,5],['E2-SKIP','E2',16,6],
        ...(ctx.additionalGeometryStages??[])];
    /** @type {any[]} */const successfulGeometry=[];
    /** @param {string} label @param {string} batch @param {number} index @param {number} inspectedAtMs */
    function source(label,batch,index,inspectedAtMs) {
        const p=phases[batch],call=p.calls[index-1],capture=p.calls.slice(0,index).filter((/** @type {any} */ c)=>c.post.tool==='screenshot').length;
        const automatic=p.originals[capture-1];
        const frame={pngPath:automatic.original.path,pngSha256:automatic.original.sha256,metadataPath:automatic.metadata.path,
            metadataSha256:automatic.metadata.sha256,capturedAtMs:Date.parse(call.post.callCompleted)};
        const framePath=resolve(s,label+'-before-frame.private.json');json(framePath,frame);
        const snapshot=resolve(s,label+'-owned-snapshot.private.json');
        const sampledGame={...game,mainWindowHandle:ctx.postStartGeometryReview?.handle?.(call.window.windowId)??call.window.windowId};
        if(label==='E2-START_GAME'&&ctx.options.finalSnapshotExtraPid)sampledGame.supervisorJobPids=[...game.supervisorJobPids,ctx.options.finalSnapshotExtraPid];
        const snapshotOffset=ctx.postStartGeometryReview?.snapshotLabel===label?ctx.postStartGeometryReview.snapshotOffsetMs:0;
        json(snapshot,{at:new Date(inspectedAtMs+snapshotOffset).toISOString(),controllerPid:100,controllerCreationTime:old.controller.controllerCreationTime,
            controllerKernelCreationFileTime:old.controller.controllerKernelCreationFileTime,processes:[sampledGame]});
        const path=resolve(s,label+'-source-before.private.json');
        json(path,{schemaVersion:1,kind:'PRIVATE_FIXED_LATEST_SOURCE_CAPTURE',scenario:s,sourceHead:'1'.repeat(40),label,runID:p.runID,
            callLabel:'call-'+String(index).padStart(3,'0'),captureLabel:'capture-'+String(capture).padStart(3,'0'),controller:controllerPin,
            source:p.source,claim:p.claim,owned:ref(resolve(s,'driver-'+batch+'-owned.private.json')),post:call.refs.post,attempt:call.refs.attempt,
            raw:call.refs.raw,captureRaw:automatic.raw,automatic:automatic.metadata,frame:ref(framePath),closedFrame:frame,
            window:call.window,ownedSnapshot:ref(snapshot),rootClock:clock,inspectedAtMs,inputGranted:false});
        return {path,frame,window:call.window,snapshot};
    }
    for(const [labelValue,batchValue,indexValue,actionValue] of rows) {
        const label=String(labelValue),batch=String(batchValue),index=Number(indexValue),actionIndex=Number(actionValue),p=phases[batch];
        const capture=Date.parse(p.calls[index-1].post.callCompleted),ordinaryInspection=ctx.completedTerminalInput&&actionIndex>=0?
            actions[actionIndex].sentAtMs-200:label==='B1-NEW_GAME'?anchor+16000:capture+50;
        const inspected=ordinaryInspection+(ctx.postStartGeometryReview?.timingLabel===label?ctx.postStartGeometryReview.inspectionOffsetMs:0);
        const delay=ctx.postStartGeometryReview?.timingLabel===label?ctx.postStartGeometryReview.geometryDelayMs??0:0;
        const src=source(label,batch,index,inspected),started=inspected+10+delay,observed=inspected+30+delay,completed=inspected+110+delay,computed=inspected+130+delay;
        const project=resolve(s,'geometry-owned-'+label);mkdirSync(resolve(project,'.agentic'),{recursive:true});
        const w=src.window;
        const childCommand={program:powershell,args:['-NoProfile','-NonInteractive','-File',resolve(root,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1'),
            '-EvidenceRoot',s,'-WindowId',String(w.windowId),'-ExpectedX','10','-ExpectedY','20','-ExpectedWidth','1','-ExpectedHeight','1'],cwd:'.'};
        json(resolve(project,'.agentic/project.json'),{version:2,overrides:{workspace:{commands:{'node:.':[childCommand]}}}});
        const geometryPath=resolve(s,label+'-client-geometry.actual.private.json');
        const geometry={schemaVersion:1,scope:'READ_ONLY_OWNED_HWND_CLIENT_GEOMETRY',windowId:w.windowId,pid:101,
            creationTime:game.creationTime,controllerPid:100,observedAtUtc:new Date(observed).toISOString(),outerBounds:w.bounds,
            clientOriginScreen:{x:10,y:20},clientSize:{width:1,height:1},offsetFromOuter:{x:0,y:0},mcpOuterBoundsMatched:true,inputGranted:false,
            originMeaning:'Synthetic client origin'};
        json(geometryPath,geometry);
        const rootBudget=clock.effectiveDeadlineMs-150000-started;
        const budget=Math.floor(label==='B1-NEW_GAME'?rootBudget:Math.min(rootBudget,p.providerClock.deadlineMs-started-30000));
        const cleanup=Math.min(10000,Math.floor(budget/2)),child=Math.min(5000,budget-cleanup);
        const nested={schemaVersion:1,command:'verify',ok:true,result:{blocked:[],verdict:'PASS',steps:[{command:powershell,args:childCommand.args,cwd:'.',module:'node:.',
            processOwnership:'windows-job-object',status:0,timedOut:false,stdout:JSON.stringify(geometry),stderr:''}]}};
        const terminalPath=resolve(s,label+'-client-geometry-terminal.actual.private.json');
        json(terminalPath,{schemaVersion:1,kind:'ACTUAL_OWNED_GEOMETRY_COMMAND',sourceHead:'1'.repeat(40),scenario:s,label,sourceCapture:ref(src.path),
            controller:controllerPin,sourceFrame:ref(resolve(s,label+'-before-frame.private.json')),program:supervisor,
            args:['verify',project,'--scope','module','--module','node:.','--concurrency','1','--timeout',String(child)],cwd:project,childCommand,
            configuration:ref(resolve(project,'.agentic/project.json')),startedAtMs:started,completedAtMs:completed,childTimeoutMs:child,cleanupAllowanceMs:cleanup,
            outerTimeoutMs:child+cleanup,status:0,signal:null,error:null,stdout:JSON.stringify(nested),stderr:'',terminalReaped:true,outputComplete:true,
            supervisorJSON:nested,rootClock:clock,providerClock:label==='B1-NEW_GAME'?null:p.providerClock,
            closedProvider:label==='B1-NEW_GAME'?p.terminal:null,geometry:ref(geometryPath),inputGranted:false});
        const pointPath=resolve(s,label+'-client-point.private.json');
        json(pointPath,{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_CLIENT_POINT',label,frame:src.frame,geometry:ref(geometryPath),terminal:ref(terminalPath),
            sourceCapture:ref(src.path),ownedSnapshot:ref(src.snapshot),localPoint:{x:0,y:0},coordinate:[10,20],inspectedAtMs:inspected,computedAtMs:computed,
            controllerSha256:controllerPin.sha256,inputGranted:false});
        const canonical=label==='B1-PLAY'?ref(src.path):ref(resolve(s,label==='B1-NEW_GAME'||label==='E1-NEW_GAME'?'B1-menu-inspected.private.json':
            label==='E1-GENERATE'?'E1-wizard-2-inspected.private.json':label==='E1-CONTINUE'?'E1-wizard-3-inspected.private.json':
                'E2-wizard-'+(actionIndex-3)+'-inspected.private.json'));
        json(resolve(s,label+'-coordinate-proof.private.json'),{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_POINT_CANONICAL_BINDING',point:ref(pointPath),canonical,frame:src.frame,inputGranted:false});
        successfulGeometry.push({terminal:ref(terminalPath),geometry:ref(geometryPath),point:ref(pointPath),stage:label});
    }
    if(ctx.completedTerminalInput)return {successfulGeometry};
    const inspected=anchor+273000+(ctx.options.lastClickDelayMs??0),final=source('E2-START_GAME','E2',22,inspected),helper=resolve(s,'construct-actual-phase-inputs.private.mjs');
    const q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'",run=phases.E2.runID,node=realNode();
    const command="$ErrorActionPreference='Stop';$h="+q(helper.replaceAll('\\','/'))+";$n="+q(node.replaceAll('\\','/'))+
        ";&$n $h frame E2 "+run+" call-022 capture-004 E2-wizard-4|Out-Null;if($LASTEXITCODE-ne0){throw 'frame failed'};"+
        "$gr=&$n $h geometry E2-START_GAME "+run+" call-022 capture-004 "+inspected+";if($LASTEXITCODE-ne0){throw 'geometry failed'};"+
        "$g=$gr|ConvertFrom-Json -DateKind String;$mr=&$n $h control E2 4 'Seed Captain' 0 0 "+inspected+
        " true $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw 'control failed'};"+
        "$m=$mr|ConvertFrom-Json -DateKind String;$cr=&$n $h point-check E2-START_GAME $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;"+
        "if($LASTEXITCODE-ne0){throw 'pointcheck failed'};$c=$cr|ConvertFrom-Json -DateKind String;"+
        "$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json -DateKind String;"+
        "[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress";
    const labels=['B1-PLAY','B1-NEW_GAME-ONCE-WRAPPER','E1-NEW_GAME','E1-GENERATE','E1-CONTINUE','E1-OCCUPATION-FRAME',
        'E1-OCCUPATION-E2-PHASE','E2-EXPLORER','E2-NORMAL','E2-SKIP','E2-START_GAME'];
    json(resolve(s,'driver-metadata-calls.actual.private.json'),{schemaVersion:1,kind:'ACTUAL_DRIVER_METADATA_CALLS',driver:'/root/recovery_game_driver',scenario:s,
        entries:labels.map((label,i)=>({label,args:i===10?{cmd:command,shell:powershell.replaceAll('\\','/'),sandbox_permissions:'require_escalated',max_output_tokens:1500}:{},
            raw:{exit_code:i===10?1:0,chunk_id:'synthetic',wall_time_seconds:0.1,output:i===10?'Error: No original budget for geometry plus bounded cleanup\nrunOwnedClientGeometry\ngeometry failed':''},polls:[]}))});
    return {successfulGeometry,finalSourceCapture:ref(final.path),failure:{packetChunk:'synthetic',exitCode:1,message:'No original budget for geometry plus bounded cleanup',
        inspectionAtMs:inspected,originalCapMs:clock.effectiveDeadlineMs,approxRemainingMsAtInspection:clock.effectiveDeadlineMs-inspected,requiredUnchangedMargins:{input:120000,cleanup:30000},clockRenewal:false,retry:false}};
}

import {realpathSync} from 'node:fs';
function realNode(){return realpathSync.native(process.execPath);}
