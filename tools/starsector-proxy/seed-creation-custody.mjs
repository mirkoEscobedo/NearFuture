// @ts-check
// Read-only consumer of completed source evidence. It cannot grant input or create a seed.
import {resolve,dirname,basename,isAbsolute} from 'node:path';
import {readdirSync,lstatSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual} from 'node:util';
import {read,file,requireThat,closed,knownSeedTerminalClosure} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';
import {seedCreationActionLimit,seedCreationFlow} from './seed-creation-history.mjs';
import {seedWizardInput,seedWizardHandoff} from './seed-wizard-proof.mjs';
import {inspectSeedCreationLauncher} from './campaign-authority.mjs';
import {requiredOwnedHttpFiles} from './owned-http-batch.mjs';

/** Exact complete inventory for the CURRENT consumer, including its native supervisor.
 * @param {string} path @param {string} sha256 @param {string} powershell @param {string} supervisor */
export function inspectModernSeedPins(path,sha256,powershell,supervisor) {
    const root=fileURLToPath(new URL('../../',import.meta.url)),found=read(path);
    requireThat(found.sha256===sha256&&closed(found.value,['pins'])&&Array.isArray(found.value.pins),
        'current modern seed source manifest required');
    const expected=[...new Set([...requiredOwnedHttpFiles(root,powershell,supervisor),
        ...['prepare-fixture.ps1','run-native.ps1','read-owned-client-geometry.ps1'].map(n=>resolve(root,'tools/starsector-proxy/fixture',n))])];
    const rows=found.value.pins,map=new Map();
    requireThat(rows.length===expected.length&&rows.length<=1024,'complete modern seed inventory required');
    for(const row of rows) {
        requireThat(closed(row,['path','sha256'])&&typeof row.path==='string'&&resolve(row.path)===row.path&&
            /^[a-f0-9]{64}$/i.test(row.sha256)&&!map.has(row.path),'unique modern seed inventory row required');
        map.set(row.path,row.sha256.toLowerCase());
    }
    for(const p of expected)requireThat(map.get(p)===file(p,128*1024*1024).sha256,'current modern seed inventory changed');
    return rows.length;
}

/** @param {any} result */
function jsonText(result) {
    const text=result.content?.filter((/** @type {any} */ c)=>c.type==='text');
    requireThat(result.isError!==true&&text?.length===1,'known source response required');return JSON.parse(text[0].text);
}

/** @param {any} raw @param {any} window */
function original(raw,window) {
    const images=raw.content?.filter((/** @type {any} */ c)=>c.type==='image');
    requireThat(raw.isError!==true&&images?.length===1&&images[0].mimeType==='image/png'&&
        typeof images[0].data==='string'&&images[0].data.length<=44*1024*1024,'bounded source original required');
    const bytes=Buffer.from(images[0].data,'base64');
    requireThat(bytes.toString('base64')===images[0].data,'canonical source original required');
    const dimensions=completePng(bytes);
    requireThat(dimensions.width===window.bounds.width&&dimensions.height===window.bounds.height,'source original dimensions changed');
    return bytes;
}

/** Exact recorded native response and immediate post-image focus pair for one input.
 * @param {string} workspace @param {string} scenario @param {any} action */
