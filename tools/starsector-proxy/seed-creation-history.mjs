// @ts-check
import {resolve,isAbsolute} from 'node:path';
import {isDeepStrictEqual} from 'node:util';
import {requireThat,read,closed} from './campaign-proof.mjs';
import {failedSeedAttemptKind,knownPlayFailedSeedAttemptKind,captureStopFailedSeedAttemptKind,wrongSceneFailedSeedAttemptKind,captureLinkageFailedSeedAttemptKind,
 readFailedSeedAttempt,readKnownPlayFailedSeedAttempt,readCaptureStopFailedSeedAttempt,readWrongSceneFailedSeedAttempt,readCaptureLinkageFailedSeedAttempt} from './seed-failed-attempt.mjs';

export const seedCreationFlow='DISPOSABLE_SEED_CREATION';

/** Closed root-issued grants use canonical JSON scalars; insignificant whitespace is allowed.
 * Reject ambiguous duplicate fields before accepting even a freshly repinned grant.
 * @param {string} path */
function readCanonicalGrant(path) {
    const found=read(path),text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(found.bytes);
    const compact=text.replace(/("(?:\\.|[^"\\])*")|\s+/g,(match,string)=>string??'');
    requireThat(compact===JSON.stringify(found.value),'canonical seed authority JSON without duplicate fields required');
    return found;
}

/** @param {any} plan @param {any} lease @param {string} scenario */
export function readSeedCreationPreference(plan,lease,scenario) {
    const spec=plan.seedCreation,pin=spec?.preference;
    requireThat(closed(spec,['mode','preference'])&&spec.mode==='CREATE_ONE_PRIVATE_DISPOSABLE_SEED'&&
        isDeepStrictEqual(spec,lease.seedCreation)&&closed(pin,['path','sha256'])&&
        pin.path===resolve(scenario,'seed-creation-preference.private.json')&&/^[a-f0-9]{64}$/.test(pin.sha256),
    'fixed seed shared preference witness required');
    const witness=readCanonicalGrant(pin.path),expected={schemaVersion:1,kind:'NATIVE_SEED_SHARED_PREFERENCE_V1',view:'Registry64',
        node:'/com/fs/starfarer',valueName:'canSkipTutorial',registryValueName:'can/Skip/Tutorial',valueKind:'String',value:'true'};
    requireThat(witness.sha256===pin.sha256&&isDeepStrictEqual(witness.value,expected),'eligible seed shared preference required');
    return witness.value;
}

/** Explicit new authority extends, rather than erases, the previous actual history.
 * Every reread remains bound to the same grant; no provider can replenish it.
 * @param {any} plan @param {any} lease @param {string} scenario */
export function seedCreationActionLimit(plan,lease,scenario) {
    readSeedCreationPreference(plan,lease,scenario);
    const reference=plan.historyAdmission,limit=plan.absoluteMaxReservedActions,baseline=plan.historyBaseline;
    requireThat(closed(reference,['path','sha256'])&&reference.path===resolve(scenario,'root-history-baseline.admitted.private.json')&&
        /^[a-f0-9]{64}$/.test(reference.sha256)&&isDeepStrictEqual(reference,lease.historyAdmission),
    'fixed seed-creation history admission required');
    const receipt=readCanonicalGrant(reference.path),v=receipt.value;
    requireThat(receipt.sha256===reference.sha256&&closed(v,['schemaVersion','kind','issuer','driver','historyBaseline',
        'maxReservedActions','previous'])&&v.schemaVersion===2&&v.kind==='ROOT_ADMITTED_SEED_CREATION_HISTORY'&&
        v.issuer==='/root'&&v.driver==='/root/recovery_game_driver'&&isDeepStrictEqual(v.historyBaseline,baseline)&&
        closed(baseline,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&
        Object.values(baseline).every(n=>Number.isSafeInteger(n)&&n>=0)&&baseline.sentActions<=baseline.reservedActions&&
        Number.isSafeInteger(limit)&&limit>baseline.reservedActions&&limit<=baseline.reservedActions+8&&
        limit===lease.absoluteMaxReservedActions&&limit===v.maxReservedActions&&closed(v.previous,['result','ledger']),
    'explicit bounded seed-creation extension required');
    /** @param {any} pin */
    function previous(pin) {
        requireThat(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&isAbsolute(pin.path)&&
            /^[a-f0-9]{64}$/.test(pin.sha256),'closed previous seed history reference required');
        const found=read(pin.path);requireThat(found.sha256===pin.sha256,'previous seed history changed');return found.value;
    }
    const result=previous(v.previous.result),ledger=previous(v.previous.ledger);
    if(result.kind==='ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT')requireThat(
        closed(result,['schemaVersion','kind','scenarioId','toolingHead','rootSessionId','allOwnedHandlesTerminalReaped',
            'actions','cleanup','protection','files','evidence'])&&
        closed(result.actions,['inheritedBaseline','actualGuard','currentRunContext','absoluteMaxReservedActions','clicks','activations','providers'])&&
        result.actions.clicks===8&&result.actions.activations===0&&result.actions.providers===3,
    'completed seed history must retain its distinct successful provenance shape');
    const failed=result.kind===failedSeedAttemptKind,known=result.kind===knownPlayFailedSeedAttemptKind,
        capture=result.kind===captureStopFailedSeedAttemptKind,wrong=result.kind===wrongSceneFailedSeedAttemptKind,linkage=result.kind===captureLinkageFailedSeedAttemptKind;
    const reader=linkage?readCaptureLinkageFailedSeedAttempt:wrong?readWrongSceneFailedSeedAttempt:capture?readCaptureStopFailedSeedAttempt:known?readKnownPlayFailedSeedAttempt:readFailedSeedAttempt;
    const actions=failed||known||capture||wrong||linkage?reader(readCanonicalGrant(v.previous.result.path).value,ledger,v.previous.ledger,readSeedCreationPreference):result.actions;
    requireThat(result.schemaVersion===1&&['ACTUAL_SCHEMA5_COMMON_PREFERENCE_PREFIX_DRIVER_CLOSEOUT',
        'ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT',failedSeedAttemptKind,knownPlayFailedSeedAttemptKind,captureStopFailedSeedAttemptKind,wrongSceneFailedSeedAttemptKind,captureLinkageFailedSeedAttemptKind].includes(result.kind)&&result.allOwnedHandlesTerminalReaped===true&&
        ledger.schemaVersion===2&&ledger.kind==='STARSECTOR_FINITE_LAUNCH_HISTORY'&&
        isDeepStrictEqual(actions?.actualGuard,ledger.history)&&
        actions.currentRunContext?.overallAttempts===baseline.overallAttempts&&
        actions.currentRunContext?.uiAttempts===baseline.uiAttempts&&
        ledger.history.overallAttempts+1===baseline.overallAttempts&&ledger.history.uiAttempts+1===baseline.uiAttempts&&
        ledger.history.reservedActions===baseline.reservedActions&&ledger.history.sentActions===baseline.sentActions&&
        Number.isSafeInteger(actions.absoluteMaxReservedActions)&&actions.absoluteMaxReservedActions>=baseline.reservedActions,
    'truthful completed previous seed history required');
    return limit;
}
