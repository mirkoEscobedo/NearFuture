// @ts-check
// Two closed creation profiles. The older profile never inherits the activation exception.
import {closed,requireThat} from './campaign-proof.mjs';
import {seedProviderTimeoutMs} from './seed-provider-budget.mjs';

export const initialFocusCreationFlow='DISPOSABLE_SEED_CREATION_INITIAL_FOCUS';
export const initialFocusCreationCloseoutKind='ACTUAL_DISPOSABLE_SEED_CREATION_INITIAL_FOCUS_CLOSEOUT';
export const initialFocusHistoryGrantKind='ROOT_ADMITTED_INITIAL_FOCUS_SEED_CREATION_HISTORY';
export const initialFocusActivationTimeoutMs=2000;
export const initialFocusB1MaximumCalls=21;
/** @param {any} binding */
export function isInitialFocusCreation(binding){return binding?.schemaVersion===7&&binding.campaign?.flow===initialFocusCreationFlow;}
/** @param {any} plan @param {any} lease */
export function seedCreationRequestAllowance(plan,lease){
    const providerTimeoutMs=seedProviderTimeoutMs(plan,lease);
    const focus=plan.campaignControl?.schemaVersion===5||lease.campaignControl?.schemaVersion===5||
        plan.campaignControl?.profile===initialFocusCreationFlow||lease.campaignControl?.profile===initialFocusCreationFlow;
    // Legacy phase-local documents inherit control4 through the immutable root
    // predecessor; preserve their original eight-request admission unchanged.
    if(!focus)return 8;
    const version=5,profile=initialFocusCreationFlow;
    for(const control of [plan.campaignControl,lease.campaignControl])requireThat(closed(control,['schemaVersion','profile'])&&
        control.schemaVersion===version&&control.profile===profile,'paired fixed seed creation profile required');
    if(focus)requireThat(plan.sessionWatchdogMs===1800000&&lease.sessionWatchdogMs===1800000&&
        plan.bounds?.maxScenarioActionsIncludingActivations===9&&lease.maxActions===9&&plan.bounds?.maxProxyInvocations===3&&lease.maxProxyInvocations===3&&
        plan.bounds?.proxyTimeoutMs===providerTimeoutMs&&plan.bounds?.ownedGameHardTimeoutMs===1800000&&lease.ownedGameHardTimeoutMs===1800000&&
        plan.bounds?.plannedMutationsWithoutActivation===8&&plan.bounds?.maxPlannedMutationsIncludingActivation===9&&plan.bounds?.optionalActivations===1&&
        plan.bounds?.maxCallsPerBatch===32,'closed initial focus request budget required');
    return focus?9:8;
}
