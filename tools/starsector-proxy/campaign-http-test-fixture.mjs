// @ts-check
import assert from 'node:assert/strict';
import { existsSync, readFileSync, rmSync } from 'node:fs';
import { resolve, basename, relative, isAbsolute } from 'node:path';
import { exitFixture } from './campaign-exit-test-fixture.mjs';
import { target, sha } from './campaign-test-fixture.mjs';
import { write } from './batch-test-fixture.mjs';
import { directFixture, syntheticDoctor, syntheticDispatch } from './direct-test-fixture.mjs';
import { requiredOwnedHttpFiles } from './owned-http-batch.mjs';
import { createSession } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';

/** Existing campaign authority plus existing owned-HTTP harness. Process creation,
 * external observations and native effects are synthetic; admission, guard,
 * vendor finite policy and authenticated SDK HTTP are the maintained real path.
 * @param {{e2?: boolean, beforeKey?: () => void, beforeExit?: () => void, uncertainKey?: boolean}} [changes] */
export async function campaignHttpFixture(changes = {}) {
    const f = exitFixture();
    const { powershell, supervisor } = f;
    const pins = [...new Set([...requiredOwnedHttpFiles(f.root, powershell, supervisor), resolve(f.root, 'tools/starsector-proxy/campaign-authority.mjs')])];
    const pinsPath = resolve(f.scenario, 'frozen-support-pins.private.json');
    write(pinsPath, { pins: pins.map(path => ({ path, sha256: sha(path) })) });
    f.lease.supportPinsSha256 = sha(pinsPath); write(f.leasePath, f.lease);
    f.auth.leaseSha256 = sha(f.leasePath); write(f.source, f.auth);
    const source = changes.e2 ? f.stageExit(await f.completeEscape()) : { source: f.source, runID: f.runID };
    const output = resolve(f.root, '.tmp/starsector-proxy', source.runID);
    /** @type {unknown[][]} */ const keys = [];
    const original = { content: [{ type: 'image', mimeType: 'image/png',
        data: readFileSync(new URL('./capture/fixtures/readiness.synthetic.png', import.meta.url)).toString('base64') }] };
    /** @param {any} value @returns {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */
    const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }], structuredContent: value });
    const provider = directFixture({ authority: { root: f.root, ...source, powershell, supervisor, consentMode: 'finite-explicit-consent' },
        sessionFactory: options => {
            const native = { getWindow: () => ({ ...target, displayName: 'Synthetic', displayId: 1 }),
                getFrontmostApp: () => ({ pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' }),
                getDisplaySize: () => ({ width: 640, height: 480 }), isNativeEmergencyStopActive: () => false,
                keyPress: (/** @type {string} */ text, /** @type {number | undefined} */ repeat) => {
                    changes.beforeKey?.(); keys.push([text, repeat]);
                    if (changes.uncertainKey) throw new Error('Synthetic uncertain native key result');
                },
                mouseMove: (/** @type {number} */ x, /** @type {number} */ y) => { f.moves.push([x, y]); },
                mouseClick: (/** @type {number} */ x, /** @type {number} */ y, /** @type {string} */ button, /** @type {number} */ count) => {
                    changes.beforeExit?.(); f.clicks.push([x, y, button, count]);
                } };
            const real = createSession(/** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').SessionOptions} */
                (/** @type {unknown} */ ({ ...options, native })));
            return { ...real, dispatch: async (tool, args, signal, progress, request) => {
                if (tool === 'key' || tool === 'left_click') return real.dispatch(tool, args, signal, progress, request);
                const approved = await options?.elicitApproval?.({ tool, args, destructive: false, reasons: ['approval_required_for_all'] });
                assert.equal(approved, true, 'Current finite observation consent is required');
                if (tool === 'doctor') return json(syntheticDoctor);
                if (tool === 'policy_status') return /** @type {Promise<import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult>} */ (syntheticDispatch(tool));
                if (tool === 'get_window') return json(target);
                if (tool === 'get_frontmost_app') return json({ app: native.getFrontmostApp() });
                if (tool === 'screenshot') return /** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */ (original);
                throw new Error('Unexpected external observation');
            } };
        } });
    /** @param {Awaited<ReturnType<typeof import('./owned-http-batch.mjs').startOwnedHttpBatch>>} batch */
    async function before(batch) {
        for (const [tool, args] of [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
            ['screenshot', { target_window_id: 19, quality: 0, width: 480 }], ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]]) {
            assert.notEqual((await batch.call(/** @type {string} */ (tool), args)).isError, true);
        }
    }
    return { ...f, provider, output, keys, before,
        async close() {
            await provider.close();
            const parent = resolve(f.root, '.tmp/starsector-proxy');
            for (const path of new Set([output, resolve(parent, f.runID + '-exit')])) {
                const suffix = relative(parent, path);
                assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix) && basename(path).startsWith('synthetic-campaign-exit-'));
                if (existsSync(path)) rmSync(path, { recursive: true });
            }
            await f.close();
        } };
}
