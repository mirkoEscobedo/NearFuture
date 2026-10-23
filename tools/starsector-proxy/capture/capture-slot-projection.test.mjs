import { projectionInputRoot } from './test-output.mjs';
// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { projectCaptureSlots } from './capture-slot-projection.mjs';
test('existing 12 batches and 65 canonical frames project without changing their original identities or paths', () => {
    const base = projectionInputRoot();
    const profileBytes = readFileSync(new URL('live-profile-binding.private.json', base));
    const bindingBytes = readFileSync(new URL('actual-materialized-evidence-bindings.private.json', base));
    assert.equal(createHash('sha256').update(profileBytes).digest('hex'), 'c24040920ed83dd60b6d4cd9301a66a96c5b3c6a4b20435813799476d18ea92d');
    assert.equal(createHash('sha256').update(bindingBytes).digest('hex'), 'ed43f208eee60df5b4f7208c84a32effed2aea248fae050b5986a4d728210fef');
    const profile = JSON.parse(profileBytes.toString('utf8')).profile, materialized = JSON.parse(bindingBytes.toString('utf8'));
    const before = JSON.stringify([profile, materialized]);
    const projected = projectCaptureSlots(profile, materialized);
    assert.equal(projected.length, 12);
    assert.equal(projected.flatMap((/** @type {any} */ b) => b.frames).length, 65);
    for (const batch of profile.batches) {
        const binding = projected.find((/** @type {any} */ row) => row.batchId === batch.id);
        const original = materialized.bindings.find((/** @type {any} */ row) => row.id === batch.compositionBindingId);
        assert.ok(binding);
        assert.equal(binding.id, batch.compositionBindingId);
        assert.equal(binding.evidenceRoot, batch.evidenceRoot);
        assert.deepEqual(binding.frames.map((/** @type {any} */ slot) => slot.id), batch.frameSlots);
        for (let i = 0; i < binding.frames.length; i++) {
            const slot = binding.frames[i], prior = original.frames[i];
            assert.equal(slot.png, prior.png);
            assert.equal(slot.payload, prior.payload);
            assert.equal(slot.receiptRelative, prior.receiptRelative);
            assert.equal(slot.intentPath, resolve(dirname(prior.png), 'F' + (i + 1) + '.capture-intent.private.json'));
            assert.equal(slot.callbackReceipt, resolve(dirname(prior.png), 'F' + (i + 1) + '.callback-completion.private.json'));
        }
    }
    assert.equal(JSON.stringify([profile, materialized]), before, 'projection must leave historical source data unchanged');
});
