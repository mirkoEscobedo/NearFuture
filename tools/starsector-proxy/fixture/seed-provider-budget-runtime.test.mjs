// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {startOwnedHttpBatch} from '../owned-http-batch.mjs';
import {providerBudgetHost} from './seed-provider-budget-test-support.mjs';
import {existsSync} from 'node:fs';
import {resolve} from 'node:path';

test('selected provider owns one eight-minute ledger interval and verifier budget without renewing the root',async()=>{
    const h=providerBudgetHost();let batch;
    try{
        const root=structuredClone(h.f.campaign.rootClock),before=Date.now();
        batch=await startOwnedHttpBatch(h.provider.options,{spawn:h.provider.spawn});
        const after=Date.now(),state=h.ledger(),args=h.provider.args();
        assert.deepEqual({interval:state.providerClock.deadlineMs-state.providerClock.startMs,
            verifierMs:args[args.indexOf('--timeout')+1]},{interval:480000,verifierMs:'480000'});
        assert.ok(batch.deadline>=before+480000&&batch.deadline<=after+480000);
        assert.deepEqual(h.f.campaign.rootClock,root);
        assert.deepEqual(state.history,h.f.plan.historyBaseline);
        assert.equal(state.pending,null);assert.equal(state.uncertain,false);assert.equal(h.provider.starts(),1);
        const terminal=await batch.close();assert.equal(terminal.verdict,'PASS');assert.equal(terminal.terminalBoundToChild,true);
    }finally{await batch?.close();await h.close();}
});

test('absent selection retains the original three-minute provider timer and verifier',async()=>{
    const h=providerBudgetHost(false);let batch;
    try{
        batch=await startOwnedHttpBatch(h.provider.options,{spawn:h.provider.spawn});
        const state=h.ledger(),args=h.provider.args();
        assert.deepEqual({interval:state.providerClock.deadlineMs-state.providerClock.startMs,
            verifierMs:args[args.indexOf('--timeout')+1]},{interval:180000,verifierMs:'180000'});
        assert.equal(Object.hasOwn(state.provider,'providerTimeoutMs'),false);
    }finally{await batch?.close();await h.close();}
});

test('an opted-in provider refuses insufficient original root room before claim or spawn',async()=>{
    const h=providerBudgetHost();
    try{
        const cap=Date.now()+450000;
        for(const value of [h.f.plan,h.f.launchLease,h.f.phasePlan,h.f.phaseLease])value.callerAdmissionDeadlineMs=cap;
        Object.assign(h.f.campaign.rootClock,{callerAdmissionDeadlineMs:cap,originalCallerAdmissionDeadlineMs:cap,effectiveDeadlineMs:cap});
        h.f.campaign.producerWitness.effectiveDeadlineMs=cap;h.f.refreshLaunch();h.f.launcher(h.provider.options.runID);
        await assert.rejects(startOwnedHttpBatch(h.provider.options,{spawn:h.provider.spawn}),/full original eight-minute provider interval/);
        assert.equal(h.provider.starts(),0);assert.equal(existsSync(resolve(h.f.scenario,'guard-B1-consumed.private.json')),false);
        assert.deepEqual(h.ledger().history,h.f.plan.historyBaseline);
    }finally{await h.close();}
});
