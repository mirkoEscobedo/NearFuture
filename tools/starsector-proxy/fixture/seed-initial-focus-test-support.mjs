// @ts-check
// Fresh synthetic opt-in authority referencing the independently closed three-call history.
import {resolve,basename,relative,isAbsolute} from 'node:path';
import {mkdirSync,rmSync} from 'node:fs';
import {earlyWindowFixture} from './seed-early-window-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';

export const initialFocusFlow='DISPOSABLE_SEED_CREATION_INITIAL_FOCUS';
/** @param {any} [options] */
export function initialFocusFixture(options={}){
    const f=earlyWindowFixture(),control={schemaVersion:5,profile:initialFocusFlow},baseline=f.plan.historyBaseline;
    const start=f.campaign.rootClock.startMs,cap=start+1740000,outer=new Date(start+1800000).toISOString();
    for(const p of [f.plan,f.phasePlan]){
        p.campaignControl=control;p.sessionWatchdogMs=1800000;p.callerAdmissionDeadlineMs=cap;
        Object.assign(p.bounds,{ownedGameHardTimeoutMs:1800000,maxScenarioActionsIncludingActivations:9,maxCallsPerBatch:32,
            plannedMutationsWithoutActivation:8,maxPlannedMutationsIncludingActivation:9,optionalActivations:1});
    }
    for(const l of [f.launchLease,f.phaseLease])Object.assign(l,{campaignControl:control,sessionWatchdogMs:1800000,ownedGameHardTimeoutMs:1800000,
        callerAdmissionDeadlineMs:cap,maxActions:9,absoluteMaxReservedActions:100});
    f.phaseLease.schemaVersion=7;f.phaseLease.phase=initialFocusFlow;
    Object.assign(f.controller,{sessionWatchdogMs:1800000,hardTimeoutMs:1800000,conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:outer});
    Object.assign(f.campaign.rootClock,{sessionWatchdogMs:1800000,deadlineMs:start+1800000,callerAdmissionDeadlineMs:cap,
        originalCallerAdmissionDeadlineMs:cap,effectiveDeadlineMs:cap});
    Object.assign(f.campaign.producerWitness,{sessionWatchdogMs:1800000,conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:outer,effectiveDeadlineMs:cap});
    f.campaign.flow=initialFocusFlow;f.campaign.maxReservedActions=f.predecessor.maxReservedActions=100;
    function refreshFocus(){
        json(f.grantPath,{schemaVersion:options.legacyGrant?2:3,kind:options.legacyGrant?'ROOT_ADMITTED_SEED_CREATION_HISTORY':'ROOT_ADMITTED_INITIAL_FOCUS_SEED_CREATION_HISTORY',issuer:'/root',driver:'/root/recovery_game_driver',
            historyBaseline:baseline,maxReservedActions:100,previous:{result:ref(f.closeoutPath),ledger:ref(f.ledgerPath)}});
        for(const p of [f.plan,f.phasePlan,f.launchLease,f.phaseLease])Object.assign(p,{absoluteMaxReservedActions:100,historyAdmission:ref(f.grantPath)});
        f.refreshLaunch();
    }
    refreshFocus();
    /** @type {string[]} */const inProcessOutputs=[];
    function launcher(runID='initial-focus-B1-'+basename(f.scenario)){
        const b=f.launcher(runID),auth=value(b.source);auth.schemaVersion=7;json(b.source,auth);
        if(options.inProcessProvider){const output=resolve(f.root,'.tmp/starsector-proxy',runID);mkdirSync(output);inProcessOutputs.push(output);}
        return b;
    }
    return{...f,launcher,refreshFocus,options,close(){
        for(const output of inProcessOutputs){const suffix=relative(resolve(f.root,'.tmp/starsector-proxy'),output);
            if(!suffix||suffix.startsWith('..')||isAbsolute(suffix))throw new Error('Synthetic in-process output must remain contained');
            rmSync(output,{recursive:true});}
        f.close();
    }};
}
