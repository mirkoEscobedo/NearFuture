'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { persistSyntheticReproduction } = require('./nex-first-proposal-repro.cjs');
const root = path.resolve(__dirname, '../.tmp/nex-first-proposal-reproductions-v1');
function literal() {
    // Host framing contract only; actual canonical Rust identities are qualified by the Rust producer.
    const header = Buffer.from('NF-NEX-FIRST-PROPOSAL-1\0', 'ascii');
    const marker = Buffer.from([1, 0, 1]);
    return Buffer.concat([header, marker, ...[1,2,3].map(selector => {
        const frame = Buffer.alloc(6); frame[0] = selector; frame.writeUInt32LE(1, 1); frame[5] = selector;
        return frame;
    })]);
}
test('fixed-root atomic publication is immutable and idempotent', () => {
    const bytes = literal(), before = Buffer.from(bytes);
    const expected = crypto.createHash('sha256').update(bytes).digest('hex');
    assert.deepEqual(persistSyntheticReproduction(bytes, { reviewedPublicSynthetic: true }),
        { sha256: expected, byteLength: bytes.length });
    const destination = path.join(root, expected + '.bin');
    assert.deepEqual(fs.readFileSync(destination), bytes);
    const original = fs.statSync(destination);
    assert.deepEqual(persistSyntheticReproduction(bytes, { reviewedPublicSynthetic: true }),
        { sha256: expected, byteLength: bytes.length });
    assert.equal(fs.statSync(destination).ino, original.ino);
    assert.deepEqual(bytes, before);
});
test('captured bytes and missing host review are refused before publication', () => {
    const bytes = literal();
    assert.throws(() => persistSyntheticReproduction(bytes, {}), /HOST_PUBLIC_SYNTHETIC_REVIEW_REQUIRED/);
    const captured = Buffer.from(bytes); captured[Buffer.byteLength('NF-NEX-FIRST-PROPOSAL-1\0') + 2] = 2;
    assert.throws(() => persistSyntheticReproduction(captured, { reviewedPublicSynthetic: true }), /SYNTHETIC_COMPOSITE_REQUIRED/);
    assert.throws(() => persistSyntheticReproduction(bytes, { reviewedPublicSynthetic: true, path: '../save' }),
        /HOST_PUBLIC_SYNTHETIC_REVIEW_REQUIRED/);
});
test('byte cap, complete framing and EOF are enforced', () => {
    assert.throws(() => persistSyntheticReproduction(Buffer.alloc(65537), { reviewedPublicSynthetic: true }), /BOUNDED_COMPOSITE_BYTES_REQUIRED/);
    const truncated = literal().subarray(0, -1);
    assert.throws(() => persistSyntheticReproduction(truncated, { reviewedPublicSynthetic: true }), /COMPOSITE_FRAMING_REQUIRED/);
    assert.throws(() => persistSyntheticReproduction(Buffer.concat([literal(), Buffer.from([0])]), { reviewedPublicSynthetic: true }), /COMPOSITE_EOF_REQUIRED/);
});
