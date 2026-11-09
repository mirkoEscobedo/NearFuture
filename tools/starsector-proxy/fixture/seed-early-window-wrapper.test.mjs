// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {earlyWindowFixture} from './seed-early-window-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {json,filePin} from './seed-test-support.mjs';

test('outer previous-result duplicate keys refuse through both real history gates after complete repinning',()=>{
    const f=earlyWindowFixture();
    try{
        const text=readFileSync(f.closeoutPath,'utf8').replace('"kind":','"kind":"ROOT_CLOSED_EARLY_WINDOW_FOCUS_STOP_HISTORY","kind":');
        writeFileSync(f.closeoutPath,text);
        const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));grant.previous.result.sha256=filePin(f.closeoutPath).sha256.toLowerCase();json(f.grantPath,grant);
        for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission={path:f.grantPath,sha256:filePin(f.grantPath).sha256.toLowerCase()};
        f.refreshLaunch();let jsAccepted=false;try{seedCreationActionLimit(f.plan,f.launchLease,f.scenario);jsAccepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsAccepted,psStatus:ps.status},{jsAccepted:false,psStatus:1},ps.stderr);
    }finally{f.close();}
});
