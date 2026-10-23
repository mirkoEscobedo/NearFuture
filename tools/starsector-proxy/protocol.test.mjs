// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { screenshotFrames } from './frames.mjs';

// Independent synthetic PNG, plus fields observed in CLI2.0.32's safe session.
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=';
const window = { windowId: 42, pid: 123, bundleId: 'java.exe', title: 'Starsector 0.98a-RC8', bounds: { x: 0, y: 0, width: 1, height: 1 } };
const capture = /** @param {string} id */ id => ({
    type: 'tool', id, name: 'computer_use_screenshot', executed: true,
    time: { start: 1000, end: 1100 },
    state: { status: 'completed', input: { target_window_id: 42, quality: 0, width: 1 },
        content: [{ type: 'image', mimeType: 'image/png', data: png }, { type: 'text', text: '1x1' }] },
});

test('current CLI typed messages preserve distinct capture IDs and original provenance', () => {
    const session = { info: { id: 'ses_current' }, messages: [
        { type: 'agent-switched', agent: 'starsector-observer' },
        { type: 'user', text: 'Observe only.' },
        { type: 'assistant', content: [
            { type: 'tool', id: 'window-call', name: 'computer_use_get_window', executed: true,
                state: { status: 'completed', input: { window_id: 42 }, content: [{ type: 'text', text: JSON.stringify(window) }] } },
            capture('capture-one'), capture('capture-two'),
        ] },
        { type: 'idle' },
    ] };
    const frames = screenshotFrames(session);
    assert.equal(frames.length, 2);
    assert.deepEqual(frames.map(frame => frame.metadata.callID), ['capture-one', 'capture-two']);
    assert.deepEqual(frames[0].metadata.time, { start: 1000, end: 1100 });
    assert.deepEqual(frames[0].bytes, Buffer.from(png, 'base64'));
    assert.deepEqual(frames[0].metadata.window, window);
    assert.equal(frames[0].metadata.freshness, 'unknown');
});

test('capture custody refuses conflicting tool names or call IDs', () => {
    for (const conflict of [{ callID: 'different-call' }, { tool: 'computer_use_left_click' }]) {
        const frame = { ...capture('capture-one'), ...conflict };
        const identified = { type: 'tool', id: 'window-call', name: 'computer_use_get_window',
            state: { status: 'completed', input: { window_id: 42 }, content: [{ type: 'text', text: JSON.stringify(window) }] } };
        assert.throws(() => screenshotFrames({ messages: [{ type: 'assistant', content: [identified, frame] }] }), /Conflicting tool provenance/);
    }
});

test('stream metadata retains original MCP PNG and preceding authorized pointer movement', () => {
    const move = { type: 'tool', id: 'move-call', tool: 'computer_use_mouse_move',
        state: { status: 'completed', input: { target_window_id: 42, focus_strategy: 'strict', coordinate: [0, 0] }, time: { start: 800, end: 900 } } };
    const original = capture('stream-capture');
    const frame = { type: 'tool', partID: 'prt_stream', id: original.id, tool: original.name,
        state: { status: 'completed', input: original.state.input,
            metadata: { content: original.state.content }, time: { start: 1000, end: 1100 } } };
    const frames = screenshotFrames({ info: { id: 'ses_stream' }, messages: [{ parts: [
        { type: 'tool', tool: 'computer_use_get_window', state: { status: 'completed', input: { window_id: 42 }, output: JSON.stringify(window) } },
        move, frame,
    ] }] });
    assert.equal(frames[0].metadata.callID, 'stream-capture');
    assert.deepEqual(frames[0].bytes, Buffer.from(png, 'base64'));
    assert.deepEqual(frames[0].metadata.precedingActions, [{ tool: 'computer_use_mouse_move', input: move.state.input, time: move.state.time }]);
});

test('current capture refuses absent IDs and ambiguous image payloads', () => {
    const identified = { type: 'tool', id: 'window-call', name: 'computer_use_get_window',
        state: { status: 'completed', input: { window_id: 42 }, content: [{ type: 'text', text: JSON.stringify(window) }] } };
    const absent = { ...capture(''), id: '' };
    assert.throws(() => screenshotFrames({ messages: [{ type: 'assistant', content: [identified, absent] }] }), /lacks a call ID/);
    const repeated = capture('repeated-images');
    repeated.state.content.push(repeated.state.content[0]);
    assert.throws(() => screenshotFrames({ messages: [{ type: 'assistant', content: [identified, repeated] }] }), /exactly one original/);
});

test('retained legacy internal part IDs remain distinct from tool-call IDs', () => {
    const frames = screenshotFrames({ info: { id: 'ses_legacy' }, messages: [{ parts: [
        { type: 'tool', tool: 'computer_use_get_window', state: { status: 'completed', input: { window_id: 42 }, output: JSON.stringify(window) } },
        { type: 'tool', id: 'prt_internal', callID: 'legacy-tool-call', tool: 'computer_use_screenshot',
            state: { status: 'completed', input: { target_window_id: 42, quality: 0, width: 1 }, time: { start: 1, end: 2 },
                attachments: [{ mime: 'image/png', url: 'data:image/png;base64,' + png }] } },
    ] }] });
    assert.equal(frames[0].metadata.callID, 'legacy-tool-call');
    assert.deepEqual(frames[0].bytes, Buffer.from(png, 'base64'));
});

test('legacy missing callID cannot substitute an internal part ID', () => {
    const session = { messages: [{ parts: [
        { type: 'tool', tool: 'computer_use_get_window', state: { status: 'completed', input: { window_id: 42 }, output: JSON.stringify(window) } },
        { type: 'tool', id: 'prt_internal', tool: 'computer_use_screenshot',
            state: { status: 'completed', input: { target_window_id: 42, quality: 0, width: 1 },
                attachments: [{ mime: 'image/png', url: 'data:image/png;base64,' + png }] } },
    ] }] };
    assert.throws(() => screenshotFrames(session), /lacks a call ID/);
});

test('actual CLI inline file content preserves original PNG and refuses external URIs', () => {
    const identified = { type: 'tool', id: 'window-call', name: 'computer_use_get_window',
        state: { status: 'completed', input: { window_id: 42 }, content: [{ type: 'text', text: JSON.stringify(window) }] } };
    for (const uri of ['data:image/png;base64,' + png, 'file:///private/image.png', 'https://example.invalid/image.png']) {
        const frame = capture('file-capture');
        const fileFrame = { ...frame, state: { ...frame.state, content: [{ type: 'file', mime: 'image/png', uri }] } };
        const session = { messages: [{ type: 'assistant', content: [identified, fileFrame] }] };
        if (uri.startsWith('data:')) assert.deepEqual(screenshotFrames(session)[0].bytes, Buffer.from(png, 'base64'));
        else assert.throws(() => screenshotFrames(session), /inline PNG/);
    }
});
