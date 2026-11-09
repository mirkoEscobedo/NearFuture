// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {readFileSync} from 'node:fs';
import {json,filePin} from './seed-test-support.mjs';

for(const[name,mutate]of /** @type {[string,(f:any)=>void][]} */([
    ['unpaired control',f=>{f.launchLease.campaignControl={schemaVersion:4,profile:'DISPOSABLE_SEED_CREATION'};}],
    ['ten requests',f=>{f.plan.bounds.maxScenarioActionsIncludingActivations=10;f.launchLease.maxActions=10;}],
    ['two activations',f=>{f.plan.bounds.optionalActivations=2;}],
    ['extra wizard click',f=>{f.plan.bounds.plannedMutationsWithoutActivation=9;}],
    ['extra provider calls',f=>{f.plan.bounds.maxCallsPerBatch=33;}],
    ['shorter root profile',f=>{f.plan.sessionWatchdogMs=f.launchLease.sessionWatchdogMs=720000;}],
    ['array profile',f=>{f.plan.campaignControl={schemaVersion:5,profile:['DISPOSABLE_SEED_CREATION_INITIAL_FOCUS']};}],
    ['old eight-request ceiling',f=>{const g=JSON.parse(readFileSync(f.grantPath,'utf8'));g.maxReservedActions=99;json(f.grantPath,g);
        for(const v of [f.plan,f.launchLease]){v.absoluteMaxReservedActions=99;v.historyAdmission={path:f.grantPath,sha256:filePin(f.grantPath).sha256.toLowerCase()};}}],
]))test('both launch gates refuse initial focus '+name,()=>{
    const f=initialFocusFixture();
    try{
        mutate(f);f.refreshLaunch();let accepted=false;try{seedCreationActionLimit(f.plan,f.launchLease,f.scenario);accepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,1,ps.stderr);
        assert.match(ps.stderr,/campaign|seed|control|fixed|bounded|integer/i);assert.equal(accepted,false);
    }finally{f.close();}
});

test('legacy control4 cannot inherit a ninth request',()=>{
    const f=seedCreationFixture();
    try{
        const grant=JSON.parse(readFileSync(f.grantPath,'utf8'));grant.maxReservedActions=81;json(f.grantPath,grant);
        f.plan.historyAdmission=f.launchLease.historyAdmission={path:f.grantPath,sha256:filePin(f.grantPath).sha256.toLowerCase()};
        f.plan.absoluteMaxReservedActions=f.launchLease.absoluteMaxReservedActions=81;f.refreshLaunch();
        let accepted=false;try{seedCreationActionLimit(f.plan,f.launchLease,f.scenario);accepted=true;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,1,ps.stderr);assert.equal(accepted,false);
    }finally{f.close();}
});
