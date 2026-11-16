// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { withApplicationQuit } from './campaign-application-quit-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';

/** @type {Array<[string, 'confirmation'|'menu', (proof:any)=>void]>} */
const faults = [
    ['missing confirmation', 'confirmation', proof => unlinkSync(proof.path)],
    ['Save button', 'confirmation', proof => { proof.receipt.inspection.confirmLabel = 'Save'; }],
    ['legacy dialog', 'confirmation', proof => { proof.receipt.inspection.dialogText = 'Exit game?'; }],
    ['uninspected confirmation coordinate', 'confirmation', proof => { proof.receipt.inspection.confirmCoordinate = [71, 110]; }],
    ['legacy proof kind', 'confirmation', proof => { proof.receipt.kind = 'STARSECTOR_EXIT_CONFIRMATION_INSPECTION'; }],
    ['Cancel at returned menu', 'menu', proof => { proof.receipt.inspection.quitLabel = 'Cancel'; }],
    ['uninspected Quit coordinate', 'menu', proof => { proof.receipt.inspection.quitCoordinate = [81, 120]; }],
    ['previous reservation', 'menu', proof => { proof.receipt.reservationID += '-old'; }],
    ['another claim', 'menu', proof => { proof.receipt.claimSha256 = '0'.repeat(64); }],
    ['another source', 'confirmation', proof => { proof.receipt.sourceSha256 = '0'.repeat(64); }],
    ['another seed', 'menu', proof => { proof.receipt.manifestSha256 = '0'.repeat(64); }],
    ['another stage', 'confirmation', proof => { proof.receipt.stagingReceiptSha256 = '0'.repeat(64); }],
    ['future inspection', 'menu', proof => { proof.receipt.inspection.inspectedAtMs = Date.now() + 60000; }],
    ['extra visible authority', 'menu', proof => { proof.receipt.inspection.saveAllowed = true; }],
    ['changed PNG', 'menu', proof => { writeFileSync(proof.frame.pngPath, Buffer.from('not a PNG')); }],
    ['changed metadata focus', 'confirmation', proof => {
        const metadata = JSON.parse(readFileSync(proof.frame.metadataPath, 'utf8'));
        metadata.window.isFocused = false; json(proof.frame.metadataPath, metadata);
        proof.receipt.frame.metadataSha256 = filePin(proof.frame.metadataPath).sha256.toLowerCase();
    }],
    ['changed raw original', 'menu', proof => {
        const metadata = JSON.parse(readFileSync(proof.frame.metadataPath, 'utf8'));
        const raw = JSON.parse(readFileSync(metadata.rawPath, 'utf8'));
        raw.isError = true; json(metadata.rawPath, raw);
        metadata.rawSha256 = filePin(metadata.rawPath).sha256.toLowerCase(); json(proof.frame.metadataPath, metadata);
        proof.receipt.frame.metadataSha256 = filePin(proof.frame.metadataPath).sha256.toLowerCase();
    }],
];

for (const [name, scene, change] of faults) test('application Quit refuses ' + name + ' before another reservation or native effect',
    () => withApplicationQuit(async f => {
        await f.h.before(); await f.click([40, 80]);
        if (scene === 'menu') { await f.inspect('confirmation'); await f.click([70, 110]); }
        const proof = await f.inspect(scene);
        change(proof);
        if (name !== 'missing confirmation') json(proof.path, proof.receipt);
        const before = f.ledger(), effects = [...f.effects];
        await assert.rejects(f.click(scene === 'confirmation' ? [70, 110] : [80, 120]));
        const after = f.ledger();
        assert.deepEqual(f.effects, effects);
        assert.deepEqual(after.history, before.history);
        assert.deepEqual(after.pending, before.pending);
        assert.deepEqual(after.resolvedActions, before.resolvedActions);
        assert.equal(after.uncertain, true);
        await assert.rejects(f.click([80, 120]), /already latched|uncertain launch input/);
        assert.deepEqual(f.effects, effects);
    }));
