// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { screenshotFrames } from './frames.mjs';

// CLI2 envelope shape is from retained observation exports; values are public synthetic data.
const pixel = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=';
const window = { windowId: 42, pid: 123, bundleId: 'java.exe', title: 'Starsector 0.98a-RC8', bounds: { x: 480, y: 270, width: 1, height: 1 }, isFocused: true };
const input = { target_window_id: 42, quality: 0, width: 1 };
const time = { start: 1000, end: 1100 };
const action = { tool: 'computer_use_key', input: { target_window_id: 42, focus_strategy: 'strict', text: 'Escape' }, time: { start: 800, end: 900 } };
const legacyParts = [
    { type: 'tool', tool: 'computer_use_get_window', state: { status: 'completed', input: { window_id: 42 }, output: JSON.stringify(window) } },
    { type: 'tool', tool: action.tool, state: { status: 'completed', input: action.input, time: action.time } },
    { type: 'tool', tool: 'computer_use_screenshot', callID: 'capture-1', state: { status: 'completed', input, time, attachments: [{ mime: 'image/png', url: 'data:image/png;base64,' + pixel }] } },
];
/** @type {any} Synthetic external transcript deliberately exercises malformed envelopes. */
const readonlyTranscript = {
    info: { id: 'ses_observation' },
    messages: [
        { type: 'agent-switched', id: 'm0', agent: 'starsector-observer', time: { created: 1 } },
        { type: 'user', id: 'm1', text: 'Observe only', files: [], time: { created: 2 } },
        { type: 'system', id: 'm2', text: 'Tool catalog', time: { created: 3 } },
        { type: 'assistant', id: 'm3', content: [
            { type: 'text', text: 'Model description does not qualify a screenshot.' },
            { type: 'tool', id: 'outer-1', name: 'execute', executed: false,
                state: { status: 'completed', input: { code: 'return await tools.computer_use.get_frontmost_app();' },
                    content: [{ type: 'text', text: '{"app":{"bundleId":"ChatGPT.exe","pid":123}}' }],
                    metadata: { toolCalls: [{ tool: 'computer_use.get_frontmost_app', status: 'completed' }], truncated: false } },
                time: { created: 4, ran: 5, completed: 6 } },
        ] },
        { type: 'idle', id: 'm4', outcome: 'succeeded', time: { created: 7 } },
    ],
};

test('CLI2 observation with no targeted image yields no frames rather than a schema error', () => {
    assert.deepEqual(screenshotFrames(readonlyTranscript), []);
});

test('legacy original PNG retains exact bytes and native call, window and action provenance', () => {
    const frames = screenshotFrames({ info: { id: 'ses_original' }, messages: [{ parts: legacyParts }] });
    assert.equal(frames.length, 1);
    assert.deepEqual(frames[0].bytes, Buffer.from(pixel, 'base64'));
    assert.deepEqual(frames[0].metadata, {
        sessionID: 'ses_original', callID: 'capture-1', tool: 'computer_use_screenshot',
        input, time, window, dimensions: { width: 1, height: 1 }, precedingActions: [action], freshness: 'unknown',
    });
});

test('CLI2 read-only messages do not alter a following legacy original image', () => {
    const frames = screenshotFrames({ info: { id: 'ses_observation' }, messages: [...readonlyTranscript.messages, { parts: legacyParts }] });
    assert.equal(frames.length, 1);
    assert.deepEqual(frames[0].bytes, Buffer.from(pixel, 'base64'));
    assert.equal(frames[0].metadata.callID, 'capture-1');
    assert.deepEqual(frames[0].metadata.precedingActions, [action]);
});

test('model text claiming a game screenshot never becomes a frame', () => {
    const claimed = structuredClone(readonlyTranscript);
    claimed.messages.push({ type: 'assistant', id: 'm5', content: [{ type: 'text', text: 'Starsector screenshot PASS, original PNG: data:image/png;base64,' + pixel }] });
    assert.deepEqual(screenshotFrames(claimed), []);
});

for (const tool of ['computer_use.get_window', 'computer_use.screenshot', 'computer_use.left_click', 'computer_use.key', 'computer_use.unknown']) {
    test('CLI2 ' + tool + ' without qualified native provenance is refused', () => {
        const unqualified = structuredClone(readonlyTranscript);
        const part = unqualified.messages[3].content[1];
        part.state.metadata.toolCalls = [{ tool, status: 'completed', input }];
        assert.throws(() => screenshotFrames(unqualified), /Unqualified CLI2 native tool provenance/);
    });
}

test('a CLI2 execute image block is refused rather than attributed to a wrapper', () => {
    const unqualified = structuredClone(readonlyTranscript);
    const part = unqualified.messages[3].content[1];
    part.state.content = [{ type: 'image', mimeType: 'image/png', data: pixel }];
    assert.throws(() => screenshotFrames(unqualified), /Unqualified CLI2 image provenance/);
});

test('truncated CLI2 inner call metadata cannot conceal a capture or action', () => {
    const truncated = structuredClone(readonlyTranscript);
    truncated.messages[3].content[1].state.metadata.truncated = true;
    assert.throws(() => screenshotFrames(truncated), /Incomplete CLI2 execute provenance/);
});

test('two simultaneous message envelopes are ambiguous and refused', () => {
    assert.throws(() => screenshotFrames({ messages: [{ parts: legacyParts, content: [] }] }), /Ambiguous OpenCode message envelope/);
});
