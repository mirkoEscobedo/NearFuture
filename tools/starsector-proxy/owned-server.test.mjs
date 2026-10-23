// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { PassThrough } from 'node:stream';
import { startOwnedServer } from './owned-server.mjs';

const syntheticRoot = process.platform === 'win32' ? 'E:\\repo' : '/synthetic-repo';
const expectedAuditPath = process.platform === 'win32'
    ? 'E:/repo/.tmp/starsector-proxy/computer-use-audit.jsonl'
    : '/synthetic-repo/.tmp/starsector-proxy/computer-use-audit.jsonl';

for (const initial of ['pending', 'partial']) test(initial + ' profile waits for all four closed-profile rows and closes through stdin', async () => {
    /** @type {Array<{program: string, args: string[], options: any}>} */
    const calls = [];
    /** @type {any} */ let host;
    let lists = 0;
    /** @param {string} program @param {string[]} args @param {any} options */
    const spawn = (program, args, options) => {
        calls.push({ program, args, options });
        const child = /** @type {any} */ (new EventEmitter());
        child.stdout = new PassThrough(); child.stderr = new PassThrough(); child.stdin = new PassThrough();
        if (args[0] === 'serve') {
            host = child;
            child.stdin.on('finish', () => child.emit('close', 0, null));
            setImmediate(() => child.stdout.write('{"url":"http://127.0.0.1:45678"}\n'));
        } else {
            setImmediate(() => {
                child.stdout.end(args.includes('mcp.list')
                    ? JSON.stringify({ data: [
                        { name: 'computer_use', status: { status: ++lists === 1 && initial === 'pending' ? 'pending' : 'connected' } },
                        ...(lists === 1 && initial === 'partial' ? [] : [
                            { name: 'chrome-devtools', status: { status: 'disabled' } },
                            { name: 'debugMcp', status: { status: 'disabled' } },
                            { name: 'dart', status: { status: 'disabled' } },
                        ]),
                    ] }) : '');
                child.stderr.end(); child.emit('close', 0, null);
            });
        }
        return child;
    };
    const runtime = await startOwnedServer({ executable: 'existing-cli', root: syntheticRoot, spawn: /** @type {typeof import('node:child_process').spawn} */ (spawn) });
    assert.deepEqual(runtime.args, ['--server', 'http://127.0.0.1:45678']);
    assert.equal(runtime.env.OPENCODE_DB, ':memory:');
    assert.equal(runtime.env.OPENCODE_PASSWORD.length, 64);
    const add = calls.find(call => call.args.includes('experimental.mcp.add'));
    assert.ok(add);
    const config = JSON.parse(add.args[add.args.indexOf('--data') + 1]).config;
    assert.equal(config.codemode, false);
    assert.equal(config.cwd, syntheticRoot);
    assert.deepEqual(config.timeout, { startup: 20000, catalog: 20000, execution: 20000 });
    assert.equal(config.env, undefined);
    assert.deepEqual(config.environment, { COMPUTER_USE_PROFILE: 'core', COMPUTER_USE_ACTIVE_PROFILE: 'core',
        COMPUTER_USE_AUDIT_LOG: expectedAuditPath });
    assert.equal(calls.filter(call => call.args.includes('experimental.mcp.add')).length, 1);
    assert.equal(lists, 2, 'complete ready profile is awaited without registering again');
    for (const call of calls) assert.ok(call.args.every(arg => !arg.includes(runtime.env.OPENCODE_PASSWORD)));
    await runtime.close();
    assert.equal(host.stdin.writableEnded, true);
});

