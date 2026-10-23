// @ts-check
// The real guard, consent policy and vendor input adapter are retained.
// Only external native effects, screenshots and window observations are replaced.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { json as writeJson, filePin } from './seed-test-support.mjs';
import { claimBatchAuthorization, readBatchAuthorization } from '../batch-authorization.mjs';
import { createFiniteConsentMcp } from '../finite-consent.mjs';
import { syntheticDoctor, syntheticDispatch } from '../direct-test-fixture.mjs';
import { createSession } from '../../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';

export const seedTarget = { pid: 101, windowId: 19, title: 'Starsector', bundleId: 'java.exe',
    isFocused: true, isOnScreen: true, bounds: { x: 10, y: 10, width: 480, height: 220 } };
const original = { content: [{ type: 'image', mimeType: 'image/png',
    data: readFileSync(new URL('../capture/fixtures/readiness.synthetic.png', import.meta.url)).toString('base64') }] };
/** @param {any} value */
const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }], structuredContent: value });

/** @param {(tool: string, args: unknown[]) => void} beforeEffect
 * @param {{screenshot?: () => any, window?: () => any}} [observations]
 * @returns {typeof createSession} */
export function seedNativeSessionFactory(beforeEffect, observations = {}) {
    const window = () => observations.window?.() ?? seedTarget;
    const native = { getWindow: () => ({ ...window(), displayName: 'Synthetic', displayId: 1 }),
        getFrontmostApp: () => ({ pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' }),
        getDisplaySize: () => ({ width: 640, height: 480 }), isNativeEmergencyStopActive: () => false,
        listRunningApps: () => [], unhideApp: () => {}, activateApp: () => ({ activated: true }),
        activateWindow: () => { beforeEffect('activate_window', [19]); return { activated: true }; },
        keyPress: (/** @type {string} */ text, /** @type {number | undefined} */ repeat) => beforeEffect('key', [text, repeat]),
        mouseMove: () => {},
        mouseClick: (/** @type {number} */ x, /** @type {number} */ y, /** @type {string} */ button, /** @type {number} */ count) => beforeEffect('left_click', [x, y, button, count]) };
    return (/** @type {Parameters<typeof createSession>[0]} */ options) => {
            const real = createSession(/** @type {any} */ ({ ...options, native }));
            return { ...real, dispatch: async (tool, args, signal, progress, context) => {
                if (['key', 'left_click', 'activate_window'].includes(tool)) return real.dispatch(tool, args, signal, progress, context);
                assert.equal(await options?.elicitApproval?.({ tool, args, destructive: false,
                    reasons: ['approval_required_for_all'] }), true, 'fresh exact observation consent');
                if (tool === 'doctor') return json(syntheticDoctor);
                if (tool === 'policy_status') return syntheticDispatch(tool);
                if (tool === 'get_window') return json(window());
                if (tool === 'get_frontmost_app') return json({ app: native.getFrontmostApp() });
                if (tool === 'screenshot') return observations.screenshot?.() ?? /** @type {any} */ (original);
                throw new Error('Unexpected synthetic observation');
            } };
    };
}

/** @param {any} f @param {(tool: string, args: unknown[]) => void} beforeEffect
 * @param {{screenshot?: () => any, window?: () => any}} [observations] */
export function seedInputSession(f, beforeEffect, observations = {}) {
    const window = () => observations.window?.() ?? seedTarget;
    const embedded = createFiniteConsentMcp({ root: f.root, source: f.source, runID: f.runID }, {
        sessionFactory: seedNativeSessionFactory(beforeEffect, observations) });
    const context = { mcpReq: { method: 'tools/call', signal: new AbortController().signal,
        requestState: () => undefined, inputResponses: {}, log: async () => {}, notify: async () => {}, envelope: {} } };
    /** @param {string} tool @param {any} args */
    const call = (tool, args) => embedded.registry.executeRegistered(tool, args, context);
    let diagnosed = false;
    async function before() {
        const target = window();
        if (!diagnosed) { await call('doctor', {}); await call('policy_status', {}); diagnosed = true; }
        for (const [tool, args] of [['get_window', { window_id: target.windowId }],
            ['screenshot', { target_window_id: target.windowId, quality: 0, width: target.bounds.width }],
            ['get_frontmost_app', {}], ['get_window', { window_id: target.windowId }]]) await call(/** @type {string} */ (tool), args);
    }
    return { embedded, call, before };
}

/** @param {any} f @param {string} runID @param {boolean} [claim] */
export function claimSeededLauncher(f, runID, claim = true) {
    const inspection = f.plan.campaignControl.profile === 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE';
    const schemaVersion = inspection ? 4 : 3;
    const campaign = { ...f.campaign, flow: inspection ? 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE' : 'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE' };
    const planPath = resolve(f.scenario, 'B1-plan.private.json'), leasePath = resolve(f.scenario, 'B1-lease.private.json');
    writeJson(planPath, { ...f.phasePlan, campaign, batches: [{ id: 'B1' }] });
    writeJson(leasePath, { ...f.phaseLease, schemaVersion, phase: campaign.flow, campaign, planSha256: filePin(planPath).sha256 });
    const ownedReceiptPath = resolve(f.scenario, 'driver-B1-owned.private.json');
    writeJson(ownedReceiptPath, JSON.parse(readFileSync(resolve(f.scenario, 'driver-E1-owned.private.json'), 'utf8')));
    const source = resolve(f.scenario, 'B1-authorization.private.json');
    writeJson(source, { schemaVersion, scenario: f.scenario, batch: 'B1', planPath,
        planSha256: filePin(planPath).sha256, leasePath, leaseSha256: filePin(leasePath).sha256,
        ownedReceiptPath, ownedReceiptSha256: filePin(ownedReceiptPath).sha256 });
    if (claim) claimBatchAuthorization(readBatchAuthorization(source, { root: f.root, runID }));
    return { ...f, source, runID };
}
