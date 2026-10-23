// @ts-check
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { McpServer } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/index.mjs';
import { serveStdio, StdioServerTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { z } from '../computer-use-mcp/node_modules/zod/index.js';
import { defineV7Tools, approvalTokenParam } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/registry/definitions.js';
import { toMcpAnnotations } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/tool-catalog.js';
import { PRIORITY_OUTPUT_SCHEMAS } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/output-schemas.js';
import { startOwnedHttpBatch, directToolNames, OwnedHttpStartupError } from './owned-http-batch.mjs';

/** Fixed finite vendor surface; no native session is created in this frontend.
 * @param {{root: string, powershell: string, supervisor?: string, spawn?: typeof import('node:child_process').spawn}} options */
export function createDirectMcp({ root, powershell, supervisor, spawn }) {
    const server = new McpServer({ name: 'near-future-finite-computer-use', version: '1' });
    /** @type {Awaited<ReturnType<typeof startOwnedHttpBatch>> | undefined} */ let batch;
    /** @type {Promise<Awaited<ReturnType<typeof startOwnedHttpBatch>>> | undefined} */ let startup;
    const shutdown = new AbortController();
    let starting = false, closing = false, retiring = false, unusable = false;
    /** @type {(Error & {cleanupReceipt: any}) | undefined} */ let failure;
    /** @type {Promise<void> | undefined} */ let closure;
    /** Lifecycle failures are monotonic, even after the active handle is cleared.
     * @param {string} message @param {any} [cleanupReceipt] */
    function rememberFailure(message, cleanupReceipt = null) {
        unusable = true; failure ??= Object.assign(new Error(message), { cleanupReceipt }); return failure;
    }
    /** @param {any} receipt */
    function cleanCleanup(receipt) {
        const step = receipt?.steps?.[0];
        const ok = receipt?.terminalObserved === true && receipt.cleanupUnknown === false && receipt.verifierStatus === 0 &&
            receipt.terminalBoundToChild === true && receipt.closeFailed === false && receipt.overflow === false && receipt.spawnFailed === false &&
            receipt.verdict === 'PASS' && receipt.steps.length === 1 && step?.processOwnership === 'windows-job-object' && step.status === 0 && step.timedOut === false;
        if (!ok) rememberFailure('Owned provider cleanup failed or remains UNKNOWN; inspect private evidence', receipt);
        return ok;
    }
    server.registerTool('begin_batch', { description: 'Consume one current caller-authorized finite batch and start its owned provider.',
        inputSchema: z.object({ source: z.string(), runID: z.string().regex(/^[A-Za-z0-9_-]+$/),
            consentMode: z.literal('finite-explicit-consent').optional() }).strict(),
    }, async (args, ctx) => {
        if (batch || starting || closing || retiring || unusable) throw new Error('One qualified finite batch may be active; failed frontend requires review');
        starting = true;
        try {
            startup = startOwnedHttpBatch({ root, powershell, supervisor, ...args,
                signal: AbortSignal.any([shutdown.signal, ctx.mcpReq.signal]) }, { spawn });
            batch = await startup;
            if (closing) { const receipt = await batch.close(); cleanCleanup(receipt); batch = undefined; throw new OwnedHttpStartupError(receipt); }
            return { content: [{ type: 'text', text: JSON.stringify({ runID: args.runID, output: batch.output, deadline: new Date(batch.deadline).toISOString() }) }] };
        } catch (error) {
            const receipt = error instanceof OwnedHttpStartupError ? error.cleanupReceipt : null;
            rememberFailure('Owned batch startup failed; cleanup evidence retained', receipt);
            return { content: [{ type: 'text', text: JSON.stringify({ status: 'STARTUP_FAILED', cleanupReceipt: receipt,
                cleanupUnknown: !receipt || receipt.terminalObserved !== true || receipt.cleanupUnknown !== false }) }], isError: true };
        }
        finally { starting = false; startup = undefined; }
    });
    server.registerTool('finish_batch', { description: 'Terminate the current HTTP session and await its actual owned verifier terminal receipt.', inputSchema: z.object({}).strict() }, async () => {
        if (!batch || retiring) throw new Error('No finite batch is active');
        const current = batch; retiring = true;
        let receipt;
        try { receipt = await current.close(); }
        catch { throw rememberFailure('Owned provider cleanup failed; no qualified cleanup receipt'); }
        finally { batch = undefined; retiring = false; }
        const ok = cleanCleanup(receipt);
        return { content: [{ type: 'text', text: JSON.stringify(receipt) }], ...(ok ? {} : { isError: true }) };
    });
    /** @type {any[]} */ const definitions = [];
    defineV7Tools(/** @type {any} */ ({ define: (/** @type {any} */ definition) => definitions.push(definition) }));
    for (const definition of definitions.filter(value => directToolNames.includes(value.name) || value.name === 'key')) {
        const meta = definition.meta;
        server.registerTool(definition.name, { title: definition.name.split('_').map((/** @type {string} */ part) => part[0].toUpperCase() + part.slice(1)).join(' '),
            description: definition.description,
            inputSchema: meta.mutates ? { ...definition.inputSchema, approval_token: approvalTokenParam } : definition.inputSchema,
            ...(PRIORITY_OUTPUT_SCHEMAS[definition.name] ? { outputSchema: PRIORITY_OUTPUT_SCHEMAS[definition.name] } : {}),
            annotations: toMcpAnnotations(meta), _meta: Object.fromEntries(['focusRequired', 'mutates', 'requiresFocus', 'movesUserCursor', 'usesVirtualPointer', 'physicalInput', 'tier']
                .map(key => ['computer-use/' + key, meta[key]])),
        }, async (args, ctx) => {
            if (!batch || starting || closing || retiring || unusable) throw new Error('Begin a current finite batch first');
            try {
                /** @type {Promise<void>[]} */ const progressWrites = [];
                const result = await batch.call(definition.name, args, { signal: ctx.mcpReq.signal, outerProtocolID: ctx.mcpReq.id,
                    ...(ctx.mcpReq._meta?.progressToken !== undefined ? { onprogress: (/** @type {any} */ progress) => {
                        progressWrites.push(ctx.mcpReq.notify({ method: 'notifications/progress', params: { ...progress, progressToken: ctx.mcpReq._meta?.progressToken } }).catch(() => {}));
                    } } : {}) });
                await Promise.all(progressWrites); if (result.isError) unusable = true; return result;
            } catch { unusable = true; throw new Error('Finite computer-use call stopped; inspect its private evidence'); }
        });
    }
    function close() {
        if (!closure) {
            closing = true; shutdown.abort();
            closure = (async () => {
                try {
                    try { await startup; }
                    catch (error) { rememberFailure('Owned batch startup failed; cleanup evidence retained', error instanceof OwnedHttpStartupError ? error.cleanupReceipt : null); }
                    if (batch) {
                        try { cleanCleanup(await batch.close()); }
                        catch { rememberFailure('Owned provider cleanup failed; no qualified cleanup receipt'); }
                    }
                } finally {
                    try { await server.close(); }
                    catch { rememberFailure('Frontend cleanup failed; shutdown is not qualified'); }
                }
                if (failure) throw failure;
            })();
        }
        return closure;
    }
    return { server, close };
}

/** Standard stdio framing with explicit EOF ownership cleanup.
 * @param {Parameters<typeof createDirectMcp>[0] & {stdin?: NodeJS.ReadableStream, stdout?: NodeJS.WritableStream}} options */
export function serveDirectMcp(options) {
    const frontend = createDirectMcp(options), stdin = options.stdin ?? process.stdin, stdout = options.stdout ?? process.stdout;
    const handle = serveStdio(() => frontend.server, { legacy: 'serve', transport: new StdioServerTransport(/** @type {any} */ (stdin), /** @type {any} */ (stdout)),
        onerror: () => { void close().catch(() => {}); } });
    /** @type {Promise<void> | undefined} */ let closing;
    /** @type {() => void} */ let resolveClosed;
    /** @type {(error: unknown) => void} */ let rejectClosed;
    /** @type {Promise<void>} */ const closed = new Promise((done, fail) => { resolveClosed = done; rejectClosed = fail; });
    void closed.catch(() => {});
    async function close() {
        if (!closing) closing = (async () => { stdin.off('end', onEnd); stdin.off('close', onEnd);
            try { await frontend.close(); } finally { await handle.close(); } })();
        void closing.then(resolveClosed, rejectClosed);
        return closing;
    }
    const onEnd = () => { void close().catch(() => {}); };
    stdin.once('end', onEnd); stdin.once('close', onEnd);
    return { close, closed };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    const frontend = serveDirectMcp({ root: resolve(fileURLToPath(new URL('../../', import.meta.url))),
        powershell: process.env.NF_PROXY_OWNERSHIP_POWERSHELL ?? '' });
    const stop = () => { void frontend.close().catch(() => { process.exitCode = 1; }); };
    process.once('SIGINT', stop); process.once('SIGTERM', stop);
    try { await frontend.closed; }
    catch { console.error('[near-future-direct-mcp] Owned frontend shutdown failed'); process.exitCode = 1; }
    finally { process.off('SIGINT', stop); process.off('SIGTERM', stop); }
}
