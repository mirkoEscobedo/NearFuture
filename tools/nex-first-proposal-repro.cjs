'use strict';
// Trusted host-call API only: no CLI or wire-supplied filesystem path.
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const repo = path.resolve(__dirname, '..');
const root = path.join(repo, '.tmp', 'nex-first-proposal-reproductions-v1');
const header = Buffer.from('NF-NEX-FIRST-PROPOSAL-1\0', 'ascii');
function directory(directoryPath) {
    try { fs.mkdirSync(directoryPath); } catch (error) { if (error.code !== 'EEXIST') throw error; }
    const stat = fs.lstatSync(directoryPath);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('OWNED_DIRECTORY_REQUIRED');
}
function persistSyntheticReproduction(bytes, review) {
    // Observation is not a privacy classification. The caller must explicitly review public synthetic input.
    if (!review || Object.keys(review).length !== 1 || review.reviewedPublicSynthetic !== true) {
        throw new Error('HOST_PUBLIC_SYNTHETIC_REVIEW_REQUIRED');
    }
    if (!Buffer.isBuffer(bytes) || bytes.length > 65536 || bytes.length < header.length + 3) {
        throw new Error('BOUNDED_COMPOSITE_BYTES_REQUIRED');
    }
    const owned = Buffer.from(bytes);
    if (!owned.subarray(0, header.length).equals(header) || owned.readUInt16LE(header.length) !== 1
            || owned[header.length + 2] !== 1) throw new Error('SYNTHETIC_COMPOSITE_REQUIRED');
    // Check complete closed selector framing and EOF; raw metadata remains the no_std core's responsibility.
    let offset = header.length + 3;
    for (const selector of [1, 2, 3]) {
        if (offset + 5 > owned.length || owned[offset] !== selector) throw new Error('COMPOSITE_FRAMING_REQUIRED');
        const length = owned.readUInt32LE(offset + 1); offset += 5;
        if (length === 0 || length > owned.length - offset) throw new Error('COMPOSITE_FRAMING_REQUIRED');
        offset += length;
    }
    if (offset !== owned.length) throw new Error('COMPOSITE_EOF_REQUIRED');
    const repoStat = fs.lstatSync(repo);
    if (!repoStat.isDirectory() || repoStat.isSymbolicLink()) throw new Error('OWNED_REPOSITORY_REQUIRED');
    directory(path.join(repo, '.tmp')); directory(root);
    const digest = crypto.createHash('sha256').update(owned).digest('hex');
    const destination = path.join(root, digest + '.bin');
    const temporary = path.join(root, digest + '.' + crypto.randomUUID() + '.pending');
    const fd = fs.openSync(temporary, 'wx', 0o600);
    try { fs.writeFileSync(fd, owned); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
    // Hard-link publication is atomic and cannot overwrite an existing reproduction.
    try { fs.linkSync(temporary, destination); }
    catch (error) {
        if (error.code !== 'EEXIST') throw error; // Retain owned partial for diagnosis on unexpected failure.
        const stat = fs.lstatSync(destination);
        if (!stat.isFile() || stat.isSymbolicLink() || stat.size !== owned.length || !fs.readFileSync(destination).equals(owned)) throw error;
    }
    fs.unlinkSync(temporary); // Only this call's generated owned temporary is disposed after publication.
    return Object.freeze({ sha256: digest, byteLength: owned.length });
}
module.exports = { persistSyntheticReproduction };