for (const record of [
    { name: 'disconnected', response: { data: [{ name: 'computer_use', status: { status: 'disconnected' } }] }, expected: '"status":"disconnected"' },
    { name: 'wrong name', response: { data: [{ name: 'other', status: { status: 'connected' } }] }, expected: '"name":"other"' },
    { name: 'wrong count', response: { data: [] }, expected: '"rowCount":0' },
    { name: 'malformed shape', response: { servers: [] }, expected: '"rowCount":null' },
    { name: 'missing own server', response: { data: [{ name: 'dart', status: { status: 'disabled' } }] }, expected: '"name":"dart"' },
    { name: 'unknown disabled companion', response: { data: [{ name: 'computer_use', status: { status: 'connected' } }, { name: 'extra', status: { status: 'disabled' } }] }, expected: '"name":"extra"' },
    { name: 'duplicate own server', response: { data: [{ name: 'computer_use', status: { status: 'connected' } }, { name: 'computer_use', status: { status: 'connected' } }] }, expected: '"rowCount":2' },
    { name: 'duplicate companion', response: { data: [{ name: 'computer_use', status: { status: 'connected' } }, { name: 'dart', status: { status: 'disabled' } }, { name: 'dart', status: { status: 'disabled' } }] }, expected: '"rowCount":3' },
    ...['connected', 'pending', 'failed', 'needs_auth'].map(status => ({ name: status + ' companion', response: { data: [{ name: 'computer_use', status: { status: 'connected' } }, { name: 'dart', status: { status } }] }, expected: '"status":"' + status + '"' })),
    { name: 'string own status', response: { data: [{ name: 'computer_use', status: 'connected' }] }, expected: '"statusKind":"string"' },
    { name: 'failed own server', response: { data: [{ name: 'computer_use', status: { status: 'failed' } }] }, expected: '"status":"failed"' },
    { name: 'disabled own server', response: { data: [{ name: 'computer_use', status: { status: 'disabled' } }] }, expected: '"status":"disabled"' },
]) test(record.name + ' refuses model admission, identifies its response and closes owned host', async () => {
    /** @type {any} */ let host;
    let password = '';
    /** @param {string} _program @param {string[]} args @param {any} options */
    const spawn = (_program, args, options) => {
        password = options.env.OPENCODE_PASSWORD;
        const child = /** @type {any} */ (new EventEmitter());
        child.stdout = new PassThrough(); child.stderr = new PassThrough(); child.stdin = new PassThrough();
        if (args[0] === 'serve') {
            host = child;
            child.stdin.on('finish', () => child.emit('close', 0, null));
            setImmediate(() => child.stdout.write('{"url":"http://127.0.0.1:45678"}\n'));
        } else setImmediate(() => {
            const response = /** @type {any} */ (structuredClone(record.response));
            if (record.name === 'disconnected') response.data[0].status.error = 'nonce=' + password +
                '; Basic ' + Buffer.from('opencode:' + password).toString('base64') + '; Bearer sk-synthetic-placeholder';
            child.stdout.end(args.includes('mcp.list') ? JSON.stringify(response) : '');
            child.stderr.end(); child.emit('close', 0, null);
        });
        return child;
    };
    await assert.rejects(startOwnedServer({ executable: 'existing-cli', root: 'E:/repo', spawn: /** @type {typeof import('node:child_process').spawn} */ (spawn) }), error => {
        const message = String(error);
        assert.match(message, /not connected/);
        assert.ok(message.includes(record.expected), 'Diagnostic must identify the actual refused response');
        assert.equal(message.includes(password), false, 'No ephemeral auth in diagnostics');
        assert.equal(message.includes('sk-synthetic-placeholder'), false, 'No bearer credential in diagnostics');
        return true;
    });
    assert.equal(host.stdin.writableEnded, true);
});

test('a perpetually pending connection closes at the existing twenty-second bound', async () => {
    /** @type {any} */ let host;
    /** @param {string} _program @param {string[]} args */
    const spawn = (_program, args) => {
        const child = /** @type {any} */ (new EventEmitter());
        child.stdout = new PassThrough(); child.stderr = new PassThrough(); child.stdin = new PassThrough();
        if (args[0] === 'serve') {
            host = child;
            child.stdin.on('finish', () => child.emit('close', 0, null));
            setImmediate(() => child.stdout.write('{"url":"http://127.0.0.1:45678"}\n'));
        } else setImmediate(() => {
            child.stdout.end(args.includes('mcp.list')
                ? JSON.stringify({ data: [{ name: 'computer_use', status: { status: 'pending' } }] }) : '');
            child.stderr.end(); child.emit('close', 0, null);
        });
        return child;
    };
    const started = Date.now();
    await assert.rejects(startOwnedServer({ executable: 'existing-cli', root: 'E:/repo',
        spawn: /** @type {typeof import('node:child_process').spawn} */ (spawn) }), /within 20000ms/);
    assert.ok(Date.now() - started >= 20000);
    assert.equal(host.stdin.writableEnded, true);
});
