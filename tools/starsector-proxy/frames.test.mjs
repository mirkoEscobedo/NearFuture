// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { screenshotFrames } from './frames.mjs';

const pixel = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=';
const window = { windowId: 42, pid: 123, bundleId: 'java.exe', title: 'Starsector 0.98a-RC8', bounds: { x: 480, y: 270, width: 1, height: 1 }, isFocused: false };
const session = {
    info: { id: 'ses_observation' },
    messages: [{ parts: [
        { type: 'tool', tool: 'computer_use_doctor', state: { status: 'completed', attachments: [{ mime: 'image/png', url: 'data:image/png;base64,ignored' }] } },
        { type: 'tool', tool: 'computer_use_get_window', state: { status: 'completed', input: { window_id: 42 }, output: JSON.stringify(window) } },
        { type: 'tool', tool: 'computer_use_screenshot', callID: 'capture-1', state: { status: 'completed', input: { target_window_id: 42, quality: 0, width: 1 }, output: '1x1', time: { start: 1000, end: 1100 }, attachments: [{ mime: 'image/png', url: 'data:image/png;base64,' + pixel }] } },
    ] }],
};

test('exports only the original targeted PNG with its verified game identity and uncertain freshness', () => {
    const frames = screenshotFrames(session);
    assert.equal(frames.length, 1);
    assert.deepEqual(frames[0].bytes, Buffer.from(pixel, 'base64'));
    assert.equal(frames[0].metadata.callID, 'capture-1');
    assert.equal(frames[0].metadata.sessionID, 'ses_observation');
    assert.equal(frames[0].metadata.freshness, 'unknown');
    assert.deepEqual(frames[0].metadata.window, window);
    assert.deepEqual(frames[0].metadata.dimensions, { width: 1, height: 1 });
});
test('rejects an image aimed at a window that was not identified as Starsector', () => {
    const wrongTarget = structuredClone(session);
    wrongTarget.messages[0].parts[2].state.input = { target_window_id: 99, quality: 0, width: 1 };
    assert.throws(() => screenshotFrames(wrongTarget), /verified Starsector identity/);
});

test('rejects a truncated image rather than reporting a usable capture', () => {
    const truncated = structuredClone(session);
    truncated.messages[0].parts[2].state.attachments = [{ mime: 'image/png', url: 'data:image/png;base64,AAAA' }];
    assert.throws(() => screenshotFrames(truncated), /Invalid original PNG/);
});