function knownInput(workspace,scenario,action) {
    requireThat(action.status==='AWAITING_AFTER'&&action.tool==='left_click'&&
        action.args?.target_app==='java.exe'&&action.args.focus_strategy==='strict'&&
        /^[A-Za-z0-9_-]+$/.test(action.runID),'known source seed click required');
    const output=resolve(workspace,'.tmp/starsector-proxy',action.runID);
    requireThat(lstatSync(output).isDirectory()&&!lstatSync(output).isSymbolicLink(),'ordinary source provider required');
    const calls=readdirSync(output).filter(n=>/^call-[0-9]{3}\.private\.json$/.test(n)).sort();
    requireThat(calls.length>0&&calls.length<=32,'bounded source provider calls required');
    const matches=calls.map(name=>({name,value:read(resolve(output,name)).value})).filter(({value})=>
        value.tool==='left_click'&&isDeepStrictEqual(value.args,action.args)&&
        Date.parse(value.callStarted)<=action.sentAtMs&&Date.parse(value.callCompleted)>=action.knownResultAtMs);
    requireThat(matches.length===1,'unique acknowledged source click required');
    const call=matches[0],index=Number(call.name.slice(5,8));
    /** @param {number} offset @param {string} suffix */
    const record=(offset,suffix)=>read(resolve(output,'call-'+String(index-offset).padStart(3,'0')+suffix),32*1024*1024).value;
    const meta=call.value,attempt=record(0,'.attempt.private.json'),raw=record(0,'.raw.private.json');
    const owner=read(resolve(scenario,'guard-'+action.batch+'-consumed.private.json')).value.binding;
    requireThat(owner?.schemaVersion===6&&owner.batch===action.batch&&owner.sourceSha256===action.sourceSha256&&
        read(resolve(scenario,action.batch+'-authorization.private.json')).sha256===action.sourceSha256,'source input owner changed');
    const fg=record(2,'.private.json'),win=record(1,'.private.json'),before=record(3,'.private.json');
    const window=jsonText(record(1,'.raw.private.json')),app=jsonText(record(2,'.raw.private.json')).app;
    const [x,y]=action.args.coordinate??[];
    requireThat(meta.isError===false&&meta.batch===action.batch&&meta.runID===action.runID&&attempt.sourceSha256===action.sourceSha256&&
        raw.isError!==true&&raw.content?.length===1&&raw.content[0].type==='text'&&raw.content[0].text===`Clicked (${x}, ${y})`&&
        fg.tool==='get_frontmost_app'&&win.tool==='get_window'&&before.tool==='screenshot'&&
        before.args?.target_window_id===action.args.target_window_id&&before.args.quality===0&&before.args.width===window.bounds?.width&&
        window.windowId===action.args.target_window_id&&window.bundleId==='java.exe'&&window.isFocused===true&&app?.pid===window.pid&&
        owner.ownedPids?.includes(window.pid)&&
        Number.isSafeInteger(x)&&Number.isSafeInteger(y)&&x>=window.bounds.x&&x<window.bounds.x+window.bounds.width&&
        y>=window.bounds.y&&y<window.bounds.y+window.bounds.height&&
        Date.parse(before.callCompleted)<=Date.parse(fg.callStarted)&&Date.parse(fg.callCompleted)<=Date.parse(win.callStarted)&&
        Date.parse(win.callCompleted)<=Date.parse(meta.callStarted),'source click focus or acknowledgement changed');
    const beforeRaw=record(3,'.raw.private.json'),beforeAttempt=record(3,'.attempt.private.json');
    requireThat(beforeAttempt.sourceSha256===action.sourceSha256,'source BEFORE owner changed');
    return {output,window,before,beforeRaw,beforePng:original(beforeRaw,window)};
}

/** An archived inspection must describe the original actually preceding its click.
 * @param {any} input @param {any} receipt @param {any} metadata @param {Buffer} png @param {any} action */
function knownControlBefore(input,receipt,metadata,png,action) {
    requireThat(png.equals(input.beforePng)&&isDeepStrictEqual(metadata.window,input.window)&&
        receipt.frame.capturedAtMs===Date.parse(input.before.callCompleted)&&
        Number.isSafeInteger(receipt.inspection.inspectedAtMs)&&Number.isSafeInteger(action.sentAtMs)&&
        receipt.inspection.inspectedAtMs<=action.sentAtMs,'control original or pre-dispatch inspection changed');
}

