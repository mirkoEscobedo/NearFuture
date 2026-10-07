'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {fixture} = require('./fixture.cjs');
const {hash,bytes,verifyDigest} = require('./fields.cjs');
const {PROTOCOL,header,handshake,subscribe,notice,proof,subscribedPrefix,noticePrefix,ackPrefix,limits} = require('./profiles.cjs');
test('617-byte handshake binds exact ASCII notification protocol without terminating NUL',()=>{
  const c=fixture(),h=handshake(c,1,c.membership);
  assert.equal(h.length,617);assert.equal(h.subarray(0,17).toString(),'NF-NOTIFY-AUTH-1\0');
  assert.equal(h.readUInt16LE(18),1);assert.equal(h[20],3);
  assert.deepEqual(h.subarray(21,53),hash(Buffer.from(PROTOCOL,'ascii')));
  assert.equal(header(c,1,bytes(16)).length,128);assert.equal(limits(c.selected).length,14);
});
test('244-byte subscription binds selector lifetime and exact signed response prefix',()=>{
  const c=fixture(),p=subscribedPrefix(c),first=subscribe(c,1),reply=subscribe(c,2,hash(p));
  assert.equal(first.length,244);assert.equal(first.readBigUInt64LE(194),12n);
  assert.equal(first.readBigUInt64LE(202),11n);assert.equal(first.readUInt16LE(210),10);
  assert.equal(p.length,219);assert.deepEqual(reply.subarray(-32),hash(p));
  assert.notDeepEqual(hash(first),hash(reply));
});
test('212-byte Notice and Ack purposes bind exact original notice and admitted byte',()=>{
  const c=fixture(),n=noticePrefix(c),a=ackPrefix(c),t=notice(c,2,hash(a));
  assert.equal(n.length,248);assert.equal(a.length,185);assert.equal(t.length,212);
  assert.deepEqual(a.subarray(152,184),hash(n));assert.equal(a[184],1);
  assert.deepEqual(t.subarray(180),hash(a));assert.equal(t.readBigUInt64LE(68),1n);
  assert.notDeepEqual(hash(t),hash(notice(c,1,hash(n))));
});
test('all wire proofs retain maintained canonical DeviceProof signatures',()=>{
  const c=fixture(),p=proof(c,'server',hash(handshake(c,1,c.membership)));
  assert.equal(p.bytes.length,297);assert.equal(p.canonical.length,163);
  assert.equal(verifyDigest(p.digest,p.signature,p.public_key),true);
  const changed=Buffer.from(p.digest);changed[0]^=1;
  assert.equal(verifyDigest(changed,p.signature,p.public_key),false);
});
