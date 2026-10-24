// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher} from './campaign-input-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {json,filePin} from './seed-test-support.mjs';

/** @type {Array<[string,(f:any)=>void]>} */
const cases=[
    ['legacy first-page control',f=>{Object.assign(f.plan.campaignControl,{schemaVersion:2,profile:'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE'});}],
    ['wrong control version',f=>{f.plan.campaignControl.schemaVersion=2;}],
    ['extra control field',f=>{f.plan.campaignControl.saveAllowed=true;}],
    ['seed authority',f=>{f.plan.seed=f.launchLease.seed={};}],
    ['seed metadata',f=>{json(resolve(f.scenario,'seed-stage-plan.private.json'),{});}],
    ['additional action',f=>{f.plan.bounds.maxScenarioActionsIncludingActivations=f.launchLease.maxActions=9;}],
    ['general save permission',f=>{f.launchLease.campaignLoadSaveAllowed=true;}],
    ['owned node kill authority',f=>{f.plan.ownedNodeKill=f.launchLease.ownedNodeKill={};}],
    ['legacy fixture',f=>{
        const binding=JSON.parse(readFileSync(f.bindingPath,'utf8'));binding.schemaVersion=1;delete binding.fixtureProfile;
        Object.assign(f.bindingPin,json(f.bindingPath,binding));f.plan.fixtureBinding.sha256=f.bindingPin.sha256;
    }],
    ['unknown fixture profile',f=>{
        const binding=JSON.parse(readFileSync(f.bindingPath,'utf8'));binding.fixtureProfile='MODDED_GENERATION';
        Object.assign(f.bindingPin,json(f.bindingPath,binding));f.plan.fixtureBinding.sha256=f.bindingPin.sha256;
    }],
];
for(const [label,change] of cases)test('generation launch refuses '+label+' through real PowerShell and JavaScript admission',()=>{
    const f=campaignLaunchFixture({generation:true});
    try {
        change(f);f.refreshLaunch();
        const launch=f.admit();assert.notEqual(launch.status,0,label);
        const authority=claimSeededLauncher(f,'generation-launch-refusal',false);
        assert.throws(()=>readBatchAuthorization(authority.source,{root:f.root,runID:authority.runID}));
    } finally {f.close();}
});

test('JavaScript generation admission refuses ambiguous duplicate binding keys despite refreshed pins',()=>{
    const f=campaignLaunchFixture({generation:true});
    try {
        const text=readFileSync(f.bindingPath,'utf8').replace('"fixtureProfile":',
            '"fixtureProfile":"VANILLA_UNCOMPRESSED_GENERATION","fixtureProfile":');
        writeFileSync(f.bindingPath,text);Object.assign(f.bindingPin,filePin(f.bindingPath));
        f.plan.fixtureBinding.sha256=f.bindingPin.sha256;f.refreshLaunch();
        const authority=claimSeededLauncher(f,'generation-duplicate-binding',false);
        assert.throws(()=>readBatchAuthorization(authority.source,{root:f.root,runID:authority.runID}),/Duplicate generation binding field/);
    } finally {f.close();}
});
