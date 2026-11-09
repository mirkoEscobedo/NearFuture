// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';

test('initially unfocused new profile sends exactly one acknowledged activation and retains its reservation',async()=>{
    const f=initialFocusFixture({inProcessProvider:true}),prior=process.env.COMPUTER_USE_REQUIRE_APPROVAL;
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];let focused=false;
    try{
        const b=f.launcher();claimBatchAuthorization(readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
        h=seedInputSession(b,tool=>{effects.push(tool);focused=true;},{window:()=>({...seedTarget,isFocused:focused})});
        await h.before();
        const result=await h.call('activate_window',{window_id:19,timeout_ms:2000});
        assert.notEqual(result.isError,true,JSON.stringify(result));assert.deepEqual(effects,['activate_window']);
        const state=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
        assert.deepEqual(state.history,{overallAttempts:53,uiAttempts:50,reservedActions:92,sentActions:90});
        assert.equal(state.pending.tool,'activate_window');assert.equal(state.pending.status,'AWAITING_AFTER');
        assert.equal(state.resolvedActions?.length??0,0);assert.equal(state.uncertain,false);
    }finally{await h?.embedded.server.close();f.close();if(prior===undefined)delete process.env.COMPUTER_USE_REQUIRE_APPROVAL;else process.env.COMPUTER_USE_REQUIRE_APPROVAL=prior;}
});

for(const mode of ['focused-click','focused-activation','missing-fresh-pair','second-activation','later-focus-loss','unresolved-play','post-play-focus-loss','unknown-activation','wrong-activation-window'])test('initial focus input: '+mode,async()=>{
    const f=initialFocusFixture({inProcessProvider:true}),prior=process.env.COMPUTER_USE_REQUIRE_APPROVAL;
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';
    /** @type {ReturnType<typeof seedInputSession>|undefined} */let h;
    /** @type {string[]} */const effects=[];let focused=mode.startsWith('focused')||mode==='post-play-focus-loss';
    try{
        const b=f.launcher();claimBatchAuthorization(readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
        h=seedInputSession(b,tool=>{effects.push(tool);focused=true;},{window:()=>({...seedTarget,isFocused:focused}),activationResult:r=>
            mode==='unknown-activation'?{...r,isError:true}:mode==='wrong-activation-window'?
                {content:[{type:'text',text:JSON.stringify({activated:true,windowId:999})}]}:r});
        if(mode==='missing-fresh-pair'){
            await h.call('doctor',{});await h.call('policy_status',{});await h.call('get_window',{window_id:19});
            await h.call('screenshot',{target_window_id:19,quality:0,width:480});
        }else await h.before();
        const client=h;const activate=()=>client.call('activate_window',{window_id:19,timeout_ms:2000});
        if(mode==='focused-click'||mode==='post-play-focus-loss'){
            assert.notEqual((await h.call('left_click',{target_window_id:19,target_app:'java.exe',focus_strategy:'strict',coordinate:[40,80]})).isError,true);
            assert.deepEqual(effects,['left_click']);
            if(mode==='post-play-focus-loss'){focused=false;await assert.rejects(()=>client.call('get_window',{window_id:19}));}
        }else if(mode==='focused-activation'||mode==='missing-fresh-pair'){
            await assert.rejects(activate);assert.deepEqual(effects,[]);
        }else{
            await activate();assert.deepEqual(effects,['activate_window']);
            if(mode==='unknown-activation'||mode==='wrong-activation-window'){
                await assert.rejects(()=>client.call('left_click',{target_window_id:19,target_app:'java.exe',focus_strategy:'strict',coordinate:[40,80]}));
            }else{
            await h.before();
            if(mode==='unresolved-play')await assert.rejects(()=>client.call('left_click',{target_window_id:19,target_app:'java.exe',focus_strategy:'strict',coordinate:[40,80]}));
            else if(mode==='later-focus-loss'){focused=false;await assert.rejects(()=>client.call('get_window',{window_id:19}));}
            else await assert.rejects(activate);
            }
            assert.deepEqual(effects,['activate_window']);
        }
        const state=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
        if(mode==='focused-click'){assert.equal(state.pending.wizardStep.ordinal,1);assert.equal(state.history.reservedActions,92);assert.equal(state.resolvedInitialActivation,undefined);}
        else assert.equal(state.uncertain,true);
    }finally{await h?.embedded.server.close();f.close();if(prior===undefined)delete process.env.COMPUTER_USE_REQUIRE_APPROVAL;else process.env.COMPUTER_USE_REQUIRE_APPROVAL=prior;}
});
