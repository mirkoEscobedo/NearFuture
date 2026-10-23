// @ts-check
import { AsyncLocalStorage } from 'node:async_hooks';
import { isDeepStrictEqual } from 'node:util';
import { createHash } from 'node:crypto';
import { readFileSync, lstatSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { createSession } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';
import { readClaimedBatchAuthorization } from './batch-authorization.mjs';
import { createGuardedMcp } from './guarded-mcp.mjs';
import { markCampaignActionSent } from './campaign-authority.mjs';

const observations = ['doctor', 'policy_status', 'list_windows', 'get_window', 'get_frontmost_app',
    'get_display_size', 'list_running_apps', 'screenshot', 'wait'];
/** @type {Record<string, string[]>} */
const mutations = { B1: ['activate_window', 'left_click'], B2: ['activate_window', 'mouse_move'], B3: ['left_click'] };
/** @param {string} path */
function read(path) {
    for (let part = resolve(path);; part = dirname(part)) {
        if (lstatSync(part).isSymbolicLink()) throw new Error('Ordinary consent custody required');
        if (dirname(part) === part) break;
    }
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.size > 256 * 1024) throw new Error('Bounded consent custody required');
    const bytes = readFileSync(path);
    return { value: JSON.parse(bytes.toString('utf8').replace(/^\uFEFF/, '')),
        sha256: createHash('sha256').update(bytes).digest('hex') };
}
/** @param {ReturnType<typeof readClaimedBatchAuthorization>} admitted */
function grant(admitted) {
    const source = read(admitted.sourcePath);
    if (source.sha256 !== admitted.sourceSha256) throw new Error('Finite consent custody changed');
    // The already validated authorization selects either the immutable original
    // menu documents or fixed phase-local successors; do not reopen the launch pair.
    const plan = read(source.value.planPath);
    const lease = read(source.value.leasePath);
    if (plan.sha256 !== admitted.binding.planSha256 ||
        lease.sha256 !== source.value.leaseSha256?.toLowerCase()) throw new Error('Finite consent custody changed');
    const value = plan.value.directHostConsent;
    if (!value || Array.isArray(value) || Object.keys(value).sort().join() !== 'maxCallsPerBatch,mode,schemaVersion' ||
        value.schemaVersion !== 1 || value.mode !== 'finite-explicit-consent' ||
        !Number.isSafeInteger(value.maxCallsPerBatch) || value.maxCallsPerBatch < 1 || value.maxCallsPerBatch > 32 ||
        !isDeepStrictEqual(value, lease.value.directHostConsent)) throw new Error('Explicit current finite host consent required');
    return value;
}

/** Read-only admission, also required BEFORE an owned child's OS probe.
 * @param {{root: string, source: string, runID: string}} authority */
export function readFiniteConsentAuthority({ root, source, runID }) {
    if (process.env.COMPUTER_USE_REQUIRE_APPROVAL !== 'true' || process.env.COMPUTER_USE_APPROVAL_TOKEN)
        throw new Error('Finite host consent requires unchanged all-call approval without a token shortcut');
    const admitted = readClaimedBatchAuthorization(source, { root, runID });
    return { admitted, consent: grant(admitted) };
}

/** Supported one-shot host decisions for a NEW explicitly consenting batch.
 * The original guard remains the authority for sequence, reservation and fresh
 * owned BEFORE/focus checks. The private context below is entered only by its
 * underlying dispatch, after those checks; an audit record grants no authority.
 * No token, interactive continuation or cached approval is accepted here.
 * @param {{root: string, source: string, runID: string}} authority
 * @param {{sessionFactory?: typeof createSession, record?: (event: Record<string, unknown>) => void}} [options] */
export function createFiniteConsentMcp({ root, source, runID }, options = {}) {
    const { admitted, consent } = readFiniteConsentAuthority({ root, source, runID });
    const campaign = [2, 3].includes(admitted.binding.schemaVersion);
    const allowed = new Set([...observations, ...(admitted.binding.schemaVersion === 3 ?
        admitted.binding.batch === 'B1' ? mutations.B1 : admitted.binding.batch === 'E1' ? ['left_click', 'key'] : ['left_click'] :
        campaign ? [admitted.binding.batch === 'E1' ? 'key' : 'left_click'] : mutations[admitted.binding.batch])]);
    /** @type {AsyncLocalStorage<{tool: string, args: any, signal?: AbortSignal, active: boolean, used: boolean}>} */
    const active = new AsyncLocalStorage();
    let attempts = 0;
    /** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ElicitApproval} */
    const approve = async details => {
        const call = active.getStore();
        if (!call?.active || call.used) return false;
        call.used = true; // Failed, mismatched or uncertain decisions are never reused.
        const attempt = ++attempts;
        let approved = false;
        try {
            if (attempt > consent.maxCallsPerBatch || call.signal?.aborted || !allowed.has(call.tool) ||
                details.tool !== call.tool || !isDeepStrictEqual(details.args, call.args) ||
                call.args.approval_token !== undefined || details.destructive !== false ||
                (details.targetApp !== undefined && !/^(java|starsector)\.exe$/i.test(details.targetApp)) ||
                !Array.isArray(details.reasons) || details.reasons.length !== 1 ||
                !['approval_required_for_all', 'tool_requires_approval:' + call.tool].includes(details.reasons[0])) return false;
            const current = readClaimedBatchAuthorization(source, { root, runID });
            if (current.sourceSha256 !== admitted.sourceSha256 || !isDeepStrictEqual(current.binding, admitted.binding) ||
                !isDeepStrictEqual(grant(current), consent) || call.signal?.aborted) return false;
            if (campaign && ['key', 'left_click', 'activate_window'].includes(call.tool)) markCampaignActionSent(admitted.binding, call.args, call.tool);
            approved = true;
            return true;
        } catch { return false; }
        finally { options.record?.({ phase: 'host-consent-decision', tool: call.tool, attempt, approved }); }
    };
    return createGuardedMcp(admitted.binding, { record: options.record, sessionFactory: nativeOptions => {
        const native = (options.sessionFactory ?? createSession)({ ...nativeOptions, elicitApproval: approve });
        return { ...native, async dispatch(tool, args, signal, progress, context) {
            const call = { tool, args: structuredClone(args), signal, active: true, used: false };
            return active.run(call, async () => {
                try { return await native.dispatch(tool, args, signal, progress, context); }
                finally { call.active = false; }
            });
        } };
    } });
}
