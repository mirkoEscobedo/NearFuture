// @ts-check
import { crc32, inflateSync } from 'node:zlib';

// This is attachment admission, not native-capture or frame-freshness certification.
export const PNG_MAX_BYTES = 64 * 1024 * 1024;

/** PNG scanline sizes, including Adam7's seven reduced images.
 * https://www.w3.org/TR/png-3/#8Interlace
 * @param {Buffer} header @returns {Array<{stride: number, rows: number}>}
 */
function scanlines(header) {
    const width = header.readUInt32BE(0), height = header.readUInt32BE(4);
    const depth = header[8], color = header[9];
    const channels = [1, 0, 3, 1, 2, 0, 4][color];
    const depths = color === 0 ? [1, 2, 4, 8, 16] : color === 3 ? [1, 2, 4, 8] : [8, 16];
    if (!width || !height || width > 0x7fffffff || height > 0x7fffffff || !channels ||
        !depths.includes(depth) || header[10] !== 0 || header[11] !== 0 || header[12] > 1) {
        throw new Error('Invalid original PNG');
    }
    const passes = header[12] === 0 ? [[0, 0, 1, 1]] : [
        [0, 0, 8, 8], [4, 0, 8, 8], [0, 4, 4, 8], [2, 0, 4, 4],
        [0, 2, 2, 4], [1, 0, 2, 2], [0, 1, 1, 2],
    ];
    const layout = [];
    let total = 0;
    for (const [x, y, dx, dy] of passes) {
        const columns = Math.max(0, Math.ceil((width - x) / dx));
        const rows = Math.max(0, Math.ceil((height - y) / dy));
        if (!columns || !rows) continue;
        const stride = 1 + Math.ceil(columns * channels * depth / 8);
        total += stride * rows;
        if (!Number.isSafeInteger(total) || total > PNG_MAX_BYTES) throw new Error('Invalid original PNG');
        layout.push({ stride, rows });
    }
    return layout;
}

/** @param {Buffer[]} parts @param {Array<{stride: number, rows: number}>} layout */
function verifyImage(parts, layout) {
    const expected = layout.reduce((sum, row) => sum + row.stride * row.rows, 0);
    const compressed = Buffer.concat(parts);
    try {
        const result = /** @type {{buffer: Buffer, engine: {bytesWritten: number}}} */ (
            /** @type {unknown} */ (inflateSync(compressed, { maxOutputLength: expected, info: true })));
        if (result.buffer.length !== expected || result.engine.bytesWritten !== compressed.length) {
            throw new Error('Invalid original PNG');
        }
        let offset = 0;
        for (const { stride, rows } of layout) {
            for (let row = 0; row < rows; row++, offset += stride) {
                if (result.buffer[offset] > 4) throw new Error('Invalid original PNG');
            }
        }
    } catch {
        throw new Error('Invalid original PNG');
    }
}

/** @param {Buffer} bytes @returns {{width: number, height: number}} */
export function pngDimensions(bytes) {
    if (bytes.length > PNG_MAX_BYTES || bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') {
        throw new Error('Invalid original PNG');
    }
    let offset = 8;
    let imageData = 0;
    let chunks = 0;
    let palette = false;
    let dataEnded = false;
    /** @type {Buffer[]} */
    const parts = [];
    /** @type {Array<{stride: number, rows: number}>|undefined} */
    let layout;
    let dimensions;
    while (offset < bytes.length) {
        if (bytes.length - offset < 12 || ++chunks > 16_384) throw new Error('Invalid original PNG');
        const length = bytes.readUInt32BE(offset);
        const end = offset + length + 12;
        if (end > bytes.length || length > 0x7fffffff) throw new Error('Invalid original PNG');
        if (crc32(bytes.subarray(offset + 4, end - 4)) !== bytes.readUInt32BE(end - 4)) {
            throw new Error('Invalid original PNG');
        }
        const type = bytes.toString('latin1', offset + 4, offset + 8);
        if (!/^[A-Za-z]{2}[A-Z][A-Za-z]$/.test(type) ||
            (/^[A-Z]/.test(type) && !['IHDR', 'PLTE', 'IDAT', 'IEND'].includes(type))) {
            throw new Error('Invalid original PNG');
        }
        if (offset === 8) {
            if (type !== 'IHDR' || length !== 13) throw new Error('Invalid original PNG');
            dimensions = { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
            layout = scanlines(bytes.subarray(16, 29));
        } else if (type === 'IHDR') throw new Error('Invalid original PNG');
        if (type === 'PLTE') {
            if (palette || parts.length || ![2, 3, 6].includes(bytes[25]) ||
                !length || length % 3 || length > 768 || (bytes[25] === 3 && length / 3 > 2 ** bytes[24])) {
                throw new Error('Invalid original PNG');
            }
            palette = true;
        }
        if (type === 'IDAT') {
            if (dataEnded || (bytes[25] === 3 && !palette)) throw new Error('Invalid original PNG');
            imageData += length;
            parts.push(bytes.subarray(offset + 8, end - 4));
        } else if (parts.length) dataEnded = true;
        if (type === 'IEND') {
            if (length || end !== bytes.length || !imageData || !dimensions || !layout) throw new Error('Invalid original PNG');
            verifyImage(parts, layout);
            return dimensions;
        }
        offset = end;
    }
    throw new Error('Invalid original PNG');
}
