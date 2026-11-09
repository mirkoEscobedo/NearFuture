// @ts-check
// Synthetic measured HWND and immutable command custody, never executed.
import {resolve} from 'node:path';
import {mkdirSync,realpathSync} from 'node:fs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
/** @param {any} c */
export function generatedBudgetGeometry(c){
    const {s,root,anchor,iso,old,clock,controllerPin,powershell}=c,node=realpathSync.native(process.execPath),q=(/** @type {string} */ x)=>"'"+x.replaceAll("'","''")+"'";
    const rows=[['B1-PLAY','B1',5,3500],['B1-NEW_GAME','B1',12,18500],['E1-NEW_GAME','E1',5,22500],['E1-GENERATE','E1',11,24300],['E1-CONTINUE','E1',17,197000]];
    /** @type {any[]} */const commands=[];
    for(const [i,[label,batch,ordinal,offset]] of rows.entries()){
        const name=String(label),b=String(batch),p=c.phases[b],call=p.calls[Number(ordinal)-1],capture=p.originals[p.calls.slice(0,Number(ordinal)).filter((/** @type {any} */ x)=>x.post.tool==='screenshot').length-1],
            inspected=anchor+Number(offset),frame={pngPath:capture.png.path,pngSha256:capture.png.sha256,metadataPath:capture.metadata.path,metadataSha256:capture.metadata.sha256,capturedAtMs:Date.parse(call.post.callCompleted)},
            snapshotPath=c.put(name+'-owned-snapshot.private.json',{at:iso(Number(offset)),controllerPid:100,controllerCreationTime:old.controller.controllerCreationTime,controllerKernelCreationFileTime:old.controller.controllerKernelCreationFileTime,
                processes:[{...c.game,mainWindowHandle:String(call.window.windowId)}]}),framePath=c.put(name+'-before-frame.private.json',frame),sourcePath=resolve(s,name+'-source-before.private.json');
        const source={schemaVersion:1,kind:'PRIVATE_FIXED_LATEST_SOURCE_CAPTURE',scenario:s,sourceHead:'1'.repeat(40),label:name,runID:p.runID,callLabel:'call-'+String(ordinal).padStart(3,'0'),captureLabel:'capture-'+String(capture.captureOrdinal).padStart(3,'0'),
            controller:controllerPin,source:p.source,claim:p.claim,owned:ref(p.auth.ownedReceiptPath),post:ref(call.prefix+'.private.json'),attempt:ref(call.prefix+'.attempt.private.json'),raw:ref(call.prefix+'.raw.private.json'),captureRaw:capture.raw,
            automatic:capture.metadata,frame:ref(framePath),closedFrame:frame,window:call.window,ownedSnapshot:ref(snapshotPath),rootClock:clock,inspectedAtMs:inspected,inputGranted:false};c.put(name+'-source-before.private.json',source);
        let output='';
        const localX=i===1&&c.options.differentMappedMenuPoint?1:0;
        if(i<4){
            const project=resolve(s,'geometry-owned-'+name);mkdirSync(resolve(project,'.agentic'),{recursive:true});
            const w=call.window,childCommand={program:powershell,args:['-NoProfile','-NonInteractive','-File',resolve(root,'tools/starsector-proxy/fixture/read-owned-client-geometry.ps1'),'-EvidenceRoot',s,'-WindowId',String(w.windowId),'-ExpectedX','10','-ExpectedY','20','-ExpectedWidth',String(w.bounds.width),'-ExpectedHeight',String(w.bounds.height)],cwd:'.'};
            const config=resolve(project,'.agentic/project.json');json(config,{version:2,overrides:{workspace:{commands:{'node:.':[childCommand]}}}});
            const geometry={schemaVersion:1,scope:'READ_ONLY_OWNED_HWND_CLIENT_GEOMETRY',windowId:w.windowId,pid:101,creationTime:c.game.creationTime,controllerPid:100,observedAtUtc:iso(Number(offset)+30),
                outerBounds:w.bounds,clientOriginScreen:{x:10,y:20},clientSize:{width:w.bounds.width,height:w.bounds.height},offsetFromOuter:{x:0,y:0},mcpOuterBoundsMatched:true,inputGranted:false,originMeaning:'Generated client origin'},
                geometryPath=resolve(s,name+'-client-geometry.actual.private.json');c.options.geometry?.(geometry,name);c.put(name+'-client-geometry.actual.private.json',geometry);
            const nested={schemaVersion:1,command:'verify',ok:true,result:{blocked:[],verdict:'PASS',steps:[{command:powershell,args:childCommand.args,cwd:'.',module:'node:.',processOwnership:'windows-job-object',status:0,timedOut:false,stdout:JSON.stringify(geometry),stderr:''}]}},
                terminalPath=c.put(name+'-client-geometry-terminal.actual.private.json',{schemaVersion:1,kind:'ACTUAL_OWNED_GEOMETRY_COMMAND',sourceHead:'1'.repeat(40),scenario:s,label:name,sourceCapture:ref(sourcePath),controller:controllerPin,sourceFrame:ref(framePath),program:c.supervisor,
                    args:['verify',project,'--scope','module','--module','node:.','--concurrency','1','--timeout','5000'],cwd:project,childCommand,configuration:ref(config),startedAtMs:inspected+10,completedAtMs:inspected+110,childTimeoutMs:5000,cleanupAllowanceMs:10000,outerTimeoutMs:15000,status:0,signal:null,error:null,
                    stdout:JSON.stringify(nested),stderr:'',terminalReaped:true,outputComplete:true,supervisorJSON:nested,rootClock:clock,providerClock:i===1?null:p.providerClock,closedProvider:i===1?ref(p.terminal.terminalReceipt):null,geometry:ref(geometryPath),inputGranted:false});
            const selectedFrame=i===1?value(resolve(s,'B1-frame-custody.private.json')):i===3?value(resolve(s,'E1-wizard-2-frame-custody.private.json')):frame;
            const pointPath=c.put(name+'-client-point.private.json',{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_CLIENT_POINT',label:name,frame:selectedFrame,geometry:ref(geometryPath),terminal:ref(terminalPath),sourceCapture:ref(sourcePath),ownedSnapshot:ref(snapshotPath),
                localPoint:{x:localX,y:0},coordinate:[10+localX,20],inspectedAtMs:inspected,computedAtMs:inspected+130,controllerSha256:controllerPin.sha256,inputGranted:false});
            const canonical=i===0?ref(sourcePath):ref(resolve(s,i===3?'E1-wizard-2-inspected.private.json':'B1-menu-inspected.private.json')),
                envelope=c.put(name+'-coordinate-proof.private.json',{schemaVersion:1,kind:'PRIVATE_FIXED_SEED_POINT_CANONICAL_BINDING',point:ref(pointPath),canonical,frame:selectedFrame,inputGranted:false});
            output=JSON.stringify({args:{coordinate:[10+localX,20],target_app:'java.exe',target_window_id:w.windowId,focus_strategy:'strict'},point:ref(pointPath),envelope:ref(envelope),canonical,checkedAtMs:inspected+140,inputGranted:false});
        }else output='Error: No original budget for geometry plus bounded cleanup\nrunOwnedClientGeometry\ngeometry failed';
        const run=p.runID,callLabel=source.callLabel,captureLabel=source.captureLabel,base="$ErrorActionPreference='Stop';$env:PATH='"+resolve(node,'..').replaceAll('\\','/')+';'+resolve(powershell,'..').replaceAll('\\','/')+";'+$env:PATH;$h="+q(resolve(s,'construct-actual-phase-inputs.private.mjs'))+';',n=q(node.replaceAll('\\','/'));
        let cmd;
        if(i===1)cmd=base+'$gRaw=& '+n+' $h geometry B1-NEW_GAME '+run+' '+callLabel+' '+captureLabel+' '+inspected+";if($LASTEXITCODE-ne0){throw 'menu geometry failed'};$g=$gRaw|ConvertFrom-Json -DateKind String;& "+
            q(resolve(s,'frame-menu-phase.failfast.proposed.private.ps1'))+' -RootDispatchedWholeFlow -HelperSha256 '+old.plan.preclockHelpers['construct-actual-phase-inputs.private.mjs'].sha256+
            ' -CallLabel '+callLabel+' -CaptureLabel '+captureLabel+' -NewGameX '+localX+' -NewGameY 0 -GeometryPath $g.geometry.path -GeometrySha256 $g.geometry.sha256 -GeometryTerminalSha256 $g.terminal.sha256 -ActualInspectedAtMs '+inspected+' -ActualGameHwnd 20';
        else{
            cmd=base+(i>=3?'& '+n+' $h frame E1 '+q(run)+' '+callLabel+' '+captureLabel+' E1-wizard-'+(i-1)+";if($LASTEXITCODE-ne0){throw 'frame failed'};":'')+
                '$gRaw=& '+n+' $h geometry '+name+' '+q(run)+' '+callLabel+' '+captureLabel+' '+inspected+";if($LASTEXITCODE-ne0){throw 'geometry failed'};$g=$gRaw|ConvertFrom-Json -DateKind String;";
            const map=i===0?'play-point '+q(run)+' '+callLabel+' '+captureLabel+' 0 0 '+inspected:i===2?'new-game-point '+q(run)+' '+callLabel+' '+captureLabel+' 0 0 '+inspected:
                'control E1 '+(i-1)+' '+q(i===4?'Seed Captain':'')+' 0 0 '+inspected+' true';
            cmd+='$mRaw=& '+n+' $h '+map+" $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw '"+(i>=3?'control':'mapping')+" failed'};$m=$mRaw|ConvertFrom-Json -DateKind String;$cRaw=& "+n+' $h point-check '+name+
                " $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw '"+(i===0?'point check':'pointcheck')+" failed'};$cRaw";
        }
        commands.push({stage:name,cmd,startedAtMs:inspected,completedAtMs:inspected+(i===4?1000:200),raw:{chunk_id:i===4?'generated-budget-stop':'generated-geometry-'+i,wall_time_seconds:0.1,exit_code:i===4?1:0,original_token_count:1,output}});
    }
    return{commands,inspections:[{stage:'E1-CONTINUE',atMs:anchor+197000,localPoint:[0,0],name:'Seed Captain'}]};
}
