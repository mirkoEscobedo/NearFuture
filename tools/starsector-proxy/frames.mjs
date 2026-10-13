// @ts-check

/** @param {unknown} value @returns {Record<string, any>} */
function object(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Expected an OpenCode object');
    return /** @type {Record<string, any>} */ (value);
}

/** Accept only the CLI2 observational envelope demonstrated by retained exports.
 * Wrapped native capture/input results have no qualified per-call PNG provenance yet.
 * @param {unknown} message @returns {any[]}
 */
function messageParts(message) {
    const row = object(message);
    if (Array.isArray(row.parts)) {
        if (Object.hasOwn(row, 'content')) throw new Error('Ambiguous OpenCode message envelope');
        return row.parts;
    }
    if (row.type !== 'assistant') {
        if (['agent-switched', 'user', 'system', 'idle'].includes(row.type)) return [];
        throw new Error('Missing exported message parts');
    }
    if (!Array.isArray(row.content)) throw new Error('Missing CLI2 assistant content');
    const readonlyCalls = new Set([
        'computer_use.doctor', 'computer_use.policy_status',
        'computer_use.get_frontmost_app', 'computer_use.list_windows', 'computer_use.wait',
    ]);
    for (const part of row.content) {
        if (part.type === 'image' || part.type === 'file') {
            throw new Error('Unqualified CLI2 image provenance');
        }
        if (part.type !== 'tool') continue;
        const state = object(part.state);
        if (Array.isArray(state.content) && state.content.some(item => item.type === 'image' || item.type === 'file')) {
            throw new Error('Unqualified CLI2 image provenance');
        }
        if (part.name !== 'execute') {
            if (typeof part.name === 'string' && /^computer_use[._]/.test(part.name)) {
                throw new Error('Unqualified CLI2 native tool provenance');
            }
            continue;
        }
        const metadata = object(state.metadata);
        if (metadata.truncated !== false || !Array.isArray(metadata.toolCalls)) {
            throw new Error('Incomplete CLI2 execute provenance');
        }
        for (const call of metadata.toolCalls) {
            if (typeof call.tool !== 'string') throw new Error('Incomplete CLI2 execute provenance');
            if (!/^computer_use[._]/.test(call.tool)) continue;
            if (!readonlyCalls.has(call.tool)) throw new Error('Unqualified CLI2 native tool provenance');
        }
    }
    // Text, wrapper times and model narratives never certify a targeted image.
    return [];
}

/** @param {unknown} session @returns {Array<{bytes: Buffer, metadata: Record<string, unknown>}>} */
export function screenshotFrames(session) {
    const data = object(session);
    if (!Array.isArray(data.messages)) throw new Error('Expected an OpenCode session export');
    /** @type {Map<number, Record<string, any>>} */
    const windows = new Map();
    /** @type {Array<{tool: string, input: unknown, time: unknown}>} */
    const actions = [];
    /** @type {Array<{bytes: Buffer, metadata: Record<string, unknown>}>} */
    const frames = [];
    for (const message of data.messages) {
        for (const part of messageParts(message)) {
            if (part.type !== 'tool' || part.state?.status !== 'completed') continue;
            const state = object(part.state);
            if (part.tool === 'computer_use_get_window') {
                const window = object(JSON.parse(state.output));
                if (window.windowId !== state.input?.window_id) throw new Error('Window identity mismatch');
                windows.set(window.windowId, window);
            }
            if (/^computer_use_(left_click|scroll|type|key|activate_window)$/.test(part.tool)) {
                actions.push({ tool: part.tool, input: state.input, time: state.time });
            }
            if (part.tool !== 'computer_use_screenshot') continue;
            const input = object(state.input);
            const window = windows.get(input.target_window_id);
            if (!window || !/^(java|starsector)(\.exe)?$/i.test(window.bundleId)
                || !/^Starsector\b/i.test(window.title)) throw new Error('Screenshot lacks verified Starsector identity');
            if (input.quality !== 0 || input.width !== window.bounds?.width) throw new Error('Expected native-width lossless capture');
            const images = state.attachments;
            if (!Array.isArray(images) || images.length !== 1) throw new Error('Expected exactly one original screenshot attachment');
            const image = object(images[0]);
            const match = typeof image.url === 'string' && /^data:image\/png;base64,([A-Za-z0-9+/]+={0,2})$/.exec(image.url);
            if (image.mime !== 'image/png' || !match) throw new Error('Expected an inline PNG attachment');
            const bytes = Buffer.from(match[1], 'base64');
            if (bytes.toString('base64') !== match[1] || bytes.length < 24
                || bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a'
                || bytes.toString('ascii', 12, 16) !== 'IHDR') throw new Error('Invalid original PNG');
            const dimensions = { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
            if (dimensions.width !== window.bounds.width || dimensions.height !== window.bounds.height) {
                throw new Error('Screenshot dimensions do not match the verified window');
            }
            frames.push({ bytes, metadata: {
                sessionID: data.info?.id, callID: part.callID, tool: part.tool,
                input, time: state.time, window, dimensions,
                precedingActions: [...actions], freshness: 'unknown',
            } });
        }
    }
    return frames;
}
