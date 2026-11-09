// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {seedInputSession} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';

test('seed creation sends one guarded Play with the previous72 reservations retained',async()=>{
    const f=seedCreationFixture();
    const names=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const prior=names.map(n=>process.env[n]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];
    try {
        const first=f.launcher();claimBatchAuthorization(readBatchAuthorization(first.source,{root:f.root,runID:first.runID}));
        h=seedInputSession(first,tool=>effects.push(tool));await h.before();
        assert.notEqual((await h.call('left_click',{target_window_id:19,target_app:'java.exe',focus_strategy:'strict',coordinate:[40,80]})).isError,true);
        assert.deepEqual(effects,['left_click']);
        const state=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
        assert.deepEqual(state.history,{overallAttempts:40,uiAttempts:37,reservedActions:73,sentActions:71});
        assert.equal(state.uncertain,false);
    } finally {
        await h?.embedded.server.close();f.close();
        names.forEach((n,i)=>{if(prior[i]===undefined)delete process.env[n];else process.env[n]=prior[i];});
    }
});
