// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';

for(const fresh of [false,true])test('natural focus acquisition '+(fresh?'uses a fresh focused original':'refuses the earlier unfocused original'),async()=>{
    const f=initialFocusFixture({inProcessProvider:true}),prior=process.env.COMPUTER_USE_REQUIRE_APPROVAL;process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];let focused=false;
    try{
        const b=f.launcher();claimBatchAuthorization(readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
        h=seedInputSession(b,t=>effects.push(t),{window:()=>({...seedTarget,isFocused:focused})});
        await h.before();focused=true;await h.call('get_frontmost_app',{});await h.call('get_window',{window_id:19});
        if(fresh){await h.call('screenshot',{target_window_id:19,quality:0,width:480});await h.call('get_frontmost_app',{});await h.call('get_window',{window_id:19});}
        const client=h,play=()=>client.call('left_click',{target_window_id:19,target_app:'java.exe',focus_strategy:'strict',coordinate:[40,80]});
        if(fresh){assert.notEqual((await play()).isError,true);assert.deepEqual(effects,['left_click']);}
        else{await assert.rejects(play,/focused original/);assert.deepEqual(effects,[]);}
        const l=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
        assert.equal(l.history.reservedActions,92);assert.equal(l.history.sentActions,fresh?90:89);assert.equal(l.uncertain,!fresh);
        assert.equal(l.resolvedInitialActivation,undefined);
    }finally{await h?.embedded.server.close();f.close();if(prior===undefined)delete process.env.COMPUTER_USE_REQUIRE_APPROVAL;else process.env.COMPUTER_USE_REQUIRE_APPROVAL=prior;}
});

test('legacy control4/auth6 still refuses any activation before a native effect',async()=>{
    const f=seedCreationFixture(),prior=process.env.COMPUTER_USE_REQUIRE_APPROVAL;process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];
    try{
        const b=f.launcher();claimBatchAuthorization(readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
        h=seedInputSession(b,t=>effects.push(t));await h.before();
        const client=h;await assert.rejects(()=>client.call('activate_window',{window_id:19,timeout_ms:2000}));
        assert.deepEqual(effects,[]);
        const l=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
        assert.equal(l.history.reservedActions,72);assert.equal(l.history.sentActions,70);assert.equal(l.uncertain,true);
    }finally{await h?.embedded.server.close();f.close();if(prior===undefined)delete process.env.COMPUTER_USE_REQUIRE_APPROVAL;else process.env.COMPUTER_USE_REQUIRE_APPROVAL=prior;}
});
