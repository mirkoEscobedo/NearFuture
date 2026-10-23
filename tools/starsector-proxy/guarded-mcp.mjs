// @ts-check
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { openSync, writeSync, closeSync } from 'node:fs';
import { createComputerUseServer } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/server.js';
import { createSession } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';
import { McpTaskManager, TasksExtensionTransport } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/mcp-tasks.js';
import { serveStdio, StdioServerTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { createBatchSession, validateBatchBinding } from './batch-session.mjs';
import { readClaimedBatchAuthorization } from './batch-authorization.mjs';

/** Core-only embedding of the pinned server. The vendor still owns schemas, native
 * policy, focus, lock, results, resources and task protocol. Approval-required
 * input stops; this finite flow cannot resume from receipts predating user interaction.
 * @param {any} binding @param {{session?: any, sessionFactory?: typeof createSession,
 * record?: (event: Record<string, unknown>) => void}} [options] */
export function createGuardedMcp(binding, options = {}) {
    validateBatchBinding(binding);
    /** @type {any} */ let server;
    /** @type {any} */ let registry;
    // Decline legacy interactive approval BEFORE any dialog/native input. The native
    // policy remains authoritative and returns its ordinary denial. No automatic approval.
    /** @param {any} ctx */
    async function elicitApproval(ctx) {
        options.record?.({ phase: 'unsupported-approval-continuation', protocol: 'legacy', tool: ctx.tool });
        return false;
    }
    const underlying = options.session ?? (options.sessionFactory ?? createSession)({ profile: 'core',
        vision: process.env.COMPUTER_USE_VISION !== 'false', provider: process.env.COMPUTER_USE_PROVIDER, elicitApproval });
    // No filesystem dispatch is reachable through this closed core batch. The vendor
    // retains its own roots controller for resource reads; requestContext is untouched.
    const session = createBatchSession(underlying, binding, options.record);
    server = createComputerUseServer({ profile: 'core', activeProfile: 'core', session,
        onRegistry: value => { registry = value; } });
    // The transport and tools/call must use the SAME existing vendor task manager.
    const taskManager = new McpTaskManager({ maxConcurrentPerOwner: Number(process.env.COMPUTER_USE_MAX_TASKS ?? 16),
        ttlMs: Number(process.env.COMPUTER_USE_TASK_TTL_MS ?? 3600000),
        pollIntervalMs: Number(process.env.COMPUTER_USE_TASK_POLL_INTERVAL_MS ?? 1000) });
    // Preserve the existing registry's schemas, policy and result wrapper while
    // holding the request gate through its complete execution/logging/notifications.
    const guardedRegistry = new Proxy(registry, { get(target, property) {
        if (property === 'executeRegistered') return (/** @type {string} */ name, /** @type {any} */ args, /** @type {any} */ context) =>
            session.runRequest(name, () => target.executeRegistered(name, args, context));
        const value = Reflect.get(target, property, target);
        return typeof value === 'function' ? value.bind(target) : value;
    } });
    taskManager.install(server, guardedRegistry);
    return { server, registry: guardedRegistry, session, taskManager };
}

async function main() {
    const root = resolve(fileURLToPath(new URL('../../', import.meta.url)));
    const source = process.env.NF_PROXY_BATCH_AUTHORIZATION;
    const runID = process.env.NF_PROXY_BATCH_RUN_ID;
    if (!source || !runID) throw new Error('Owned caller batch authorization and run ID required');
    const admittedSource = source, admittedRunID = runID;
    const admitted = readClaimedBatchAuthorization(source, { root, runID });
    const log = resolve(root, '.tmp/starsector-proxy', runID, 'batch-dispatch.private.jsonl');
    const fd = openSync(log, 'wx');
    /** @param {Record<string, unknown>} event */
    function record(event) {
        if (event.phase === 'before-dispatch' && event.mutation) readClaimedBatchAuthorization(admittedSource, { root, runID: admittedRunID });
        writeSync(fd, JSON.stringify({ at: new Date().toISOString(), runID, batch: admitted.binding.batch, ...event }) + '\n');
    }
    const embedded = createGuardedMcp(admitted.binding, { record });
    const handle = serveStdio(() => embedded.server, { legacy: 'serve', maxSubscriptions: 256,
        transport: new TasksExtensionTransport(new StdioServerTransport(), embedded.taskManager),
        onerror: error => console.error('[guarded-computer-use]', error.message) });
    async function shutdown() { try { await handle.close(); } finally { closeSync(fd); } }
    process.once('SIGINT', () => { void shutdown().finally(() => process.exit(0)); });
    process.once('SIGTERM', () => { void shutdown().finally(() => process.exit(0)); });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    await main();
}
