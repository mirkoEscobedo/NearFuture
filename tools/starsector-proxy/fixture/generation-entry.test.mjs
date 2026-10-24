// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {completedSeedLaunch} from './campaign-seed-handoff-test-support.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';
import {inspectGenerate} from './generation-input-test-support.mjs';

test('generation E1 transfers the known closed Play provider and dispatches only the inspected New Game control',async()=>{
    const names=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const prior=names.map(name=>process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>>|undefined} */let f;
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];
    try {
        f=await completedSeedLaunch({generation:true});
        const authority={...f,runID:'generation-entry-new-game'};
        claimBatchAuthorization(readBatchAuthorization(authority.source,{root:f.root,runID:authority.runID}));
        h=seedInputSession(authority,tool=>effects.push(tool),{window:()=>({...seedTarget,windowId:21})});
        await h.before();
        assert.notEqual((await h.call('left_click',{target_window_id:21,target_app:'java.exe',
            focus_strategy:'strict',coordinate:[40,80]})).isError,true);
        assert.deepEqual(effects,['left_click']);
        assert.equal(f.ledger().history.reservedActions,2);
        assert.equal(f.ledger().history.sentActions,2);
        assert.equal(f.ledger().pending.status,'AWAITING_AFTER');
    } finally {
        await h?.embedded.server.close();await f?.close();
        names.forEach((name,index)=>{if(prior[index]===undefined)delete process.env[name];else process.env[name]=prior[index];});
    }
});

test('generation E1 sends Generate only after inspecting the original blank-name page and then exhausts input',async()=>{
    const names=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const prior=names.map(name=>process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>>|undefined} */let f;
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];
    try {
        f=await completedSeedLaunch({generation:true});
        const authority={...f,runID:'generation-entry-generate'};
        claimBatchAuthorization(readBatchAuthorization(authority.source,{root:f.root,runID:authority.runID}));
        h=seedInputSession(authority,tool=>effects.push(tool),{window:()=>({...seedTarget,windowId:21})});
        await h.before();
        const args={target_window_id:21,target_app:'java.exe',focus_strategy:'strict',coordinate:[40,80]};
        await h.call('left_click',args);await inspectGenerate(authority,h);
        assert.notEqual((await h.call('left_click',{...args,coordinate:[70,90]})).isError,true);
        assert.deepEqual(effects,['left_click','left_click']);
        assert.equal(f.ledger().history.reservedActions,3);assert.equal(f.ledger().history.sentActions,3);
        assert.match(f.ledger().resolvedActions[0].generateInspectionSha256,/^[a-f0-9]{64}$/);
        await h.before();const active=h;
        await assert.rejects(()=>active.call('left_click',args),/remaining sequence/);
        assert.deepEqual(effects,['left_click','left_click']);
    } finally {
        await h?.embedded.server.close();await f?.close();
        names.forEach((name,index)=>{if(prior[index]===undefined)delete process.env[name];else process.env[name]=prior[index];});
    }
});
