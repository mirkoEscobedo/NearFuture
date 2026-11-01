// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { withApplicationQuit } from './campaign-application-quit-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';
import { readClaimedBatchAuthorization } from '../batch-authorization.mjs';
import { reserveCampaignAction } from '../campaign-authority.mjs';

/** @type {Array<[string,(proof:any)=>void]>} */
const faults = [
    ['missing inspection', p => unlinkSync(p.path)],
    ['unknown dialog', p => { p.receipt.inspection.dialogText = 'Save and exit?'; }],
    ['No button', p => { p.receipt.inspection.confirmLabel = 'No'; }],
    ['Save button', p => { p.receipt.inspection.confirmLabel = 'Save'; }],
    ['uninspected coordinate', p => { p.receipt.inspection.confirmCoordinate = [91, 130]; }],
    ['legacy confirmation kind', p => { p.receipt.kind = 'STARSECTOR_EXIT_CONFIRMATION_INSPECTION'; }],
    ['previous reservation', p => { p.receipt.reservationID += '-old'; }],
    ...['rootRecordKey', 'controllerSha256', 'sourceSha256', 'claimSha256', 'runID', 'manifestSha256',
        'stagingReceiptSha256'].map(key => /** @type {[string,(proof:any)=>void]} */ ([
        'replayed ' + key, p => { p.receipt[key] = '0'.repeat(64); }])),
    ['stale inspection', p => { p.receipt.inspection.inspectedAtMs = p.receipt.frame.capturedAtMs - 1; }],
    ['changed original PNG', p => writeFileSync(p.frame.pngPath, Buffer.from('not a PNG'))],
    ['changed focus metadata', p => {
        const m = JSON.parse(readFileSync(p.frame.metadataPath, 'utf8'));
        m.window.isFocused = false; json(p.frame.metadataPath, m);
        p.receipt.frame.metadataSha256 = filePin(p.frame.metadataPath).sha256.toLowerCase();
    }],
    ['changed raw original', p => {
        const m = JSON.parse(readFileSync(p.frame.metadataPath, 'utf8'));
        const raw = JSON.parse(readFileSync(m.rawPath, 'utf8')); raw.isError = true; json(m.rawPath, raw);
        m.rawSha256 = filePin(m.rawPath).sha256.toLowerCase(); json(p.frame.metadataPath, m);
        p.receipt.frame.metadataSha256 = filePin(p.frame.metadataPath).sha256.toLowerCase();
    }],
];
for (const [name, change] of faults) test('final confirmation refuses ' + name + ' without a ninth reservation or effect', () =>
    withApplicationQuit(async f => {
        await f.h.before(); await f.click([40, 80]);
        await f.inspect('confirmation'); await f.click([70, 110]);
        await f.inspect('menu'); await f.click([80, 120]);
        const proof = await f.inspect('application-confirmation'); change(proof);
        if (name !== 'missing inspection') json(proof.path, proof.receipt);
        const before = f.ledger();
        await assert.rejects(f.click([90, 130]));
        const after = f.ledger();
        assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click']);
        assert.deepEqual(after.history, before.history); assert.equal(after.history.reservedActions, 8);
        assert.deepEqual(after.pending, before.pending); assert.deepEqual(after.resolvedActions, before.resolvedActions);
        assert.equal(after.uncertain, true);
    }, { confirmation: true }));

test('the nine-input profile refuses activation at both registered-tool and reservation boundaries', () =>
    withApplicationQuit(async f => {
        const before = f.ledger();
        const { binding } = readClaimedBatchAuthorization(f.source, { root: f.root, runID: f.runID });
        assert.throws(() => reserveCampaignAction(binding, { window_id: 21, timeout_ms: 2000 }, 'activate_window'),
            /application Quit refuses activation/);
        assert.deepEqual(f.ledger(), before);
        await assert.rejects(f.h.call('activate_window', { window_id: 21, timeout_ms: 2000 }));
        assert.deepEqual(f.effects, []); assert.deepEqual(f.ledger(), { ...before, uncertain: true });
    }, { confirmation: true }));
