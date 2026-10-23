// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readdirSync } from 'node:fs';
import { createCaptureCustody } from './capture-custody.mjs';
import { custodyFixture } from './capture-custody-fixture.mjs';
test('constructor requires explicit admitted identities and pins before exposing callbacks', () => {
    /** @type {Array<[string, (config: any) => void, string]>} */
    const cases = [
        ['missing profile', x => { delete x.profileId; }, 'ADMITTED_CAPTURE_BINDINGS_REQUIRED'],
        ['missing root command', x => { delete x.rootRunnerCommandExact; }, 'ADMITTED_CAPTURE_BINDINGS_REQUIRED'],
        ['empty pins', x => { x.sourcePins = []; }, 'ADMITTED_CAPTURE_BINDINGS_REQUIRED'],
        ['malformed pin', x => { x.sourcePins[0].sha256 = []; }, 'ADMITTED_CAPTURE_BINDINGS_REQUIRED'],
        ['missing batch', x => { delete x.bindings[0].batchId; }, 'EXACT_ADMITTED_BINDING_IDENTITY_REQUIRED'],
        ['duplicate binding', x => { x.bindings.push(JSON.parse(JSON.stringify(x.bindings[0]))); }, 'EXACT_ADMITTED_BINDING_IDENTITY_REQUIRED'],
    ];
    for (const [name, mutate, reason] of cases) {
        const f = custodyFixture();
        mutate(f.admitted);
        const before = readdirSync(f.output);
        assert.throws(() => createCaptureCustody({ admitted: f.admitted, readIntent: f.readIntent, now: f.now }),
            { message: reason }, name);
        assert.deepEqual(readdirSync(f.output), before);
    }
});