/** @param {string} scenarioPath @param {string} closeoutSha256 */
export function inspectSeedCreationCustody(scenarioPath,closeoutSha256) {
    requireThat(isAbsolute(scenarioPath)&&resolve(scenarioPath)===scenarioPath&&/^[a-f0-9]{64}$/.test(closeoutSha256),'canonical completed seed source required');
    const scenario=resolve(scenarioPath);
    // scenario = workspace/.tmp/starsector-proxy/scenarios/<id>
    const actualWorkspace=dirname(dirname(dirname(dirname(scenario))));
    requireThat(scenario===resolve(actualWorkspace,'.tmp/starsector-proxy/scenarios',basename(scenario)),'fixed source scenario containment required');
    const closeout=read(resolve(scenario,'seed-creation-closeout.private.json'));
    requireThat(closeout.sha256===closeoutSha256,'modern seed closeout changed');
    const plan=read(resolve(scenario,'plan.json')),lease=read(resolve(scenario,'native-lane-lease.private.json'));
    const controller=read(resolve(scenario,'controller-start.private.json')),ledger=read(resolve(scenario,'guard-launch-history.private.json')).value;
    requireThat(plan.value.campaignControl?.schemaVersion===4&&plan.value.campaignControl.profile===seedCreationFlow&&
        lease.value.planSha256?.toLowerCase()===plan.sha256&&controller.value.rootLeaseSha256?.toLowerCase()===lease.sha256&&
        ledger.controllerSha256===controller.sha256&&ledger.uncertain===false,'completed seed authority changed');
    const limit=seedCreationActionLimit(plan.value,lease.value,scenario);
    const first=read(resolve(scenario,'E1-authorization.private.json')),last=read(resolve(scenario,'E2-authorization.private.json'));
    const claim1=read(resolve(scenario,'guard-E1-consumed.private.json')),claim2=read(resolve(scenario,'guard-E2-consumed.private.json'));
    const initial=claim1.value.binding,binding=claim2.value.binding;
    requireThat(initial?.schemaVersion===6&&binding?.schemaVersion===6&&initial.batch==='E1'&&binding.batch==='E2'&&
        initial.workspaceRoot===actualWorkspace&&binding.workspaceRoot===actualWorkspace&&initial.scenario===scenario&&binding.scenario===scenario&&
        initial.sourceSha256===first.sha256&&binding.sourceSha256===last.sha256&&initial.originalPlanSha256===plan.sha256&&
        binding.originalPlanSha256===plan.sha256&&binding.controllerSha256===controller.sha256&&initial.controllerSha256===controller.sha256&&
        isDeepStrictEqual(ledger.history,closeout.value.actions?.actualGuard)&&ledger.history.reservedActions<=limit,
    'completed seed claim lineage changed');
    inspectSeedCreationLauncher(initial);
    seedWizardHandoff({binding,reference:{path:binding.e1WizardReceipt.path,sha256:binding.e1WizardReceipt.sha256},now:Date.now(),
        initial,initialSource:first,claim:claim1,state:ledger,action:ledger.resolvedE1,
        providerClock:ledger.resolvedE1.providerClock,alreadyTransferred:true});
    const resolved=ledger.resolvedActions??[],e1=resolved.filter((/** @type {any} */ a)=>a.batch==='E1'),e2=resolved.filter((/** @type {any} */ a)=>a.batch==='E2');
    requireThat(e1.length===2&&e2.length===3&&ledger.pending?.wizardStep?.stage==='START_GAME'&&
        ledger.pending.wizardStep.ordinal===4&&ledger.pending.batch==='E2'&&ledger.pending.sourceSha256===last.sha256,'known final seed Start required');
    const all=[ledger.resolvedB1,...e1,ledger.resolvedE1,...e2,ledger.pending];
    requireThat(all.length===8,'eight completed seed inputs required');
    const inputs=all.map(action=>knownInput(actualWorkspace,scenario,action));
    for(let ordinal=2;ordinal<=3;ordinal++) {
        const receipt=read(resolve(scenario,'E1-wizard-'+ordinal+'-inspected.private.json')).value;
        knownControlBefore(inputs[ordinal],receipt,read(receipt.frame.metadataPath).value,
            file(receipt.frame.pngPath,32*1024*1024).bytes,all[ordinal]);
    }
    const actions=[...e2,ledger.pending];
    for(let i=0;i<actions.length;i++) {
        const previous=i===0?ledger.resolvedE1:actions[i-1],prefix='E2-wizard-'+(i+1);
        const receipt=read(resolve(scenario,prefix+'-inspected.private.json')).value;
        const metadata=read(receipt.frame.metadataPath).value,png=file(receipt.frame.pngPath,32*1024*1024);
        const input=inputs[4+i];
        knownControlBefore(input,receipt,metadata,png.bytes,actions[i]);
        if(i>0)requireThat(previous.after?.pngSha256===png.sha256&&
            previous.seedWizardInspectionSha256===read(resolve(scenario,prefix+'-inspected.private.json')).sha256,
        'recorded seed control AFTER changed');
        const after=i>0?{...previous.after,pngBytes:png.bytes}:
            {pngBytes:png.bytes,window:metadata.window,capturedAtMs:receipt.frame.capturedAtMs,observedAtMs:receipt.inspection.inspectedAtMs};
        const proof=seedWizardInput(binding,{resolvedActions:e2.slice(0,Math.max(0,i-1)),
            resolvedE1:ledger.resolvedE1,pending:i===0?null:previous},actions[i].args,after,Date.now());
        requireThat(actions[i].wizardStep.inspectionSha256===proof.inspectionSha256&&actions[i].wizardStep.stage===proof.stage,'completed seed control changed');
    }
    const frame=read(resolve(scenario,'E2-created-seed/capture-001.metadata.private.json')).value;
    const png=file(resolve(scenario,'E2-created-seed/F1.original.png'),32*1024*1024);
    const raw=read(resolve(scenario,'E2-created-seed/capture-001.raw.private.json'),32*1024*1024);
    requireThat(frame.captureReceipt===true&&frame.pngSha256===png.sha256&&frame.rawSha256===raw.sha256&&
        frame.reservationID===ledger.pending.reservationID&&frame.sourceSha256===last.sha256&&
        frame.batch==='E2'&&frame.runID===binding.runID&&frame.window?.windowId===binding.campaign.windowId&&
        binding.ownedPids.includes(frame.window.pid)&&frame.window.isFocused===true&&frame.args?.target_window_id===binding.campaign.windowId&&
        frame.args.quality===0&&frame.args.width===frame.window.bounds.width&&
        frame.capturedAtMs>=ledger.pending.knownResultAtMs&&png.bytes.equals(original(raw.value,frame.window)),'final seed original missing or changed');
    const final=read(resolve(scenario,'E2-created-seed-inspected.private.json')).value;
    const finalDirectory=resolve(scenario,'E2-created-seed');
    requireThat(closed(final.frame,['pngPath','pngSha256','metadataPath','metadataSha256','capturedAtMs'])&&
        final.frame.pngPath===resolve(finalDirectory,'F1.original.png')&&
        final.frame.metadataPath===resolve(finalDirectory,'capture-001.metadata.private.json')&&
        frame.pngPath===final.frame.pngPath&&frame.rawPath===resolve(finalDirectory,'capture-001.raw.private.json')&&
        Number.isSafeInteger(frame.capturedAtMs)&&final.frame.capturedAtMs===frame.capturedAtMs&&
        frame.capturedAtMs===Date.parse(frame.callCompleted)&&Date.parse(frame.callStarted)<=frame.capturedAtMs&&
        isDeepStrictEqual(frame.dimensions,completePng(png.bytes)),'final seed fixed frame chronology changed');
    const output=inputs[7].output;
    const captures=readdirSync(output).filter(n=>/^call-[0-9]{3}\.private\.json$/.test(n)).map(name=>({name,value:read(resolve(output,name)).value}))
        .filter(({value})=>value.tool==='screenshot'&&value.callCompleted===frame.callCompleted&&
            value.callStarted===frame.callStarted&&isDeepStrictEqual(value.args,frame.args));
    requireThat(captures.length===1,'unique final seed source capture required');
    const capture=captures[0],stem=capture.name.replace('.private.json','');
    requireThat(capture.value.batch==='E2'&&capture.value.runID===binding.runID&&capture.value.isError===false&&
        read(resolve(output,stem+'.attempt.private.json')).value.sourceSha256===last.sha256&&
        isDeepStrictEqual(read(resolve(output,stem+'.raw.private.json'),32*1024*1024).value,raw.value),
    'actual final seed source capture changed');
    requireThat(closed(final,['schemaVersion','kind','scenario','rootRecordKey','controllerSha256','sourceSha256','claimSha256',
        'runID','reservationID','frame','inspection','provider'])&&final.schemaVersion===1&&final.kind==='STARSECTOR_NEW_SEED_INSPECTION'&&
        final.scenario===scenario&&final.rootRecordKey===ledger.rootRecordKey&&final.controllerSha256===controller.sha256&&
        final.sourceSha256===last.sha256&&final.claimSha256===claim2.sha256&&final.runID===binding.runID&&
        final.reservationID===ledger.pending.reservationID&&final.frame.pngSha256===png.sha256&&
        final.frame.metadataSha256===read(resolve(scenario,'E2-created-seed/capture-001.metadata.private.json')).sha256&&
        closed(final.inspection,['issuer','driver','inspectedAtMs','state','generatedName'])&&
        final.inspection.issuer==='/root'&&final.inspection.driver==='/root/recovery_game_driver'&&
        final.inspection.state==='NEW_GAME_CAMPAIGN_VISIBLE'&&final.inspection.generatedName===binding.e1WizardReceipt.generatedName,
    'final seed visible inspection required');
    knownSeedTerminalClosure({binding,initial:binding,initialSourceSha256:last.sha256,proof:final,
        providerClock:ledger.providerClock,action:ledger.pending,now:Date.now()});
    return {schemaVersion:1,kind:'COMPLETED_SEED_WIZARD_CUSTODY',ok:true,inputs:8,scope:'HISTORICAL_CUSTODY_ONLY'};
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
    try {
        requireThat(process.argv.length===8,'source, closeout and current complete inventory required');
        const count=inspectModernSeedPins(process.argv[4],process.argv[5],process.argv[6],process.argv[7]);
        process.stdout.write(JSON.stringify({...inspectSeedCreationCustody(process.argv[2],process.argv[3]),sourcePinsCount:count})+'\n');
    } catch {process.stderr.write('Seed creation custody refused\n');process.exitCode=1;}
}
