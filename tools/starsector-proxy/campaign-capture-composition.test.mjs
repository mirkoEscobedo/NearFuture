// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { Client, InMemoryTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { createDirectMcp } from './direct-mcp.mjs';
import { campaignHttpFixture } from './campaign-http-test-fixture.mjs';
import { escapeArgs, sha } from './campaign-test-fixture.mjs';
import { exitArgs } from './campaign-exit-test-fixture.mjs';
import { readBatchAuthorization, readClaimedBatchAuthorization } from './batch-authorization.mjs';
import { persistDirectFrame } from './direct-frames.mjs';

/** @param {any} result */
function jsonText(result) {
    const texts = result.content.filter((/** @type {any} */ item) => item.type === 'text');
    assert.equal(texts.length, 1); return JSON.parse(texts[0].text);
}

/** Specific closed E1-capture-to-E2 composition. Native/OS/semantic inspection
 * boundaries are synthetic; all source/reservation/protocol/time provenance in
 * the positive path comes from actual maintained transport/guard artifacts.
 * @param {'none' | 'stale-before' | 'foreign-source'} fault */
async function captureExit(fault) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    const f = await campaignHttpFixture({ beforeExit: () => {
        const current = f.history();
        assert.equal(current.history.reservedActions, 58); assert.equal(current.history.sentActions, 56);
        assert.equal(current.pending.batch, 'E2'); assert.equal(current.pending.status, 'SENT');
    } });
    const frontend = createDirectMcp({ ...f.provider.options, spawn: f.provider.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'actual-http-capture-composition', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    /** @type {(string | number)[]} */ const screenshotOuterIDs = [];
    const send = remote.send.bind(remote);
    remote.send = async message => {
        if ('method' in message && message.method === 'tools/call' && message.params?.name === 'screenshot' && 'id' in message)
            screenshotOuterIDs.push(message.id);
        return send(message);
    };
    /** @param {string} name @param {Record<string, unknown>} args */
    async function call(name, args) {
        const result = await client.callTool({ name, arguments: args });
        assert.notEqual(result.isError, true, name + ': ' + JSON.stringify(result)); return result;
    }
    /** @type {[string, Record<string, unknown>][]} */
    const before = [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
        ['screenshot', { target_window_id: 19, width: 480, quality: 0 }], ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]];
    try {
        await frontend.server.connect(local); await client.connect(remote);
        const originalCampaign = structuredClone(f.campaign), originalSourceSha256 = sha(f.source);
        await call('begin_batch', { source: f.source, runID: f.runID, consentMode: 'finite-explicit-consent' });
        for (const [name, args] of before) await call(name, args);
        await call('key', escapeArgs);
        assert.deepEqual(f.keys, [['escape', 1]]);
        const admitted = readClaimedBatchAuthorization(f.source, { root: f.root, runID: f.runID }), known = f.history();
        assert.equal(known.pending.status, 'AWAITING_AFTER'); assert.equal(known.uncertain, false);
        assert.equal(known.pending.sourceSha256, admitted.sourceSha256); assert.equal(known.pending.runID, f.runID);
        assert.equal(known.history.reservedActions, 57); assert.equal(known.history.sentActions, 55);

        const window = jsonText(await call('get_window', { window_id: 19 }));
        const args = { target_window_id: window.windowId, quality: 0, width: window.bounds.width };
        const afterResult = await call('screenshot', args);
        const selectedCall = fault === 'stale-before' ? 'call-004' : 'call-009';
        const selectedCapture = fault === 'stale-before' ? 'capture-001' : 'capture-002';
        const post = JSON.parse(readFileSync(resolve(f.output, selectedCall + '.private.json'), 'utf8'));
        const attempt = JSON.parse(readFileSync(resolve(f.output, selectedCall + '.attempt.private.json'), 'utf8'));
        const actualRaw = readFileSync(resolve(f.output, selectedCall + '.raw.private.json'));
        const selectedResult = fault === 'stale-before' ? JSON.parse(actualRaw.toString('utf8')) : afterResult;
        const captureCompletedAtMs = Date.parse(post.callCompleted), current = f.history();
        assert.ok(Number.isSafeInteger(captureCompletedAtMs));
        assert.deepEqual(post.args, args); assert.equal(post.tool, 'screenshot');
        assert.equal(post.outerProtocolID, screenshotOuterIDs[fault === 'stale-before' ? 0 : 1]);
        assert.equal(typeof post.innerProtocolID, 'number'); assert.equal(attempt.innerProtocolID, 'UNKNOWN');
        assert.equal(attempt.sourceSha256, admitted.sourceSha256); assert.equal(attempt.runID, f.runID); assert.equal(attempt.batch, 'E1');
        assert.equal(current.pending.reservationID, known.pending.reservationID);
        assert.equal(current.pending.sourceSha256, attempt.sourceSha256); assert.equal(current.uncertain, false);
        assert.equal(sha(f.source), originalSourceSha256);
        if (fault === 'stale-before') assert.ok(captureCompletedAtMs < current.pending.knownResultAtMs);
        else assert.ok(captureCompletedAtMs >= current.pending.knownResultAtMs);

        const provenance = { ...post, sourceSha256: attempt.sourceSha256,
            reservationID: current.pending.reservationID, capturedAtMs: captureCompletedAtMs };
        if (fault === 'foreign-source') {
            // Deliberate negative substitution of another actual authorization's
            // digest, never used to manufacture provenance for the positive case.
            provenance.sourceSha256 = sha(f.provider.next.source);
            assert.notEqual(provenance.sourceSha256, admitted.sourceSha256);
        }
        const fixedOutput = resolve(f.scenario, 'E1-after'); mkdirSync(fixedOutput);
        const fixed = persistDirectFrame({ output: fixedOutput, label: 'capture-001', pngName: 'F1.original.png',
            result: selectedResult, args, window, ownedPids: admitted.binding.ownedPids, provenance });
        assert.ok(fixed.pngPath);
        assert.ok(readFileSync(fixed.rawPath).equals(actualRaw));
        assert.ok(readFileSync(fixed.pngPath).equals(readFileSync(resolve(f.output, selectedCapture + '.png'))));
        assert.equal(JSON.parse(readFileSync(fixed.metadataPath, 'utf8')).freshness, 'UNKNOWN');

        // Existing owned producer closes itself and writes its actual receipts.
        // Do not call completeEscape or synthesize terminal files after this call.
        const terminal = jsonText(await call('finish_batch', {})), observedClosedAtMs = Date.now();
        assert.equal(terminal.terminalObserved, true); assert.equal(terminal.cleanupUnknown, false);
        assert.equal(terminal.verifierStatus, 0); assert.equal(terminal.terminalBoundToChild, true);
        assert.equal(terminal.steps[0].processOwnership, 'windows-job-object');
        assert.equal(terminal.steps[0].timedOut, false); assert.equal(f.provider.starts(), 1);
        const readyPath = resolve(f.output, 'guarded-http-ready.private.json');
        const terminalPath = resolve(f.output, 'owned-provider-terminal.private.json');
        const receipt = { schemaVersion: 1, kind: 'STARSECTOR_E1_MENU_INSPECTION', scenario: f.scenario,
            flow: originalCampaign.flow, rootRecordKey: originalCampaign.rootClock.recordKey, controllerSha256: sha(f.controllerPath),
            e1: { sourceSha256: admitted.sourceSha256, claimSha256: sha(resolve(f.scenario, 'guard-E1-consumed.private.json')),
                runID: f.runID, reservationID: current.pending.reservationID },
            frame: { pngPath: fixed.pngPath, pngSha256: sha(fixed.pngPath), metadataPath: fixed.metadataPath,
                metadataSha256: sha(fixed.metadataPath), capturedAtMs: captureCompletedAtMs },
            // Explicit trusted semantic inspection boundary, not pixel/hash inference.
            inspection: { issuer: f.lease.issuer, driver: f.lease.driver, inspectedAtMs: Date.now(), state: 'ORDINARY_GAME_MENU',
                exitCoordinate: [...exitArgs.coordinate] },
            provider: { readySha256: sha(readyPath), terminalSha256: sha(terminalPath), observedClosedAtMs, providerReaped: true } };
        const exit = f.stageExit({ receipt });
        if (fault !== 'none') {
            const historyBefore = readFileSync(f.historyPath);
            const reason = fault === 'foreign-source' ? /menu frame custody/i : /known provider closure chronology/i;
            assert.throws(() => readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID }), reason);
            assert.ok(readFileSync(f.historyPath).equals(historyBefore));
            assert.equal(existsSync(resolve(f.scenario, 'guard-E2-consumed.private.json')), false);
            assert.deepEqual(f.clicks, []); assert.equal(f.provider.starts(), 1);
            assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 55);
            return;
        }
        const e2 = readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID });
        assert.ok('campaign' in e2.binding); assert.deepEqual(e2.binding.campaign, originalCampaign);
        await call('begin_batch', { source: exit.source, runID: exit.runID, consentMode: 'finite-explicit-consent' });
        for (const [name, beforeArgs] of before) await call(name, beforeArgs);
        await call('left_click', exitArgs);
        assert.deepEqual(f.moves, [[40, 80]]); assert.deepEqual(f.clicks, [[40, 80, 'left', 1]]);
        assert.deepEqual(f.keys, [['escape', 1]]); assert.equal(f.provider.starts(), 2);
        assert.equal(f.history().history.reservedActions, 58); assert.equal(f.history().history.sentActions, 56);
        assert.equal(f.history().resolvedE1.reservationID, current.pending.reservationID);
        assert.equal(f.history().uncertain, false);
        assert.notEqual((await call('finish_batch', {})).isError, true);
    } finally {
        await client.close(); await frontend.close(); await f.close();
        names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; });
    }
}

test('actual HTTP AFTER provenance and untouched fixed original compose known closure into E2 HTTP Exit', async () => captureExit('none'));
test('actual stale BEFORE capture cannot qualify as E1 AFTER or claim E2', async () => captureExit('stale-before'));
test('foreign source context on an actual HTTP original refuses before E2 claim or effect', async () => captureExit('foreign-source'));
