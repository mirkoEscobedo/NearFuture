// @ts-check
// Generated authority and OS-boundary data only; no actual seed or game provenance.
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';

export const creationFlow='DISPOSABLE_SEED_CREATION';
/** @param {string} path */
const reference=path=>({path,sha256:filePin(path).sha256.toLowerCase()});

/** @param {{watchdogMs?:number,callerBudgetMs?:number}} [options] */
export function seedCreationFixture(options={}) {
    const historyBaseline={overallAttempts:40,uiAttempts:37,reservedActions:72,sentActions:70};
    const f=/** @type {any} */ (campaignLaunchFixture({generation:true,absoluteMaxReservedActions:80,historyBaseline}));
    const priorGuard={overallAttempts:39,uiAttempts:36,reservedActions:72,sentActions:70};
    const priorLedgerPath=resolve(f.scenario,'previous-ledger.private.json');
    json(priorLedgerPath,{schemaVersion:2,kind:'STARSECTOR_FINITE_LAUNCH_HISTORY',history:priorGuard,uncertain:false});
    const priorResultPath=resolve(f.scenario,'previous-result.private.json');
    json(priorResultPath,{schemaVersion:1,kind:'ACTUAL_SCHEMA5_COMMON_PREFERENCE_PREFIX_DRIVER_CLOSEOUT',
        allOwnedHandlesTerminalReaped:true,actions:{actualGuard:priorGuard,
            currentRunContext:{overallAttempts:40,uiAttempts:37},absoluteMaxReservedActions:72}});
    const grantPath=f.plan.historyAdmission.path;
    json(grantPath,{schemaVersion:2,kind:'ROOT_ADMITTED_SEED_CREATION_HISTORY',issuer:'/root',
        driver:'/root/recovery_game_driver',historyBaseline,maxReservedActions:80,
        previous:{result:reference(priorResultPath),ledger:reference(priorLedgerPath)}});
    for(const value of [f.plan,f.launchLease,f.phasePlan,f.phaseLease]) {
        value.historyAdmission=reference(grantPath);value.absoluteMaxReservedActions=80;
    }
    const control={schemaVersion:4,profile:creationFlow};
    f.plan.campaignControl=f.launchLease.campaignControl=control;
    f.plan.source={qualifiedHead:'1'.repeat(40)};
    f.campaign.rootCallRecord.value.args.workdir=f.root;
    const preferencePath=resolve(f.scenario,'seed-creation-preference.private.json');
    json(preferencePath,{schemaVersion:1,kind:'NATIVE_SEED_SHARED_PREFERENCE_V1',view:'Registry64',node:'/com/fs/starfarer',
        valueName:'canSkipTutorial',registryValueName:'can/Skip/Tutorial',valueKind:'String',value:'true'});
    const seedCreation={mode:'CREATE_ONE_PRIVATE_DISPOSABLE_SEED',preference:reference(preferencePath)};
    for(const value of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])value.seedCreation=seedCreation;
    f.campaign.flow=creationFlow;f.phaseLease.schemaVersion=6;f.phaseLease.phase=creationFlow;
    if(options.watchdogMs!==undefined){
        const watchdog=options.watchdogMs,start=f.campaign.rootClock.startMs;
        const cap=Date.now()+(options.callerBudgetMs??watchdog-60000),outer=new Date(start+watchdog).toISOString();
        f.plan.bounds.ownedGameHardTimeoutMs=watchdog;
        for(const value of [f.plan,f.phasePlan])Object.assign(value,{sessionWatchdogMs:watchdog,callerAdmissionDeadlineMs:cap});
        for(const value of [f.launchLease,f.phaseLease])Object.assign(value,{sessionWatchdogMs:watchdog,
            ownedGameHardTimeoutMs:watchdog,callerAdmissionDeadlineMs:cap});
        Object.assign(f.controller,{sessionWatchdogMs:watchdog,hardTimeoutMs:watchdog,
            conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:outer});
        Object.assign(f.campaign.rootClock,{sessionWatchdogMs:watchdog,deadlineMs:start+watchdog,
            callerAdmissionDeadlineMs:cap,originalCallerAdmissionDeadlineMs:cap,effectiveDeadlineMs:cap});
        Object.assign(f.campaign.producerWitness,{sessionWatchdogMs:watchdog,conservativeOuterDeadlineUtc:outer,
            controllerDeadlineUtc:outer,effectiveDeadlineMs:cap});
    }
    f.refreshLaunch();
    function launcher(runID='seed-creation-B1') {
        const planPath=resolve(f.scenario,'B1-plan.private.json'),leasePath=resolve(f.scenario,'B1-lease.private.json');
        json(planPath,{...f.phasePlan,batches:[{id:'B1'}]});
        json(leasePath,{...f.phaseLease,planSha256:reference(planPath).sha256});
        const ownedReceiptPath=resolve(f.scenario,'driver-B1-owned.private.json');
        json(ownedReceiptPath,JSON.parse(readFileSync(resolve(f.scenario,'driver-E1-owned.private.json'),'utf8')));
        const source=resolve(f.scenario,'B1-authorization.private.json');
        json(source,{schemaVersion:6,scenario:f.scenario,batch:'B1',planPath,planSha256:reference(planPath).sha256,
            leasePath,leaseSha256:reference(leasePath).sha256,ownedReceiptPath,ownedReceiptSha256:reference(ownedReceiptPath).sha256});
        return {...f,source,runID};
    }
    return {...f,launcher,priorResultPath,priorLedgerPath,grantPath,preferencePath};
}
