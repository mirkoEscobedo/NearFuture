// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash, randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { sha256Ascii } from './capture-ascii-sha256.mjs';
test('ASCII SHA256 matches published empty and abc vectors and Node crypto across padding boundaries and capacity', () => {
    assert.equal(sha256Ascii(''), 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
    assert.equal(sha256Ascii('abc'), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
    for (const n of [1, 55, 56, 63, 64, 65, 127, 128, 129, 4096, 4194304]) {
        const text = randomBytes(Math.ceil(n * 3 / 4)).toString('base64').slice(0,n);
        assert.equal(sha256Ascii(text), createHash('sha256').update(text, 'ascii').digest('hex'), String(n));
    }
});
test('hash helper runs unchanged without Node, TextEncoder or WebCrypto', () => {
    const source = readFileSync(new URL('./capture-ascii-sha256.mjs',import.meta.url),'utf8').replace('export function','function');
    const context = vm.createContext({});
    assert.equal(vm.runInContext(source+'; sha256Ascii("abc")',context), sha256Ascii('abc'));
});
test('reject non ASCII and above original capacity', () => {
    assert.throws(()=>sha256Ascii('é'), /BOUNDED_ASCII/);
    assert.throws(()=>sha256Ascii('a'.repeat(4194305)), /BOUNDED_ASCII/);
});
