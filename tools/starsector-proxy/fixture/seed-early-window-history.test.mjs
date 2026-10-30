// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {earlyWindowFixture} from './seed-early-window-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';

test('closed three-call early window focus stop carries spent context without invented screenshot or frontmost call through both real gates',()=>{
    const f=earlyWindowFixture();
    try{
        let js;try{js=seedCreationActionLimit(f.plan,f.launchLease,f.scenario);}catch(error){js=error;}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({js,psStatus:ps.status,psError:ps.status===0?'':ps.stderr},{js:99,psStatus:0,psError:''});
        assert.deepEqual(f.closeout.actions.actualGuard,{overallAttempts:52,uiAttempts:49,reservedActions:91,sentActions:89});
        assert.deepEqual(f.closeout.actions.currentRunContext,{overallAttempts:53,uiAttempts:50});
        assert.deepEqual(f.result.callOrder,['doctor','policy_status','get_window']);
        assert.equal(f.result.counts.originalPNGs,0);
    }finally{f.close();}
});
