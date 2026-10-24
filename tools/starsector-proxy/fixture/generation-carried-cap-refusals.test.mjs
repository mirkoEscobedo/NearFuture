// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher} from './campaign-input-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {json} from './seed-test-support.mjs';

/** @type {Array<[string,(f:any)=>void]>} */
const cases=[
    ['missing original cap',f=>{delete f.plan.absoluteMaxReservedActions;}],
    ['missing original lease cap',f=>{delete f.launchLease.absoluteMaxReservedActions;}],
    ['missing carried-history reference',f=>{delete f.plan.historyAdmission;}],
    ['changed history receipt',f=>{json(f.plan.historyAdmission.path,{maxReservedActions:76});}],
    ['self-consistent76 against admitted72',f=>{
        f.plan.absoluteMaxReservedActions=f.launchLease.absoluteMaxReservedActions=76;
        f.phasePlan.absoluteMaxReservedActions=f.phaseLease.absoluteMaxReservedActions=76;
        f.campaign.maxReservedActions=f.predecessor.maxReservedActions=76;
    }],
    ['lower than already reserved',f=>{
        f.plan.absoluteMaxReservedActions=f.launchLease.absoluteMaxReservedActions=67;
        f.campaign.maxReservedActions=f.predecessor.maxReservedActions=67;
    }],
    ['phase cap changed',f=>{f.phasePlan.absoluteMaxReservedActions=f.phaseLease.absoluteMaxReservedActions=76;}],
    ['history baseline changed',f=>{
        const path=f.plan.historyAdmission.path,receipt=JSON.parse(readFileSync(path,'utf8'));
        receipt.historyBaseline.reservedActions=64;const pin=json(path,receipt);
        f.plan.historyAdmission.sha256=pin.sha256.toLowerCase();
    }],
];
for(const [label,change] of cases)test('carried generation cap refuses '+label+' before any provider claim',()=>{
    const f=campaignLaunchFixture({generation:true,absoluteMaxReservedActions:72,
        historyBaseline:{overallAttempts:39,uiAttempts:36,reservedActions:68,sentActions:66}});
    try {
        change(f);f.refreshLaunch();
        const b1=claimSeededLauncher(f,'cap-refusal',false);
        assert.throws(()=>readBatchAuthorization(b1.source,{root:f.root,runID:b1.runID}),/action|history/);
        assert.equal(existsSync(resolve(f.scenario,'guard-B1-consumed.private.json')),false);
    } finally {f.close();}
});
