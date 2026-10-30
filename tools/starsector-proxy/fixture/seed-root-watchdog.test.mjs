// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher} from './campaign-input-test-support.mjs';

/** Generated paired authority only; no native process or clock is issued.
 * @param {number} watchdog */
function creationWithWatchdog(watchdog) {return withWatchdog(seedCreationFixture(),watchdog);}
/** @param {any} f @param {number} watchdog */
function withWatchdog(f,watchdog) {
    const start=f.campaign.rootClock.startMs;
    const callerCap=start+watchdog-60000,outer=new Date(start+watchdog).toISOString();
    f.plan.bounds.ownedGameHardTimeoutMs=watchdog;
    for(const v of [f.plan,f.phasePlan])Object.assign(v,{sessionWatchdogMs:watchdog,callerAdmissionDeadlineMs:callerCap});
    for(const v of [f.launchLease,f.phaseLease])Object.assign(v,{sessionWatchdogMs:watchdog,
        ownedGameHardTimeoutMs:watchdog,callerAdmissionDeadlineMs:callerCap});
    Object.assign(f.controller,{sessionWatchdogMs:watchdog,hardTimeoutMs:watchdog,
        conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:outer});
    Object.assign(f.campaign.rootClock,{sessionWatchdogMs:watchdog,deadlineMs:start+watchdog,
        callerAdmissionDeadlineMs:callerCap,originalCallerAdmissionDeadlineMs:callerCap,effectiveDeadlineMs:callerCap});
    Object.assign(f.campaign.producerWitness,{sessionWatchdogMs:watchdog,conservativeOuterDeadlineUtc:outer,
        controllerDeadlineUtc:outer,effectiveDeadlineMs:callerCap});
    f.refreshLaunch();return f;
}

test('explicit twelve-minute seed creation retains paired original clocks through both production launch gates',()=>{
    const f=creationWithWatchdog(720000);
    try {
        const next=f.launcher();let admitted,error;
        try{admitted=readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
        const ps=f.admit();
        assert.deepEqual({powershell:ps.status,javascript:!!admitted},{powershell:0,javascript:true},
            ps.stderr+'\n'+String(error));
        assert.ok(admitted&&'campaign' in admitted.binding);
        assert.equal(admitted.binding.campaign.rootClock.sessionWatchdogMs,720000);
        assert.equal(admitted.binding.campaign.rootClock.effectiveDeadlineMs,
            f.campaign.rootClock.startMs+660000);
        assert.equal(admitted.binding.campaign.rootClock.cleanupReserveMs,30000);
        assert.equal(admitted.binding.campaign.maxReservedActions,80);
    } finally {f.close();}
});

/** @param {any} f */
function assertRefused(f) {
    f.refreshLaunch();const next=f.launcher();
    assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
    const ps=f.admit();assert.notEqual(ps.status,0,ps.stdout);
}

for(const watchdog of [600000,720001])test('both gates refuse an unrecognized original watchdog '+watchdog,()=>{
    const f=creationWithWatchdog(watchdog);try{assertRefused(f);}finally{f.close();}
});

/** @type {[string,(f:any)=>void][]} */
const mutations=[
    ['string original watchdog',f=>{f.plan.sessionWatchdogMs='720000';}],
    ['unpaired lease watchdog',f=>{f.launchLease.sessionWatchdogMs=480000;}],
    ['missing peer watchdog',f=>{delete f.launchLease.sessionWatchdogMs;}],
    ['unpaired owned-game bound',f=>{f.plan.bounds.ownedGameHardTimeoutMs=480000;}],
    ['twelve minutes on a first-page-only profile',f=>{
        f.plan.campaignControl=f.launchLease.campaignControl={schemaVersion:2,profile:'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE'};
    }],
];
for(const [label,change] of mutations)test('both gates refuse '+label,()=>{
    const f=creationWithWatchdog(720000);try{change(f);assertRefused(f);}finally{f.close();}
});

/** Controller/predecessor records exist after PowerShell's prelaunch gate.
 * @type {[string,(f:any)=>void][]} */
const postlaunchChanges=[
    ['renewed root deadline',f=>{f.campaign.rootClock.deadlineMs++;}],
    ['renewed caller admission',f=>{f.campaign.rootClock.originalCallerAdmissionDeadlineMs++;}],
    ['controller beyond original watchdog',f=>{f.controller.conservativeOuterDeadlineUtc=
        new Date(f.campaign.rootClock.startMs+720101).toISOString();}],
    ['controller hard timeout different from the declared new watchdog',f=>{f.controller.hardTimeoutMs=720001;}],
];
for(const [label,change] of postlaunchChanges)test('postlaunch admission refuses '+label,()=>{
    const f=creationWithWatchdog(720000);
    try{
        change(f);f.refreshLaunch();const next=f.launcher();
        assert.equal(f.admit().status,0,'the earlier prelaunch authority is unchanged');
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
    }finally{f.close();}
});

/** These phase schemas and flow declarations are coherent before the unsupported budget.
 * @type {[string,Parameters<typeof campaignLaunchFixture>[0]][]} */
const otherProfiles=[['copied disposable seed',{}],['first-page inspection',{inspection:true}],['generated-name inspection',{generation:true}]];
for(const watchdog of [720000,1800000])for(const [label,options] of otherProfiles)test('both gates keep the original budget for '+label+' at '+watchdog,()=>{
    const f=withWatchdog(campaignLaunchFixture(options),watchdog);
    try {
        const next=claimSeededLauncher(f,'unsupported-watchdog',false);
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
        const ps=f.admit();assert.notEqual(ps.status,0,ps.stdout);
    }finally{f.close();}
});
