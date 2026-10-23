// @ts-check
import { createServer } from 'node:http';
import { once } from 'node:events';
import { randomUUID, timingSafeEqual } from 'node:crypto';
import { NodeStreamableHTTPServerTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/node/dist/index.mjs';
import { TasksExtensionTransport } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/mcp-tasks.js';

const legacyVersion = '2025-11-25';

/** Serve ONE already-created guarded batch, never a fresh guard per request.
 * The caller owns admission, the exclusive claim/log, and the existing Windows
 * Job lifetime. This transport grants no approval and creates no native session.
 * Only the finite legacy client is supported; authenticated Tasks continuation
 * is deliberately unavailable until its complete ownership path is qualified.
 * @param {{embedded: Pick<ReturnType<import('./guarded-mcp.mjs').createGuardedMcp>, 'server' | 'taskManager'>,
 * token: string}} options */
export async function serveGuardedHttp({ embedded, token }) {
    if (!/^[a-f0-9]{64}$/.test(token)) throw new Error('A fresh 32-byte in-memory bearer is required');
    const expected = Buffer.from('Bearer ' + token);
    let origin = '';
    /** @type {Promise<void> | undefined} */ let closing;
    /** @type {() => void} */ let resolveClosed;
    /** @type {(error: unknown) => void} */ let rejectClosed;
    /** @type {Promise<void>} */ const closed = new Promise((done, fail) => { resolveClosed = done; rejectClosed = fail; });
    // Observe failures immediately, while retaining the original rejecting receipt
    // for a caller that is still completing its DELETE request.
    void closed.catch(() => {});
    const transport = new NodeStreamableHTTPServerTransport({ sessionIdGenerator: randomUUID,
        onsessionclosed: () => { setImmediate(() => { void close().catch(() => {}); }); } });

    /** @param {import('node:http').ServerResponse} res @param {number} status */
    function reject(res, status) {
        res.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
        res.end(JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32600, message: 'Guarded batch request refused' } }));
    }
    const http = createServer((req, res) => {
        void handle(req, res).catch(() => { if (!res.headersSent) reject(res, 400); else res.destroy(); });
    });
    http.requestTimeout = 20000;
    http.headersTimeout = 5000;
    /** @param {import('node:http').IncomingMessage} req @param {import('node:http').ServerResponse} res */
    async function handle(req, res) {
        const supplied = typeof req.headers.authorization === 'string' ? Buffer.from(req.headers.authorization) : Buffer.alloc(0);
        if (supplied.length !== expected.length || !timingSafeEqual(supplied, expected)) return reject(res, 401);
        if (req.headers.host !== new URL(origin).host ||
            (req.headers.origin !== undefined && req.headers.origin !== origin)) return reject(res, 403);
        if (closing || req.url !== '/mcp') return reject(res, 404);
        if (!['GET', 'POST', 'DELETE'].includes(req.method ?? '')) return reject(res, 405);
        const version = req.headers['mcp-protocol-version'];
        if (version !== undefined && version !== legacyVersion) return reject(res, 400);
        /** @type {any} */ let body;
        if (req.method === 'POST') {
            const chunks = []; let size = 0;
            for await (const chunk of req) {
                size += chunk.length;
                if (size > 65536) { reject(res, 413); return; }
                chunks.push(chunk);
            }
            body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
            if (!body || Array.isArray(body) || typeof body !== 'object' ||
                typeof body.method !== 'string' || body.method.startsWith('tasks/') ||
                Object.keys(body.params?._meta ?? {}).some(key => key.startsWith('io.modelcontextprotocol/')) ||
                (body.method === 'initialize' && (body.params?.protocolVersion !== legacyVersion ||
                    !body.params.capabilities || typeof body.params.capabilities !== 'object' ||
                    Array.isArray(body.params.capabilities) || Object.keys(body.params.capabilities).length !== 0))) return reject(res, 400);
        }
        const authenticated = /** @type {import('node:http').IncomingMessage & {auth: any}} */ (req);
        authenticated.auth = { token, clientId: 'near-future-finite-batch', scopes: [] };
        await transport.handleRequest(authenticated, res, body);
    }
    async function close() {
        if (closing) return closing;
        closing = (async () => {
            try { await embedded.server.close(); }
            finally {
                await new Promise((done, fail) => {
                    http.close(error => {
                        if (error && !('code' in error && error.code === 'ERR_SERVER_NOT_RUNNING')) fail(error);
                        else done(undefined);
                    });
                    http.closeAllConnections();
                });
            }
        })();
        void closing.then(resolveClosed, rejectClosed);
        return closing;
    }
    try {
        await embedded.server.connect(new TasksExtensionTransport(transport, embedded.taskManager));
        http.listen(0, '127.0.0.1');
        await once(http, 'listening');
        const address = http.address();
        if (!address || typeof address === 'string') throw new Error('Assigned loopback address required');
        origin = 'http://127.0.0.1:' + address.port;
        return { url: origin + '/mcp', closed, close };
    } catch (error) { await close(); throw error; }
}
