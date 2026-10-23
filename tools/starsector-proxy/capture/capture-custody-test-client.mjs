// @ts-check
import { Client, InMemoryTransport } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { createComputerUseServer } from './vendor-runtime.mjs';
import { createCaptureCustody } from './capture-custody.mjs';
/** Only native dispatch and intent-file receipt are supplied by tests.
 * @param {any} f @param {{dispatch: (name: string, args: any) => Promise<any>, readIntent?: () => any, now?: () => number}} options */
export async function openCaptureClient(f, options) {
    const custody = createCaptureCustody({ admitted: f.admitted, readIntent: options.readIntent ?? f.readIntent,
        now: options.now ?? f.now });
    const server = createComputerUseServer({ profile: 'core', activeProfile: 'core', ...custody,
        session: /** @type {any} */ ({ dispatch: options.dispatch }) });
    const client = new Client({ name: 'capture-custody-boundary-test', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    try { await server.connect(local); await client.connect(remote); }
    catch (error) { await client.close(); await server.close(); throw error; }
    return { client, close: async () => { await client.close(); await server.close(); } };
}
