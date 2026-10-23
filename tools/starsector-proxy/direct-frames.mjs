// @ts-check
import { writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { crc32, inflateSync } from 'node:zlib';

/** Validate a complete native lossless PNG without changing a byte.
 * @param {Buffer} bytes */
export function completePng(bytes) {
    if (bytes.length < 57 || bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw new Error('Complete original PNG required');
    let offset = 8, header, ended = false, idatEnded = false, palette = false;
    /** @type {Buffer[]} */ const compressed = [];
    while (offset < bytes.length) {
        if (offset + 12 > bytes.length) throw new Error('Truncated original PNG chunk');
        const length = bytes.readUInt32BE(offset), end = offset + 12 + length;
        if (end > bytes.length) throw new Error('Truncated original PNG chunk');
        const type = bytes.toString('ascii', offset + 4, offset + 8), data = bytes.subarray(offset + 8, end - 4);
        if (!/^[A-Za-z]{4}$/.test(type) || crc32(bytes.subarray(offset + 4, end - 4)) !== bytes.readUInt32BE(end - 4)) throw new Error('Original PNG chunk CRC mismatch');
        if (!header && type !== 'IHDR') throw new Error('Original PNG must start with IHDR');
        if (type === 'IHDR') { if (header || length !== 13) throw new Error('Single complete PNG IHDR required'); header = data; }
        else if (type === 'PLTE') { if (compressed.length || palette || length === 0 || length > 768 || length % 3) throw new Error('Invalid original PNG palette'); palette = true; }
        else if (type === 'IDAT') { if (idatEnded) throw new Error('Contiguous PNG image data required'); compressed.push(data); }
        else if (type === 'IEND') { if (length || !compressed.length || end !== bytes.length) throw new Error('Complete terminal PNG IEND required'); ended = true; }
        else if (type[0] === type[0].toUpperCase()) throw new Error('Unsupported critical PNG chunk');
        if (compressed.length && type !== 'IDAT') idatEnded = true;
        offset = end;
    }
    if (!header || !ended) throw new Error('Complete original PNG image data and IEND required');
    const width = header.readUInt32BE(0), height = header.readUInt32BE(4), depth = header[8], color = header[9];
    const channels = /** @type {Record<number, number>} */ ({ 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 })[color];
    const allowed = color === 0 ? [1, 2, 4, 8, 16] : color === 3 ? [1, 2, 4, 8] : [8, 16];
    if (!width || !height || !channels || !allowed.includes(depth) || header[10] || header[11] || header[12] || (color === 3 && !palette)) throw new Error('Supported native noninterlaced PNG header required');
    const stride = Math.ceil(width * channels * depth / 8) + 1, size = stride * height;
    if (!Number.isSafeInteger(size) || size > 128 * 1024 * 1024) throw new Error('Bounded original PNG image data required');
    const decoded = inflateSync(Buffer.concat(compressed), { maxOutputLength: size });
    if (decoded.length !== size) throw new Error('Complete PNG scanlines required');
    for (let at = 0; at < size; at += stride) if (decoded[at] > 4) throw new Error('Valid original PNG scanline filter required');
    return { width, height };
}

/** Preserve the complete MCP result BEFORE inspecting its original image. The
 * caller provides a current owned window receipt; receipt is not frame freshness.
 * @param {{output: string, label: string, pngName?: string, result: any, args: any, window: any,
 * ownedPids: number[], provenance: Record<string, unknown>}} options */
export function persistDirectFrame({ output, label, pngName, result, args, window, ownedPids, provenance }) {
    if (!/^capture-\d{3}$/.test(label)) throw new Error('Closed capture artifact label required');
    if (pngName !== undefined && (typeof pngName !== 'string' || !/^F[1-9][0-9]{0,2}\.original\.png$/.test(pngName))) throw new Error('Declared original frame basename required');
    const rawPath = resolve(output, label + '.raw.private.json');
    const raw = JSON.stringify(result) + '\n';
    if (Buffer.byteLength(raw) > 32 * 1024 * 1024) throw new Error('Bounded original MCP result required');
    writeFileSync(rawPath, raw, { flag: 'wx' });
    const metadataPath = resolve(output, label + '.metadata.private.json');
    if (result?.isError === true) {
        writeFileSync(metadataPath, JSON.stringify({ ...provenance, args, isError: true, captureReceipt: false,
            freshness: 'UNKNOWN', rawPath }) + '\n', { flag: 'wx' });
        return { rawPath, metadataPath };
    }
    const images = result?.content?.filter((/** @type {any} */ item) => item.type === 'image');
    if (images?.length !== 1 || images[0].mimeType !== 'image/png' ||
        !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(images[0].data ?? '')) throw new Error('One exact original PNG required');
    const bytes = Buffer.from(images[0].data, 'base64');
    if (bytes.toString('base64') !== images[0].data || bytes.length < 24 || bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a' ||
        bytes.toString('ascii', 12, 16) !== 'IHDR') throw new Error('Original PNG signature and IHDR required');
    const dimensions = completePng(bytes);
    if (!window || !ownedPids.includes(window.pid) || window.windowId !== args.target_window_id || args.quality !== 0 ||
        args.width !== window.bounds?.width || dimensions.width !== window.bounds.width || dimensions.height !== window.bounds.height)
        throw new Error('Original PNG dimensions must match the current owned window');
    const pngPath = resolve(output, pngName ?? label + '.png');
    writeFileSync(pngPath, bytes, { flag: 'wx' });
    writeFileSync(metadataPath, JSON.stringify({ ...provenance, args, window, dimensions, rawPath, pngPath,
        pngSha256: createHash('sha256').update(bytes).digest('hex'), rawSha256: createHash('sha256').update(raw).digest('hex'),
        captureReceipt: true, freshness: 'UNKNOWN' }) + '\n', { flag: 'wx' });
    return { rawPath, pngPath, metadataPath };
}
