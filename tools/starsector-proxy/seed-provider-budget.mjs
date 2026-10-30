// @ts-check
// An absent selector retains every existing provider budget. Only paired
// initial-focus authority may select the fixed eight-minute interval.
import {TextDecoder,isDeepStrictEqual} from 'node:util';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';
/** @param {any} value @param {string[]} fields */
const closed=(value,fields)=>value!==null&&typeof value==='object'&&!Array.isArray(value)&&
    Object.keys(value).sort().join()===fields.slice().sort().join();
/** @param {any} ok */
function need(ok){if(!ok)throw new Error('Campaign authority refused: closed paired seed provider timing required');}
/** Strict bytes only for the new opt-in document boundary; legacy parsing is unchanged.
 * @param {any} value @param {Buffer} bytes */
export function assertSeedProviderTimingJson(value,bytes){
    if(!Object.hasOwn(value,'providerTiming'))return;
    need(bytes.length>=2&&bytes.length<=65536);
    const text=new TextDecoder('utf-8',{fatal:true}).decode(bytes),strict=zeroProviderObject(text,need);
    need(isDeepStrictEqual(strict,value));
}
/** @param {any} plan @param {any} lease */
export function seedProviderTimeoutMs(plan,lease){
    const selected=[plan,lease].map(value=>Object.hasOwn(value,'providerTiming'));
    if(!selected[0]&&!selected[1])return 180000;
    need(selected[0]&&selected[1]);
    for(const value of [plan,lease]){
        const timing=value.providerTiming,control=value.campaignControl;
        need(closed(timing,['schemaVersion','profile','providerTimeoutMs'])&&timing.schemaVersion===1&&
            timing.profile==='INITIAL_FOCUS_PROVIDER_480000'&&timing.providerTimeoutMs===480000&&
            closed(control,['schemaVersion','profile'])&&control.schemaVersion===5&&
            control.profile==='DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'&&value.sessionWatchdogMs===1800000);
        need(closed(value.historyBaseline,['overallAttempts','uiAttempts','reservedActions','sentActions'])&&
            Object.values(value.historyBaseline).every(n=>Number.isSafeInteger(n)&&n>=0));
    }
    for(const key of ['overallAttempts','uiAttempts','reservedActions','sentActions'])
        need(plan.historyBaseline[key]===lease.historyBaseline[key]);
    return 480000;
}
/** Derived only after root/phase admission; callers cannot select a duration.
 * @param {any} binding */
export function bindingProviderTimeoutMs(binding){
    if(!Object.hasOwn(binding,'providerTimeoutMs'))return 180000;
    need(binding.providerTimeoutMs===480000&&binding.schemaVersion===7&&
        binding.campaign?.flow==='DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'&&
        binding.campaign.rootClock?.sessionWatchdogMs===1800000);
    return 480000;
}

/** Reconstruct one original interval; reading it never renews its origin.
 * @param {any} binding @param {number} startMs */
export function campaignProviderClock(binding,startMs){
    const duration=bindingProviderTimeoutMs(binding),root=binding.campaign.rootClock;
    need(Number.isSafeInteger(startMs)&&Number.isSafeInteger(root.effectiveDeadlineMs)&&
        Number.isSafeInteger(root.cleanupReserveMs));
    if(duration===480000&&root.effectiveDeadlineMs-root.cleanupReserveMs-startMs<480000)
        throw new Error('Campaign authority refused: full original eight-minute provider interval required');
    return {startMs,deadlineMs:Math.min(startMs+duration,root.effectiveDeadlineMs-root.cleanupReserveMs)};
}
