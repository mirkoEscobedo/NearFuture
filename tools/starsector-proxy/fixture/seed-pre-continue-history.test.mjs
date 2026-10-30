// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {preContinueHistoryFixture} from './seed-pre-continue-history-test-support.mjs';

test('known Generate with unknown E1 cleanup carries three spent requests through both public history gates',()=>{
    const f=preContinueHistoryFixture();
    try{
        let js;try{js=seedCreationActionLimit(f.plan,f.launchLease,f.scenario);}catch(error){js=/** @type {Error} */(error).stack;}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({js,ps:ps.status,stderr:ps.status===0?'':ps.stderr},{js:103,ps:0,stderr:''});
        assert.deepEqual(f.plan.historyBaseline,{overallAttempts:54,uiAttempts:51,reservedActions:94,sentActions:92});
    }finally{f.close();}
});
