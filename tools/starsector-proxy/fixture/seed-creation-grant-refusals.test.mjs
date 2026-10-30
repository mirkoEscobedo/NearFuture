// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @param {any} f */
function repin(f) {
    for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])
        v.historyAdmission={path:f.grantPath,sha256:filePin(f.grantPath).sha256.toLowerCase()};
    f.refreshLaunch();
}

test('duplicate seed-creation grant fields refuse despite refreshed authority digests',()=>{
    const f=seedCreationFixture();
    try {
        const text=readFileSync(f.grantPath,'utf8');writeFileSync(f.grantPath,'{"schemaVersion":2,'+text.slice(1));repin(f);
        const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/duplicate|canonical/i);
        const b1=f.launcher();assert.throws(()=>readBatchAuthorization(b1.source,{root:f.root,runID:b1.runID}),/duplicate|canonical/i);
        assert.equal(existsSync(resolve(f.scenario,'guard-B1-consumed.private.json')),false);
    } finally {f.close();}
});

for(const [name,change] of /** @type {[string,(f:any)=>void][]} */ ([
    ['ninth action',f=>{for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.absoluteMaxReservedActions=81;
        const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));grant.maxReservedActions=81;json(f.grantPath,grant);}],
    ['prior terminal history changed',f=>{const prior=JSON.parse(readFileSync(f.priorResultPath,'utf8'));
        prior.allOwnedHandlesTerminalReaped=false;json(f.priorResultPath,prior);
        const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));grant.previous.result.sha256=filePin(f.priorResultPath).sha256.toLowerCase();json(f.grantPath,grant);}],
    ['counter refund',f=>{const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));grant.historyBaseline.sentActions=0;
        for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyBaseline={...grant.historyBaseline};json(f.grantPath,grant);}]
]))test('seed authority refuses '+name+' in both real validators',()=>{
    const f=seedCreationFixture();
    try {
        change(f);repin(f);
        assert.notEqual(f.admit().status,0);
        const b1=f.launcher();assert.throws(()=>readBatchAuthorization(b1.source,{root:f.root,runID:b1.runID}));
        assert.equal(existsSync(resolve(f.scenario,'guard-B1-consumed.private.json')),false);
    }finally{f.close();}
});
