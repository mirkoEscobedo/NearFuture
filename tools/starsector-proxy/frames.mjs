// @ts-check
import { PNG_MAX_BYTES, pngDimensions } from './png.mjs';

/** @param {unknown} value @returns {Record<string, any>} */
function object(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Expected an OpenCode object');
    return /** @type {Record<string, any>} */ (value);
}

/** Admit direct typed native calls; execute wrappers remain observation-only.
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
    const direct = [];
    const nativeCalls = new Set(['doctor', 'policy_status', 'get_frontmost_app', 'list_windows',
        'get_window', 'get_display_size', 'screenshot', 'wait', 'activate_window', 'left_click', 'mouse_move', 'key']
        .map(name => 'computer_use_' + name));
    for (const part of row.content) {
        if (part.type === 'image' || part.type === 'file') {
            throw new Error('Unqualified CLI2 image provenance');
        }
        if (part.type !== 'tool') continue;
        const state = object(part.state);
        if (nativeCalls.has(part.name)) {
            direct.push(part);
            continue;
        }
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
    return direct;
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
        const legacy = Array.isArray(message.parts);
        for (const part of messageParts(message)) {
            if (part.type !== 'tool' || part.state?.status !== 'completed') continue;
            const currentStream = legacy && typeof part.partID === 'string' && part.partID.length > 0;
            if ((part.tool !== undefined && part.name !== undefined && part.tool !== part.name)
                || ((!legacy || currentStream) && part.callID !== undefined && part.id !== undefined && part.callID !== part.id)) {
                throw new Error('Conflicting tool provenance');
            }
            const state = object(part.state);
            const tool = part.tool ?? part.name;
            const time = state.time ?? part.time;
            if (tool === 'computer_use_get_window') {
                const output = state.output ?? (state.content?.length === 1 && state.content[0].type === 'text'
                    ? state.content[0].text : undefined);
                if (typeof output !== 'string') throw new Error('Expected exported window JSON text');
                const window = object(JSON.parse(output));
                if (window.windowId !== state.input?.window_id) throw new Error('Window identity mismatch');
                windows.set(window.windowId, window);
            }
            if (/^computer_use_(left_click|mouse_move|scroll|type|key|activate_window)$/.test(tool)) {
                actions.push({ tool, input: state.input, time });
            }
            if (tool !== 'computer_use_screenshot') continue;
            const callID = legacy && !currentStream ? part.callID : part.id;
            if ((!legacy || currentStream || part.id !== undefined) && (typeof callID !== 'string' || !callID)) throw new Error('Screenshot lacks a call ID');
            const input = object(state.input);
            const window = windows.get(input.target_window_id);
            if (!window || !/^(java|starsector)(\.exe)?$/i.test(window.bundleId)
                || !/^Starsector\b/i.test(window.title)) throw new Error('Screenshot lacks verified Starsector identity');
            if (input.quality !== 0 || input.width !== window.bounds?.width) throw new Error('Expected native-width lossless capture');
            const contents = [state.content, state.metadata?.content].filter(Array.isArray)
                .map(content => content.filter(item => item.type === 'image' || item.type === 'file'))
                .filter(content => content.length > 0);
            if (contents.length > 1 || (state.attachments !== undefined && contents.length > 0)) {
                throw new Error('Ambiguous screenshot content');
            }
            const images = state.attachments ?? contents[0]?.map(item => item.type === 'file'
                ? { mime: item.mime, url: item.uri }
                : { mime: item.mimeType, url: 'data:' + item.mimeType + ';base64,' + item.data });
            if (!Array.isArray(images) || images.length !== 1) throw new Error('Expected exactly one original screenshot attachment');
            const image = object(images[0]);
            const match = typeof image.url === 'string' && /^data:image\/png;base64,([A-Za-z0-9+/]+={0,2})$/.exec(image.url);
            if (image.mime !== 'image/png' || !match) throw new Error('Expected an inline PNG attachment');
            if (match[1].length > 4 * Math.ceil(PNG_MAX_BYTES / 3)) throw new Error('Invalid original PNG');
            const bytes = Buffer.from(match[1], 'base64');
            if (bytes.toString('base64') !== match[1]) throw new Error('Invalid original PNG');
            const dimensions = pngDimensions(bytes);
            if (dimensions.width !== window.bounds.width || dimensions.height !== window.bounds.height) {
                throw new Error('Screenshot dimensions do not match the verified window');
            }
            frames.push({ bytes, metadata: {
                sessionID: data.info?.id, callID, tool,
                input, time, window, dimensions,
                precedingActions: [...actions], freshness: 'unknown',
            } });
        }
    }
    return frames;
}
