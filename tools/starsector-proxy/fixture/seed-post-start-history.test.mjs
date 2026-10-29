// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {postStartHistoryFixture} from './seed-post-start-history-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';

test('campaign reached and saved with incomplete verification carries all eight ACKs without seed credit through both real history gates',()=>{
    const f=postStartHistoryFixture();
    try{
        let js;try{js=seedCreationActionLimit(f.plan,f.launchLease,f.scenario);}catch(error){js=error;}
        const ps=f.admit();
        assert.deepEqual({js,ps:ps.status,psError:ps.status===0?'':ps.stderr.trim()}, {js:99,ps:0,psError:''});
        assert.deepEqual(f.closeout.actions.actualGuard,{overallAttempts:51,uiAttempts:48,reservedActions:91,sentActions:89});
        assert.deepEqual(f.result.calls,{total:61,B1:11,E1:22,E2:28,originalPngs:11,providersStarted:3,providersPassed:2,providersUnknown:1});
        assert.equal(f.result.seedQualified,false);
    }finally{f.close();}
});
