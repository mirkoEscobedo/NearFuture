// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { assertTerminalPowerShellResult } from '../../tools/starsector-proxy/fixture/powershell-child-result.mjs';

test('an incomplete child retains its original refusal despite observer failure or mutation', () => {
    /** @type {import('../../tools/starsector-proxy/fixture/powershell-child-result.mjs').PowerShellResult} */
    const child = { status: null, signal: 'SIGTERM', pid: 41,
        error: Object.assign(new Error('spawnSync pwsh ETIMEDOUT'), { code: 'ETIMEDOUT' }),
        stdout: 'original out\n', stderr: 'original error\r\n' };
    const args = ['-File', 'synthetic-controller.ps1'];
    /** @type {import('../../tools/starsector-proxy/fixture/powershell-child-result.mjs').IncompletePowerShellResult|undefined} */
    let observed;
    assert.throws(() => assertTerminalPowerShellResult(child, args, value => {
        observed = value;
        child.status = 0; child.stderr = 'mutated'; args[1] = 'mutated';
        throw new Error('capture refused');
    }), error => error instanceof assert.AssertionError && error.actual === null &&
        /Fixture PowerShell requires terminal status/.test(error.message) &&
        /ETIMEDOUT/.test(error.message) && /failure evidence capture refused/.test(error.message));
    assert.ok(observed);
    assert.deepEqual(observed.args, ['-File', 'synthetic-controller.ps1']);
    assert.equal(observed.status, null);
    assert.equal(observed.stdout, 'original out\n');
    assert.equal(observed.stderr, 'original error\r\n');
    assert.equal(observed.error?.code, 'ETIMEDOUT');
    assert.ok(Object.isFrozen(observed) && Object.isFrozen(observed.args) && Object.isFrozen(observed.error));
});

test('a terminal child never invokes failure observation', () => {
    const child = { status: 1, signal: null, pid: 42, stdout: '', stderr: 'expected refusal' };
    let calls = 0;
    assertTerminalPowerShellResult(child, ['-File', 'synthetic.ps1'], () => { calls += 1; });
    assert.equal(calls, 0);
    assert.equal(child.status, 1);
});

test('an incomplete child without an observer retains the original refusal message', () => {
    const child = { status: null, signal: 'SIGTERM', pid: 0, stdout: null, stderr: null };
    assert.throws(() => assertTerminalPowerShellResult(child, []), {
        message: 'Fixture PowerShell requires terminal status; error=NONE; signal=SIGTERM',
    });
});
