// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {generationProtectedFixture} from './generation-fixture-test-support.mjs';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher,seedInputSession} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';
import {readFixtureBinding} from '../fixture-binding.mjs';
import {readFileSync,writeFileSync,unlinkSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {powershell} from './seed-test-support.mjs';

test('the generation fixture cannot launch under an ordinary no-control campaign lease',()=>{
    const f=generationProtectedFixture();
    try {
        f.launch('neither');
        const result=f.protection('admit-launch');
        assert.notEqual(result.status,0,'fixed generation profile must require its separate flow');
        assert.match(result.stderr,/Generation fixture requires its explicit campaign control/);
    } finally {f.close();}
});

test('generation reaches both maintained public launch gates and the fixed watchdog before controlled pre-effect refusals',()=>{
    const f=campaignLaunchFixture({generation:true});
    try {
        assert.equal(f.watchdog().status,0,'the original eight-minute watchdog remains valid');
        const original=[f.planPath,f.leasePath].map(path=>readFileSync(path));
        const controller=readFileSync(f.controllerPath);unlinkSync(f.controllerPath);
        assert.equal(existsSync(resolve(f.installation,'jre/bin/java.exe')),false);
        writeFileSync(resolve(f.scenario,'launch-supervisor.private.json'),'Controlled pre-effect sentinel');
        try {
            let result=powershell(['-File',resolve(f.root,'tools/starsector-proxy/fixture/run-native.ps1'),
                '-BindingPath',f.bindingPath,'-BindingSha256',f.bindingPin.sha256,'-EvidenceRoot',f.scenario,'-Phase','launch']);
            assert.notEqual(result.status,0);assert.match(result.stderr,/Preserve existing native evidence/);
            assert.equal(existsSync(resolve(f.scenario,'supervision-launch')),false);
            result=powershell(['-File',resolve(f.root,'tools/starsector-proxy/fixture/native-controller.ps1'),
                '-BindingPath',f.bindingPath,'-BindingSha256',f.bindingPin.sha256,'-EvidenceRoot',f.scenario,
                '-OuterDeadlineUtc',new Date(Date.now()+470000).toISOString().replace(/Z$/,'0000Z')]);
            assert.notEqual(result.status,0);assert.match(result.stderr,/java\.exe/);
            assert.equal(existsSync(f.controllerPath),false);
        } finally {writeFileSync(f.controllerPath,controller,{flag:'wx'});}
        assert.deepEqual([f.planPath,f.leasePath].map(path=>readFileSync(path)),original);
    } finally {f.close();}
});

test('generation B1 dispatches one guarded Play through the maintained consent and native adapter',async()=>{
    const f=campaignLaunchFixture({generation:true});
    const names=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const prior=names.map(name=>process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];
    try {
        const authority=claimSeededLauncher(f,'generation-guarded-play',false);
        claimBatchAuthorization(readBatchAuthorization(authority.source,{root:f.root,runID:authority.runID}));
        h=seedInputSession(authority,tool=>effects.push(tool));await h.before();
        assert.notEqual((await h.call('left_click',{target_window_id:19,target_app:'java.exe',
            focus_strategy:'strict',coordinate:[40,80]})).isError,true);
        assert.deepEqual(effects,['left_click']);
        await h.before();const active=h;
        await assert.rejects(()=>active.call('left_click',{target_window_id:19,target_app:'java.exe',
            focus_strategy:'strict',coordinate:[40,80]}),/remaining sequence/);
        assert.deepEqual(effects,['left_click']);
    } finally {
        await h?.embedded.server.close();f.close();
        names.forEach((name,index)=>{if(prior[index]===undefined)delete process.env[name];else process.env[name]=prior[index];});
    }
});

test('the fixed generation profile composes real PowerShell launch admission and JavaScript B1 fixture binding',()=>{
    const f=campaignLaunchFixture({generation:true});
    try {
        const launch=f.admit();assert.equal(launch.status,0,launch.stderr);
        const authority=claimSeededLauncher(f,'generation-launch-binding',false);
        const admitted=readBatchAuthorization(authority.source,{root:f.root,runID:authority.runID});
        assert.equal(admitted.binding.schemaVersion,5);
        assert.deepEqual(readFixtureBinding(admitted),{path:f.bindingPath,sha256:f.bindingPin.sha256.toLowerCase()});
    } finally {f.close();}
});
