// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { crc32, deflateSync } from 'node:zlib';
import { screenshotFrames } from './frames.mjs';

// One greyscale/alpha pixel; complete IHDR/IDAT/IEND, valid CRCs and zlib data.
const pixel = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=', 'base64');
/** @param {Buffer} bytes @param {number} [width] @param {number} [height] */
function transcript(bytes, width = 1, height = 1) {
    const window = { windowId: 42, bundleId: 'java.exe', title: 'Starsector', bounds: { width, height } };
    return { messages: [{ parts: [
        { type: 'tool', tool: 'computer_use_get_window', state: { status: 'completed', input: { window_id: 42 }, output: JSON.stringify(window) } },
        { type: 'tool', tool: 'computer_use_screenshot', state: { status: 'completed', input: { target_window_id: 42, quality: 0, width }, attachments: [{ mime: 'image/png', url: 'data:image/png;base64,' + bytes.toString('base64') }] } },
    ] }] };
}

/** Synthetic input encoder, independent of the parser under test.
 * @param {string} type @param {Buffer} data
 */
function chunk(type, data) {
    const chunk = Buffer.alloc(data.length + 12);
    chunk.writeUInt32BE(data.length);
    chunk.write(type, 4);
    data.copy(chunk, 8);
    chunk.writeUInt32BE(crc32(chunk.subarray(4, -4)), chunk.length - 4);
    return chunk;
}

/** @param {Buffer} data */
function imageData(data) {
    return Buffer.concat([pixel.subarray(0, 33), chunk('IDAT', data), pixel.subarray(-12)]);
}

test('no truncation after the PNG dimensions can become an original frame', () => {
    assert.deepEqual(screenshotFrames(transcript(pixel))[0].bytes, pixel);
    for (const length of [24, 32, 33, 40, pixel.length - 12, pixel.length - 1]) {
        assert.throws(() => screenshotFrames(transcript(pixel.subarray(0, length))), /Invalid original PNG/, `cut at ${length}`);
    }
});

test('a checksum-corrupted PNG cannot become an original frame', () => {
    for (const offset of [32, 45, pixel.length - 1]) {
        const corrupt = Buffer.from(pixel);
        corrupt[offset] ^= 1;
        assert.throws(() => screenshotFrames(transcript(corrupt)), /Invalid original PNG/);
    }
});

test('valid chunk checksums do not admit truncated zlib data or invalid scanlines', () => {
    for (const data of [
        pixel.subarray(41, 51), // Original zlib stream lacks its last checksum byte.
        Buffer.concat([pixel.subarray(41, 52), Buffer.from([0])]), // Trailing compressed-stream bytes.
        deflateSync(Buffer.from([5, 0, 255])), // Filter type 5 does not exist.
        deflateSync(Buffer.from([0, 0])), // One grey/alpha pixel needs two samples.
    ]) assert.throws(() => screenshotFrames(transcript(imageData(data))), /Invalid original PNG/);
});

test('valid Adam7 scanlines and contiguous empty IDAT chunks retain the exact original', () => {
    const header = Buffer.from(pixel.subarray(16, 29));
    header.writeUInt32BE(2, 0);
    header.writeUInt32BE(2, 4);
    header[12] = 1;
    // Two-by-two grey/alpha: Adam7 passes 1, 6 and 7 contain 3, 3 and 5 bytes.
    const rows = Buffer.from([0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 255]);
    const image = Buffer.concat([pixel.subarray(0, 8), chunk('IHDR', header),
        chunk('IDAT', Buffer.alloc(0)), chunk('IDAT', deflateSync(rows)),
        chunk('IDAT', Buffer.alloc(0)), pixel.subarray(-12)]);
    assert.deepEqual(screenshotFrames(transcript(image, 2, 2))[0].bytes, image);
});

test('unknown critical chunks and nonconsecutive image-data chunks are refused', () => {
    for (const middle of [
        chunk('ABCD', Buffer.alloc(0)),
        Buffer.concat([chunk('IDAT', Buffer.alloc(0)), chunk('tEXt', Buffer.from('key\0value'))]),
    ]) {
        const image = Buffer.concat([pixel.subarray(0, 33), middle, pixel.subarray(33)]);
        assert.throws(() => screenshotFrames(transcript(image)), /Invalid original PNG/);
    }
});

test('indexed image admission requires its palette before image data', () => {
    const header = Buffer.from(pixel.subarray(16, 29));
    header[9] = 3;
    const prefix = Buffer.concat([pixel.subarray(0, 8), chunk('IHDR', header)]);
    const suffix = Buffer.concat([chunk('IDAT', deflateSync(Buffer.from([0, 0]))), pixel.subarray(-12)]);
    assert.throws(() => screenshotFrames(transcript(Buffer.concat([prefix, suffix]))), /Invalid original PNG/);
    const valid = Buffer.concat([prefix, chunk('PLTE', Buffer.from([0, 0, 0])), suffix]);
    assert.deepEqual(screenshotFrames(transcript(valid))[0].bytes, valid);
});
