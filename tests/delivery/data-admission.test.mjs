// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { admitWrite } from '../../tools/delivery/profile.mjs';

const profile = JSON.parse(readFileSync(new URL('../../config/profiles/headless-foundation.json', import.meta.url), 'utf8'));

test('the accepted headless profile refuses every persistent writer before effects', () => {
    assert.throws(() => admitWrite(profile, 'development', { schema: 1, provider: 'local', writer: 'node' }), /disabled/);
});

test('data environments and persisted compatibility must match before write admission', () => {
    const candidate = { ...profile, persistence: { ...profile.persistence,
        writes_enabled: true, schema: 1, provider: 'local', writer: 'node' } };
    assert.equal(admitWrite(candidate, 'test', { schema: 1, provider: 'local', writer: 'node' }), '.tmp/data/test');
    for (const metadata of [
        { schema: 2, provider: 'local', writer: 'node' },
        { schema: 1, provider: 'foreign', writer: 'node' },
        { schema: 1, provider: 'local', writer: 'other' },
    ]) assert.throws(() => admitWrite(candidate, 'test', metadata), /incompatible/);
    assert.throws(() => admitWrite(candidate, 'production', { schema: 1, provider: 'local', writer: 'node' }), /environment/);
    const shared = { ...candidate, persistence: { ...candidate.persistence,
        roots: { development: '.tmp/data/shared', test: '.tmp/data/shared', release: 'runtime-private/data/release' } } };
    assert.throws(() => admitWrite(shared, 'test', { schema: 1, provider: 'local', writer: 'node' }), /distinct/);
});
