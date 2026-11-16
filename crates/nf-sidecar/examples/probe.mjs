// Independent Node/crypto caller for the actual Rust executable. No game/vault.
import { spawn } from 'node:child_process';
import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto';
import { createConnection } from 'node:net';
import { once } from 'node:events';
import assert from 'node:assert/strict';

const executable = process.argv[2];
if (!executable) throw new Error('usage: node probe.mjs <absolute nf-sidecar executable>');
const u16 = n => { const b = Buffer.alloc(2); b.writeUInt16BE(n); return b; };
const u32 = n => { const b = Buffer.alloc(4); b.writeUInt32BE(n); return b; };
const u64 = n => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; };
const tag = (key, domain, ...parts) => {
    const h = createHmac('sha256', key).update(domain + '\0');
    for (const part of parts) h.update(part);
    return h.digest();
};
const signed = (key, domain, ...parts) => Buffer.concat([...parts, tag(key, domain, ...parts)]);
async function exact(stream, size, timeout = 2000) {
    const end = Date.now() + timeout;
    while (true) {
        const b = stream.read(size);
        if (b) return b;
        if (stream.readableEnded || stream.destroyed) throw new Error('unexpected EOF');
        await new Promise((resolve, reject) => {
            const done = () => { clearTimeout(timer); stream.off('readable', done); stream.off('end', done); stream.off('error', fail); resolve(); };
            const fail = e => { clearTimeout(timer); stream.off('readable', done); stream.off('end', done); stream.off('error', fail); reject(e); };
            const timer = setTimeout(() => fail(new Error('read deadline')), Math.max(1, end - Date.now()));
            stream.once('readable', done); stream.once('end', done); stream.once('error', fail);
        });
        if (Date.now() >= end) throw new Error('read deadline');
    }
}
async function frame(socket) {
    const length = (await exact(socket, 4)).readUInt32BE();
    assert(length > 0 && length <= 65536);
    return exact(socket, length);
}
function send(socket, body) { socket.write(Buffer.concat([u32(body.length), body])); }
async function start(enabled = 1) {
    const token = randomBytes(32);
    const child = spawn(executable, ['serve-parent'], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true });
    // Install the bounded exit waiter immediately; never orphan an example child.
    const exited = once(child, 'exit');
    let stderr = '';
    child.stderr.on('data', b => { stderr = (stderr + b.toString()).slice(-1024); });
    const stop = async () => {
        child.stdin.end();
        const timer = setTimeout(() => child.kill(), 2500);
        try { const [code] = await exited; assert.equal(code, 0, stderr); }
        finally { clearTimeout(timer); token.fill(0); }
    };
    try {
        child.stdin.write(Buffer.concat([Buffer.from('NFMPAR01'), token, u32(10000), u32(enabled)]));
        const ready = await exact(child.stdout, 34);
        assert.equal(ready.subarray(0, 8).toString(), 'NFMRDY01');
        return { child, token, port: ready.readUInt16BE(8), session: ready.subarray(10, 26), stop };
    } catch (e) { await stop(); throw e; }
}
async function connect(owner, requested = 1, required = 1, version = 1) {
    const socket = createConnection({ host: '127.0.0.1', port: owner.port });
    socket.on('error', () => {});
    socket.setTimeout(2000, () => socket.destroy(new Error('socket deadline')));
    await once(socket, 'connect');
    try {
        const h = signed(owner.token, 'NF-MODULE-HELLO-1', Buffer.concat([
            Buffer.from('NFMHEL01'), owner.session, u16(owner.port), u16(version), u16(version), u16(0),
            randomBytes(16), u32(requested), u32(required), u32(65536), u32(346), u32(1000), Buffer.alloc(12),
        ]));
        assert.equal(h.length, 112); send(socket, h);
        const a = await frame(socket); assert.equal(a.length, 120);
        assert(timingSafeEqual(a.subarray(88), tag(owner.token, 'NF-MODULE-ACCEPT-1', h, a.subarray(0, 88))));
        assert.equal(a.subarray(0, 8).toString(), 'NFMACK01'); assert.deepEqual(a.subarray(8, 24), owner.session);
        const key = tag(owner.token, 'NF-MODULE-KEY-1', h, a.subarray(0, 88));
        return { socket, key, status: a.readUInt16BE(26), sequence: 1 };
    } catch (e) { socket.destroy(); throw e; }
}
async function invoke(owner, connection, { generation = 0, module = 1, capabilities = 1, payload = randomBytes(32) } = {}) {
    const id = randomBytes(16), seq = connection.sequence++;
    const b = signed(connection.key, 'NF-MODULE-REQUEST-1', Buffer.concat([
        Buffer.from('NFMREQ01'), owner.session, id, u64(generation), u64(seq), u16(module), u16(1), u16(1),
        u16(0), u32(capabilities), u32(1000), payload,
    ]));
    send(connection.socket, b);
    const r = await frame(connection.socket);
    assert(timingSafeEqual(r.subarray(-32), tag(connection.key, 'NF-MODULE-REPLY-1', r.subarray(0, -32))));
    assert.equal(r.subarray(0, 8).toString(), 'NFMRSP01');
    assert.deepEqual(r.subarray(8, 24), owner.session); assert.deepEqual(r.subarray(24, 40), id);
    assert.equal(r.readBigUInt64BE(40), BigInt(generation)); assert.equal(r.readBigUInt64BE(48), BigInt(seq));
    return { status: r.readUInt16BE(56), payload: r.subarray(58, -32), id: id.toString('hex'), challenge: payload };
}
const owner = await start();
const connections = [];
try {
    const c = await connect(owner); connections.push(c); assert.equal(c.status, 0);
    const probe = await invoke(owner, c); assert.equal(probe.status, 0); assert.deepEqual(probe.payload, probe.challenge);
    const stale = await invoke(owner, c, { generation: 42 }); assert.equal(stale.status, 8);
    const unsupported = await invoke(owner, c, { module: 999 }); assert.equal(unsupported.status, 5);
    const version = await connect(owner, 1, 1, 2); connections.push(version); assert.equal(version.status, 2);
    const capability = await connect(owner, 3, 2); connections.push(capability); assert.equal(capability.status, 3);
    owner.child.stdin.write(Buffer.concat([Buffer.from([1]), u64(1)]));
    const acknowledged = await exact(owner.child.stdout, 32);
    assert.equal(acknowledged.subarray(0, 8).toString(), 'NFMBND01');
    assert.deepEqual(acknowledged.subarray(8, 24), owner.session);
    assert.equal(acknowledged.readBigUInt64BE(24), 1n);
    // The old connection must close at the generation fence before a new one is admitted.
    if (!c.socket.destroyed) await once(c.socket, 'close');
    const bound = await connect(owner); connections.push(bound);
    const reply = await invoke(owner, bound, { generation: 1 }); assert.equal(reply.status, 0);
    const older = await invoke(owner, bound, { generation: 2 }); assert.equal(older.status, 8);
    console.log(JSON.stringify({ protocol: 1, authenticated: true, requestId: probe.id,
        serviceSession: owner.session.toString('hex'), neutralEcho: true, campaignGeneration: 1,
        currentBindingEcho: true, staleBinding: true, unsupportedModule: true, negotiationRefusals: true }));
} finally { for (const c of connections) { c.socket.destroy(); c.key.fill(0); } await owner.stop(); }
const disabled = await start(0);
let connection;
try {
    assert.notDeepEqual(disabled.session, owner.session);
    connection = await connect(disabled, 0, 0); assert.equal(connection.status, 0);
    const reply = await invoke(disabled, connection, { capabilities: 0 }); assert.equal(reply.status, 7);
    console.log(JSON.stringify({ freshRestartSession: true, disabledProbe: true, parentEofExit: true }));
} finally { connection?.socket.destroy(); connection?.key.fill(0); await disabled.stop(); }
