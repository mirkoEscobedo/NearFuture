// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {providerBudgetFixture} from './seed-provider-budget-test-support.mjs';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

for(const [name,change] of /** @type {[string,(f:any)=>void][]} */([
    ['one-sided selector',f=>{delete f.launchLease.providerTiming;}],
    ['null selector',f=>{f.plan.providerTiming=f.launchLease.providerTiming=null;}],
    ['array selector',f=>{f.plan.providerTiming=f.launchLease.providerTiming=[f.plan.providerTiming];}],
    ['array profile',f=>{f.plan.providerTiming.profile=['INITIAL_FOCUS_PROVIDER_480000'];}],
    ['string interval',f=>{f.plan.providerTiming.providerTimeoutMs='480000';}],
    ['boolean interval',f=>{f.plan.providerTiming.providerTimeoutMs=true;}],
    ['unbounded interval',f=>{f.plan.providerTiming.providerTimeoutMs=600000;}],
    ['unknown selector field',f=>{f.plan.providerTiming.extra=true;}],
    ['missing lease baseline',f=>{delete f.launchLease.historyBaseline;}],
    ['different lease baseline',f=>{f.launchLease.historyBaseline={...f.plan.historyBaseline,sentActions:88};}],
    ['legacy control',f=>{for(const v of [f.plan,f.launchLease])v.campaignControl={schemaVersion:4,profile:'DISPOSABLE_SEED_CREATION'};}],
    ['shorter original root',f=>{f.plan.sessionWatchdogMs=f.launchLease.sessionWatchdogMs=720000;}],
    ['long interval without selection',f=>{for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])delete v.providerTiming;}],
]))test('both real launch readers refuse '+name,()=>{
    const f=providerBudgetFixture();
    try{
        change(f);f.refreshLaunch();
        let limitAccepted=false;try{seedCreationActionLimit(f.plan,f.launchLease,f.scenario);limitAccepted=true;}catch{}
        const b=f.launcher();let phaseAccepted=false;
        try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});phaseAccepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.equal(ps.status,1,ps.stderr);assert.match(ps.stderr,/seed|timing|bounded|profile|integer/i);
        assert.deepEqual({limitAccepted,phaseAccepted},{limitAccepted:false,phaseAccepted:false});
    }finally{f.close();}
});

test('a coherently repinned phase cannot upgrade the immutable legacy root provider budget',()=>{
    const f=initialFocusFixture();
    try{
        for(const v of [f.phasePlan,f.phaseLease])v.providerTiming={schemaVersion:1,profile:'INITIAL_FOCUS_PROVIDER_480000',providerTimeoutMs:480000};
        f.phasePlan.bounds={...f.phasePlan.bounds,proxyTimeoutMs:480000};f.refreshLaunch();
        assert.equal(seedCreationActionLimit(f.plan,f.launchLease,f.scenario),100);
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,0,ps.stderr);
        const b=f.launcher();assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}),/immutable original provider timing/);
    }finally{f.close();}
});
