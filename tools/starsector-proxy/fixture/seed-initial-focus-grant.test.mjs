// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {json,filePin} from './seed-test-support.mjs';

test('new control5 refuses an old v2 creation grant through both real launch gates',()=>{
    const f=initialFocusFixture({legacyGrant:true});
    try{
        const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));assert.equal(grant.schemaVersion,2);
        assert.equal(grant.kind,'ROOT_ADMITTED_SEED_CREATION_HISTORY');
        let jsAccepted=false;try{seedCreationActionLimit(f.plan,f.launchLease,f.scenario);jsAccepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsAccepted,psStatus:ps.status},{jsAccepted:false,psStatus:1},ps.stderr);
    }finally{f.close();}
});

for(const mode of ['duplicate v3 grant','array v3 grant issuer'])test(mode+' refuses with coherently updated authority pins',()=>{
    const f=initialFocusFixture();
    try{
        const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));assert.equal(grant.schemaVersion,3);
        if(mode==='duplicate v3 grant')writeFileSync(f.grantPath,JSON.stringify(grant).replace('"schemaVersion":3','"schemaVersion":3,"schemaVersion":3'));
        else{grant.issuer=['/root'];json(f.grantPath,grant);}
        for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission={path:f.grantPath,sha256:filePin(f.grantPath).sha256.toLowerCase()};
        f.refreshLaunch();const b=f.launcher();let jsAccepted=false;try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});jsAccepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsAccepted,psStatus:ps.status},{jsAccepted:false,psStatus:1},ps.stderr);
    }finally{f.close();}
});

test('legacy root4/auth6 cannot borrow the distinct nine-request grant',()=>{
    const f=seedCreationFixture();
    try{
        const g=JSON.parse(readFileSync(f.grantPath,'utf8'));g.schemaVersion=3;g.kind='ROOT_ADMITTED_INITIAL_FOCUS_SEED_CREATION_HISTORY';g.maxReservedActions=81;json(f.grantPath,g);
        for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]){v.historyAdmission={path:f.grantPath,sha256:filePin(f.grantPath).sha256.toLowerCase()};v.absoluteMaxReservedActions=81;}
        f.campaign.maxReservedActions=f.predecessor.maxReservedActions=81;f.refreshLaunch();const b=f.launcher();
        let jsAccepted=false;try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});jsAccepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,1,ps.stderr);assert.equal(jsAccepted,false);
    }finally{f.close();}
});

test('new root5 grant cannot be downgraded to an old phase6 flow',()=>{
    const f=initialFocusFixture();
    try{
        const b=f.launcher(),a=JSON.parse(readFileSync(b.source,'utf8'));a.schemaVersion=6;json(b.source,a);
        assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}),/campaign|flow|profile/i);
    }finally{f.close();}
});
