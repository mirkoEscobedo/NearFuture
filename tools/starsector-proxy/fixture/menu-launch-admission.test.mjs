// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { menuLaunchFixture } from './menu-launch-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';

test('one immutable menu authority admits the real protected launch and finite input boundaries', () => {
    const f = menuLaunchFixture();
    try {
        const frozen = [f.planPath, f.leasePath, f.grantPath].map(path => readFileSync(path));
        const launch = f.admit();
        assert.equal(launch.status, 0, launch.stderr);
        assert.deepEqual(JSON.parse(launch.stdout), { launchGrant: f.plan.launchGrant, rootLeaseSha256: filePin(f.leasePath).sha256 });
        assert.deepEqual(f.input().binding.ownedPids, [101]);
        assert.deepEqual([f.planPath, f.leasePath, f.grantPath].map(path => readFileSync(path)), frozen);
    } finally { f.close(); }
});

/** @type {Array<[string, (f: any) => void]>} */
const incompatible = [
    ['grant phase', f => { f.grant.phase = 'MENU_SMOKE_ONLY'; }],
    ['grant issuer', f => { f.grant.issuer = '/another-root'; }],
    ['grant driver', f => { f.grant.driver = '/root/game_driver'; }],
    ['grant extra authority', f => { f.grant.ownedNodeKill = {}; }],
    ['plan seed', f => { f.plan.seed = {}; }],
    ['lease seed', f => { f.lease.seed = {}; }],
    ['plan recording', f => { f.plan.recordingProfileSha256 = 'a'.repeat(64); }],
    ['lease killed-node case', f => { f.lease.ownedNodeKill = {}; }],
    ['plan watchdog extension', f => { f.plan.sessionWatchdogMs = 480001; }],
    ['lease watchdog extension', f => { f.lease.sessionWatchdogMs = 480001; }],
    ['plan action extension', f => { f.plan.bounds.maxScenarioActionsIncludingActivations = 9; }],
    ['lease action extension', f => { f.lease.maxActions = 9; }],
    ['lease save permission', f => { f.lease.campaignLoadSaveAllowed = true; }],
    ['missing launch batch', f => { f.plan.batches = []; }],
    ['unknown menu batch', f => { f.plan.batches = [{ id: 'B1' }, { id: 'UNKNOWN' }]; }],
    ['missing validator pin', f => {
        const pins = JSON.parse(readFileSync(f.supportPinsPath, 'utf8'));
        pins.pins = pins.pins.filter((/** @type {any} */ pin) => !pin.path.endsWith('menu-launch.ps1'));
        json(f.supportPinsPath, pins);
    }],
    ['private authority pin', f => {
        const pins = JSON.parse(readFileSync(f.supportPinsPath, 'utf8'));
        pins.pins.push({ path: f.grantPath, sha256: 'a'.repeat(64) }); json(f.supportPinsPath, pins);
    }],
];
for (const [name, change] of incompatible) test(name + ' refuses both real admission boundaries', () => {
    const f = menuLaunchFixture();
    try {
        change(f); f.refreshGrant();
        const launch = f.admit();
        assert.notEqual(launch.status, 0, name);
        assert.match(launch.stderr, /menu/i);
        assert.throws(() => f.input(), /Menu launch grant/);
    } finally { f.close(); }
});

test('changed grant bytes and missing grant cannot admit either boundary', () => {
    for (const missing of [false, true]) {
        const f = menuLaunchFixture();
        try {
            if (missing) { delete /** @type {any} */ (f.plan).launchGrant; f.refreshAuthority(); }
            else json(f.grantPath, { ...f.grant, authorized: false });
            assert.notEqual(f.admit().status, 0);
            assert.throws(() => f.input(), /Menu launch grant/);
        } finally { f.close(); }
    }
});

test('finite input requires the exact lease and grant recorded by its original controller', () => {
    for (const field of ['rootLeaseSha256', 'launchGrant']) {
        const f = menuLaunchFixture();
        try {
            assert.equal(f.admit().status, 0);
            /** @type {any} */ (f.controller)[field] = field === 'launchGrant' ? { ...f.plan.launchGrant, sha256: '0'.repeat(64) } : '0'.repeat(64);
            json(f.controllerPath, f.controller);
            assert.throws(() => f.input(), /Original menu launch authority changed/);
        } finally { f.close(); }
    }
});
