// @ts-check
// Real stdio/HTTP/claims/consent with only native OS effects and visual interpretation generated.
import assert from 'node:assert/strict';
import {PassThrough} from 'node:stream';
import {mkdirSync,writeFileSync,readFileSync,existsSync,rmSync,realpathSync} from 'node:fs';
import {resolve,basename,relative,isAbsolute} from 'node:path';
import {Client} from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import {StdioServerTransport} from '../../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import {serveDirectMcp} from '../direct-mcp.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {directFixture} from '../direct-test-fixture.mjs';
import {requiredOwnedHttpFiles} from '../owned-http-batch.mjs';
import {persistDirectFrame} from '../direct-frames.mjs';
import {seedNativeSessionFactory,seedTarget} from './campaign-input-test-support.mjs';
import {seedCreationFixture,creationFlow as originalCreationFlow} from './seed-creation-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
/** @param {string} p */
const sha=p=>filePin(p).sha256.toLowerCase();
/** @param {any} r */
const textJson=r=>JSON.parse(r.content.find((/** @type {any} */ c)=>c.type==='text').text);

/** @param {{retain?:boolean,watchdogMs?:number,callerBudgetMs?:number,fixture?:()=>any,initialActivation?:boolean,expectedProviderTimeoutMs?:480000}} [options] */
export async function exerciseSeedCreationTransport(options={}) {
    const f=options.fixture?.()??seedCreationFixture({watchdogMs:options.watchdogMs,callerBudgetMs:options.callerBudgetMs}),input=new PassThrough(),output=new PassThrough();
    const creationFlow=f.plan.campaignControl.profile,version=creationFlow===originalCreationFlow?6:7;
    const names=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const prior=names.map(n=>process.env[n]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    const client=new Client({name:'seed-creation-composition',version:'1'},
        {capabilities:{},supportedProtocolVersions:['2025-11-25']});
    const parent=resolve(f.root,'.tmp/starsector-proxy'),prefix='synthetic-seed-creation-'+basename(f.scenario);
    /** @type {string[]} */const outputs=[];
    /** @type {string[]} */const effects=[];
    /** @type {Record<string,number>} */const callCounts={};
    /** @type {ReturnType<typeof directFixture>|undefined} */let provider;
    /** @type {ReturnType<typeof serveDirectMcp>|undefined} */let frontend;
    let batch='B1',runID=prefix+'-B1',source='',calls=0,window={...seedTarget,isFocused:!options.initialActivation};
    let retained=false;
    function cleanup() {
        for(const p of outputs)if(existsSync(p)){const suffix=relative(parent,p);assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));rmSync(p,{recursive:true});}
        f.close();
    }
    const ledger=()=>JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
    /** @param {string} tool @param {any} args */
    async function call(tool,args) {
        if(!['begin_batch','finish_batch'].includes(tool))calls++;
        const result=await client.callTool({name:tool,arguments:args});
        assert.notEqual(result.isError,true,tool+': '+JSON.stringify(result));return result;
    }
    /** @param {boolean} [diagnose] */
    async function before(diagnose=false) {
        if(diagnose){await call('doctor',{});await call('policy_status',{});}
        await call('get_window',{window_id:window.windowId});
        await call('screenshot',{target_window_id:window.windowId,quality:0,width:480});
    }
    async function focus() {await call('get_frontmost_app',{});await call('get_window',{window_id:window.windowId});}
    /** @param {number[]} coordinate */
    async function click(coordinate) {
        await focus();await call('left_click',{target_window_id:window.windowId,target_app:'java.exe',focus_strategy:'strict',coordinate});
    }
    /** @param {string} directory @param {boolean} [capture] */
    async function frame(directory,capture=true) {
        if(capture){await call('get_window',{window_id:window.windowId});await call('screenshot',{target_window_id:window.windowId,quality:0,width:480});}
        const label='call-'+String(calls).padStart(3,'0'),providerRoot=resolve(parent,runID);
        const post=JSON.parse(readFileSync(resolve(providerRoot,label+'.private.json'),'utf8'));
        const attempt=JSON.parse(readFileSync(resolve(providerRoot,label+'.attempt.private.json'),'utf8'));
        const result=JSON.parse(readFileSync(resolve(providerRoot,label+'.raw.private.json'),'utf8'));
        assert.equal(post.tool,'screenshot');assert.equal(attempt.sourceSha256,sha(source));
        const action=ledger().pending??ledger().resolvedE1,dir=resolve(f.scenario,directory);mkdirSync(dir);
        const receipt=persistDirectFrame({output:dir,label:'capture-001',pngName:'F1.original.png',result,
            args:post.args,window,ownedPids:[101],provenance:{...post,sourceSha256:sha(source),
                reservationID:action.reservationID,capturedAtMs:Date.parse(post.callCompleted)}});
        assert.ok(receipt.pngPath&&receipt.metadataPath);
        return {pngPath:receipt.pngPath,pngSha256:sha(receipt.pngPath),metadataPath:receipt.metadataPath,
            metadataSha256:sha(receipt.metadataPath),capturedAtMs:Date.parse(post.callCompleted)};
    }
    /** @param {string} phase @param {any} extra */
    function phase(phase,extra) {
        const campaign={...f.campaign,windowId:window.windowId};
        const planPath=resolve(f.scenario,phase+'-plan.private.json'),leasePath=resolve(f.scenario,phase+'-lease.private.json');
        json(planPath,{...f.phasePlan,campaign,batches:[{id:phase}],...extra});
        json(leasePath,{...f.phaseLease,schemaVersion:version,campaign,phase:creationFlow,planSha256:sha(planPath),...extra});
        const ownedReceiptPath=resolve(f.scenario,'driver-'+phase+'-owned.private.json');
        if(phase!=='E1')json(ownedReceiptPath,JSON.parse(readFileSync(resolve(f.scenario,'driver-E1-owned.private.json'),'utf8')));
        const path=resolve(f.scenario,phase+'-authorization.private.json');
        json(path,{schemaVersion:version,scenario:f.scenario,batch:phase,planPath,planSha256:sha(planPath),leasePath,
            leaseSha256:sha(leasePath),ownedReceiptPath,ownedReceiptSha256:sha(ownedReceiptPath)});return path;
    }
    /** @param {string} phase @param {string} path */
    async function begin(phase,path) {
        batch=phase;source=path;runID=prefix+'-'+phase;calls=0;outputs.push(resolve(parent,runID));
        assert.equal(readBatchAuthorization(source,{root:f.root,runID}).binding.batch,phase);
        const ready=textJson(await call('begin_batch',{source,runID,consentMode:'finite-explicit-consent'}));
        if(version===6)assert.equal(Date.parse(ready.deadline),f.campaign.rootClock.callerAdmissionDeadlineMs-30000);
        else{const duration=options.expectedProviderTimeoutMs??180000;
            assert.ok(Date.parse(ready.deadline)<=Date.now()+duration&&Date.parse(ready.deadline)>Date.now()+duration-30000);
            if(options.expectedProviderTimeoutMs!==undefined){
                assert.ok(provider);
                const pc=ledger().providerClock,args=provider.args();
                assert.equal(pc.deadlineMs-pc.startMs,480000);
                assert.equal(args[args.indexOf('--timeout')+1],'480000');
            }
        }
    }
    async function finish() {
        const t=textJson(await call('finish_batch',{})),dir=resolve(parent,runID);callCounts[batch]=calls;
        assert.equal(t.terminalObserved,true);assert.equal(t.cleanupUnknown,false);assert.equal(t.verifierStatus,0);
        assert.equal(t.terminalBoundToChild,true);assert.ok(calls<=32);
        return {readySha256:sha(resolve(dir,'guarded-http-ready.private.json')),
            terminalSha256:sha(resolve(dir,'owned-provider-terminal.private.json')),observedClosedAtMs:Date.now(),providerReaped:true};
    }
    /** @param {number} ordinal @param {string} state @param {string} label @param {number[]} coordinate @param {string} name @param {boolean} [capture] */
    async function control(ordinal,state,label,coordinate,name,capture=true) {
        const prefix=batch+'-wizard-'+ordinal,original=await frame(prefix,capture),action=ledger().pending??ledger().resolvedE1;
        json(resolve(f.scenario,prefix+'-inspected.private.json'),{schemaVersion:1,kind:'STARSECTOR_SEED_WIZARD_CONTROL',
            scenario:f.scenario,rootRecordKey:f.campaign.rootClock.recordKey,controllerSha256:sha(f.controllerPath),
            sourceSha256:sha(source),claimSha256:sha(resolve(f.scenario,'guard-'+batch+'-consumed.private.json')),
            runID,reservationID:action.reservationID,frame:original,inspection:{issuer:'/root',driver:'/root/recovery_game_driver',
                inspectedAtMs:Date.now(),state,controlLabel:label,enabled:true,coordinate,generatedName:name}});
        await click(coordinate);
    }
    try {
        const powershell=version===7?realpathSync.native(process.env.NF_TEST_POWERSHELL??'pwsh'):resolve(f.scenario,'synthetic-creation-helper.exe'),
            supervisor=version===7?resolve(f.root,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe'):resolve(f.scenario,'synthetic-creation-verifier.exe');
        if(version===6)for(const p of [powershell,supervisor])writeFileSync(p,'Synthetic external process identity; never executed.');
        const pinsPath=resolve(f.scenario,'frozen-support-pins.private.json');
        json(pinsPath,{pins:[...new Set([...requiredOwnedHttpFiles(f.root,powershell,supervisor),...(version===7?['prepare-fixture.ps1','run-native.ps1','read-owned-client-geometry.ps1'].map(n=>resolve(f.root,'tools/starsector-proxy/fixture',n)):[])])].map(p=>({path:p,sha256:sha(p)}))});
        f.phaseLease.supportPinsSha256=f.launchLease.supportPinsSha256=sha(pinsPath);
        const cap=Date.now()+(options.callerBudgetMs??(version===7?500000:170000));
        for(const value of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])value.callerAdmissionDeadlineMs=cap;
        Object.assign(f.campaign.rootClock,{callerAdmissionDeadlineMs:cap,originalCallerAdmissionDeadlineMs:cap,effectiveDeadlineMs:cap});
        f.campaign.producerWitness.effectiveDeadlineMs=cap;f.refreshLaunch();
        const authority=f.launcher(runID),originalAuthority=[f.planPath,f.leasePath].map(p=>readFileSync(p));
        provider=directFixture({authority:{root:f.root,source:authority.source,runID,powershell,supervisor,consentMode:'finite-explicit-consent'},
            sessionFactory:seedNativeSessionFactory(tool=>{
                assert.equal(ledger().history.sentActions,f.plan.historyBaseline.sentActions+1+effects.length);effects.push(tool);if(tool==='activate_window')window={...window,isFocused:true};
                if(effects.filter(n=>n==='left_click').length===4){mkdirSync(resolve(f.installation,'saves/common'));
                    writeFileSync(resolve(f.installation,'saves/common/core_shared_settings.json.data'),'{"campaignHelpPopupsOptionChecked": true}');}
                if(effects.filter(n=>n==='left_click').length===8){const dir=resolve(f.installation,'saves/save_created_fixture');mkdirSync(dir);
                    writeFileSync(resolve(dir,'campaign.xml'),'<CampaignEngine><saveDirName>save_created_fixture</saveDirName><playerFleet ref="7"/><characterData><level>1</level></characterData><saveDesc/><Flt z="7"/></CampaignEngine>');
                    writeFileSync(resolve(dir,'descriptor.xml'),'<SaveGameData><compressed>false</compressed><gameVersion>0.98a-RC8</gameVersion><enabledMods/></SaveGameData>');}
            },{window:()=>window})});
        frontend=serveDirectMcp({...provider.options,spawn:provider.spawn,stdin:input,stdout:output});
        await client.connect(new StdioServerTransport(output,input));
        await begin('B1',authority.source);await before(true);
        if(options.initialActivation){
            await focus();await call('activate_window',{window_id:window.windowId,timeout_ms:2000});
            const reserved=ledger().history;await focus();await call('screenshot',{target_window_id:window.windowId,quality:0,width:480});
            const resolved=ledger();assert.equal(resolved.pending,null,'initial activation resolved before geometry');
            assert.equal(resolved.resolvedInitialActivation.tool,'activate_window');assert.deepEqual(resolved.history,reserved);
            assert.equal(resolved.resolvedActions?.length??0,0,'activation is separate from wizard click ordinals');
        }
        await click([40,80]);window={...seedTarget,windowId:21};
        const main=await frame('B1-after'),play=ledger().pending,closedB1=await finish();
        const menu=resolve(f.scenario,'B1-menu-inspected.private.json');
        json(menu,{schemaVersion:1,kind:'STARSECTOR_B1_MAIN_MENU_INSPECTION',scenario:f.scenario,flow:creationFlow,
            rootRecordKey:f.campaign.rootClock.recordKey,controllerSha256:sha(f.controllerPath),
            b1:{sourceSha256:sha(source),claimSha256:sha(resolve(f.scenario,'guard-B1-consumed.private.json')),runID,reservationID:play.reservationID},
            frame:main,inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:Date.now(),state:'STARSECTOR_MAIN_MENU',newGameCoordinate:[40,80]},provider:closedB1});
        await begin('E1',phase('E1',{b1MenuReceipt:{path:menu,sha256:sha(menu)}}));
        await before(true);await click([40,80]);
        await control(2,'IDENTITY_EMPTY','GENERATE',[70,90],'');
        await control(3,'IDENTITY_NAMED','1. Continue',[100,120],'Seed Captain');
        const occupation=await frame('E1-occupation'),state=ledger(),closedE1=await finish();
        const handoff=resolve(f.scenario,'E1-occupation-inspected.private.json');
        json(handoff,{schemaVersion:1,kind:'STARSECTOR_E1_OCCUPATION_INSPECTION',scenario:f.scenario,flow:creationFlow,
            rootRecordKey:f.campaign.rootClock.recordKey,controllerSha256:sha(f.controllerPath),
            e1:{sourceSha256:sha(source),claimSha256:sha(resolve(f.scenario,'guard-E1-consumed.private.json')),runID,
                reservations:[...state.resolvedActions.filter((/** @type {any} */ a)=>a.batch==='E1'),state.pending].map(a=>a.reservationID)},
            frame:occupation,inspection:{issuer:'/root',driver:'/root/recovery_game_driver',inspectedAtMs:Date.now(),state:'OCCUPATION_SELECTION',generatedName:'Seed Captain'},provider:closedE1});
        await begin('E2',phase('E2',{e1WizardReceipt:{path:handoff,sha256:sha(handoff)}}));
        await before(true);
        await control(1,'OCCUPATION_SELECTION','An explorer, leading a salvage expedition from aboard an Apogee-class cruiser (faster start)',[80,120],'Seed Captain',false);
        await control(2,'CAMPAIGN_DIFFICULTY','Normal',[90,130],'Seed Captain');
        await control(3,'TUTORIAL_CHOICE','Skip it',[110,140],'Seed Captain');
        await control(4,'NEW_GAME_STATS','Start game',[120,150],'Seed Captain');
        const created=await frame('E2-created-seed'),inspectedAtMs=Date.now(),closedE2=await finish();
        json(resolve(f.scenario,'E2-created-seed-inspected.private.json'),{schemaVersion:1,kind:'STARSECTOR_NEW_SEED_INSPECTION',
            scenario:f.scenario,rootRecordKey:f.campaign.rootClock.recordKey,controllerSha256:sha(f.controllerPath),
            sourceSha256:sha(source),claimSha256:sha(resolve(f.scenario,'guard-E2-consumed.private.json')),runID,
            reservationID:ledger().pending.reservationID,frame:created,inspection:{issuer:'/root',driver:'/root/recovery_game_driver',
                inspectedAtMs,state:'NEW_GAME_CAMPAIGN_VISIBLE',generatedName:'Seed Captain'},provider:closedE2});
        assert.deepEqual(effects,[...(options.initialActivation?['activate_window']:[]),...Array(8).fill('left_click')]);
        assert.deepEqual(ledger().history,{...f.plan.historyBaseline,reservedActions:f.plan.historyBaseline.reservedActions+effects.length,sentActions:f.plan.historyBaseline.sentActions+effects.length});
        assert.equal(ledger().uncertain,false);assert.equal(ledger().pending.wizardStep.stage,'START_GAME');
        assert.equal(provider.starts(),3);assert.ok(callCounts.E1<=32&&callCounts.E2<=32);
        assert.deepEqual([f.planPath,f.leasePath].map(p=>readFileSync(p)),originalAuthority);
        if(options.retain){retained=true;return {fixture:f,callCounts,outputs,close:cleanup};}
    } finally {
        await client.close();await frontend?.close();await provider?.close();
        if(!retained)cleanup();
        names.forEach((n,i)=>{if(prior[i]===undefined)delete process.env[n];else process.env[n]=prior[i];});
    }
}
