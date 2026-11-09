// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {exerciseSeedCreationTransport} from './seed-creation-transport-support.mjs';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {readCampaignAuthorization} from '../campaign-authority.mjs';

test('paired seed emergency watchdog admits call and cleanup time through both production launch gates',()=>{
    const f=seedCreationFixture({watchdogMs:1800000,callerBudgetMs:70000});
    try{
        const next=f.launcher();let admitted,error;
        try{admitted=readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
        const ps=f.admit();
        assert.deepEqual({powershell:ps.status,javascript:!!admitted},{powershell:0,javascript:true},
            ps.stderr+'\n'+String(error));
        assert.ok(admitted&&'campaign' in admitted.binding);
        assert.equal(admitted.binding.campaign.rootClock.sessionWatchdogMs,1800000);
        assert.equal(admitted.binding.campaign.rootClock.cleanupReserveMs,30000);
        assert.equal(admitted.binding.campaign.maxReservedActions,80);
    }finally{f.close();}
});

test('seed emergency input retains the exact per-call floor and cleanup reserve',()=>{
    const f=seedCreationFixture({watchdogMs:1800000,callerBudgetMs:50000});
    try{
        const next=f.launcher(),now=f.campaign.rootClock.effectiveDeadlineMs-50000;
        assert.doesNotThrow(()=>readCampaignAuthorization(next.source,{root:f.root,runID:next.runID,now,input:true}));
        assert.throws(()=>readCampaignAuthorization(next.source,{root:f.root,runID:next.runID,now:now+1,input:true}),
            /original effective deadline/i);
    }finally{f.close();}
});

test('both seed emergency launch gates refuse less than one call plus cleanup',()=>{
    const f=seedCreationFixture({watchdogMs:1800000,callerBudgetMs:40000});
    try{
        const next=f.launcher();
        assert.notEqual(f.admit().status,0);
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}),/original effective deadline/i);
    }finally{f.close();}
});

for(const watchdogMs of [600000,1800001])test('both emergency gates refuse the unrecognized watchdog '+watchdogMs,()=>{
    const f=seedCreationFixture({watchdogMs,callerBudgetMs:170000});
    try{
        const next=f.launcher();
        assert.notEqual(f.admit().status,0);
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
    }finally{f.close();}
});

/** @type {[string,(f:any)=>void][]} */
const mutations=[
    ['string declaration',f=>{f.plan.sessionWatchdogMs='1800000';f.launchLease.sessionWatchdogMs='1800000';}],
    ['boolean declaration',f=>{f.plan.sessionWatchdogMs=true;f.launchLease.sessionWatchdogMs=true;}],
    ['missing peer declaration',f=>{delete f.launchLease.sessionWatchdogMs;}],
    ['unpaired lease',f=>{f.launchLease.sessionWatchdogMs=720000;}],
    ['changed owned-game bound',f=>{f.plan.bounds.ownedGameHardTimeoutMs=720000;}],
    ['ninth action',f=>{f.plan.bounds.maxScenarioActionsIncludingActivations=9;f.launchLease.maxActions=9;}],
    ['fourth provider',f=>{f.plan.bounds.maxProxyInvocations=4;f.launchLease.maxProxyInvocations=4;}],
];
for(const [label,change] of mutations)test('both emergency gates refuse '+label,()=>{
    const f=seedCreationFixture({watchdogMs:1800000,callerBudgetMs:170000});
    try{
        change(f);f.refreshLaunch();const next=f.launcher();
        assert.notEqual(f.admit().status,0);
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
    }finally{f.close();}
});

test('seed emergency policy reaches the enabled Start game with less than the legacy two-minute input reserve',async()=>{
    const result=await exerciseSeedCreationTransport({retain:true,watchdogMs:1800000,callerBudgetMs:149999});
    assert.ok(result);
    try{
        const f=result.fixture,state=JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8'));
        assert.equal(state.pending.wizardStep.stage,'START_GAME');
        const inputSlack=f.campaign.rootClock.effectiveDeadlineMs-30000-state.pending.sentAtMs;
        assert.ok(inputSlack<120000&&inputSlack>=20000,'original input slack at Start: '+inputSlack);
        assert.deepEqual(state.history,{overallAttempts:40,uiAttempts:37,reservedActions:80,sentActions:78});
        assert.equal(f.campaign.rootClock.sessionWatchdogMs,1800000);
    }finally{result.close();}
});
